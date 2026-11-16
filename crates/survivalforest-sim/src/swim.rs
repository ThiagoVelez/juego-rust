//! Nadar: hundirse despacio, subir con Espacio y moverse más lento en el agua.

use crate::physics::{Aabb, EYE_HEIGHT, WALK_SPEED};
use glam::DVec3;
use survivalforest_core::BlockPos;
use survivalforest_core::block::WATER;
use survivalforest_world::ChunkAccess;

pub const SWIM_UP_SPEED: f64 = 3.0;
pub const SINK_SPEED: f64 = 2.0;
const WATER_GRAVITY: f64 = 6.0;

fn water_at(w: &impl ChunkAccess, p: DVec3) -> bool {
    w.block_at(BlockPos::new(
        p.x.floor() as i32,
        p.y.floor() as i32,
        p.z.floor() as i32,
    )) == Some(WATER)
}

/// ¿Está el cuerpo (los pies) y la cabeza (los ojos) dentro del agua?
pub fn in_water(w: &impl ChunkAccess, b: &Aabb) -> (bool, bool) {
    let f = b.feet();
    (
        water_at(w, f + DVec3::new(0.0, 0.1, 0.0)),
        water_at(w, f + DVec3::new(0.0, EYE_HEIGHT, 0.0)),
    )
}

/// Velocidad dentro del agua: la mitad de rápido, se hunde despacio y Espacio hace subir.
pub fn swim_velocity(vel: DVec3, wish: DVec3, jump: bool, dt: f64) -> DVec3 {
    let target = wish * WALK_SPEED * 0.5;
    let k = (dt * 6.0).min(1.0);
    let mut v = vel;
    v.x += (target.x - v.x) * k;
    v.z += (target.z - v.z) * k;
    if jump {
        v.y += (SWIM_UP_SPEED - v.y) * (dt * 8.0).min(1.0);
    } else {
        v.y = (v.y - WATER_GRAVITY * dt).max(-SINK_SPEED);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::Aabb;
    use std::collections::HashMap;
    use survivalforest_core::block::{STONE, WATER};
    use survivalforest_core::{BlockPos, ChunkPos};
    use survivalforest_world::ChunkColumn;

    /// Suelo en y=60 y agua de y=61 a y=63.
    fn pool() -> HashMap<ChunkPos, ChunkColumn> {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        for x in 0..16 {
            for z in 0..16 {
                c.set(BlockPos::new(x, 60, z).local(), STONE);
                for y in 61..=63 {
                    c.set(BlockPos::new(x, y, z).local(), WATER);
                }
            }
        }
        HashMap::from([(c.pos, c)])
    }

    #[test]
    fn swim_up_with_jump() {
        let mut v = DVec3::ZERO;
        for _ in 0..20 {
            v = swim_velocity(v, DVec3::ZERO, true, 0.05);
        }
        assert!(v.y > 2.5, "{}", v.y);
    }

    #[test]
    fn sinks_slowly_without_jump() {
        let mut v = DVec3::new(0.0, -20.0, 0.0);
        for _ in 0..40 {
            v = swim_velocity(v, DVec3::ZERO, false, 0.05);
        }
        assert!(v.y < 0.0 && v.y >= -2.0 - 1e-9, "{}", v.y);
    }

    #[test]
    fn slower_horizontally() {
        let mut v = DVec3::ZERO;
        for _ in 0..60 {
            v = swim_velocity(v, DVec3::X, false, 0.05);
        }
        assert!((v.x - crate::physics::WALK_SPEED * 0.5).abs() < 0.05);
    }

    #[test]
    fn in_water_detects_head() {
        let w = pool();
        assert_eq!(
            in_water(&w, &Aabb::player(DVec3::new(5.5, 61.0, 5.5))),
            (true, true)
        );
        assert_eq!(
            in_water(&w, &Aabb::player(DVec3::new(5.5, 63.0, 5.5))),
            (true, false)
        );
        assert_eq!(
            in_water(&w, &Aabb::player(DVec3::new(5.5, 64.0, 5.5))),
            (false, false)
        );
    }
}
