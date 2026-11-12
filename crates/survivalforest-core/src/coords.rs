use serde::{Deserialize, Serialize};

pub const CHUNK: i32 = 16;
pub const MIN_Y: i32 = -64;
/// Exclusivo.
pub const MAX_Y: i32 = 320;
pub const HEIGHT: i32 = MAX_Y - MIN_Y;
pub const SECTIONS: usize = (HEIGHT / CHUNK) as usize;
pub const WORLD_LIMIT: i32 = 30_000_000;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct LocalPos {
    pub x: u8,
    /// Cuenta desde `MIN_Y`.
    pub y: u16,
    pub z: u8,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub fn chunk(self) -> ChunkPos {
        ChunkPos {
            x: self.x.div_euclid(CHUNK),
            z: self.z.div_euclid(CHUNK),
        }
    }
    pub fn local(self) -> LocalPos {
        LocalPos {
            x: self.x.rem_euclid(CHUNK) as u8,
            y: (self.y - MIN_Y) as u16,
            z: self.z.rem_euclid(CHUNK) as u8,
        }
    }
    pub fn offset(self, dx: i32, dy: i32, dz: i32) -> BlockPos {
        BlockPos {
            x: self.x + dx,
            y: self.y + dy,
            z: self.z + dz,
        }
    }
    /// ¿Está dentro de la altura del mundo?
    pub fn in_height(self) -> bool {
        (MIN_Y..MAX_Y).contains(&self.y)
    }
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
    pub fn origin(self) -> BlockPos {
        BlockPos {
            x: self.x * CHUNK,
            y: MIN_Y,
            z: self.z * CHUNK,
        }
    }
    /// Este, oeste, sur, norte: (+x, -x, +z, -z).
    pub fn neighbors(self) -> [ChunkPos; 4] {
        [
            ChunkPos {
                x: self.x + 1,
                z: self.z,
            },
            ChunkPos {
                x: self.x - 1,
                z: self.z,
            },
            ChunkPos {
                x: self.x,
                z: self.z + 1,
            },
            ChunkPos {
                x: self.x,
                z: self.z - 1,
            },
        ]
    }
    pub fn block(self, l: LocalPos) -> BlockPos {
        BlockPos {
            x: self.x * CHUNK + l.x as i32,
            y: l.y as i32 + MIN_Y,
            z: self.z * CHUNK + l.z as i32,
        }
    }
    pub fn dist2(self, o: ChunkPos) -> i64 {
        let dx = (self.x - o.x) as i64;
        let dz = (self.z - o.z) as i64;
        dx * dx + dz * dz
    }
}

impl LocalPos {
    pub const fn new(x: u8, y: u16, z: u8) -> Self {
        Self { x, y, z }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_block_maps_to_previous_chunk() {
        let p = BlockPos {
            x: -1,
            y: 0,
            z: -17,
        };
        assert_eq!(p.chunk(), ChunkPos { x: -1, z: -2 });
        assert_eq!(
            p.local(),
            LocalPos {
                x: 15,
                y: 64,
                z: 15
            }
        );
    }

    #[test]
    fn chunk_origin_roundtrip() {
        for (x, z) in [(0, 0), (-1, 5), (1_874_999, -1_874_999)] {
            let c = ChunkPos { x, z };
            assert_eq!(c.origin().chunk(), c);
            assert_eq!(c.origin().y, MIN_Y);
        }
    }

    #[test]
    fn local_back_to_world() {
        let p = BlockPos {
            x: -33,
            y: 200,
            z: 47,
        };
        assert_eq!(p.chunk().block(p.local()), p);
    }

    #[test]
    fn sizes() {
        assert_eq!(SECTIONS, 24);
    }
}
