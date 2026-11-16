//! Pantallas del juego: inventario (con crafteo 2×2), mesa (3×3), horno, baúl, piedra de
//! amolar, muerte y lista creativa. La lógica de las casillas está separada de la interfaz.

use crate::survival::{PlayerState, WorldExtras};
use bevy::math::DVec3;
use bevy::prelude::*;
use survivalforest_core::BlockPos;
use survivalforest_core::item::{ItemId, item, item_by_key};
use survivalforest_core::recipe::smelt_recipes;
use survivalforest_core::tuning::tuning;
use survivalforest_sim::containers::BlockEntity;
use survivalforest_sim::crafting::{craft, match_recipe};
use survivalforest_sim::grindstone::{RepairError, repair};
use survivalforest_sim::inventory::{HOTBAR_SLOTS, ItemStack, Slots, quick_move};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GameScreen {
    #[default]
    None,
    Inventory,
    Table(BlockPos),
    Furnace(BlockPos),
    Chest(BlockPos),
    Grindstone(BlockPos),
    Death,
    CreativeList,
}

/// Pantalla abierta, rejilla de crafteo (9 casillas; en 2×2 se usan las 4 primeras, por filas)
/// y la casilla de la piedra de amolar.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ScreenState {
    pub screen: GameScreen,
    pub grid: Vec<Option<ItemStack>>,
    pub grind: Option<ItemStack>,
    pub message: Option<String>,
}

impl Default for ScreenState {
    fn default() -> Self {
        Self {
            screen: GameScreen::None,
            grid: vec![None; 9],
            grind: None,
            message: None,
        }
    }
}

impl ScreenState {
    pub fn grid_w(&self) -> usize {
        if matches!(self.screen, GameScreen::Table(_)) {
            3
        } else {
            2
        }
    }
    fn grid_cells(&self) -> Vec<Option<ItemStack>> {
        self.grid[..self.grid_w() * self.grid_w()].to_vec()
    }
}

/// Casilla de una pantalla.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotTarget {
    Inv(usize),
    Grid(usize),
    Result,
    Chest(usize),
    FurnaceIn,
    FurnaceFuel,
    FurnaceOut,
    GrindTool,
    GrindResult,
    Creative(ItemId),
}

/// Resultado que daría la rejilla ahora mismo.
pub fn craft_preview(ss: &ScreenState) -> Option<ItemStack> {
    let w = ss.grid_w();
    let r = match_recipe(&ss.grid_cells(), w)?;
    Some(ItemStack::new(item_by_key(&r.output.0)?, r.output.1))
}

/// Hace la receta una vez. Devuelve el resultado (y suma la experiencia de la primera vez).
fn craft_once(ss: &mut ScreenState, p: &mut PlayerState) -> Option<ItemStack> {
    let w = ss.grid_w();
    let mut cells = ss.grid_cells();
    let id = match_recipe(&cells, w)?.id.clone();
    let out = craft(&mut cells, w)?;
    ss.grid[..w * w].clone_from_slice(&cells);
    if p.crafted.insert(id) {
        p.xp.add(tuning().xp_first_craft);
    }
    Some(out)
}

/// Clic en el resultado: al cursor (si cabe); con Mayús, todas las veces posibles a la mochila.
fn take_result(ss: &mut ScreenState, p: &mut PlayerState, shift: bool) {
    if shift {
        for _ in 0..64 {
            let Some(prev) = craft_preview(ss) else {
                return;
            };
            let mut probe = p.inv.slots.clone();
            if probe.add(prev).is_some() {
                return;
            }
            if let Some(out) = craft_once(ss, p) {
                p.inv.add(out);
            }
        }
        return;
    }
    let Some(prev) = craft_preview(ss) else {
        return;
    };
    let fits = match &p.cursor {
        None => true,
        Some(c) => c.stacks_with(&prev) && c.count + prev.count <= c.max(),
    };
    if !fits {
        return;
    }
    if let Some(out) = craft_once(ss, p) {
        match &mut p.cursor {
            Some(c) => c.count += out.count,
            None => p.cursor = Some(out),
        }
    }
}

/// Clic en una casilla suelta (entrada o combustible del horno, herramienta de la amoladora).
fn click_single(slot: &mut Option<ItemStack>, cursor: &mut Option<ItemStack>, right: bool) {
    let mut s = Slots(vec![slot.take()]);
    s.click(0, cursor, right);
    *slot = s.0.pop().flatten();
}

/// Mueve la casilla `i` de la mochila a un rango de la propia mochila (barra ↔ resto).
fn move_within(p: &mut PlayerState, i: usize) {
    let Some(s) = p.inv.slots.0[i].take() else {
        return;
    };
    let (a, b) = if i < HOTBAR_SLOTS {
        (HOTBAR_SLOTS, p.inv.slots.len())
    } else {
        (0, HOTBAR_SLOTS)
    };
    let mut part = Slots(p.inv.slots.0[a..b].to_vec());
    let rest = part.add(s);
    p.inv.slots.0[a..b].clone_from_slice(&part.0);
    p.inv.slots.0[i] = rest;
}

pub fn click(
    ss: &mut ScreenState,
    p: &mut PlayerState,
    ex: &mut WorldExtras,
    t: SlotTarget,
    right: bool,
    shift: bool,
) {
    ss.message = None;
    match t {
        SlotTarget::Inv(i) => {
            if !shift {
                p.inv.slots.click(i, &mut p.cursor, right);
                return;
            }
            match ss.screen {
                GameScreen::Chest(pos) => {
                    if let Some(BlockEntity::Chest(c)) = ex.be.0.get_mut(&pos) {
                        quick_move(&mut p.inv.slots, i, &mut c.slots);
                    }
                }
                GameScreen::Furnace(pos) => {
                    let Some(BlockEntity::Furnace(f)) = ex.be.0.get_mut(&pos) else {
                        return;
                    };
                    let Some(s) = p.inv.slots.0[i].clone() else {
                        return;
                    };
                    let key = &item(s.item).key;
                    let target = if smelt_recipes().iter().any(|r| &r.input == key) {
                        &mut f.input
                    } else if item(s.item).fuel_smelts.is_some() {
                        &mut f.fuel
                    } else {
                        return;
                    };
                    let mut one = Slots(vec![target.take()]);
                    quick_move(&mut p.inv.slots, i, &mut one);
                    *target = one.0.pop().flatten();
                }
                _ => move_within(p, i),
            }
        }
        SlotTarget::Grid(i) => {
            let mut s = Slots(ss.grid.clone());
            s.click(i, &mut p.cursor, right);
            ss.grid = s.0;
        }
        SlotTarget::Result => take_result(ss, p, shift),
        SlotTarget::Chest(i) => {
            let GameScreen::Chest(pos) = ss.screen else {
                return;
            };
            let Some(BlockEntity::Chest(c)) = ex.be.0.get_mut(&pos) else {
                return;
            };
            if shift {
                quick_move(&mut c.slots, i, &mut p.inv.slots);
            } else {
                c.slots.click(i, &mut p.cursor, right);
            }
        }
        SlotTarget::FurnaceIn | SlotTarget::FurnaceFuel | SlotTarget::FurnaceOut => {
            let GameScreen::Furnace(pos) = ss.screen else {
                return;
            };
            let Some(BlockEntity::Furnace(f)) = ex.be.0.get_mut(&pos) else {
                return;
            };
            match t {
                SlotTarget::FurnaceIn => click_single(&mut f.input, &mut p.cursor, right),
                SlotTarget::FurnaceFuel => click_single(&mut f.fuel, &mut p.cursor, right),
                _ => {
                    if f.output.is_none() || (!shift && p.cursor.is_some()) {
                        return;
                    }
                    let (out, xp) = f.take_output();
                    p.xp.add(xp as f32);
                    if let Some(o) = out {
                        if shift {
                            if let Some(rest) = p.inv.add(o) {
                                f.output = Some(rest);
                            }
                        } else {
                            p.cursor = Some(o);
                        }
                    }
                }
            }
        }
        SlotTarget::GrindTool => click_single(&mut ss.grind, &mut p.cursor, right),
        SlotTarget::GrindResult => {
            if p.cursor.is_some() {
                return;
            }
            let Some(tool) = ss.grind.as_mut() else {
                return;
            };
            match repair(tool, &mut p.xp) {
                Ok(_) => p.cursor = ss.grind.take(),
                Err(RepairError::NotEnoughLevels { need }) => {
                    ss.message = Some(format!("Te faltan {} niveles", need - p.xp.level))
                }
                Err(_) => ss.message = Some("Esta herramienta no necesita reparación".into()),
            }
        }
        SlotTarget::Creative(id) => {
            p.cursor = Some(ItemStack::new(id, item(id).stack));
        }
    }
}

/// Cierra la pantalla: lo de la rejilla, la amoladora y el cursor vuelve a la mochila;
/// lo que no cabe cae al suelo.
pub fn close_screen(ss: &mut ScreenState, p: &mut PlayerState, ex: &mut WorldExtras, feet: DVec3) {
    let mut back: Vec<ItemStack> = ss.grid.iter_mut().filter_map(|c| c.take()).collect();
    back.extend(ss.grind.take());
    back.extend(p.cursor.take());
    for s in back {
        if let Some(rest) = p.inv.add(s) {
            ex.items.spawn(feet, rest, tuning().ground_item_ttl);
        }
    }
    ss.screen = GameScreen::None;
    ss.message = None;
}

// ---------- Interfaz (Bevy) ----------

use crate::hud::ItemIcons;
use crate::interact::{OpenRequest, UiOpen};
use crate::player::{Mode, Player, WorldPos};
use bevy::picking::hover::Hovered;
use bevy::ui_widgets::{Activate, Button, observe};
use bevy::window::{CursorGrabMode, CursorOptions};
use survivalforest_core::GameMode;
use survivalforest_core::block::{CHEST, CRAFTING_TABLE, FURNACE, FURNACE_LIT, GRINDSTONE};
use survivalforest_core::item::items;

const PANEL: Color = Color::srgba(0.33, 0.23, 0.14, 0.96);
const PANEL_EDGE: Color = Color::srgb(0.2, 0.13, 0.08);
const SLOT_BG: Color = Color::srgb(0.47, 0.32, 0.21);
const SLOT_HOVER: Color = Color::srgb(0.6, 0.42, 0.27);
const INK: Color = Color::srgb(1.0, 0.95, 0.86);

#[derive(Component)]
pub struct ScreenRoot;
#[derive(Component, Clone, Copy)]
pub struct SlotUi(pub SlotTarget);
#[derive(Component)]
pub struct SlotUiIcon;
#[derive(Component)]
pub struct SlotUiCount;
#[derive(Component)]
pub struct CursorItem;
#[derive(Component)]
pub struct Tooltip;
#[derive(Component)]
pub struct MessageText;
#[derive(Component)]
pub struct RespawnButton;
#[derive(Component)]
pub struct FurnaceBar(pub bool);

/// Se pulsó «Reaparecer».
#[derive(Resource, Default)]
pub struct RespawnClicked(pub bool);

/// Esc ya usado este frame para cerrar una pantalla (así no abre la pausa).
#[derive(Resource, Default)]
pub struct EscConsumed(pub bool);

#[allow(clippy::too_many_arguments)]
pub fn open_screens(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<Mode>,
    mut ss: ResMut<ScreenState>,
    mut st: ResMut<PlayerState>,
    mut ex: ResMut<WorldExtras>,
    mut open: ResMut<OpenRequest>,
    mut ui: ResMut<UiOpen>,
    mut esc: ResMut<EscConsumed>,
    mut cursor: Single<&mut CursorOptions>,
    q: Single<&WorldPos, With<Player>>,
) {
    esc.0 = false;
    let feet = q.pos;
    let was = ss.screen;
    if st.dead {
        if ss.screen != GameScreen::Death {
            close_screen(&mut ss, &mut st, &mut ex, feet);
            ss.screen = GameScreen::Death;
        }
    } else if let Some((b, pos)) = open.0.take() {
        if ss.screen == GameScreen::None {
            ss.screen = match b {
                CRAFTING_TABLE => GameScreen::Table(pos),
                FURNACE | FURNACE_LIT => GameScreen::Furnace(pos),
                CHEST => GameScreen::Chest(pos),
                GRINDSTONE => GameScreen::Grindstone(pos),
                _ => GameScreen::None,
            };
            ex.be.on_placed(pos, b);
        }
    } else if keys.just_pressed(KeyCode::KeyE) {
        if ss.screen == GameScreen::None {
            ss.screen = if mode.0 == GameMode::Creative {
                GameScreen::CreativeList
            } else {
                GameScreen::Inventory
            };
        } else if ss.screen != GameScreen::Death {
            close_screen(&mut ss, &mut st, &mut ex, feet);
        }
    } else if keys.just_pressed(KeyCode::Escape)
        && !matches!(ss.screen, GameScreen::None | GameScreen::Death)
    {
        close_screen(&mut ss, &mut st, &mut ex, feet);
        esc.0 = true;
    }
    ui.0 = ss.screen != GameScreen::None;
    if ss.screen != was {
        if ui.0 {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        } else {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

fn title(t: impl Into<String>, size: f32) -> impl Bundle {
    (
        Text::new(t),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(INK),
    )
}

fn slot(t: SlotTarget) -> impl Bundle {
    (
        Hovered::default(),
        SlotUi(t),
        Node {
            width: px(44),
            height: px(44),
            border: UiRect::all(px(2)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(SLOT_BG),
        BorderColor::all(PANEL_EDGE),
        children![
            (
                SlotUiIcon,
                ImageNode::default(),
                Visibility::Hidden,
                Node {
                    width: px(32),
                    height: px(32),
                    ..default()
                }
            ),
            (
                SlotUiCount,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(2),
                    bottom: px(0),
                    ..default()
                }
            ),
        ],
    )
}

fn row() -> Node {
    Node {
        column_gap: px(4),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_grid(
    p: &mut ChildSpawnerCommands,
    w: usize,
    start: usize,
    n: usize,
    f: impl Fn(usize) -> SlotTarget,
) {
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        ..default()
    })
    .with_children(|c| {
        for r in 0..n.div_ceil(w) {
            c.spawn(row()).with_children(|rw| {
                for k in 0..w {
                    let i = start + r * w + k;
                    if i < start + n {
                        rw.spawn(slot(f(i)));
                    }
                }
            });
        }
    });
}

fn spawn_inventory(p: &mut ChildSpawnerCommands) {
    p.spawn(title("Mochila", 15.0));
    spawn_grid(p, 9, HOTBAR_SLOTS, 27, SlotTarget::Inv);
    p.spawn(Node {
        height: px(6),
        ..default()
    });
    spawn_grid(p, 9, 0, HOTBAR_SLOTS, SlotTarget::Inv);
}

fn arrow() -> impl Bundle {
    title(">", 26.0)
}

pub fn build_screen(
    mut commands: Commands,
    ss: Res<ScreenState>,
    st: Res<PlayerState>,
    mut last: Local<GameScreen>,
    roots: Query<Entity, With<ScreenRoot>>,
) {
    if ss.screen == *last && !roots.is_empty() == (ss.screen != GameScreen::None) {
        return;
    }
    *last = ss.screen;
    for e in &roots {
        commands.entity(e).despawn();
    }
    if ss.screen == GameScreen::None {
        return;
    }
    let dim = if ss.screen == GameScreen::Death {
        Color::srgba(0.35, 0.02, 0.0, 0.55)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.35)
    };
    commands
        .spawn((
            ScreenRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(dim),
            GlobalZIndex(10),
        ))
        .with_children(|root| {
            if ss.screen == GameScreen::Death {
                root.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(14),
                    ..default()
                })
                .with_children(|c| {
                    c.spawn(title("Te has quedado en el cafetal", 34.0));
                    let at = st
                        .death_pos
                        .map(|p| {
                            format!(
                                "Tus cosas quedaron en {:.0}, {:.0}, {:.0} durante 10 minutos",
                                p.x, p.y, p.z
                            )
                        })
                        .unwrap_or_default();
                    c.spawn(title(at, 16.0));
                    c.spawn((
                        Button,
                        Hovered::default(),
                        RespawnButton,
                        observe(|_: On<Activate>, mut r: ResMut<RespawnClicked>| r.0 = true),
                        Node {
                            padding: UiRect::axes(px(26), px(10)),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        BackgroundColor(SLOT_BG),
                        BorderColor::all(PANEL_EDGE),
                        children![title("Reaparecer", 18.0)],
                    ));
                });
                return;
            }
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    padding: UiRect::all(px(16)),
                    border: UiRect::all(px(3)),
                    border_radius: BorderRadius::all(px(10)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(PANEL_EDGE),
            ))
            .with_children(|panel| {
                match ss.screen {
                    GameScreen::Inventory | GameScreen::Table(_) => {
                        let w = ss.grid_w();
                        panel.spawn(title(
                            if w == 3 {
                                "Mesa de trabajo"
                            } else {
                                "Inventario"
                            },
                            18.0,
                        ));
                        panel
                            .spawn(Node {
                                column_gap: px(18),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|r| {
                                spawn_grid(r, w, 0, w * w, SlotTarget::Grid);
                                r.spawn(arrow());
                                r.spawn(slot(SlotTarget::Result));
                            });
                    }
                    GameScreen::Furnace(_) => {
                        panel.spawn(title("Horno de barro", 18.0));
                        panel
                            .spawn(Node {
                                column_gap: px(14),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|r| {
                                r.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(6),
                                    align_items: AlignItems::Center,
                                    ..default()
                                })
                                .with_children(|c| {
                                    c.spawn(slot(SlotTarget::FurnaceIn));
                                    c.spawn((
                                        FurnaceBar(false),
                                        Node {
                                            width: px(34),
                                            height: px(6),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(1.0, 0.55, 0.15)),
                                    ));
                                    c.spawn(slot(SlotTarget::FurnaceFuel));
                                });
                                r.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    ..default()
                                })
                                .with_children(|c| {
                                    c.spawn(arrow());
                                    c.spawn((
                                        FurnaceBar(true),
                                        Node {
                                            width: px(0),
                                            height: px(5),
                                            ..default()
                                        },
                                        BackgroundColor(INK),
                                    ));
                                });
                                r.spawn(slot(SlotTarget::FurnaceOut));
                            });
                    }
                    GameScreen::Chest(_) => {
                        panel.spawn(title("Baúl", 18.0));
                        spawn_grid(panel, 9, 0, 27, SlotTarget::Chest);
                    }
                    GameScreen::Grindstone(_) => {
                        panel.spawn(title("Piedra de amolar", 18.0));
                        panel
                            .spawn(Node {
                                column_gap: px(18),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|r| {
                                r.spawn(slot(SlotTarget::GrindTool));
                                r.spawn(arrow());
                                r.spawn(slot(SlotTarget::GrindResult));
                            });
                        panel.spawn((MessageText, title("", 14.0)));
                    }
                    GameScreen::CreativeList => {
                        panel.spawn(title("Todos los objetos", 18.0));
                        let ids: Vec<ItemId> = items().iter().map(|i| i.id).collect();
                        panel
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: px(4),
                                ..default()
                            })
                            .with_children(|c| {
                                for chunk in ids.chunks(12) {
                                    c.spawn(row()).with_children(|rw| {
                                        for id in chunk {
                                            rw.spawn(slot(SlotTarget::Creative(*id)));
                                        }
                                    });
                                }
                            });
                    }
                    _ => {}
                }
                if ss.screen != GameScreen::CreativeList {
                    spawn_inventory(panel);
                } else {
                    panel.spawn(title("Barra", 15.0));
                    spawn_grid(panel, 9, 0, HOTBAR_SLOTS, SlotTarget::Inv);
                }
            });
            // Lo que lleva el cursor y la burbuja de información.
            root.spawn((
                CursorItem,
                ImageNode::default(),
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(32),
                    height: px(32),
                    ..default()
                },
                GlobalZIndex(20),
            ));
            root.spawn((
                Tooltip,
                title("", 14.0),
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    padding: UiRect::axes(px(8), px(4)),
                    border_radius: BorderRadius::all(px(5)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.07, 0.05, 0.92)),
                GlobalZIndex(21),
            ));
        });
}

fn stack_at(
    t: SlotTarget,
    ss: &ScreenState,
    st: &PlayerState,
    ex: &WorldExtras,
) -> Option<ItemStack> {
    match t {
        SlotTarget::Inv(i) => st.inv.slots.0[i].clone(),
        SlotTarget::Grid(i) => ss.grid[i].clone(),
        SlotTarget::Result => craft_preview(ss),
        SlotTarget::Chest(i) => match ss.screen {
            GameScreen::Chest(p) => match ex.be.0.get(&p) {
                Some(BlockEntity::Chest(c)) => c.slots.0[i].clone(),
                _ => None,
            },
            _ => None,
        },
        SlotTarget::FurnaceIn | SlotTarget::FurnaceFuel | SlotTarget::FurnaceOut => match ss.screen
        {
            GameScreen::Furnace(p) => match ex.be.0.get(&p) {
                Some(BlockEntity::Furnace(f)) => match t {
                    SlotTarget::FurnaceIn => f.input.clone(),
                    SlotTarget::FurnaceFuel => f.fuel.clone(),
                    _ => f.output.clone(),
                },
                _ => None,
            },
            _ => None,
        },
        SlotTarget::GrindTool => ss.grind.clone(),
        SlotTarget::GrindResult => ss.grind.clone().map(|s| ItemStack { wear: 0, ..s }),
        SlotTarget::Creative(id) => Some(ItemStack::new(id, 1)),
    }
}

fn describe(s: &ItemStack) -> String {
    let d = item(s.item);
    let mut out = d.name_es.clone();
    if let Some(t) = &d.tool {
        out += &format!("\n{}/{}", t.durability - s.wear, t.durability);
    }
    if let Some(f) = &d.food {
        out += &format!("\n+{} jamones", f.hunger as f32 / 2.0);
    }
    out
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn refresh_screen(
    ss: Res<ScreenState>,
    st: Res<PlayerState>,
    ex: Res<WorldExtras>,
    icons: Res<ItemIcons>,
    window: Single<&Window>,
    mut slots: Query<(&SlotUi, &Hovered, &Children, &mut BackgroundColor), Without<FurnaceBar>>,
    mut imgs: Query<
        (&mut ImageNode, &mut Visibility),
        (With<SlotUiIcon>, Without<CursorItem>, Without<Tooltip>),
    >,
    mut counts: Query<&mut Text, (With<SlotUiCount>, Without<Tooltip>, Without<MessageText>)>,
    mut cur: Query<
        (&mut ImageNode, &mut Visibility, &mut Node),
        (With<CursorItem>, Without<SlotUiIcon>, Without<Tooltip>),
    >,
    mut tip: Query<
        (&mut Text, &mut Visibility, &mut Node),
        (
            With<Tooltip>,
            Without<SlotUiCount>,
            Without<CursorItem>,
            Without<SlotUiIcon>,
            Without<MessageText>,
        ),
    >,
    mut msg: Query<&mut Text, (With<MessageText>, Without<SlotUiCount>, Without<Tooltip>)>,
    mut bars: Query<(&FurnaceBar, &mut Node), (Without<CursorItem>, Without<Tooltip>)>,
) {
    if ss.screen == GameScreen::None {
        return;
    }
    let mut hovered: Option<ItemStack> = None;
    for (s, inter, children, mut bg) in &mut slots {
        let stack = stack_at(s.0, &ss, &st, &ex);
        if inter.get() {
            hovered = stack.clone();
        }
        bg.0 = if inter.get() { SLOT_HOVER } else { SLOT_BG };
        for c in children.iter() {
            if let Ok((mut img, mut vis)) = imgs.get_mut(c) {
                match stack.as_ref().and_then(|x| icons.0.get(&x.item)) {
                    Some(h) => {
                        img.image = h.clone();
                        *vis = Visibility::Inherited;
                    }
                    None => *vis = Visibility::Hidden,
                }
            }
            if let Ok(mut t) = counts.get_mut(c) {
                t.0 = stack
                    .as_ref()
                    .filter(|x| x.count > 1 && !matches!(s.0, SlotTarget::Creative(_)))
                    .map(|x| x.count.to_string())
                    .unwrap_or_default();
            }
        }
    }
    let pos = window.cursor_position();
    for (mut img, mut vis, mut node) in &mut cur {
        match (st.cursor.as_ref().and_then(|c| icons.0.get(&c.item)), pos) {
            (Some(h), Some(p)) => {
                img.image = h.clone();
                *vis = Visibility::Inherited;
                node.left = px(p.x - 16.0);
                node.top = px(p.y - 16.0);
            }
            _ => *vis = Visibility::Hidden,
        }
    }
    for (mut t, mut vis, mut node) in &mut tip {
        match (hovered.as_ref(), pos, st.cursor.is_none()) {
            (Some(h), Some(p), true) => {
                t.0 = describe(h);
                *vis = Visibility::Inherited;
                node.left = px(p.x + 14.0);
                node.top = px(p.y + 10.0);
            }
            _ => *vis = Visibility::Hidden,
        }
    }
    for mut t in &mut msg {
        t.0 = ss.message.clone().unwrap_or_else(|| match &ss.grind {
            Some(g) => survivalforest_sim::grindstone::repair_cost(g)
                .map(|n| format!("Cuesta {n} niveles (tienes {})", st.xp.level))
                .unwrap_or_default(),
            None => String::new(),
        });
    }
    if let GameScreen::Furnace(p) = ss.screen
        && let Some(BlockEntity::Furnace(f)) = ex.be.0.get(&p)
    {
        for (b, mut n) in &mut bars {
            if b.0 {
                n.width = px(34.0 * (f.progress / tuning().smelt_secs).clamp(0.0, 1.0));
            } else {
                n.width = px(if f.burn_total > 0.0 {
                    34.0 * f.burn_left / f.burn_total
                } else {
                    0.0
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn slot_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut ss: ResMut<ScreenState>,
    mut st: ResMut<PlayerState>,
    mut ex: ResMut<WorldExtras>,
    slots: Query<(&SlotUi, &Hovered)>,
) {
    if ss.screen == GameScreen::None || ss.screen == GameScreen::Death {
        return;
    }
    let right = mouse.just_pressed(MouseButton::Right);
    if !right && !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if let Some((s, _)) = slots.iter().find(|(_, h)| h.get()) {
        let t = s.0;
        click(&mut ss, &mut st, &mut ex, t, right, shift);
    }
}

pub fn respawn_button(
    mut ss: ResMut<ScreenState>,
    mut st: ResMut<PlayerState>,
    mut ui: ResMut<UiOpen>,
    mut cursor: Single<&mut CursorOptions>,
    mut clicked: ResMut<RespawnClicked>,
    mut q: Single<(&mut Player, &mut WorldPos)>,
) {
    if !std::mem::take(&mut clicked.0) {
        return;
    }
    crate::survival::respawn(&mut st);
    let [x, y, z] = st.spawn.unwrap_or([0, 100, 0]);
    let (ref mut p, ref mut wp) = *q;
    let feet = DVec3::new(x as f64 + 0.5, y as f64, z as f64 + 0.5);
    wp.pos = feet;
    wp.prev = feet;
    p.vel = DVec3::ZERO;
    ss.screen = GameScreen::None;
    ui.0 = false;
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

/// Al salir del mundo se cierran las pantallas.
pub fn reset_screens(
    mut commands: Commands,
    mut ss: ResMut<ScreenState>,
    mut ui: ResMut<UiOpen>,
    roots: Query<Entity, With<ScreenRoot>>,
) {
    *ss = ScreenState::default();
    ui.0 = false;
    for e in &roots {
        commands.entity(e).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use survivalforest_core::item::item_by_key;

    fn st(k: &str, n: u16) -> Option<ItemStack> {
        Some(ItemStack::new(item_by_key(k).unwrap(), n))
    }

    #[test]
    fn closing_returns_grid_and_cursor_to_inventory() {
        let mut ss = ScreenState {
            screen: GameScreen::Inventory,
            ..Default::default()
        };
        let mut p = PlayerState::default();
        let mut ex = WorldExtras::default();
        ss.grid[0] = st("log", 2);
        p.cursor = st("stick", 3);
        close_screen(&mut ss, &mut p, &mut ex, DVec3::ZERO);
        assert_eq!(ss.screen, GameScreen::None);
        assert!(ss.grid.iter().all(|c| c.is_none()));
        assert!(p.cursor.is_none());
        assert_eq!(p.inv.slots.count(item_by_key("log").unwrap()), 2);
        assert_eq!(p.inv.slots.count(item_by_key("stick").unwrap()), 3);
        assert!(ex.items.0.is_empty());
    }

    #[test]
    fn closing_with_full_inventory_drops_to_ground() {
        let mut ss = ScreenState {
            screen: GameScreen::Table(BlockPos::new(0, 0, 0)),
            ..Default::default()
        };
        let mut p = PlayerState::default();
        for i in 0..36 {
            p.inv.slots.0[i] = st("cobble", 64);
        }
        let mut ex = WorldExtras::default();
        ss.grid[4] = st("coal", 5);
        close_screen(&mut ss, &mut p, &mut ex, DVec3::new(1.0, 2.0, 3.0));
        assert_eq!(ex.items.0.len(), 1);
        assert_eq!(ex.items.0[0].stack, st("coal", 5));
    }

    #[test]
    fn craft_click_gives_first_time_xp_once() {
        let mut ss = ScreenState {
            screen: GameScreen::Inventory,
            ..Default::default()
        };
        let mut p = PlayerState::default();
        let mut ex = WorldExtras::default();
        ss.grid[0] = st("log", 2);
        assert_eq!(craft_preview(&ss), st("planks", 3));
        click(&mut ss, &mut p, &mut ex, SlotTarget::Result, false, false);
        assert_eq!(p.cursor, st("planks", 3));
        assert_eq!(p.xp.total, 2.0);
        click(&mut ss, &mut p, &mut ex, SlotTarget::Result, false, false);
        assert_eq!(p.cursor, st("planks", 6));
        assert_eq!(p.xp.total, 2.0);
        assert!(ss.grid[0].is_none());
    }

    #[test]
    fn shift_craft_fills_inventory() {
        let mut ss = ScreenState {
            screen: GameScreen::Inventory,
            ..Default::default()
        };
        let mut p = PlayerState::default();
        ss.grid[3] = st("log", 4);
        click(
            &mut ss,
            &mut p,
            &mut WorldExtras::default(),
            SlotTarget::Result,
            false,
            true,
        );
        assert_eq!(p.inv.slots.count(item_by_key("planks").unwrap()), 12);
    }

    #[test]
    fn three_by_three_only_at_table() {
        let mut ss = ScreenState {
            screen: GameScreen::Inventory,
            ..Default::default()
        };
        for (i, k) in [
            (0, "planks"),
            (2, "planks"),
            (3, "planks"),
            (4, "stick"),
            (5, "planks"),
            (7, "stick"),
        ] {
            ss.grid[i] = st(k, 1);
        }
        assert!(craft_preview(&ss).is_none());
        ss.screen = GameScreen::Table(BlockPos::new(0, 0, 0));
        assert_eq!(craft_preview(&ss), st("pick_wood", 1));
    }

    #[test]
    fn furnace_output_click_gives_xp() {
        let p0 = BlockPos::new(1, 2, 3);
        let mut ss = ScreenState {
            screen: GameScreen::Furnace(p0),
            ..Default::default()
        };
        let mut p = PlayerState::default();
        let mut ex = WorldExtras::default();
        ex.be.on_placed(p0, survivalforest_core::block::FURNACE);
        click(
            &mut ss,
            &mut p,
            &mut ex,
            SlotTarget::FurnaceIn,
            false,
            false,
        );
        p.cursor = st("iron_ore_item", 2);
        click(
            &mut ss,
            &mut p,
            &mut ex,
            SlotTarget::FurnaceIn,
            false,
            false,
        );
        p.cursor = st("coal", 1);
        click(
            &mut ss,
            &mut p,
            &mut ex,
            SlotTarget::FurnaceFuel,
            false,
            false,
        );
        for _ in 0..20 {
            ex.be.tick_furnaces(1.0);
        }
        click(
            &mut ss,
            &mut p,
            &mut ex,
            SlotTarget::FurnaceOut,
            false,
            false,
        );
        assert_eq!(p.cursor, st("iron_ingot", 2));
        assert_eq!(p.xp.total, 2.0);
    }

    #[test]
    fn grindstone_repairs_with_levels() {
        let mut ss = ScreenState {
            screen: GameScreen::Grindstone(BlockPos::new(0, 0, 0)),
            ..Default::default()
        };
        let mut p = PlayerState::default();
        p.xp.level = 5;
        ss.grind = Some(ItemStack {
            wear: 130,
            ..st("pick_iron", 1).unwrap()
        });
        click(
            &mut ss,
            &mut p,
            &mut WorldExtras::default(),
            SlotTarget::GrindResult,
            false,
            false,
        );
        assert_eq!(p.cursor.as_ref().map(|c| c.wear), Some(0));
        assert_eq!(p.xp.level, 3);
        assert!(ss.grind.is_none());
        ss.grind = Some(ItemStack {
            wear: 260,
            ..st("pick_iron", 1).unwrap()
        });
        p.cursor = None;
        click(
            &mut ss,
            &mut p,
            &mut WorldExtras::default(),
            SlotTarget::GrindResult,
            false,
            false,
        );
        assert!(p.cursor.is_none());
        assert_eq!(ss.message.as_deref(), Some("Te faltan 1 niveles"));
    }

    #[test]
    fn chest_shift_moves_to_inventory() {
        let p0 = BlockPos::new(4, 4, 4);
        let mut ss = ScreenState {
            screen: GameScreen::Chest(p0),
            ..Default::default()
        };
        let mut p = PlayerState::default();
        let mut ex = WorldExtras::default();
        ex.be.on_placed(p0, survivalforest_core::block::CHEST);
        p.cursor = st("coal", 7);
        click(&mut ss, &mut p, &mut ex, SlotTarget::Chest(3), false, false);
        click(&mut ss, &mut p, &mut ex, SlotTarget::Chest(3), false, true);
        assert_eq!(p.inv.slots.count(item_by_key("coal").unwrap()), 7);
    }
}
