//! Apuntar, romper y poner bloques, con contorno del bloque apuntado.

use crate::player::{Mode, Player, WorldPos, mouse_captured};
use crate::streaming::{FloatingOrigin, RemeshRequest, World, set_block};
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::window::CursorOptions;
use survivalforest_core::GameMode;
use survivalforest_core::block::{self, AIR, BlockId};
use survivalforest_sim::edit::{can_break, can_place};
use survivalforest_sim::physics::{Aabb, EYE_HEIGHT};
use survivalforest_sim::raycast::{Hit, REACH_CREATIVE, REACH_SURVIVAL, raycast};

/// Bloques de la barra de construcción (creativo).
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

#[derive(Resource, Default)]
pub struct Hotbar {
    pub selected: usize,
}

impl Hotbar {
    pub fn block(&self) -> BlockId {
        BUILD_BLOCKS[self.selected % BUILD_BLOCKS.len()]
    }
}

/// Bloque apuntado este frame.
#[derive(Resource, Default)]
pub struct Target(pub Option<Hit>);

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

pub fn select_slot(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    mut bar: ResMut<Hotbar>,
) {
    for (i, k) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*k) {
            bar.selected = i;
            debug!("Ranura {} ({})", i + 1, bar.block().def().name_es);
        }
    }
    let n = BUILD_BLOCKS.len() as i32;
    if scroll.delta.y > 0.0 {
        bar.selected = (bar.selected as i32 - 1).rem_euclid(n) as usize;
    } else if scroll.delta.y < 0.0 {
        bar.selected = (bar.selected as i32 + 1).rem_euclid(n) as usize;
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

pub fn edit_blocks(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Single<&CursorOptions>,
    target: Res<Target>,
    bar: Res<Hotbar>,
    mut world: ResMut<World>,
    mut remesh: MessageWriter<RemeshRequest>,
    q: Single<&WorldPos, With<Player>>,
    mut was_captured: Local<bool>,
) {
    let captured = mouse_captured(&cursor);
    // El clic que captura el ratón no edita.
    let active = captured && *was_captured;
    *was_captured = captured;
    let (Some(hit), true) = (target.0, active) else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) && can_break(&*world, hit.block).is_ok() {
        set_block(&mut world, hit.block, AIR, &mut remesh);
    } else if mouse.just_pressed(MouseButton::Right) {
        let pos = hit.block.offset(hit.face.x, hit.face.y, hit.face.z);
        if can_place(&*world, pos, &Aabb::player(q.pos)).is_ok() {
            debug!("Pone {} en {:?}", bar.block().def().name_es, pos);
            set_block(&mut world, pos, bar.block(), &mut remesh);
        }
    }
}

/// Contorno del bloque apuntado.
pub fn draw_outline(mut gizmos: Gizmos, target: Res<Target>, origin: Res<FloatingOrigin>) {
    if let Some(hit) = target.0 {
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
    }
}
