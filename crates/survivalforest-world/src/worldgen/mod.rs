//! Generación procedural determinista: semilla → columna de chunk.

mod biome;
mod climate;

pub use biome::Biome;

use crate::ChunkColumn;
use climate::{Climate, ClimateSample};
use survivalforest_core::block::{BEDROCK, GRAVEL, ICE, STONE, WATER};
use survivalforest_core::{CHUNK, ChunkPos, LocalPos, MIN_Y};

pub const SEA_LEVEL: i32 = 62;

/// Generador del mundo. Es `Send + Sync` y se comparte entre hilos.
pub struct WorldGen {
    pub seed: u64,
    climate: Climate,
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Curva continentalidad → altura base.
fn base_height(c: f64) -> f64 {
    const PTS: [(f64, f64); 7] = [
        (-1.0, 20.0),
        (-0.45, 34.0),
        (-0.25, 52.0),
        (-0.12, 61.0),
        (0.0, 68.0),
        (0.35, 84.0),
        (1.0, 120.0),
    ];
    for w in PTS.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if c <= x1 {
            return lerp(y0, y1, ((c - x0) / (x1 - x0)).clamp(0.0, 1.0));
        }
    }
    PTS[PTS.len() - 1].1
}

impl WorldGen {
    pub fn new(seed: u64) -> Self {
        WorldGen {
            seed,
            climate: Climate::new(seed),
        }
    }

    fn height_from(&self, s: &ClimateSample) -> i32 {
        let base = base_height(s.continental);
        // Montañas donde la erosión es baja y el terreno es continental.
        let inland = ((s.continental + 0.05) / 0.4).clamp(0.0, 1.0);
        let rugged = ((-s.erosion - 0.05) / 0.7).clamp(0.0, 1.0);
        let ridge = 1.0 - s.weird.abs();
        let mountain = inland * rugged * rugged * (0.55 + 0.45 * ridge) * 190.0;
        let h = base + mountain + s.detail * 8.0;
        (h.round() as i32).clamp(MIN_Y + 8, 300)
    }

    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        self.height_from(&self.climate.sample(x, z))
    }

    pub fn biome_at(&self, x: i32, z: i32) -> Biome {
        let s = self.climate.sample(x, z);
        Biome::pick(
            self.height_from(&s),
            SEA_LEVEL,
            s.temperature,
            s.humidity,
            s.weird,
        )
    }

    pub fn generate(&self, pos: ChunkPos) -> ChunkColumn {
        let mut col = ChunkColumn::new(pos);
        let o = pos.origin();
        for lz in 0..CHUNK {
            for lx in 0..CHUNK {
                let (wx, wz) = (o.x + lx, o.z + lz);
                let s = self.climate.sample(wx, wz);
                let h = self.height_from(&s);
                let biome = Biome::pick(h, SEA_LEVEL, s.temperature, s.humidity, s.weird);
                let (mut top, mut sub) = biome.surface();
                if h < SEA_LEVEL && biome != Biome::Mangrove {
                    let deep = h < SEA_LEVEL - 12;
                    top = if deep {
                        GRAVEL
                    } else {
                        survivalforest_core::block::SAND
                    };
                    sub = top;
                }
                for y in MIN_Y..=h {
                    let b = if y == MIN_Y {
                        BEDROCK
                    } else if y < h - 3 {
                        STONE
                    } else if y < h {
                        sub
                    } else {
                        top
                    };
                    col.set_raw(LocalPos::new(lx as u8, (y - MIN_Y) as u16, lz as u8), b);
                }
                for y in (h + 1)..=SEA_LEVEL {
                    let b = if y == SEA_LEVEL && biome == Biome::SnowPeak {
                        ICE
                    } else {
                        WATER
                    };
                    col.set_raw(LocalPos::new(lx as u8, (y - MIN_Y) as u16, lz as u8), b);
                }
            }
        }
        col.rebuild_heightmap();
        col
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use survivalforest_core::block::{BEDROCK, WATER};
    use survivalforest_core::{ChunkPos, LocalPos};

    fn hash(c: &crate::ChunkColumn) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for y in 0..384u16 {
            for z in 0..16u8 {
                for x in 0..16u8 {
                    c.get(LocalPos::new(x, y, z)).hash(&mut h);
                }
            }
        }
        h.finish()
    }

    #[test]
    fn same_seed_same_chunk() {
        let a = WorldGen::new(1234).generate(ChunkPos::new(3, -7));
        let b = WorldGen::new(1234).generate(ChunkPos::new(3, -7));
        assert_eq!(hash(&a), hash(&b));
    }

    #[test]
    fn different_seed_differs() {
        let a = WorldGen::new(1).generate(ChunkPos::new(0, 0));
        let b = WorldGen::new(2).generate(ChunkPos::new(0, 0));
        assert_ne!(hash(&a), hash(&b));
    }

    #[test]
    fn border_continuity() {
        let g = WorldGen::new(99);
        for z in 0..64 {
            let a = g.surface_height(15, z);
            let b = g.surface_height(16, z);
            assert!((a - b).abs() <= 6, "z={z}: {a} vs {b}");
        }
    }

    #[test]
    fn far_lands_still_generate() {
        let g = WorldGen::new(5);
        let c = g.generate(ChunkPos::new(1_874_937, -1_874_937));
        assert_eq!(c.get(LocalPos::new(0, 0, 0)), BEDROCK);
        let h = c.height(8, 8);
        assert!((-56..=300).contains(&h), "altura {h}");
    }

    #[test]
    fn biome_variety() {
        let g = WorldGen::new(42);
        let mut seen = std::collections::HashSet::new();
        for i in 0..64 {
            for j in 0..64 {
                seen.insert(g.biome_at(i * 128 - 4096, j * 128 - 4096));
            }
        }
        assert!(seen.len() >= 8, "solo {} biomas: {seen:?}", seen.len());
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn gen_speed() {
        let g = WorldGen::new(7);
        let t = std::time::Instant::now();
        for i in 0..64 {
            std::hint::black_box(g.generate(ChunkPos::new(i % 8, i / 8)));
        }
        assert!(t.elapsed().as_secs_f64() < 2.0, "{:?}", t.elapsed());
    }

    #[test]
    fn ocean_has_water_at_sea_level() {
        let g = WorldGen::new(42);
        let mut found = None;
        'o: for i in -64..64 {
            for j in -64..64 {
                if g.biome_at(i * 64, j * 64) == Biome::Ocean {
                    found = Some((i * 64, j * 64));
                    break 'o;
                }
            }
        }
        let (x, z) = found.expect("hay océano");
        let pos = survivalforest_core::BlockPos::new(x, SEA_LEVEL, z);
        let c = g.generate(pos.chunk());
        assert_eq!(c.get(pos.local()), WATER);
    }
}

#[cfg(test)]
mod probe {
    #[test]
    #[ignore]
    fn print_stats() {
        let g = super::WorldGen::new(42);
        let mut m = std::collections::BTreeMap::new();
        let (mut lo, mut hi) = (999, -999);
        for i in 0..128 {
            for j in 0..128 {
                let (x, z) = (i * 128 - 8192, j * 128 - 8192);
                *m.entry(format!("{:?}", g.biome_at(x, z))).or_insert(0) += 1;
                let h = g.surface_height(x, z);
                lo = lo.min(h);
                hi = hi.max(h);
            }
        }
        println!("{m:?} alturas {lo}..{hi}");
    }
}
