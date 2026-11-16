//! Entrar y salir de un mundo: aparición, carga inicial y autoguardado.

use crate::player::{Mode, Player, WorldPos, spawn_player};
use crate::settings::Settings;
use crate::sky::GameTime;
use crate::streaming::{ChunkMesh, FloatingOrigin, Focus, Streaming, ViewDistance, World, spiral};
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use std::path::PathBuf;
use std::sync::Arc;
use survivalforest_core::block::{AIR, LEAVES, WATER};
use survivalforest_core::{BlockPos, ChunkPos, MIN_Y};
use survivalforest_sim::time::WorldTime;
use survivalforest_world::save::{PlayerSave, load_meta, load_player, save_meta, save_player};
use survivalforest_world::worldgen::SEA_LEVEL;
use survivalforest_world::{Biome, RegionStore, WorldGen};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    Loading,
    InGame,
    Paused,
}

/// Mundo que se va a abrir al entrar en `Loading`.
#[derive(Resource)]
pub struct PendingWorld(pub PathBuf);

/// Cámara de los menús (se quita al jugar).
#[derive(Component)]
pub struct MenuCamera;

/// ¿Se puede aparecer en este bioma? Se evitan el agua y los biomas sin árboles ni comida
/// (para empezar con las manos vacías en supervivencia).
pub fn spawn_biome_ok(b: Biome) -> bool {
    !matches!(
        b,
        Biome::Ocean
            | Biome::Coast
            | Biome::Mangrove
            | Biome::RedDesert
            | Biome::SnowPeak
            | Biome::Paramo
    )
}

/// Primer lugar seco cerca de (0, 0): bloque sólido debajo y dos de aire encima.
pub fn spawn_point(g: &WorldGen) -> BlockPos {
    for c in spiral(ChunkPos::new(0, 0), 64) {
        let o = c.origin();
        let (cx, cz) = (o.x + 8, o.z + 8);
        let h = g.surface_height(cx, cz);
        let b = g.biome_at(cx, cz);
        if h <= SEA_LEVEL + 1 || !spawn_biome_ok(b) {
            continue;
        }
        let col = g.generate(c);
        for lz in 2..14u8 {
            for lx in 2..14u8 {
                let top = col.height(lx, lz);
                if top <= SEA_LEVEL {
                    continue;
                }
                let at = |dy: i32| {
                    col.get(survivalforest_core::LocalPos::new(
                        lx,
                        (top + dy - MIN_Y) as u16,
                        lz,
                    ))
                };
                let ground = at(0);
                let (wx, wz) = (o.x + lx as i32, o.z + lz as i32);
                if !spawn_biome_ok(g.biome_at(wx, wz)) {
                    continue;
                }
                if ground.def().solid
                    && ground != LEAVES
                    && ground != WATER
                    && at(1) == AIR
                    && at(2) == AIR
                {
                    return BlockPos::new(o.x + lx as i32, top + 1, o.z + lz as i32);
                }
            }
        }
    }
    BlockPos::new(0, g.surface_height(0, 0).max(SEA_LEVEL) + 1, 0)
}

pub fn spawn_menu_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 80.0, 0.0).looking_at(Vec3::new(1.0, 80.3, -2.0), Vec3::Y),
        MenuCamera,
    ));
}

/// Abre el mundo pendiente: recursos, hora y jugador.
pub fn enter_world(
    mut commands: Commands,
    pending: Res<PendingWorld>,
    settings: Res<Settings>,
    mut st: ResMut<Streaming>,
    mut origin: ResMut<FloatingOrigin>,
    menu_cams: Query<Entity, With<MenuCamera>>,
) {
    for e in &menu_cams {
        commands.entity(e).despawn();
    }
    let dir = pending.0.clone();
    let meta = match load_meta(&dir) {
        Ok(m) => m,
        Err(e) => {
            error!("No se pudo abrir el mundo {}: {e}", dir.display());
            commands.set_state(AppState::Menu);
            return;
        }
    };
    let gen_ = Arc::new(WorldGen::new(meta.seed));
    let player = load_player(&dir).unwrap_or_else(|_| {
        let p = spawn_point(&gen_);
        PlayerSave {
            pos: [p.x as f64 + 0.5, p.y as f64, p.z as f64 + 0.5],
            yaw: 0.0,
            pitch: 0.0,
            flying: false,
        }
    });
    let feet = DVec3::from_array(player.pos);
    *st = Streaming::default();
    origin.0 = BlockPos::new(feet.x.floor() as i32, 0, feet.z.floor() as i32).chunk();
    commands.insert_resource(GameTime {
        time: WorldTime {
            ticks: meta.time_ticks,
        },
        fast: false,
    });
    commands.insert_resource(Mode(meta.mode));
    commands.insert_resource(Focus(feet));
    commands.insert_resource(ViewDistance(settings.view_distance));
    let flying = player.flying && meta.mode == survivalforest_core::GameMode::Creative;
    spawn_player(
        &mut commands,
        feet,
        player.yaw,
        player.pitch,
        flying,
        settings.fov,
    );
    // Supervivencia: inventario, vida y experiencia; baúles, hornos y objetos cercanos.
    let mut state =
        crate::survival::PlayerState::from_save(survivalforest_sim::persist::load_survival(&dir));
    if state.spawn.is_none() {
        let p = spawn_point(&gen_);
        state.spawn = Some([p.x, p.y, p.z]);
    }
    if meta.mode == survivalforest_core::GameMode::Creative {
        crate::interact::creative_start_bar(&mut state);
    }
    let mut extras = crate::survival::WorldExtras::default();
    crate::survival::load_nearby_regions(&dir, &mut extras, feet);
    commands.insert_resource(state);
    commands.insert_resource(extras);
    commands.insert_resource(World {
        gen_,
        columns: Default::default(),
        store: RegionStore::open(&dir).expect("carpeta del mundo"),
        meta,
        dir,
    });
}

/// Chunks listos alrededor del jugador (para la barra de carga).
pub fn loading_progress(st: &Streaming, focus: DVec3) -> (usize, usize) {
    let c = BlockPos::new(focus.x.floor() as i32, 0, focus.z.floor() as i32).chunk();
    let need = spiral(c, 2);
    (
        need.iter().filter(|p| st.is_meshed(**p)).count(),
        need.len(),
    )
}

pub fn finish_loading(
    st: Res<Streaming>,
    focus: Res<Focus>,
    mut next: ResMut<NextState<AppState>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    let (done, total) = loading_progress(&st, focus.0);
    if done == total {
        next.set(AppState::InGame);
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Guarda chunks, hora, jugador y lo de la supervivencia.
pub fn save_world(
    world: &mut World,
    player: Option<(&Player, &WorldPos)>,
    surv: Option<(&crate::survival::PlayerState, &crate::survival::WorldExtras)>,
) {
    world.save_all();
    if let Some((st, ex)) = surv {
        if let Err(e) = survivalforest_sim::persist::save_survival(&world.dir, &st.to_save()) {
            warn!("No se pudo guardar el inventario: {e}");
        }
        let mut regions = survivalforest_sim::persist::regions_in(&ex.be, &ex.items);
        regions.extend(ex.loaded.iter().copied());
        // Una región cuyo archivo no se pudo leer no se reescribe (se conserva el original).
        for r in regions.into_iter().filter(|r| !ex.failed.contains(r)) {
            if let Err(e) =
                survivalforest_sim::persist::save_extras(&world.dir, r, &ex.be, &ex.items)
            {
                warn!("No se pudieron guardar los baúles y objetos: {e}");
            }
        }
    }
    if let Err(e) = save_meta(&world.dir, &world.meta) {
        warn!("No se pudo guardar el mundo: {e}");
    }
    if let Some((p, wp)) = player {
        let s = PlayerSave {
            pos: wp.pos.to_array(),
            yaw: p.yaw,
            pitch: p.pitch,
            flying: p.flying,
        };
        if let Err(e) = save_player(&world.dir, &s) {
            warn!("No se pudo guardar el jugador: {e}");
        }
    }
}

pub fn autosave(
    time: Res<Time>,
    mut last: Local<f32>,
    mut world: ResMut<World>,
    st: Res<crate::survival::PlayerState>,
    ex: Res<crate::survival::WorldExtras>,
    ss: Res<crate::screens::ScreenState>,
    q: Single<(&Player, &WorldPos)>,
) {
    let now = time.elapsed_secs();
    if now - *last >= 60.0 {
        *last = now;
        let (p, wp) = *q;
        let (st, ex) = crate::screens::save_snapshot(&ss, &st, &ex, wp.pos);
        save_world(&mut world, Some((p, wp)), Some((&st, &ex)));
        info!("Mundo guardado");
    }
}

/// Al cerrar la ventana también se guarda.
pub fn save_on_exit(
    mut exits: MessageReader<AppExit>,
    world: Option<ResMut<World>>,
    st: Option<Res<crate::survival::PlayerState>>,
    ex: Option<Res<crate::survival::WorldExtras>>,
    ss: Option<Res<crate::screens::ScreenState>>,
    q: Query<(&Player, &WorldPos)>,
) {
    if exits.read().next().is_none() {
        return;
    }
    if let Some(mut w) = world {
        let player = q.iter().next();
        let feet = player.map(|(_, wp)| wp.pos).unwrap_or_default();
        let snap = match (st.as_deref(), ex.as_deref(), ss.as_deref()) {
            (Some(st), Some(ex), Some(ss)) => Some(crate::screens::save_snapshot(ss, st, ex, feet)),
            _ => None,
        };
        save_world(&mut w, player, snap.as_ref().map(|(a, b)| (a, b)));
    }
}

pub fn leave_world(
    mut commands: Commands,
    mut world: ResMut<World>,
    q: Query<(Entity, &Player, &WorldPos)>,
    chunks: Query<Entity, With<ChunkMesh>>,
    mut st: ResMut<Streaming>,
    ps: Option<Res<crate::survival::PlayerState>>,
    ex: Option<Res<crate::survival::WorldExtras>>,
) {
    let player = q.iter().next();
    save_world(
        &mut world,
        player.map(|(_, p, wp)| (p, wp)),
        ps.as_deref().zip(ex.as_deref()),
    );
    if let Some((e, _, _)) = player {
        commands.entity(e).despawn();
    }
    for e in &chunks {
        commands.entity(e).despawn();
    }
    *st = Streaming::default();
    commands.remove_resource::<World>();
    commands.remove_resource::<PendingWorld>();
}

/// Esc alterna entre jugar y pausa.
pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    ui: Res<crate::interact::UiOpen>,
    esc: Res<crate::screens::EscConsumed>,
    mut next: ResMut<NextState<AppState>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    // Con una pantalla abierta, Esc la cierra (no abre la pausa).
    if !keys.just_pressed(KeyCode::Escape) || ui.0 || esc.0 {
        return;
    }
    match state.get() {
        AppState::InGame => {
            next.set(AppState::Paused);
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
        AppState::Paused => {
            next.set(AppState::InGame);
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use survivalforest_core::block::AIR;

    #[test]
    fn spawn_point_avoids_barren_biomes() {
        for b in [
            Biome::RedDesert,
            Biome::SnowPeak,
            Biome::Paramo,
            Biome::Ocean,
            Biome::Coast,
            Biome::Mangrove,
        ] {
            assert!(!spawn_biome_ok(b), "{b:?}");
        }
        for b in [
            Biome::CoffeeMountain,
            Biome::Rainforest,
            Biome::Savanna,
            Biome::BambooValley,
            Biome::CloudForest,
        ] {
            assert!(spawn_biome_ok(b), "{b:?}");
        }
        for seed in 0..60u64 {
            let g = WorldGen::new(seed * 7919 + 3);
            let p = spawn_point(&g);
            assert!(spawn_biome_ok(g.biome_at(p.x, p.z)), "semilla {seed}");
        }
    }

    #[test]
    fn spawn_point_is_dry_land() {
        let g = WorldGen::new(42);
        let p = spawn_point(&g);
        let col = g.generate(p.chunk());
        let below = col.get(p.offset(0, -1, 0).local());
        assert!(
            below.def().solid && below != survivalforest_core::block::LEAVES,
            "{:?}",
            below.def().key
        );
        assert_eq!(col.get(p.local()), AIR);
        assert_eq!(col.get(p.offset(0, 1, 0).local()), AIR);
        assert!(p.y > survivalforest_world::worldgen::SEA_LEVEL);
    }
}
