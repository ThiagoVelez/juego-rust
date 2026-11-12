//! Jugador: entrada, física a 20 Hz con cámara interpolada, y modos supervivencia/creativo.

use crate::streaming::{FloatingOrigin, Focus, World};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use survivalforest_core::{BlockPos, GameMode};
use survivalforest_sim::physics::{
    Aabb, EYE_HEIGHT, FLY_SPEED, FLY_SPRINT_SPEED, GRAVITY, JUMP_SPEED, SPRINT_SPEED, WALK_SPEED,
    move_and_collide,
};

#[derive(Component)]
pub struct Player {
    pub vel: DVec3,
    pub on_ground: bool,
    pub flying: bool,
    pub yaw: f32,
    pub pitch: f32,
}

/// Posición de los pies en el mundo (f64). `prev` sirve para interpolar la cámara.
#[derive(Component, Clone, Copy)]
pub struct WorldPos {
    pub pos: DVec3,
    pub prev: DVec3,
}

/// Intención de movimiento leída en `Update` y consumida en `FixedUpdate`.
#[derive(Resource, Default)]
pub struct MoveInput {
    pub dir: Vec2,
    pub jump: bool,
    pub descend: bool,
    pub sprint: bool,
    last_space: f32,
}

/// Modo de juego del mundo abierto.
#[derive(Resource, Clone, Copy)]
pub struct Mode(pub GameMode);

/// ¿Está capturado el ratón (jugando)?
pub fn mouse_captured(cursor: &CursorOptions) -> bool {
    cursor.grab_mode == CursorGrabMode::Locked
}

pub fn spawn_player(
    commands: &mut Commands,
    feet: DVec3,
    yaw: f32,
    pitch: f32,
    flying: bool,
    fov: f32,
) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: fov.to_radians(),
            far: 1200.0,
            near: 0.05,
            ..default()
        }),
        Transform::default(),
        Player {
            vel: DVec3::ZERO,
            on_ground: false,
            flying,
            yaw,
            pitch,
        },
        WorldPos {
            pos: feet,
            prev: feet,
        },
    ));
}

pub fn read_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    cursor: Single<&CursorOptions>,
    mode: Res<Mode>,
    settings: Res<crate::settings::Settings>,
    mut input: ResMut<MoveInput>,
    mut q: Single<&mut Player>,
) {
    let p = &mut *q;
    if !mouse_captured(&cursor) {
        *input = MoveInput {
            last_space: input.last_space,
            ..default()
        };
        return;
    }
    p.yaw -= motion.delta.x * 0.0025 * settings.sensitivity;
    p.pitch = (p.pitch - motion.delta.y * 0.0025 * settings.sensitivity).clamp(-1.55, 1.55);
    let mut d = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        d.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        d.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        d.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        d.x -= 1.0;
    }
    input.dir = d.normalize_or_zero();
    input.jump = keys.pressed(KeyCode::Space);
    input.descend = keys.pressed(KeyCode::ShiftLeft);
    input.sprint = keys.pressed(KeyCode::ControlLeft);
    if keys.just_pressed(KeyCode::Space) {
        let now = time.elapsed_secs();
        if mode.0 == GameMode::Creative && now - input.last_space < 0.3 {
            p.flying = !p.flying;
            p.vel = DVec3::ZERO;
            input.last_space = -1.0;
        } else {
            input.last_space = now;
        }
    }
    if mode.0 == GameMode::Survival {
        p.flying = false;
    }
}

pub fn physics_step(
    time: Res<Time>,
    world: Res<World>,
    input: Res<MoveInput>,
    mut q: Single<(&mut Player, &mut WorldPos)>,
) {
    let dt = time.delta_secs_f64();
    let (ref mut p, ref mut wp) = *q;
    wp.prev = wp.pos;
    // Sin el chunk de debajo cargado no se simula (evita caer al vacío al aparecer).
    let here = BlockPos::new(wp.pos.x.floor() as i32, 0, wp.pos.z.floor() as i32).chunk();
    if !world.columns.contains_key(&here) {
        return;
    }
    let (sy, cy) = (p.yaw.sin() as f64, p.yaw.cos() as f64);
    let fwd = DVec3::new(-sy, 0.0, -cy);
    let right = DVec3::new(cy, 0.0, -sy);
    let wish = fwd * input.dir.y as f64 + right * input.dir.x as f64;
    if p.flying {
        let speed = if input.sprint {
            FLY_SPRINT_SPEED
        } else {
            FLY_SPEED
        };
        let mut v = wish * speed;
        if input.jump {
            v.y += speed * 0.8;
        }
        if input.descend {
            v.y -= speed * 0.8;
        }
        // Suavizado: arranca y frena en ~0,1 s.
        p.vel = p.vel.lerp(v, (dt * 10.0).min(1.0));
    } else {
        let speed = if input.sprint {
            SPRINT_SPEED
        } else {
            WALK_SPEED
        };
        let target = wish * speed;
        let k = if p.on_ground { 12.0 } else { 3.0 };
        p.vel.x += (target.x - p.vel.x) * (dt * k).min(1.0);
        p.vel.z += (target.z - p.vel.z) * (dt * k).min(1.0);
        p.vel.y -= GRAVITY * dt;
        p.vel.y = p.vel.y.max(-60.0);
        if input.jump && p.on_ground {
            p.vel.y = JUMP_SPEED;
        }
    }
    let (b, moved, ground) = move_and_collide(&*world, Aabb::player(wp.pos), p.vel * dt);
    wp.pos = b.feet();
    for axis in 0..3 {
        if (moved[axis] - p.vel[axis] * dt).abs() > 1e-6 {
            p.vel[axis] = 0.0;
        }
    }
    p.on_ground = ground || (p.vel.y == 0.0 && !p.flying && moved.y == 0.0 && grounded(&world, b));
    if p.flying && ground {
        p.flying = false;
    }
}

fn grounded(world: &World, b: Aabb) -> bool {
    move_and_collide(world, b, DVec3::new(0.0, -0.01, 0.0)).1.y > -0.01 + 1e-9
}

/// Cámara en los ojos, interpolada entre ticks y relativa al origen flotante.
pub fn place_camera(
    fixed: Res<Time<Fixed>>,
    origin: Res<FloatingOrigin>,
    mut focus: ResMut<Focus>,
    mut q: Single<(&Player, &WorldPos, &mut Transform)>,
) {
    let (p, wp, ref mut tf) = *q;
    let a = fixed.overstep_fraction() as f64;
    let pos = wp.prev.lerp(wp.pos, a);
    focus.0 = pos;
    let o = origin.0.origin();
    tf.translation = Vec3::new(
        (pos.x - o.x as f64) as f32,
        (pos.y + EYE_HEIGHT) as f32,
        (pos.z - o.z as f64) as f32,
    );
    tf.rotation = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, 0.0);
}

/// Clic en la ventana captura el ratón; Esc lo suelta (la pausa llega en la tarea 12).
pub fn grab_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if mouse.just_pressed(MouseButton::Left) && !mouse_captured(&cursor) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}
