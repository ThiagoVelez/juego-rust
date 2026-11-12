//! Cuevas (ruido 3D tipo «queso» y túneles «espagueti») y vetas de menas.

use super::WorldGen;
use super::climate::mix;
use crate::ChunkColumn;
use noise::{NoiseFn, Perlin};
use survivalforest_core::block::{
    AIR, BEDROCK, COAL_ORE, COPPER_ORE, GOLD_ORE, IRON_ORE, QUARTZ_ORE, STONE, WATER,
};
use survivalforest_core::{BlockId, LocalPos, MIN_Y};

/// Generador pseudoaleatorio pequeño y determinista (splitmix64).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0, 0)
    }
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next_u64() % (hi - lo).max(1) as u64) as i32
    }
    pub fn chance(&mut self, p: f64) -> bool {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 * 1.0 < p
    }
}

pub struct CaveNoise {
    cheese: Perlin,
    spag_a: Perlin,
    spag_b: Perlin,
}

impl CaveNoise {
    pub fn new(seed: u64) -> Self {
        CaveNoise {
            cheese: Perlin::new(mix(seed, 101) as u32),
            spag_a: Perlin::new(mix(seed, 102) as u32),
            spag_b: Perlin::new(mix(seed, 103) as u32),
        }
    }
    /// > 0 = aire.
    fn density(&self, x: f64, y: f64, z: f64) -> f64 {
        let cheese = self.cheese.get([x / 72.0, y / 36.0, z / 72.0]) - 0.42;
        let a = self.spag_a.get([x / 56.0, y / 40.0, z / 56.0]);
        let b = self.spag_b.get([x / 56.0, y / 40.0, z / 56.0]);
        let spag = 0.075 - (a * a + b * b).sqrt();
        cheese.max(spag)
    }
}

const STEP: usize = 4;

pub fn carve_caves(col: &mut ChunkColumn, wg: &WorldGen) {
    let o = col.pos.origin();
    let ny = 384 / STEP + 1;
    let n = 16 / STEP + 1;
    let mut grid = vec![0.0f64; n * n * ny];
    for gy in 0..ny {
        for gz in 0..n {
            for gx in 0..n {
                let (x, y, z) = (
                    o.x + (gx * STEP) as i32,
                    MIN_Y + (gy * STEP) as i32,
                    o.z + (gz * STEP) as i32,
                );
                grid[(gy * n + gz) * n + gx] = wg.caves.density(x as f64, y as f64, z as f64);
            }
        }
    }
    let at = |gx: usize, gy: usize, gz: usize| grid[(gy * n + gz) * n + gx];
    for lz in 0..16u8 {
        for lx in 0..16u8 {
            let h = col.height(lx, lz);
            let underwater =
                col.get(LocalPos::new(lx, (h + 1 - MIN_Y).clamp(0, 383) as u16, lz)) == WATER;
            let top = if underwater { h - 5 } else { h };
            let (gx, fx) = (
                lx as usize / STEP,
                (lx as usize % STEP) as f64 / STEP as f64,
            );
            let (gz, fz) = (
                lz as usize / STEP,
                (lz as usize % STEP) as f64 / STEP as f64,
            );
            for y in (MIN_Y + 1)..=top {
                let ly = (y - MIN_Y) as usize;
                let (gy, fy) = (ly / STEP, (ly % STEP) as f64 / STEP as f64);
                let l = |a: f64, b: f64, t: f64| a + (b - a) * t;
                let c00 = l(at(gx, gy, gz), at(gx + 1, gy, gz), fx);
                let c10 = l(at(gx, gy + 1, gz), at(gx + 1, gy + 1, gz), fx);
                let c01 = l(at(gx, gy, gz + 1), at(gx + 1, gy, gz + 1), fx);
                let c11 = l(at(gx, gy + 1, gz + 1), at(gx + 1, gy + 1, gz + 1), fx);
                let d = l(l(c00, c10, fy), l(c01, c11, fy), fz);
                // Menos queso cerca de la superficie: las entradas las abren los túneles.
                let near_surface = y > h - 6 && d < 0.12;
                if d > 0.0 && !near_surface {
                    let p = LocalPos::new(lx, ly as u16, lz);
                    let b = col.get(p);
                    if b != BEDROCK && b != WATER {
                        col.set_raw(p, AIR);
                    }
                }
            }
        }
    }
}

/// (mena, y mínima, y máxima exclusiva, vetas por chunk, tamaño máximo)
const ORES: [(BlockId, i32, i32, u32, i32); 5] = [
    (COAL_ORE, 0, 192, 20, 17),
    (COPPER_ORE, -16, 112, 10, 10),
    (IRON_ORE, -64, 72, 12, 9),
    (GOLD_ORE, -64, 32, 4, 8),
    (QUARTZ_ORE, -64, 0, 3, 8),
];

pub fn place_ores(col: &mut ChunkColumn, seed: u64) {
    let mut rng = Rng::new(mix(
        seed,
        (col.pos.x as i64 as u64).wrapping_mul(31) ^ ((col.pos.z as i64 as u64) << 21) ^ 0xABCD,
    ));
    for (ore, lo, hi, veins, size) in ORES {
        for _ in 0..veins {
            let (mut x, mut y, mut z) = (rng.range(0, 16), rng.range(lo, hi), rng.range(0, 16));
            let n = rng.range(size / 2, size + 1);
            for _ in 0..n {
                if (0..16).contains(&x) && (0..16).contains(&z) && y >= lo && y < hi && y > MIN_Y {
                    let p = LocalPos::new(x as u8, (y - MIN_Y) as u16, z as u8);
                    if col.get(p) == STONE {
                        col.set_raw(p, ore);
                    }
                }
                match rng.range(0, 6) {
                    0 => x += 1,
                    1 => x -= 1,
                    2 => y += 1,
                    3 => y -= 1,
                    4 => z += 1,
                    _ => z -= 1,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::WorldGen;
    use survivalforest_core::block::{AIR, BEDROCK, COAL_ORE, GOLD_ORE, IRON_ORE};
    use survivalforest_core::{ChunkPos, LocalPos, MIN_Y};

    fn count(
        g: &WorldGen,
        n: i32,
        f: impl Fn(i32, survivalforest_core::BlockId) -> bool,
    ) -> (usize, usize) {
        let (mut hit, mut tot) = (0, 0);
        for i in 0..n {
            let c = g.generate(ChunkPos::new(i % 4 - 2, i / 4 - 2));
            for ly in 0..384u16 {
                for z in 0..16u8 {
                    for x in 0..16u8 {
                        let y = ly as i32 + MIN_Y;
                        let b = c.get(LocalPos::new(x, ly, z));
                        tot += 1;
                        if f(y, b) {
                            hit += 1;
                        }
                    }
                }
            }
        }
        (hit, tot)
    }

    #[test]
    fn caves_exist() {
        let g = WorldGen::new(7);
        let (air, _) = count(&g, 16, |y, b| y > MIN_Y && y < 50 && b == AIR);
        let below = 16 * 256 * (50 - MIN_Y - 1) as usize;
        assert!(
            air as f64 / below as f64 >= 0.02,
            "aire bajo y=50: {air}/{below}"
        );
    }

    #[test]
    fn ores_respect_ranges() {
        let g = WorldGen::new(11);
        let (bad_gold, _) = count(&g, 32, |y, b| b == GOLD_ORE && y > 32);
        let (bad_iron, _) = count(&g, 32, |y, b| b == IRON_ORE && y > 72);
        assert_eq!((bad_gold, bad_iron), (0, 0));
        for i in 0..32 {
            let c = g.generate(ChunkPos::new(i, 3));
            let mut coal = 0;
            for ly in 0..384u16 {
                for z in 0..16u8 {
                    for x in 0..16u8 {
                        coal += (c.get(LocalPos::new(x, ly, z)) == COAL_ORE) as usize;
                    }
                }
            }
            assert!(coal >= 1, "chunk {i} sin carbón");
        }
    }

    #[test]
    fn bedrock_intact() {
        let g = WorldGen::new(3);
        for i in 0..8 {
            let c = g.generate(ChunkPos::new(i, -i));
            for z in 0..16u8 {
                for x in 0..16u8 {
                    assert_eq!(c.get(LocalPos::new(x, 0, z)), BEDROCK);
                }
            }
        }
    }
}
