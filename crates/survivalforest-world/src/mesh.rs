//! Mallado por caras visibles con oclusión ambiental por vértice.

use crate::ChunkColumn;
use survivalforest_core::block::{AIR, BlockId, STONE, WATER};
use survivalforest_core::{LocalPos, MAX_Y, MIN_Y};

#[derive(Default, Debug, Clone)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub layer: Vec<u32>,
    /// (cielo, bloque) en 0..1.
    pub light: Vec<[f32; 2]>,
    pub ao: Vec<f32>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

#[derive(Default, Debug, Clone)]
pub struct ChunkMeshes {
    pub opaque: MeshData,
    /// Hojas y plantas (descarte alfa).
    pub cutout: MeshData,
    pub water: MeshData,
}

const AO: [f32; 4] = [0.5, 0.68, 0.85, 1.0];

struct Face {
    n: [i32; 3],
    u: [i32; 3],
    v: [i32; 3],
    /// 0 arriba, 1 lado, 2 abajo.
    tex: usize,
}

const FACES: [Face; 6] = [
    Face {
        n: [1, 0, 0],
        u: [0, 0, 1],
        v: [0, 1, 0],
        tex: 1,
    },
    Face {
        n: [-1, 0, 0],
        u: [0, 0, 1],
        v: [0, 1, 0],
        tex: 1,
    },
    Face {
        n: [0, 1, 0],
        u: [1, 0, 0],
        v: [0, 0, 1],
        tex: 0,
    },
    Face {
        n: [0, -1, 0],
        u: [1, 0, 0],
        v: [0, 0, 1],
        tex: 2,
    },
    Face {
        n: [0, 0, 1],
        u: [1, 0, 0],
        v: [0, 1, 0],
        tex: 1,
    },
    Face {
        n: [0, 0, -1],
        u: [1, 0, 0],
        v: [0, 1, 0],
        tex: 1,
    },
];

/// Vista de la columna con sus cuatro vecinos (+x, -x, +z, -z).
struct View<'a> {
    col: &'a ChunkColumn,
    nb: [Option<&'a ChunkColumn>; 4],
}

impl View<'_> {
    /// Bloque en coordenadas locales extendidas (x, z en -1..=16).
    fn block(&self, x: i32, y: i32, z: i32) -> BlockId {
        if y < MIN_Y {
            return STONE;
        }
        if y >= MAX_Y {
            return AIR;
        }
        let ly = (y - MIN_Y) as u16;
        let (c, lx, lz) = match (x, z) {
            (16, _) => (self.nb[0], 0, z),
            (-1, _) => (self.nb[1], 15, z),
            (_, 16) => (self.nb[2], x, 0),
            (_, -1) => (self.nb[3], x, 15),
            _ => (Some(self.col), x, z),
        };
        if !(0..16).contains(&lx) || !(0..16).contains(&lz) {
            return STONE;
        }
        match c {
            Some(c) => c.get(LocalPos::new(lx as u8, ly, lz as u8)),
            None => STONE,
        }
    }
    fn light(&self, x: i32, y: i32, z: i32) -> (u8, u8) {
        if y >= MAX_Y {
            return (15, 0);
        }
        if y < MIN_Y {
            return (0, 0);
        }
        let ly = (y - MIN_Y) as u16;
        let (c, lx, lz) = match (x, z) {
            (16, _) => (self.nb[0], 0, z),
            (-1, _) => (self.nb[1], 15, z),
            (_, 16) => (self.nb[2], x, 0),
            (_, -1) => (self.nb[3], x, 15),
            _ => (Some(self.col), x, z),
        };
        match c {
            Some(c) if (0..16).contains(&lx) && (0..16).contains(&lz) => {
                let p = LocalPos::new(lx as u8, ly, lz as u8);
                (c.sky(p), c.block_light(p))
            }
            _ => (15, 0),
        }
    }
    fn occ(&self, x: i32, y: i32, z: i32) -> u8 {
        self.block(x, y, z).def().opaque as u8
    }
}

fn is_cutout(b: BlockId) -> bool {
    let d = b.def();
    !d.opaque && b != AIR && b != WATER
}

pub fn mesh_column(col: &ChunkColumn, neighbors: [Option<&ChunkColumn>; 4]) -> ChunkMeshes {
    let view = View { col, nb: neighbors };
    let mut out = ChunkMeshes::default();
    for (si, sec) in col.sections.iter().enumerate() {
        if sec.is_empty() {
            continue;
        }
        for ly in 0..16 {
            for lz in 0..16 {
                for lx in 0..16 {
                    let b = sec.get(lx, ly, lz);
                    if b == AIR {
                        continue;
                    }
                    let (x, y, z) = (lx as i32, MIN_Y + (si * 16 + ly) as i32, lz as i32);
                    emit_block(&view, &mut out, b, x, y, z);
                }
            }
        }
    }
    out
}

fn emit_block(view: &View, out: &mut ChunkMeshes, b: BlockId, x: i32, y: i32, z: i32) {
    let def = b.def();
    let water = b == WATER;
    let cut = is_cutout(b);
    let water_top_lowered = water && view.block(x, y + 1, z) != WATER;
    for f in &FACES {
        let (nx, ny, nz) = (x + f.n[0], y + f.n[1], z + f.n[2]);
        let nb = view.block(nx, ny, nz);
        let visible = if water {
            nb == AIR
        } else {
            !nb.def().opaque && (nb != b || cut)
        };
        if !visible {
            continue;
        }
        let target = if water {
            &mut out.water
        } else if cut {
            &mut out.cutout
        } else {
            &mut out.opaque
        };
        let (sky, blk) = view.light(nx, ny, nz);
        let base = target.positions.len() as u32;
        let pos_side = f.n.iter().any(|&c| c > 0) as i32;
        let mut aos = [0usize; 4];
        for (i, (cu, cv)) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate() {
            let su = if cu == 1 { 1 } else { -1 };
            let sv = if cv == 1 { 1 } else { -1 };
            let s1 = view.occ(nx + f.u[0] * su, ny + f.u[1] * su, nz + f.u[2] * su);
            let s2 = view.occ(nx + f.v[0] * sv, ny + f.v[1] * sv, nz + f.v[2] * sv);
            let c = view.occ(
                nx + f.u[0] * su + f.v[0] * sv,
                ny + f.u[1] * su + f.v[1] * sv,
                nz + f.u[2] * su + f.v[2] * sv,
            );
            let ao = if water {
                3
            } else if s1 == 1 && s2 == 1 {
                0
            } else {
                3 - (s1 + s2 + c) as usize
            };
            aos[i] = ao;
            let mut p = [0f32; 3];
            for k in 0..3 {
                let base_c = [x, y, z][k] + if f.n[k] != 0 { pos_side } else { 0 };
                p[k] = (base_c + f.u[k] * cu + f.v[k] * cv) as f32;
            }
            if water_top_lowered && p[1] > y as f32 + 0.5 {
                p[1] -= 0.125;
            }
            let vflip = if f.n[1] == 0 { 1 - cv } else { cv };
            target.positions.push(p);
            target
                .normals
                .push([f.n[0] as f32, f.n[1] as f32, f.n[2] as f32]);
            target.uvs.push([cu as f32, vflip as f32]);
            target.layer.push(def.tex[f.tex] as u32);
            target.light.push([sky as f32 / 15.0, blk as f32 / 15.0]);
            target.ao.push(AO[ao]);
        }
        // Cara orientada hacia fuera: el orden depende del signo de la normal.
        let flip = (aos[0] + aos[2]) < (aos[1] + aos[3]);
        let outward = {
            // u × v apunta en la dirección de la normal (+) o al revés (-).
            let cross = [
                f.u[1] * f.v[2] - f.u[2] * f.v[1],
                f.u[2] * f.v[0] - f.u[0] * f.v[2],
                f.u[0] * f.v[1] - f.u[1] * f.v[0],
            ];
            cross[0] * f.n[0] + cross[1] * f.n[1] + cross[2] * f.n[2] > 0
        };
        let tri: [u32; 6] = match (flip, outward) {
            (false, true) => [0, 1, 2, 0, 2, 3],
            (true, true) => [1, 2, 3, 1, 3, 0],
            (false, false) => [0, 2, 1, 0, 3, 2],
            (true, false) => [1, 3, 2, 1, 0, 3],
        };
        target.indices.extend(tri.iter().map(|i| base + i));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkColumn;
    use survivalforest_core::block::{STONE, WATER};
    use survivalforest_core::{BlockPos, ChunkPos};

    fn col_with(blocks: &[((i32, i32, i32), survivalforest_core::BlockId)]) -> ChunkColumn {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        for &((x, y, z), b) in blocks {
            c.set(BlockPos::new(x, y, z).local(), b);
        }
        c
    }

    fn faces(m: &MeshData) -> usize {
        m.positions.len() / 4
    }

    #[test]
    fn single_block_has_6_faces() {
        let c = col_with(&[((5, 100, 5), STONE)]);
        let m = mesh_column(&c, [None, None, None, None]);
        assert_eq!(m.opaque.positions.len(), 24);
        assert_eq!(m.opaque.indices.len(), 36);
        assert_eq!(m.opaque.normals.len(), 24);
        assert_eq!(m.opaque.uvs.len(), 24);
        assert_eq!(m.opaque.layer.len(), 24);
    }

    #[test]
    fn two_adjacent_blocks_have_10_faces() {
        let c = col_with(&[((5, 100, 5), STONE), ((6, 100, 5), STONE)]);
        let m = mesh_column(&c, [None, None, None, None]);
        assert_eq!(faces(&m.opaque), 10);
    }

    #[test]
    fn water_only_faces_air() {
        let c = col_with(&[((5, 100, 5), WATER), ((6, 100, 5), STONE)]);
        let m = mesh_column(&c, [None, None, None, None]);
        assert_eq!(faces(&m.water), 5);
        let top = m
            .water
            .positions
            .iter()
            .map(|p| p[1])
            .fold(f32::MIN, f32::max);
        assert!((top - 100.875).abs() < 1e-4, "superficie del agua a {top}");
    }

    #[test]
    fn missing_neighbor_hides_border() {
        let c = col_with(&[((0, 100, 5), STONE)]);
        let without = mesh_column(&c, [None, None, None, None]);
        assert_eq!(faces(&without.opaque), 5);
        let empty = ChunkColumn::new(ChunkPos::new(-1, 0));
        let with = mesh_column(&c, [None, Some(&empty), None, None]);
        assert_eq!(faces(&with.opaque), 6);
    }

    #[test]
    fn ao_corner_darkens() {
        let c = col_with(&[
            ((5, 100, 5), STONE),
            ((6, 101, 5), STONE),
            ((5, 101, 6), STONE),
        ]);
        let m = mesh_column(&c, [None, None, None, None]);
        assert!(m.opaque.ao.iter().any(|&a| (a - 0.5).abs() < 1e-6));
        assert!(m.opaque.ao.iter().all(|&a| (0.5..=1.0).contains(&a)));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn mesh_full_chunk_under_20ms_release() {
        let g = crate::WorldGen::new(42);
        let mut c = g.generate(ChunkPos::new(2, 3));
        crate::light_column(&mut c);
        let t = std::time::Instant::now();
        let m = mesh_column(&c, [None, None, None, None]);
        let e = t.elapsed();
        assert!(!m.opaque.positions.is_empty());
        assert!(e.as_millis() < 20, "{e:?}");
    }

    #[test]
    fn triangles_face_outward() {
        let c = col_with(&[
            ((5, 100, 5), STONE),
            ((6, 101, 5), STONE),
            ((5, 101, 6), STONE),
            ((4, 99, 4), STONE),
        ]);
        let m = mesh_column(&c, [None, None, None, None]).opaque;
        for t in m.indices.chunks(3) {
            let [a, b, c] = [
                m.positions[t[0] as usize],
                m.positions[t[1] as usize],
                m.positions[t[2] as usize],
            ];
            let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let fnorm = m.normals[t[0] as usize];
            let dot = n[0] * fnorm[0] + n[1] * fnorm[1] + n[2] * fnorm[2];
            assert!(dot > 0.0, "triángulo al revés con normal {fnorm:?}");
        }
    }
}
