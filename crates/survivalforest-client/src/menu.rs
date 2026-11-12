//! Menús: título, mundos, crear mundo, ajustes, pausa y pantalla de carga.

use crate::session::{AppState, PendingWorld, loading_progress};
use crate::settings::{Settings, random_seed, seed_from_text};
use crate::streaming::{Focus, Streaming, ViewDistance};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button, observe};
use std::path::PathBuf;
use survivalforest_core::GameMode;
use survivalforest_world::save::{create_world, list_worlds, worlds_dir};

const CARD: Color = Color::srgba(0.09, 0.07, 0.06, 0.82);
const BTN: Color = Color::srgba(0.28, 0.2, 0.15, 0.95);
const BTN_HOVER: Color = Color::srgba(0.42, 0.3, 0.2, 1.0);
const BTN_DANGER: Color = Color::srgba(0.55, 0.18, 0.15, 0.95);
const TEXT: Color = Color::srgb(1.0, 0.97, 0.92);
const MUTED: Color = Color::srgb(0.85, 0.78, 0.7);

#[derive(Resource, Clone, PartialEq, Default)]
pub enum Screen {
    #[default]
    Title,
    Worlds,
    Create,
    Settings,
    ConfirmDelete(PathBuf, String),
    Pause,
    Loading,
    None,
}

#[derive(Clone)]
pub enum Action {
    SinglePlayer,
    OpenSettings,
    Quit,
    Back,
    Play(PathBuf),
    AskDelete(PathBuf, String),
    Delete(PathBuf),
    NewWorld,
    CreateWorld,
    ToggleMode,
    Focus(Field),
    Resume,
    SaveAndQuit,
    ViewDistance(i32),
    Fov(i32),
    Sensitivity(i32),
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Field {
    #[default]
    Name,
    Seed,
}

#[derive(Resource, Default)]
pub struct UiAction(pub Option<Action>);

#[derive(Resource)]
pub struct CreateForm {
    pub name: String,
    pub seed: String,
    pub mode: GameMode,
    pub focus: Field,
}

impl Default for CreateForm {
    fn default() -> Self {
        CreateForm {
            name: String::new(),
            seed: String::new(),
            mode: GameMode::Survival,
            focus: Field::Name,
        }
    }
}

#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct LoadingText;

fn txt(s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn button(label: impl Into<String>, action: Action, width: f32, danger: bool) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        Node {
            width: px(width),
            height: px(44),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BackgroundColor(if danger { BTN_DANGER } else { BTN }),
        observe(move |_: On<Activate>, mut ui: ResMut<UiAction>| ui.0 = Some(action.clone())),
        children![txt(label, 18.0, TEXT)],
    )
}

fn card(width: f32) -> Node {
    Node {
        width: px(width),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(10),
        padding: UiRect::all(px(22)),
        border_radius: BorderRadius::all(px(18)),
        ..default()
    }
}

pub fn hover_colors(mut q: Query<(&Hovered, &mut BackgroundColor), With<Button>>) {
    for (h, mut bg) in &mut q {
        let base = bg.0.alpha();
        if h.get() {
            bg.0 = BTN_HOVER;
        } else if bg.0 == BTN_HOVER {
            bg.0 = BTN.with_alpha(base.max(0.95));
        }
    }
}

pub fn on_enter_menu(mut screen: ResMut<Screen>) {
    *screen = Screen::Title;
}
pub fn on_enter_loading(mut screen: ResMut<Screen>) {
    *screen = Screen::Loading;
}
pub fn on_enter_game(mut screen: ResMut<Screen>) {
    *screen = Screen::None;
}
pub fn on_enter_pause(mut screen: ResMut<Screen>) {
    *screen = Screen::Pause;
}

/// Reconstruye el menú cuando cambia la pantalla o el formulario.
pub fn rebuild_menu(
    mut commands: Commands,
    screen: Res<Screen>,
    form: Res<CreateForm>,
    settings: Res<Settings>,
    roots: Query<Entity, With<MenuRoot>>,
) {
    if !screen.is_changed() && !form.is_changed() && !settings.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    if *screen == Screen::None {
        return;
    }
    let dim = !matches!(*screen, Screen::Pause);
    let root = commands
        .spawn((
            MenuRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.04, 0.03, if dim { 0.35 } else { 0.5 })),
        ))
        .id();
    let panel = match screen.as_ref() {
        Screen::Title => commands
            .spawn((card(420.0), BackgroundColor(CARD)))
            .with_children(|c| {
                c.spawn(txt("SurvivalForest", 46.0, TEXT));
                c.spawn(txt("Etapa 1 - mundo procedural", 15.0, MUTED));
                c.spawn(Node {
                    height: px(10),
                    ..default()
                });
                c.spawn(button("Un jugador", Action::SinglePlayer, 300.0, false));
                c.spawn(button("Ajustes", Action::OpenSettings, 300.0, false));
                c.spawn(button("Salir", Action::Quit, 300.0, false));
            })
            .id(),
        Screen::Worlds => {
            let worlds = list_worlds();
            commands
                .spawn((card(560.0), BackgroundColor(CARD)))
                .with_children(|c| {
                    c.spawn(txt("Tus mundos", 30.0, TEXT));
                    if worlds.is_empty() {
                        c.spawn(txt("Todavía no hay mundos. ¡Crea el primero!", 16.0, MUTED));
                    }
                    for (dir, meta) in worlds.iter().take(8) {
                        let mode = if meta.mode == GameMode::Creative {
                            "Creativo"
                        } else {
                            "Supervivencia"
                        };
                        c.spawn(Node {
                            column_gap: px(8),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|r| {
                            r.spawn((
                                Node {
                                    width: px(260),
                                    flex_direction: FlexDirection::Column,
                                    ..default()
                                },
                                children![
                                    txt(meta.name.clone(), 18.0, TEXT),
                                    txt(format!("{mode} - semilla {}", meta.seed), 13.0, MUTED)
                                ],
                            ));
                            r.spawn(button("Jugar", Action::Play(dir.clone()), 110.0, false));
                            r.spawn(button(
                                "Borrar",
                                Action::AskDelete(dir.clone(), meta.name.clone()),
                                110.0,
                                true,
                            ));
                        });
                    }
                    c.spawn(Node {
                        column_gap: px(10),
                        margin: UiRect::top(px(8)),
                        ..default()
                    })
                    .with_children(|r| {
                        r.spawn(button("Crear mundo", Action::NewWorld, 220.0, false));
                        r.spawn(button("Volver", Action::Back, 220.0, false));
                    });
                })
                .id()
        }
        Screen::Create => {
            let field = |label: &str, value: &str, f: Field, hint: &str| {
                let focused = form.focus == f;
                let shown = if value.is_empty() && !focused {
                    hint.to_string()
                } else {
                    format!("{value}{}", if focused { "|" } else { "" })
                };
                (
                    Button,
                    Hovered::default(),
                    Node {
                        width: px(380),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(px(12), px(8)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(10)),
                        ..default()
                    },
                    BorderColor::all(if focused {
                        Color::srgb(0.95, 0.75, 0.45)
                    } else {
                        Color::srgba(1.0, 1.0, 1.0, 0.2)
                    }),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                    observe(move |_: On<Activate>, mut ui: ResMut<UiAction>| {
                        ui.0 = Some(Action::Focus(f))
                    }),
                    children![
                        txt(label.to_string(), 13.0, MUTED),
                        txt(
                            shown,
                            18.0,
                            if value.is_empty() && !focused {
                                MUTED
                            } else {
                                TEXT
                            }
                        )
                    ],
                )
            };
            let mode = if form.mode == GameMode::Creative {
                "Modo: Creativo"
            } else {
                "Modo: Supervivencia"
            };
            let name_field = field("Nombre del mundo", &form.name, Field::Name, "Mi mundo");
            let seed_field = field(
                "Semilla (vacía = al azar; número o texto)",
                &form.seed,
                Field::Seed,
                "al azar",
            );
            commands
                .spawn((card(460.0), BackgroundColor(CARD)))
                .with_children(|c| {
                    c.spawn(txt("Crear mundo", 30.0, TEXT));
                    c.spawn(name_field);
                    c.spawn(seed_field);
                    c.spawn(button(mode, Action::ToggleMode, 380.0, false));
                    c.spawn(Node {
                        column_gap: px(10),
                        margin: UiRect::top(px(8)),
                        ..default()
                    })
                    .with_children(|r| {
                        r.spawn(button("Crear", Action::CreateWorld, 185.0, false));
                        r.spawn(button("Cancelar", Action::Back, 185.0, false));
                    });
                })
                .id()
        }
        Screen::Settings => {
            let s = settings.clone();
            let row = |label: String, minus: Action, plus: Action| {
                (
                    Node {
                        column_gap: px(10),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        button("−", minus, 52.0, false),
                        (
                            Node {
                                width: px(260),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            children![txt(label, 18.0, TEXT)]
                        ),
                        button("+", plus, 52.0, false)
                    ],
                )
            };
            commands
                .spawn((card(460.0), BackgroundColor(CARD)))
                .with_children(|c| {
                    c.spawn(txt("Ajustes", 30.0, TEXT));
                    c.spawn(row(
                        format!("Distancia de visión: {} chunks", s.view_distance),
                        Action::ViewDistance(-2),
                        Action::ViewDistance(2),
                    ));
                    c.spawn(row(
                        format!("Campo de visión: {}°", s.fov as i32),
                        Action::Fov(-5),
                        Action::Fov(5),
                    ));
                    c.spawn(row(
                        format!("Sensibilidad del ratón: {:.1}", s.sensitivity),
                        Action::Sensitivity(-1),
                        Action::Sensitivity(1),
                    ));
                    c.spawn(button("Listo", Action::Back, 300.0, false));
                })
                .id()
        }
        Screen::ConfirmDelete(dir, name) => commands
            .spawn((card(460.0), BackgroundColor(CARD)))
            .with_children(|c| {
                c.spawn(txt(format!("¿Borrar «{name}»?"), 24.0, TEXT));
                c.spawn(txt("No se puede deshacer.", 15.0, MUTED));
                c.spawn(Node {
                    column_gap: px(10),
                    ..default()
                })
                .with_children(|r| {
                    r.spawn(button("Borrar", Action::Delete(dir.clone()), 180.0, true));
                    r.spawn(button("Cancelar", Action::SinglePlayer, 180.0, false));
                });
            })
            .id(),
        Screen::Pause => commands
            .spawn((card(400.0), BackgroundColor(CARD)))
            .with_children(|c| {
                c.spawn(txt("Pausa", 30.0, TEXT));
                c.spawn(button("Volver al juego", Action::Resume, 300.0, false));
                c.spawn(button("Ajustes", Action::OpenSettings, 300.0, false));
                c.spawn(button(
                    "Guardar y salir al título",
                    Action::SaveAndQuit,
                    300.0,
                    false,
                ));
            })
            .id(),
        Screen::Loading => commands
            .spawn((card(420.0), BackgroundColor(CARD)))
            .with_children(|c| {
                c.spawn(txt("Generando el mundo…", 26.0, TEXT));
                c.spawn((txt("0 %", 18.0, MUTED), LoadingText));
            })
            .id(),
        Screen::None => return,
    };
    commands.entity(root).add_child(panel);
}

pub fn update_loading_text(
    st: Res<Streaming>,
    focus: Option<Res<Focus>>,
    mut q: Query<&mut Text, With<LoadingText>>,
) {
    let Some(focus) = focus else { return };
    let (done, total) = loading_progress(&st, focus.0);
    for mut t in &mut q {
        **t = format!("{} %  ({done}/{total} chunks)", done * 100 / total.max(1));
    }
}

/// Escribir en el campo con foco.
pub fn type_into_form(
    mut keys: MessageReader<KeyboardInput>,
    screen: Res<Screen>,
    mut form: ResMut<CreateForm>,
) {
    if *screen != Screen::Create {
        keys.clear();
        return;
    }
    for k in keys.read() {
        if k.state != ButtonState::Pressed {
            continue;
        }
        let focus = form.focus;
        let target = if focus == Field::Name {
            &mut form.name
        } else {
            &mut form.seed
        };
        match &k.logical_key {
            Key::Backspace => {
                target.pop();
            }
            Key::Tab => {
                form.focus = if focus == Field::Name {
                    Field::Seed
                } else {
                    Field::Name
                }
            }
            Key::Character(s) if target.chars().count() < 32 => {
                target.extend(s.chars().filter(|c| !c.is_control()));
            }
            Key::Space if target.chars().count() < 32 => target.push(' '),
            _ => {}
        }
    }
}

pub fn handle_actions(
    mut ui: ResMut<UiAction>,
    mut commands: Commands,
    mut screen: ResMut<Screen>,
    mut form: ResMut<CreateForm>,
    mut settings: ResMut<Settings>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut vd: ResMut<ViewDistance>,
    mut exit: MessageWriter<AppExit>,
    mut cursor: Single<&mut bevy::window::CursorOptions>,
) {
    let Some(a) = ui.0.take() else { return };
    let in_pause = *state.get() == AppState::Paused;
    match a {
        Action::SinglePlayer => *screen = Screen::Worlds,
        Action::OpenSettings => *screen = Screen::Settings,
        Action::Quit => {
            exit.write(AppExit::Success);
        }
        Action::Back => {
            *screen = if in_pause {
                Screen::Pause
            } else if *screen == Screen::Create {
                Screen::Worlds
            } else {
                Screen::Title
            }
        }
        Action::NewWorld => {
            *form = CreateForm::default();
            *screen = Screen::Create;
        }
        Action::ToggleMode => {
            form.mode = if form.mode == GameMode::Creative {
                GameMode::Survival
            } else {
                GameMode::Creative
            };
        }
        Action::Focus(f) => form.focus = f,
        Action::CreateWorld => {
            let seed = seed_from_text(&form.seed).unwrap_or_else(random_seed);
            let name = if form.name.trim().is_empty() {
                "Mi mundo".to_string()
            } else {
                form.name.trim().to_string()
            };
            match create_world(&name, seed, form.mode) {
                Ok(dir) => {
                    commands.insert_resource(PendingWorld(dir));
                    next.set(AppState::Loading);
                }
                Err(e) => error!(
                    "No se pudo crear el mundo en {}: {e}",
                    worlds_dir().display()
                ),
            }
        }
        Action::Play(dir) => {
            commands.insert_resource(PendingWorld(dir));
            next.set(AppState::Loading);
        }
        Action::AskDelete(dir, name) => *screen = Screen::ConfirmDelete(dir, name),
        Action::Delete(dir) => {
            if dir.starts_with(worlds_dir()) {
                let _ = std::fs::remove_dir_all(&dir);
            }
            *screen = Screen::Worlds;
        }
        Action::Resume => {
            next.set(AppState::InGame);
            cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
            cursor.visible = false;
        }
        Action::SaveAndQuit => next.set(AppState::Menu),
        Action::ViewDistance(d) => {
            settings.view_distance = (settings.view_distance as i32 + d).clamp(4, 32) as u32;
            vd.0 = settings.view_distance;
            settings.save();
        }
        Action::Fov(d) => {
            settings.fov = (settings.fov + d as f32).clamp(60.0, 110.0);
            settings.save();
        }
        Action::Sensitivity(d) => {
            settings.sensitivity =
                ((settings.sensitivity * 10.0).round() + d as f32).clamp(1.0, 40.0) / 10.0;
            settings.save();
        }
    }
}

/// Aplica el campo de visión a la cámara del jugador.
pub fn apply_fov(
    settings: Res<Settings>,
    mut q: Query<&mut Projection, With<crate::player::Player>>,
) {
    if !settings.is_changed() {
        return;
    }
    for mut p in &mut q {
        if let Projection::Perspective(pp) = p.as_mut() {
            pp.fov = settings.fov.to_radians();
        }
    }
}
