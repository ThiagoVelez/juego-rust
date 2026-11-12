//! Rayo por bloques (DDA): qué bloque se apunta y por qué cara.

use glam::{DVec3, IVec3};
use survivalforest_core::BlockPos;
use survivalforest_world::ChunkAccess;

pub const REACH_SURVIVAL: f64 = 5.0;
pub const REACH_CREATIVE: f64 = 6.0;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Hit {
    pub block: BlockPos,
    /// Normal de la cara golpeada (hacia donde se pone el bloque nuevo).
    pub face: IVec3,
    pub dist: f64,
}

/// Bloques que el rayo atraviesa: aire y agua.
fn passable(w: &impl ChunkAccess, p: BlockPos) -> bool {
    match w.block_at(p) {
        Some(b) => b.is_air() || b == survivalforest_core::block::WATER,
        None => true,
    }
}

/// Recorre las celdas del rayo (Amanatides–Woo) hasta `max` y devuelve el primer bloque no atravesable.
pub fn raycast(w: &impl ChunkAccess, origin: DVec3, dir: DVec3, max: f64) -> Option<Hit> {
    let dir = dir.normalize_or_zero();
    if dir == DVec3::ZERO {
        return None;
    }
    let mut cell = IVec3::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );
    let step = IVec3::new(
        dir.x.signum() as i32,
        dir.y.signum() as i32,
        dir.z.signum() as i32,
    );
    let next_boundary = |o: f64, c: i32, s: i32| {
        if s > 0 {
            c as f64 + 1.0 - o
        } else {
            o - c as f64
        }
    };
    let mut t_max = DVec3::new(
        if dir.x != 0.0 {
            next_boundary(origin.x, cell.x, step.x) / dir.x.abs()
        } else {
            f64::INFINITY
        },
        if dir.y != 0.0 {
            next_boundary(origin.y, cell.y, step.y) / dir.y.abs()
        } else {
            f64::INFINITY
        },
        if dir.z != 0.0 {
            next_boundary(origin.z, cell.z, step.z) / dir.z.abs()
        } else {
            f64::INFINITY
        },
    );
    let t_delta = DVec3::new(1.0 / dir.x.abs(), 1.0 / dir.y.abs(), 1.0 / dir.z.abs());
    let mut face = IVec3::ZERO;
    let mut t = 0.0;
    loop {
        let p = BlockPos::new(cell.x, cell.y, cell.z);
        if t > 0.0 && !passable(w, p) {
            return Some(Hit {
                block: p,
                face,
                dist: t,
            });
        }
        let axis = if t_max.x < t_max.y && t_max.x < t_max.z {
            0
        } else if t_max.y < t_max.z {
            1
        } else {
            2
        };
        t = t_max[axis];
        if t > max {
            return None;
        }
        cell[axis] += step[axis];
        t_max[axis] += t_delta[axis];
        face = IVec3::ZERO;
        face[axis] = -step[axis];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use survivalforest_core::ChunkPos;
    use survivalforest_core::block::{STONE, WATER};
    use survivalforest_world::ChunkColumn;

    fn world(
        blocks: &[(i32, i32, i32, survivalforest_core::BlockId)],
    ) -> HashMap<ChunkPos, ChunkColumn> {
        let mut m = HashMap::new();
        for cx in -2..=1 {
            for cz in -2..=1 {
                m.insert(
                    ChunkPos::new(cx, cz),
                    ChunkColumn::new(ChunkPos::new(cx, cz)),
                );
            }
        }
        for &(x, y, z, b) in blocks {
            let p = BlockPos::new(x, y, z);
            m.get_mut(&p.chunk()).unwrap().set(p.local(), b);
        }
        m
    }

    #[test]
    fn ray_hits_first_solid_and_face() {
        let w = world(&[(5, 70, 0, STONE), (7, 70, 0, STONE)]);
        let h = raycast(&w, DVec3::new(0.5, 70.5, 0.5), DVec3::X, 6.0).expect("golpea");
        assert_eq!(h.block, BlockPos::new(5, 70, 0));
        assert_eq!(h.face, IVec3::new(-1, 0, 0));
        assert!((h.dist - 4.5).abs() < 1e-9);
    }

    #[test]
    fn ray_ignores_water_and_air() {
        let w = world(&[(2, 70, 0, WATER), (3, 70, 0, WATER), (4, 70, 0, STONE)]);
        let h = raycast(&w, DVec3::new(0.5, 70.5, 0.5), DVec3::X, 6.0).expect("golpea");
        assert_eq!(h.block, BlockPos::new(4, 70, 0));
        assert!(raycast(&w, DVec3::new(0.5, 70.5, 0.5), DVec3::NEG_X, 6.0).is_none());
    }

    #[test]
    fn ray_across_negative_coords() {
        let w = world(&[(-20, 64, -20, STONE)]);
        let from = DVec3::new(-17.5, 66.5, -17.5);
        let to = DVec3::new(-19.5, 64.5, -19.5);
        let h = raycast(&w, from, (to - from).normalize(), 6.0).expect("golpea");
        assert_eq!(h.block, BlockPos::new(-20, 64, -20));
    }

    #[test]
    fn ray_hits_from_below() {
        let w = world(&[(0, 75, 0, STONE)]);
        let h = raycast(&w, DVec3::new(0.5, 71.6, 0.5), DVec3::Y, 6.0).expect("golpea");
        assert_eq!(h.face, IVec3::new(0, -1, 0));
    }
}
