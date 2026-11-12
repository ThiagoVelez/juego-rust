//! Vegetación: árboles de copa redonda, guaduales, cafetales y frailejones.

use super::carve::Rng;
use super::climate::mix;
use super::{Biome, SEA_LEVEL, WorldGen};
use crate::ChunkColumn;
use survivalforest_core::block::{AIR, BAMBOO, COFFEE_BUSH, FRAILEJON, GRASS, LEAVES, LOG};
use survivalforest_core::{BlockId, LocalPos, MIN_Y};

fn put(col: &mut ChunkColumn, x: i32, y: i32, z: i32, b: BlockId, only_air: bool) {
    if !(0..16).contains(&x) || !(0..16).contains(&z) || y <= MIN_Y || y >= 320 {
        return;
    }
    let p = LocalPos::new(x as u8, (y - MIN_Y) as u16, z as u8);
    if !only_air || col.get(p) == AIR {
        col.set_raw(p, b);
    }
}

fn tree(col: &mut ChunkColumn, x: i32, y: i32, z: i32, rng: &mut Rng) {
    let th = rng.range(4, 7);
    for t in 1..=th {
        put(col, x, y + t, z, LOG, false);
    }
    for dy in (th - 2)..=(th + 2) {
        let r: f64 = if dy > th + 1 {
            1.2
        } else if dy > th {
            2.3
        } else {
            2.9
        };
        for dz in -3i32..=3 {
            for dx in -3i32..=3 {
                if ((dx * dx + dz * dz) as f64) <= r * r {
                    put(col, x + dx, y + dy, z + dz, LEAVES, true);
                }
            }
        }
    }
}

pub fn decorate(col: &mut ChunkColumn, wg: &WorldGen) {
    let o = col.pos.origin();
    for lz in 0..16i32 {
        for lx in 0..16i32 {
            let (wx, wz) = (o.x + lx, o.z + lz);
            let h = col.height(lx as u8, lz as u8);
            if h <= SEA_LEVEL {
                continue;
            }
            let ground = LocalPos::new(lx as u8, (h - MIN_Y) as u16, lz as u8);
            if col.get(ground) != GRASS {
                continue;
            }
            let mut rng = Rng::new(mix(
                wg.seed ^ 0x7EE5,
                ((wx as i64 as u64) << 32) ^ (wz as i64 as u64 & 0xFFFF_FFFF),
            ));
            let biome = wg.biome_at(wx, wz);
            let inside = (3..13).contains(&lx) && (3..13).contains(&lz);
            let tree_p = match biome {
                Biome::Rainforest | Biome::CloudForest => 0.035,
                Biome::Mangrove => 0.02,
                Biome::CoffeeMountain => 0.006,
                Biome::BambooValley => 0.004,
                Biome::Savanna => 0.002,
                _ => 0.0,
            };
            if inside && rng.chance(tree_p) {
                tree(col, lx, h, lz, &mut rng);
                continue;
            }
            match biome {
                Biome::CoffeeMountain if wx.rem_euclid(3) == 0 && rng.chance(0.7) => {
                    put(col, lx, h + 1, lz, COFFEE_BUSH, true);
                    if rng.chance(0.4) {
                        put(col, lx, h + 2, lz, COFFEE_BUSH, true);
                    }
                }
                Biome::BambooValley | Biome::Rainforest
                    if rng.chance(if biome == Biome::BambooValley {
                        0.05
                    } else {
                        0.006
                    }) =>
                {
                    let gh = rng.range(5, 11);
                    for t in 1..=gh {
                        put(col, lx, h + t, lz, BAMBOO, true);
                    }
                    put(col, lx, h + gh + 1, lz, LEAVES, true);
                }
                Biome::Paramo if rng.chance(0.03) => {
                    put(col, lx, h + 1, lz, FRAILEJON, true);
                    if rng.chance(0.5) {
                        put(col, lx, h + 2, lz, FRAILEJON, true);
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Biome, WorldGen};
    use survivalforest_core::block::COFFEE_BUSH;
    use survivalforest_core::{BlockPos, LocalPos};

    #[test]
    fn coffee_rows_in_coffee_mountain() {
        let g = WorldGen::new(42);
        let mut at = None;
        'o: for i in -40..40 {
            for j in -40..40 {
                let (x, z) = (i * 48 + 8, j * 48 + 8);
                if g.biome_at(x, z) == Biome::CoffeeMountain
                    && g.biome_at(x + 7, z + 7) == Biome::CoffeeMountain
                {
                    at = Some(BlockPos::new(x, 0, z));
                    break 'o;
                }
            }
        }
        let c = g.generate(at.expect("hay montaña cafetera").chunk());
        let mut n = 0;
        for ly in 0..384u16 {
            for z in 0..16u8 {
                for x in 0..16u8 {
                    n += (c.get(LocalPos::new(x, ly, z)) == COFFEE_BUSH) as usize;
                }
            }
        }
        assert!(n >= 4, "solo {n} cafetos");
    }
}
