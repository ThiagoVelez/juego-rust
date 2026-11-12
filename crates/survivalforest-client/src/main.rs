//! SurvivalForest: cliente del juego.

// Los sistemas de Bevy reciben muchos parámetros por diseño.
#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]
// Temporal mientras se completan las tareas de la etapa 1 (se quita en la tarea 13).
#![allow(dead_code)]

mod render;
mod streaming;
mod textures;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{CursorGrabMode, CursorOptions};
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
    .add_systems(Startup, (setup_materials, setup_world, setup_camera))
    .add_systems(Update, (fly_camera, rebase_origin, stream_chunks).chain());
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

fn setup_world(mut commands: Commands) {
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
    commands.insert_resource(Focus(DVec3::new(0.5, h as f64 + 30.0, 0.5)));
    commands.insert_resource(World {
        gen_,
        columns: Default::default(),
        store: RegionStore::open(&dir).expect("carpeta del mundo"),
        meta,
        dir,
    });
}

#[derive(Component)]
struct FlyCam {
    yaw: f32,
    pitch: f32,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 75f32.to_radians(),
            far: 1000.0,
            ..default()
        }),
        Transform::default(),
        FlyCam {
            yaw: -0.6,
            pitch: -0.25,
        },
    ));
}

fn fly_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut cursor: Single<&mut CursorOptions>,
    mut focus: ResMut<Focus>,
    origin: Res<FloatingOrigin>,
    mut cam: Single<(&mut Transform, &mut FlyCam)>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    let (ref mut tf, ref mut fc) = *cam;
    if cursor.grab_mode == CursorGrabMode::Locked {
        fc.yaw -= motion.delta.x * 0.003;
        fc.pitch = (fc.pitch - motion.delta.y * 0.003).clamp(-1.5, 1.5);
    }
    let rot = Quat::from_euler(EulerRot::YXZ, fc.yaw, fc.pitch, 0.0);
    let fwd = rot * Vec3::NEG_Z;
    let right = rot * Vec3::X;
    let mut d = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        d += fwd;
    }
    if keys.pressed(KeyCode::KeyS) {
        d -= fwd;
    }
    if keys.pressed(KeyCode::KeyD) {
        d += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        d -= right;
    }
    if keys.pressed(KeyCode::Space) {
        d += Vec3::Y;
    }
    if keys.pressed(KeyCode::ShiftLeft) {
        d -= Vec3::Y;
    }
    let speed = if keys.pressed(KeyCode::ControlLeft) {
        80.0
    } else {
        20.0
    };
    let step = d.normalize_or_zero() * speed * time.delta_secs();
    focus.0 += DVec3::new(step.x as f64, step.y as f64, step.z as f64);
    let o = origin.0.origin();
    tf.translation = Vec3::new(
        (focus.0.x - o.x as f64) as f32,
        focus.0.y as f32,
        (focus.0.z - o.z as f64) as f32,
    );
    tf.rotation = rot;
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
