//! Apuntar, romper (con tiempo y herramientas), poner, usar, comer y tirar objetos.

use crate::player::{Mode, Player, WorldPos, mouse_captured};
use crate::streaming::{FloatingOrigin, RemeshRequest, World, set_block};
use crate::survival::{PlayerState, WorldExtras};
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::window::CursorOptions;
use survivalforest_core::block::{
    self, AIR, BlockId, CHEST, CRAFTING_TABLE, FURNACE, FURNACE_LIT, GRINDSTONE,
};
use survivalforest_core::item::{item, item_for_block};
use survivalforest_core::tuning::tuning;
use survivalforest_core::{BlockPos, GameMode};
use survivalforest_sim::drops::scatter;
use survivalforest_sim::edit::{can_break, can_place};
use survivalforest_sim::inventory::{HOTBAR_SLOTS, ItemStack};
use survivalforest_sim::mining::{break_time, drops_for, wear_tool};
use survivalforest_sim::physics::{Aabb, EYE_HEIGHT};
use survivalforest_sim::raycast::{Hit, REACH_CREATIVE, REACH_SURVIVAL, raycast};

/// Bloques con los que empieza la barra en creativo (los de la etapa 1).
pub const BUILD_BLOCKS: [BlockId; 9] = [
    block::GRASS,
    block::RED_SOIL,
    block::STONE,
    block::PLANKS,
    block::WHITEWASH,
    block::ROOF_TILE,
    block::LOG,
    block::BAMBOO,
    block::TORCH,
];

/// Bloque apuntado este frame.
#[derive(Resource, Default)]
pub struct Target(pub Option<Hit>);

/// Pantalla de un bloque que hay que abrir (la atiende `screens`).
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct OpenRequest(pub Option<(BlockId, BlockPos)>);

/// ¿Hay una pantalla abierta? (Entonces no se rompe ni se pone nada.)
#[derive(Resource, Default)]
pub struct UiOpen(pub bool);

const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// Barra inicial del creativo: los bloques de la etapa 1.
pub fn creative_start_bar(st: &mut PlayerState) {
    if st.inv.slots.is_empty() {
        for (i, b) in BUILD_BLOCKS.iter().enumerate() {
            if let Some(it) = item_for_block(*b) {
                st.inv.slots.0[i] = Some(ItemStack::new(it, 64));
            }
        }
    }
}

pub fn select_slot(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    ui: Res<UiOpen>,
    mut st: ResMut<PlayerState>,
) {
    if ui.0 {
        return;
    }
    for (i, k) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*k) {
            st.inv.selected = i as u8;
        }
    }
    let n = HOTBAR_SLOTS as i32;
    if scroll.delta.y > 0.0 {
        st.inv.selected = (st.inv.selected as i32 - 1).rem_euclid(n) as u8;
    } else if scroll.delta.y < 0.0 {
        st.inv.selected = (st.inv.selected as i32 + 1).rem_euclid(n) as u8;
    }
}

pub fn aim(
    world: Res<World>,
    mode: Res<Mode>,
    mut target: ResMut<Target>,
    q: Single<(&Player, &WorldPos)>,
) {
    let (p, wp) = *q;
    let eye = wp.pos + DVec3::new(0.0, EYE_HEIGHT, 0.0);
    let dir = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, 0.0) * Vec3::NEG_Z;
    let reach = if mode.0 == GameMode::Creative {
        REACH_CREATIVE
    } else {
        REACH_SURVIVAL
    };
    target.0 = raycast(
        &*world,
        eye,
        DVec3::new(dir.x as f64, dir.y as f64, dir.z as f64),
        reach,
    );
}

fn center(p: BlockPos) -> DVec3 {
    DVec3::new(p.x as f64 + 0.5, p.y as f64 + 0.5, p.z as f64 + 0.5)
}

fn pos_seed(p: BlockPos, t: f32) -> u64 {
    ((p.x as i64 as u64) << 40)
        ^ ((p.z as i64 as u64) << 16)
        ^ (p.y as i64 as u64)
        ^ (t.to_bits() as u64).wrapping_mul(0x9E37)
}

/// Rompe el bloque: objetos, experiencia, desgaste y contenido de baúles y hornos.
fn finish_break(
    world: &mut World,
    remesh: &mut MessageWriter<RemeshRequest>,
    st: &mut PlayerState,
    ex: &mut WorldExtras,
    mode: GameMode,
    pos: BlockPos,
    b: BlockId,
    seed: u64,
) {
    let held = st.inv.held().cloned();
    let (drops, xp) = drops_for(b, held.as_ref(), mode, seed);
    if mode == GameMode::Survival {
        wear_tool(st.inv.held_mut(), b);
        st.xp.add(xp as f32);
    }
    set_block(world, pos, AIR, remesh);
    ex.be.on_broken(pos, &mut ex.items);
    scatter(&mut ex.items, center(pos), drops, tuning().ground_item_ttl);
}

#[allow(clippy::too_many_arguments)]
pub fn edit_blocks(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Single<&CursorOptions>,
    target: Res<Target>,
    mode: Res<Mode>,
    ui: Res<UiOpen>,
    mut st: ResMut<PlayerState>,
    mut ex: ResMut<WorldExtras>,
    mut open: ResMut<OpenRequest>,
    mut world: ResMut<World>,
    mut remesh: MessageWriter<RemeshRequest>,
    q: Single<&WorldPos, With<Player>>,
    mut was_captured: Local<bool>,
) {
    let captured = mouse_captured(&cursor);
    // El clic que captura el ratón no edita.
    let active = captured && *was_captured && !ui.0 && !st.dead;
    *was_captured = captured;
    if !active {
        st.breaking = None;
        st.eating = None;
        return;
    }
    let dt = time.delta_secs();
    let mode = mode.0;
    // Comer: mantener el clic derecho con comida en la mano.
    let food = st.inv.held().and_then(|h| item(h.item).food.clone());
    if let (Some(f), true) = (food.as_ref(), mouse.pressed(MouseButton::Right)) {
        let hungry = st.vitals.hunger < tuning().max_hunger;
        let aiming_station = target
            .0
            .and_then(|h| world.block_at_pos(h.block))
            .is_some_and(is_station);
        if hungry && mode == GameMode::Survival && !aiming_station {
            let t = st.eating.unwrap_or(0.0) + dt;
            if t >= f.eat_secs {
                st.vitals.eat(f);
                if let Some(h) = st.inv.held_mut() {
                    h.count -= 1;
                    if h.count == 0 {
                        *st.inv.held_mut() = None;
                    }
                }
                st.eating = None;
            } else {
                st.eating = Some(t);
            }
            return;
        }
    } else {
        st.eating = None;
    }
    let Some(hit) = target.0 else {
        st.breaking = None;
        return;
    };
    // Romper: en creativo al instante; en supervivencia, manteniendo el clic.
    if mouse.pressed(MouseButton::Left) && can_break(&*world, hit.block).is_ok() {
        let b = world.block_at_pos(hit.block).unwrap_or(AIR);
        let held = st.inv.held().cloned();
        let Some(need) = break_time(b, held.as_ref(), mode) else {
            return;
        };
        if mode == GameMode::Creative {
            if mouse.just_pressed(MouseButton::Left) {
                finish_break(
                    &mut world,
                    &mut remesh,
                    &mut st,
                    &mut ex,
                    mode,
                    hit.block,
                    b,
                    0,
                );
            }
            return;
        }
        let t = match st.breaking {
            Some((p, t)) if p == hit.block => t + dt,
            _ => dt,
        };
        if t >= need {
            st.breaking = None;
            let seed = pos_seed(hit.block, time.elapsed_secs());
            finish_break(
                &mut world,
                &mut remesh,
                &mut st,
                &mut ex,
                mode,
                hit.block,
                b,
                seed,
            );
        } else {
            st.breaking = Some((hit.block, t));
        }
        return;
    }
    st.breaking = None;
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    // Usar un bloque con pantalla (Mayús pone un bloque encima en vez de abrirlo).
    let b = world.block_at_pos(hit.block).unwrap_or(AIR);
    if is_station(b) && !keys.pressed(KeyCode::ShiftLeft) {
        open.0 = Some((b, hit.block));
        return;
    }
    // Poner el bloque de la mano.
    let Some(place) = st.inv.held().and_then(|h| item(h.item).places) else {
        return;
    };
    let pos = hit.block.offset(hit.face.x, hit.face.y, hit.face.z);
    if can_place(&*world, pos, &Aabb::player(q.pos)).is_ok()
        && set_block(&mut world, pos, place, &mut remesh)
    {
        ex.be.on_placed(pos, place);
        if mode == GameMode::Survival {
            let h = st.inv.held_mut();
            if let Some(s) = h {
                s.count -= 1;
                if s.count == 0 {
                    *h = None;
                }
            }
        }
    }
}

pub fn is_station(b: BlockId) -> bool {
    matches!(
        b,
        CRAFTING_TABLE | FURNACE | FURNACE_LIT | CHEST | GRINDSTONE
    )
}

/// Q tira uno de la mano; Ctrl+Q, la pila entera.
pub fn drop_held(
    keys: Res<ButtonInput<KeyCode>>,
    ui: Res<UiOpen>,
    mut st: ResMut<PlayerState>,
    mut ex: ResMut<WorldExtras>,
    q: Single<(&Player, &WorldPos)>,
) {
    if ui.0 || st.dead || !keys.just_pressed(KeyCode::KeyQ) {
        return;
    }
    let all = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let (p, wp) = *q;
    let h = st.inv.held_mut();
    let Some(s) = h else { return };
    let n = if all { s.count } else { 1 };
    let out = ItemStack {
        count: n,
        ..s.clone()
    };
    s.count -= n;
    if s.count == 0 {
        *h = None;
    }
    let dir = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, 0.0) * Vec3::NEG_Z;
    ex.items.throw(
        wp.pos + DVec3::new(0.0, EYE_HEIGHT, 0.0),
        DVec3::new(dir.x as f64, dir.y as f64, dir.z as f64),
        out,
    );
}

/// Contorno del bloque apuntado y grietas mientras se rompe (6 etapas).
pub fn draw_outline(
    mut gizmos: Gizmos,
    target: Res<Target>,
    origin: Res<FloatingOrigin>,
    st: Res<PlayerState>,
    mode: Res<Mode>,
    world: Res<World>,
) {
    let Some(hit) = target.0 else { return };
    let o = origin.0.origin();
    let c = Vec3::new(
        (hit.block.x - o.x) as f32 + 0.5,
        hit.block.y as f32 + 0.5,
        (hit.block.z - o.z) as f32 + 0.5,
    );
    gizmos.cube(
        Transform::from_translation(c).with_scale(Vec3::splat(1.004)),
        Color::srgba(0.05, 0.04, 0.03, 0.85),
    );
    let Some((p, t)) = st.breaking else { return };
    if p != hit.block {
        return;
    }
    let b = world.block_at_pos(p).unwrap_or(AIR);
    let need = break_time(b, st.inv.held(), mode.0)
        .unwrap_or(1.0)
        .max(0.01);
    let stage = ((t / need) * 6.0).floor().clamp(0.0, 5.0) as i32 + 1;
    // Grietas: trazos en zigzag sobre las seis caras, más largos en cada etapa.
    let color = Color::srgba(0.08, 0.06, 0.05, 0.9);
    for (n, u, v) in [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (Vec3::Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
    ] {
        for side in [-1.0, 1.0] {
            let face = c + n * 0.503 * side;
            let mut a = face;
            for k in 0..stage {
                let k = k as f32;
                let next = face
                    + u * (0.08 * k * if k as i32 % 2 == 0 { 1.0 } else { -1.0 })
                    + v * (0.07 * (k + 1.0)) * side;
                gizmos.line(a, next, color);
                a = next;
            }
        }
    }
}

/// Ayuda: bloque en una posición del mundo (None si el chunk no está cargado).
pub trait BlockAt {
    fn block_at_pos(&self, p: BlockPos) -> Option<BlockId>;
}

impl BlockAt for World {
    fn block_at_pos(&self, p: BlockPos) -> Option<BlockId> {
        use survivalforest_world::ChunkAccess;
        self.block_at(p)
    }
}
