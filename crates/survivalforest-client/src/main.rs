//! SurvivalForest: cliente del juego.

// Los sistemas de Bevy reciben muchos parámetros por diseño.
#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]
// Temporal mientras se completan las tareas de la etapa 1 (se quita en la tarea 13).
#![allow(dead_code)]

mod interact;
mod player;
mod render;
mod streaming;
mod textures;

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use render::{BlockMaterial, setup_materials};
use std::sync::Arc;
use streaming::{
    FloatingOrigin, Focus, RemeshRequest, Streaming, ViewDistance, World, rebase_origin,
    stream_chunks,
};
use survivalforest_core::GameMode;
use survivalforest_world::{RegionStore, WorldGen, WorldMeta};

/// `--smoke <segundos>`: arranca, espera, guarda `smoke.png` y sale.
#[derive(Resource)]
struct Smoke(f32);

/// Sensibilidad del ratón (1 = normal).
#[derive(Resource)]
pub struct Sensitivity(pub f32);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let smoke = args
        .iter()
        .position(|a| a == "--smoke")
        .map(|i| args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(20.0));
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "SurvivalForest".into(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: assets_dir(),
                ..default()
            }),
    )
    .add_plugins(MaterialPlugin::<BlockMaterial>::default())
    .insert_resource(ClearColor(render::HORIZON))
    .insert_resource(ViewDistance(if smoke.is_some() { 8 } else { 12 }))
    .init_resource::<FloatingOrigin>()
    .init_resource::<Streaming>()
    .add_message::<RemeshRequest>()
    .insert_resource(Time::<Fixed>::from_hz(20.0))
    .insert_resource(Sensitivity(1.0))
    .init_resource::<player::MoveInput>()
    .init_resource::<interact::Hotbar>()
    .init_resource::<interact::Target>()
    .add_systems(Startup, (setup_materials, setup_world))
    .add_systems(FixedUpdate, player::physics_step)
    .add_systems(
        Update,
        (
            player::grab_cursor,
            player::read_input,
            player::place_camera,
            interact::select_slot,
            interact::aim,
            interact::edit_blocks,
            interact::draw_outline,
            rebase_origin,
            stream_chunks,
        )
            .chain(),
    );
    if let Some(s) = smoke {
        app.insert_resource(Smoke(s))
            .add_systems(Update, smoke_test);
    }
    app.run();
}

/// Carpeta `assets`: junto al ejecutable o, en desarrollo, en la raíz del repositorio.
fn assets_dir() -> String {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("assets")));
    if let Some(d) = exe_dir.filter(|d| d.exists()) {
        return d.to_string_lossy().into_owned();
    }
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").into()
}

fn setup_world(mut commands: Commands, smoke: Option<Res<Smoke>>) {
    // Provisional (tarea 8): mundo temporal de semilla fija; los menús llegan en la tarea 12.
    let dir = std::env::temp_dir().join("survivalforest-dev");
    let meta = WorldMeta {
        version: 1,
        name: "Desarrollo".into(),
        seed: 20_261_112,
        mode: GameMode::Creative,
        time_ticks: 6000,
        created: "0".into(),
    };
    let gen_ = Arc::new(WorldGen::new(meta.seed));
    let h = gen_
        .surface_height(0, 0)
        .max(survivalforest_world::worldgen::SEA_LEVEL);
    let (feet, pitch, flying) = if smoke.is_some() {
        (DVec3::new(0.5, h as f64 + 30.0, 0.5), -0.3, true)
    } else {
        (DVec3::new(0.5, h as f64 + 1.0, 0.5), -0.1, false)
    };
    commands.insert_resource(Focus(feet));
    commands.insert_resource(player::Mode(meta.mode));
    player::spawn_player(&mut commands, feet, -0.6, pitch, flying, 75.0);
    commands.insert_resource(World {
        gen_,
        columns: Default::default(),
        store: RegionStore::open(&dir).expect("carpeta del mundo"),
        meta,
        dir,
    });
}

fn smoke_test(
    mut commands: Commands,
    time: Res<Time>,
    smoke: Res<Smoke>,
    st: Res<Streaming>,
    mut taken: Local<bool>,
    mut wait: Local<f32>,
    mut exit: MessageWriter<AppExit>,
) {
    let t = time.elapsed_secs();
    if !*taken && t >= smoke.0 {
        info!("Prueba: {} chunks mallados", st.meshed);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("smoke.png"));
        *taken = true;
        *wait = t;
    }
    if *taken && t > *wait + 3.0 {
        exit.write(if st.meshed > 0 {
            AppExit::Success
        } else {
            AppExit::error()
        });
    }
}
