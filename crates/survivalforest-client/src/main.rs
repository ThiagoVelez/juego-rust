//! SurvivalForest: cliente del juego.

// Los sistemas de Bevy reciben muchos parámetros por diseño.
#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod hud;
mod interact;
mod menu;
mod player;
mod render;
mod session;
mod settings;
mod sky;
mod streaming;
mod textures;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use render::{BlockMaterial, setup_materials};
use session::{AppState, PendingWorld};
use settings::Settings;
use streaming::{
    FloatingOrigin, RemeshRequest, Streaming, ViewDistance, World, rebase_origin, stream_chunks,
};
use survivalforest_core::GameMode;

/// `--smoke <segundos>`: crea un mundo temporal, entra, espera, guarda `smoke.png` y sale.
#[derive(Resource)]
struct Smoke(f32);

fn arg_after(name: &str) -> Option<String> {
    std::env::args().skip_while(|a| a != name).nth(1)
}

fn main() {
    let smoke = std::env::args().any(|a| a == "--smoke").then(|| {
        arg_after("--smoke")
            .and_then(|s| s.parse().ok())
            .unwrap_or(20.0)
    });
    let settings = Settings::load();
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
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .add_plugins(MaterialPlugin::<BlockMaterial>::default())
    .add_plugins(MaterialPlugin::<sky::SkyMaterial>::default())
    .init_state::<AppState>()
    .insert_resource(ClearColor(render::HORIZON))
    .insert_resource(ViewDistance(if smoke.is_some() {
        8
    } else {
        settings.view_distance
    }))
    .insert_resource(settings)
    .insert_resource(Time::<Fixed>::from_hz(20.0))
    .init_resource::<FloatingOrigin>()
    .init_resource::<Streaming>()
    .init_resource::<sky::GameTime>()
    .init_resource::<player::MoveInput>()
    .init_resource::<interact::Hotbar>()
    .init_resource::<interact::Target>()
    .init_resource::<menu::Screen>()
    .init_resource::<menu::UiAction>()
    .init_resource::<menu::CreateForm>()
    .init_resource::<hud::ShowDebug>()
    .add_message::<RemeshRequest>()
    .add_systems(Startup, (setup_materials, sky::setup_sky, hud::setup_icons))
    // Estados
    .add_systems(
        OnEnter(AppState::Menu),
        (
            session::leave_world.run_if(resource_exists::<World>),
            session::spawn_menu_camera,
            menu::on_enter_menu,
        )
            .chain(),
    )
    .add_systems(
        OnEnter(AppState::Loading),
        (session::enter_world, menu::on_enter_loading),
    )
    .add_systems(
        OnEnter(AppState::InGame),
        (menu::on_enter_game, hud::spawn_hud),
    )
    .add_systems(OnExit(AppState::InGame), hud::despawn_hud)
    .add_systems(OnEnter(AppState::Paused), menu::on_enter_pause)
    // Simulación (solo jugando)
    .add_systems(
        FixedUpdate,
        (player::physics_step, sky::advance_time).run_if(in_state(AppState::InGame)),
    )
    // Menús
    .add_systems(
        Update,
        (
            menu::type_into_form,
            menu::handle_actions,
            menu::rebuild_menu,
            menu::hover_colors,
            menu::apply_fov,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (menu::update_loading_text, session::finish_loading).run_if(in_state(AppState::Loading)),
    )
    // Mundo cargado (cargando, jugando o en pausa)
    .add_systems(
        Update,
        (player::place_camera, rebase_origin, stream_chunks)
            .chain()
            .run_if(resource_exists::<World>)
            .after(menu::handle_actions),
    )
    // Jugando
    .add_systems(
        Update,
        (
            player::grab_cursor,
            player::read_input,
            interact::select_slot,
            interact::aim,
            interact::edit_blocks,
            interact::draw_outline,
            hud::update_slots,
            hud::update_debug,
            sky::toggle_fast,
            session::autosave,
        )
            .chain()
            .before(player::place_camera)
            .run_if(in_state(AppState::InGame)),
    )
    .add_systems(
        Update,
        session::toggle_pause.run_if(|s: Res<State<AppState>>| {
            matches!(s.get(), AppState::InGame | AppState::Paused)
        }),
    )
    .add_systems(Update, sky::apply_time)
    .add_systems(Last, session::save_on_exit);
    // Fuente con tildes y eñes (Nunito, licencia OFL) en lugar de la de Bevy.
    app.world_mut()
        .resource_mut::<Assets<bevy::text::Font>>()
        .insert(
            AssetId::default(),
            bevy::text::Font::from_bytes(
                include_bytes!("../../../assets/fonts/Nunito.ttf").to_vec(),
            ),
        )
        .expect("fuente");
    if let Some(s) = smoke {
        app.insert_resource(Smoke(s))
            .add_systems(Startup, smoke_start)
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

/// Prueba de arranque: mundo nuevo en una carpeta temporal (semilla fija).
fn smoke_start(mut commands: Commands, mut next: ResMut<NextState<AppState>>) {
    let root = std::env::temp_dir().join(format!("survivalforest-smoke-{}", std::process::id()));
    let mode = if std::env::args().any(|a| a == "--creative") {
        GameMode::Creative
    } else {
        GameMode::Survival
    };
    let dir = survivalforest_world::save::create_world_in(&root, "Prueba", 20_261_112, mode)
        .expect("mundo de prueba");
    if let Some(t) = arg_after("--time").and_then(|t| t.parse().ok()) {
        let mut m = survivalforest_world::save::load_meta(&dir).expect("meta");
        m.time_ticks = t;
        survivalforest_world::save::save_meta(&dir, &m).expect("meta");
    }
    commands.insert_resource(PendingWorld(dir));
    next.set(AppState::Loading);
}

fn smoke_test(
    mut commands: Commands,
    time: Res<Time>,
    smoke: Res<Smoke>,
    st: Res<Streaming>,
    state: Res<State<AppState>>,
    mut taken: Local<bool>,
    mut wait: Local<f32>,
    mut exit: MessageWriter<AppExit>,
) {
    let t = time.elapsed_secs();
    if !*taken && t >= smoke.0 {
        info!(
            "Prueba: estado {:?}, {} chunks mallados",
            state.get(),
            st.meshed
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("smoke.png"));
        *taken = true;
        *wait = t;
    }
    if *taken && t > *wait + 3.0 {
        let ok = st.meshed > 0 && *state.get() == AppState::InGame;
        exit.write(if ok {
            AppExit::Success
        } else {
            AppExit::error()
        });
    }
}
