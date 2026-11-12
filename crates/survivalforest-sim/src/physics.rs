//! Cajas de colisión y movimiento contra los bloques del mundo.

use glam::DVec3;
use survivalforest_core::BlockPos;
use survivalforest_world::ChunkAccess;

pub const GRAVITY: f64 = 32.0;
pub const JUMP_SPEED: f64 = 9.0;
pub const WALK_SPEED: f64 = 4.3;
pub const SPRINT_SPEED: f64 = 5.6;
pub const FLY_SPEED: f64 = 11.0;
pub const FLY_SPRINT_SPEED: f64 = 22.0;
pub const PLAYER_WIDTH: f64 = 0.6;
pub const PLAYER_HEIGHT: f64 = 1.8;
pub const EYE_HEIGHT: f64 = 1.62;
const EPS: f64 = 1e-7;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb {
    pub min: DVec3,
    pub max: DVec3,
}

impl Aabb {
    /// Caja del jugador con los pies en `feet` (centro de la base).
    pub fn player(feet: DVec3) -> Self {
        let h = PLAYER_WIDTH / 2.0;
        Aabb {
            min: feet - DVec3::new(h, 0.0, h),
            max: feet + DVec3::new(h, PLAYER_HEIGHT, h),
        }
    }
    pub fn feet(&self) -> DVec3 {
        DVec3::new(
            (self.min.x + self.max.x) / 2.0,
            self.min.y,
            (self.min.z + self.max.z) / 2.0,
        )
    }
    pub fn translate(&self, d: DVec3) -> Self {
        Aabb {
            min: self.min + d,
            max: self.max + d,
        }
    }
    /// ¿Se solapa con el bloque `p`?
    pub fn intersects_block(&self, p: BlockPos) -> bool {
        let (bx, by, bz) = (p.x as f64, p.y as f64, p.z as f64);
        self.max.x > bx + EPS
            && self.min.x < bx + 1.0 - EPS
            && self.max.y > by + EPS
            && self.min.y < by + 1.0 - EPS
            && self.max.z > bz + EPS
            && self.min.z < bz + 1.0 - EPS
    }
}

/// Un bloque sólido, o un chunk sin cargar dentro de la altura del mundo.
pub fn is_solid(w: &impl ChunkAccess, p: BlockPos) -> bool {
    if !p.in_height() {
        return false;
    }
    match w.column(p.chunk()) {
        Some(c) => c.get(p.local()).def().solid,
        None => true,
    }
}

fn blocks_in(b: &Aabb) -> impl Iterator<Item = BlockPos> {
    let (x0, x1) = (b.min.x.floor() as i32, (b.max.x - EPS).floor() as i32);
    let (y0, y1) = (b.min.y.floor() as i32, (b.max.y - EPS).floor() as i32);
    let (z0, z1) = (b.min.z.floor() as i32, (b.max.z - EPS).floor() as i32);
    (x0..=x1).flat_map(move |x| {
        (y0..=y1).flat_map(move |y| (z0..=z1).map(move |z| BlockPos::new(x, y, z)))
    })
}

/// Mueve la caja un eje cada vez y la detiene contra los bloques sólidos.
fn sweep_axis(w: &impl ChunkAccess, b: Aabb, axis: usize, d: f64) -> (Aabb, f64) {
    if d == 0.0 {
        return (b, 0.0);
    }
    let mut delta = DVec3::ZERO;
    delta[axis] = d;
    let moved = b.translate(delta);
    let mut allowed = d;
    for p in blocks_in(&moved) {
        if !is_solid(w, p) || b.intersects_block(p) {
            continue;
        }
        let face = if d > 0.0 {
            [p.x, p.y, p.z][axis] as f64 - b.max[axis]
        } else {
            [p.x, p.y, p.z][axis] as f64 + 1.0 - b.min[axis]
        };
        if d > 0.0 {
            allowed = allowed.min(face.max(0.0))
        } else {
            allowed = allowed.max(face.min(0.0))
        }
    }
    let mut delta = DVec3::ZERO;
    delta[axis] = allowed;
    (b.translate(delta), allowed)
}

/// Devuelve la caja movida, el desplazamiento real y si quedó apoyada en el suelo.
/// Los pasos grandes se dividen para no atravesar esquinas.
pub fn move_and_collide(w: &impl ChunkAccess, aabb: Aabb, motion: DVec3) -> (Aabb, DVec3, bool) {
    let steps = (motion.abs().max_element() / 0.45).ceil().max(1.0) as usize;
    let part = motion / steps as f64;
    let mut b = aabb;
    let mut total = DVec3::ZERO;
    let mut blocked = [false; 3];
    for _ in 0..steps {
        for axis in [1, 0, 2] {
            if blocked[axis] {
                continue;
            }
            let (nb, done) = sweep_axis(w, b, axis, part[axis]);
            b = nb;
            total[axis] += done;
            if (done - part[axis]).abs() > EPS {
                blocked[axis] = true;
            }
        }
    }
    let on_ground = blocked[1] && motion.y < 0.0;
    (b, total, on_ground)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use survivalforest_core::block::STONE;
    use survivalforest_core::{BlockPos, ChunkPos};
    use survivalforest_world::ChunkColumn;

    /// Suelo de piedra en y=60 en 3×3 chunks alrededor del origen.
    fn flat() -> HashMap<ChunkPos, ChunkColumn> {
        let mut m = HashMap::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                let mut c = ChunkColumn::new(ChunkPos::new(cx, cz));
                for x in 0..16 {
                    for z in 0..16 {
                        c.set(BlockPos::new(cx * 16 + x, 60, cz * 16 + z).local(), STONE);
                    }
                }
                m.insert(c.pos, c);
            }
        }
        m
    }

    fn put(m: &mut HashMap<ChunkPos, ChunkColumn>, x: i32, y: i32, z: i32) {
        let p = BlockPos::new(x, y, z);
        m.get_mut(&p.chunk()).unwrap().set(p.local(), STONE);
    }

    fn sim(
        m: &HashMap<ChunkPos, ChunkColumn>,
        mut b: Aabb,
        mut vel: DVec3,
        secs: f64,
    ) -> (Aabb, DVec3, bool) {
        let dt = 1.0 / 20.0;
        let mut ground = false;
        for _ in 0..(secs / dt) as usize {
            vel.y -= GRAVITY * dt;
            let (nb, nv, g) = move_and_collide(m, b, vel * dt);
            b = nb;
            vel = nv / dt;
            ground = g;
        }
        (b, vel, ground)
    }

    #[test]
    fn falls_and_lands_on_floor() {
        let m = flat();
        let (b, v, g) = sim(
            &m,
            Aabb::player(DVec3::new(0.5, 70.0, 0.5)),
            DVec3::ZERO,
            3.0,
        );
        assert!(g, "en el suelo");
        assert!((b.min.y - 61.0).abs() < 1e-6, "pies en {}", b.min.y);
        assert!(v.y.abs() < 1e-6);
    }

    #[test]
    fn cannot_walk_through_wall() {
        let mut m = flat();
        for y in 61..64 {
            for z in -3..4 {
                put(&mut m, 3, y, z);
            }
        }
        let (b, _, _) = sim(
            &m,
            Aabb::player(DVec3::new(0.5, 61.0, 0.5)),
            DVec3::new(WALK_SPEED, 0.0, 0.0),
            3.0,
        );
        assert!(b.max.x <= 3.0 + 1e-9, "atravesó la pared: {}", b.max.x);
        assert!(b.max.x > 2.9);
    }

    #[test]
    fn steps_into_one_block_gap_without_jitter() {
        let mut m = flat();
        // Pasillo de 1 bloque de ancho en z=0, con paredes en z=-1 y z=1.
        for x in 2..10 {
            for y in 61..64 {
                put(&mut m, x, y, -1);
                put(&mut m, x, y, 1);
            }
        }
        let (b, _, g) = sim(
            &m,
            Aabb::player(DVec3::new(0.5, 61.0, 0.5)),
            DVec3::new(WALK_SPEED, 0.0, 0.0),
            2.0,
        );
        assert!(g);
        assert!(b.min.x > 5.0, "no avanzó por el pasillo: {}", b.min.x);
        assert!(b.min.z >= 0.0 && b.max.z <= 1.0);
    }

    #[test]
    fn unloaded_chunk_is_solid() {
        let m = flat();
        let (b, _, _) = sim(
            &m,
            Aabb::player(DVec3::new(40.0, 61.0, 0.5)),
            DVec3::new(WALK_SPEED, 0.0, 0.0),
            3.0,
        );
        assert!(
            b.max.x <= 48.0 + 1e-9,
            "salió a un chunk sin cargar: {}",
            b.max.x
        );
    }

    #[test]
    fn diagonal_corner_does_not_tunnel() {
        let mut m = flat();
        for y in 61..64 {
            put(&mut m, 3, y, 3);
        }
        let start = Aabb::player(DVec3::new(1.5, 61.0, 1.5));
        let (b, _, _) = sim(&m, start, DVec3::new(30.0, 0.0, 30.0), 0.5);
        let inside =
            b.max.x > 3.0 && b.min.x < 4.0 && b.max.z > 3.0 && b.min.z < 4.0 && b.min.y < 64.0;
        assert!(!inside, "dentro del bloque: {b:?}");
    }
}
