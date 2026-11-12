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

/// Primer lugar seco cerca de (0, 0): bloque sólido debajo y dos de aire encima.
pub fn spawn_point(g: &WorldGen) -> BlockPos {
    for c in spiral(ChunkPos::new(0, 0), 64) {
        let o = c.origin();
        let (cx, cz) = (o.x + 8, o.z + 8);
        let h = g.surface_height(cx, cz);
        let b = g.biome_at(cx, cz);
        if h <= SEA_LEVEL + 1 || matches!(b, Biome::Ocean | Biome::Coast | Biome::Mangrove) {
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

/// Guarda chunks, hora y jugador.
pub fn save_world(world: &mut World, player: Option<(&Player, &WorldPos)>) {
    world.save_all();
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
    q: Single<(&Player, &WorldPos)>,
) {
    let now = time.elapsed_secs();
    if now - *last >= 60.0 {
        *last = now;
        let (p, wp) = *q;
        save_world(&mut world, Some((p, wp)));
        info!("Mundo guardado");
    }
}

/// Al cerrar la ventana también se guarda.
pub fn save_on_exit(
    mut exits: MessageReader<AppExit>,
    world: Option<ResMut<World>>,
    q: Query<(&Player, &WorldPos)>,
) {
    if exits.read().next().is_none() {
        return;
    }
    if let Some(mut w) = world {
        save_world(&mut w, q.iter().next());
    }
}

pub fn leave_world(
    mut commands: Commands,
    mut world: ResMut<World>,
    q: Query<(Entity, &Player, &WorldPos)>,
    chunks: Query<Entity, With<ChunkMesh>>,
    mut st: ResMut<Streaming>,
) {
    let player = q.iter().next();
    save_world(&mut world, player.map(|(_, p, wp)| (p, wp)));
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
    mut next: ResMut<NextState<AppState>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
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
