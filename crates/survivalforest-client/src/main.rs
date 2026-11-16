//! SurvivalForest: cliente del juego.

// Los sistemas de Bevy reciben muchos parámetros por diseño.
#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod drops_view;
mod hud;
mod icons;
mod interact;
mod menu;
mod player;
mod render;
mod screens;
mod session;
mod settings;
mod sky;
mod streaming;
mod survival;
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
    .init_resource::<interact::OpenRequest>()
    .init_resource::<interact::UiOpen>()
    .init_resource::<screens::ScreenState>()
    .init_resource::<screens::EscConsumed>()
    .init_resource::<screens::RespawnClicked>()
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
            drops_view::despawn_drops,
            screens::reset_screens,
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
        (
            player::physics_step,
            survival::survival_step,
            sky::advance_time,
        )
            .chain()
            .run_if(in_state(AppState::InGame)),
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
            interact::drop_held,
            interact::draw_outline,
            drops_view::sync_drops,
            hud::update_hud,
            screens::open_screens,
            screens::build_screen,
            screens::slot_clicks,
            screens::refresh_screen,
            screens::respawn_button,
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
        session::toggle_pause
            .after(screens::open_screens)
            .run_if(|s: Res<State<AppState>>| {
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
            .add_systems(Update, (shot_setup, smoke_test));
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

/// Depuración: `--shot-screen <hud|inventario|mesa|horno|baul|amoladora|muerte|creativo>`
/// abre esa pantalla con objetos de ejemplo poco antes de la captura de la prueba de humo.
#[allow(clippy::too_many_arguments)]
fn shot_setup(
    time: Res<Time>,
    smoke: Res<Smoke>,
    state: Res<State<AppState>>,
    st: Option<ResMut<survival::PlayerState>>,
    ex: Option<ResMut<survival::WorldExtras>>,
    ss: Option<ResMut<screens::ScreenState>>,
    mut done: Local<bool>,
) {
    let Some(name) = arg_after("--shot-screen") else {
        return;
    };
    let (Some(mut st), Some(mut ex), Some(mut ss)) = (st, ex, ss) else {
        return;
    };
    if *done || *state.get() != AppState::InGame || time.elapsed_secs() < smoke.0 - 4.0 {
        return;
    }
    *done = true;
    use survivalforest_core::item::item_by_key;
    use survivalforest_sim::containers::BlockEntity;
    use survivalforest_sim::inventory::ItemStack;
    let s = |k: &str, n: u16| Some(ItemStack::new(item_by_key(k).expect("objeto"), n));
    st.inv.slots.0[0] = Some(ItemStack {
        wear: 40,
        ..s("pick_stone", 1).expect("pico")
    });
    st.inv.slots.0[1] = s("axe_wood", 1);
    st.inv.slots.0[2] = s("machete_iron", 1);
    st.inv.slots.0[3] = s("cobble", 48);
    st.inv.slots.0[4] = s("platano", 5);
    st.inv.slots.0[5] = s("torch", 16);
    st.inv.slots.0[10] = s("iron_ingot", 7);
    st.inv.slots.0[11] = s("coal", 23);
    st.inv.slots.0[12] = s("mora", 9);
    st.inv.slots.0[20] = s("planks", 64);
    st.vitals.health = 13.0;
    st.vitals.hunger = 15.0;
    st.vitals.air = 9.0;
    st.xp.add(40.0);
    let p = survivalforest_core::BlockPos::new(0, 0, 0);
    ss.screen = match name.as_str() {
        "inventario" => {
            ss.grid[0] = s("log", 2);
            screens::GameScreen::Inventory
        }
        "mesa" => {
            for (i, k) in [
                (0, "planks"),
                (2, "planks"),
                (3, "planks"),
                (4, "stick"),
                (5, "planks"),
                (7, "stick"),
            ] {
                ss.grid[i] = s(k, 1);
            }
            screens::GameScreen::Table(p)
        }
        "horno" => {
            ex.be.on_placed(p, survivalforest_core::block::FURNACE);
            if let Some(BlockEntity::Furnace(f)) = ex.be.0.get_mut(&p) {
                f.input = s("iron_ore_item", 6);
                f.fuel = s("coal", 3);
                f.output = s("iron_ingot", 2);
                f.burn_left = 30.0;
                f.burn_total = 64.0;
                f.progress = 5.0;
            }
            screens::GameScreen::Furnace(p)
        }
        "baul" => {
            ex.be.on_placed(p, survivalforest_core::block::CHEST);
            if let Some(BlockEntity::Chest(c)) = ex.be.0.get_mut(&p) {
                c.slots.0[0] = s("guayaba", 12);
                c.slots.0[4] = s("gold_ingot", 3);
                c.slots.0[13] = s("bricks", 40);
            }
            screens::GameScreen::Chest(p)
        }
        "amoladora" => {
            ss.grind = Some(ItemStack {
                wear: 100,
                ..s("pick_iron", 1).expect("pico")
            });
            screens::GameScreen::Grindstone(p)
        }
        "muerte" => {
            st.vitals.health = 0.0;
            st.dead = true;
            st.death_pos = Some(bevy::math::DVec3::new(120.0, 71.0, -35.0));
            screens::GameScreen::Death
        }
        "creativo" => screens::GameScreen::CreativeList,
        _ => screens::GameScreen::None,
    };
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
