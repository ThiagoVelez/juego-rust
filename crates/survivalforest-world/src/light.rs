//! Luz de cielo y de bloque (0–15), propagada por BFS, también entre chunks.

use crate::ChunkColumn;
use std::collections::{HashMap, VecDeque};
use survivalforest_core::block::AIR;
use survivalforest_core::{BlockId, BlockPos, ChunkPos, LocalPos, MAX_Y, MIN_Y};

/// Acceso a las columnas cargadas, por posición de chunk.
pub trait ChunkAccess {
    fn column(&self, p: ChunkPos) -> Option<&ChunkColumn>;
    fn column_mut(&mut self, p: ChunkPos) -> Option<&mut ChunkColumn>;

    fn block_at(&self, p: BlockPos) -> Option<BlockId> {
        if !p.in_height() {
            return None;
        }
        self.column(p.chunk()).map(|c| c.get(p.local()))
    }
}

impl ChunkAccess for HashMap<ChunkPos, ChunkColumn> {
    fn column(&self, p: ChunkPos) -> Option<&ChunkColumn> {
        self.get(&p)
    }
    fn column_mut(&mut self, p: ChunkPos) -> Option<&mut ChunkColumn> {
        self.get_mut(&p)
    }
}

/// Acceso a una sola columna (lo de fuera no existe).
struct One<'a>(&'a mut ChunkColumn);

impl ChunkAccess for One<'_> {
    fn column(&self, p: ChunkPos) -> Option<&ChunkColumn> {
        (p == self.0.pos).then_some(&*self.0)
    }
    fn column_mut(&mut self, p: ChunkPos) -> Option<&mut ChunkColumn> {
        (p == self.0.pos).then_some(&mut *self.0)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Kind {
    Sky,
    Block,
}

fn get_l(w: &impl ChunkAccess, p: BlockPos, k: Kind) -> Option<u8> {
    if !p.in_height() {
        return None;
    }
    let c = w.column(p.chunk())?;
    Some(match k {
        Kind::Sky => c.sky(p.local()),
        Kind::Block => c.block_light(p.local()),
    })
}

fn set_l(w: &mut impl ChunkAccess, p: BlockPos, k: Kind, v: u8) {
    if let Some(c) = w.column_mut(p.chunk()) {
        match k {
            Kind::Sky => c.set_sky(p.local(), v),
            Kind::Block => c.set_block_light(p.local(), v),
        }
    }
}

const DIRS: [(i32, i32, i32); 6] = [
    (1, 0, 0),
    (-1, 0, 0),
    (0, 0, 1),
    (0, 0, -1),
    (0, 1, 0),
    (0, -1, 0),
];

/// Luz que llega a `to` desde un vecino con luz `from` en la dirección `d`.
fn passes(k: Kind, from: u8, d: (i32, i32, i32), to_block: BlockId) -> u8 {
    if to_block.def().opaque || from == 0 {
        return 0;
    }
    if k == Kind::Sky && from == 15 && d == (0, -1, 0) && to_block == AIR {
        return 15;
    }
    let cost = if to_block == AIR { 1 } else { 2 };
    from.saturating_sub(cost)
}

fn propagate(w: &mut impl ChunkAccess, k: Kind, mut q: VecDeque<BlockPos>) {
    while let Some(p) = q.pop_front() {
        let Some(l) = get_l(w, p, k) else { continue };
        for d in DIRS {
            let n = p.offset(d.0, d.1, d.2);
            let (Some(nb), Some(nl)) = (w.block_at(n), get_l(w, n, k)) else {
                continue;
            };
            let v = passes(k, l, d, nb);
            if v > nl {
                set_l(w, n, k, v);
                q.push_back(n);
            }
        }
    }
}

/// Ilumina una columna recién generada o cargada (sin mirar a los vecinos).
pub fn light_column(col: &mut ChunkColumn) {
    col.sky_light.iter_mut().for_each(|v| *v = 0);
    col.block_light.iter_mut().for_each(|v| *v = 0);
    let o = col.pos.origin();
    let mut sky_q = VecDeque::new();
    let mut blk_q = VecDeque::new();
    let mut heights = [0i32; 256];
    for z in 0..16u8 {
        for x in 0..16u8 {
            heights[z as usize * 16 + x as usize] = col.height(x, z);
        }
    }
    let hmax = |x: i32, z: i32| -> i32 {
        let mut m = i32::MIN;
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, nz) = (x + dx, z + dz);
            if (0..16).contains(&nx) && (0..16).contains(&nz) {
                m = m.max(heights[nz as usize * 16 + nx as usize]);
            }
        }
        m
    };
    for z in 0..16u8 {
        for x in 0..16u8 {
            // Luz de cielo: 15 hacia abajo; hojas y agua restan 1 por bloque.
            let mut l = 15u8;
            let edge = hmax(x as i32, z as i32);
            let mut ly = MAX_Y - MIN_Y - 1;
            while ly >= 0 && l > 0 {
                let p = LocalPos::new(x, ly as u16, z);
                let b = col.get(p);
                if b.def().opaque {
                    break;
                }
                if b != AIR {
                    l = l.saturating_sub(1);
                }
                col.set_sky(p, l);
                if ly + MIN_Y <= edge + 1 && l > 1 {
                    sky_q.push_back(BlockPos::new(o.x + x as i32, ly + MIN_Y, o.z + z as i32));
                }
                ly -= 1;
            }
        }
    }
    // Luz de bloque: emisores.
    let mut emitters = Vec::new();
    for (si, sec) in col.sections.iter().enumerate() {
        if sec.is_empty() {
            continue;
        }
        for y in 0..16usize {
            for z in 0..16usize {
                for x in 0..16usize {
                    let b = sec.get(x, y, z);
                    let e = b.def().emits_light;
                    if e > 0 {
                        emitters.push((LocalPos::new(x as u8, (si * 16 + y) as u16, z as u8), e));
                    }
                }
            }
        }
    }
    for (lp, e) in emitters {
        col.set_block_light(lp, e);
        blk_q.push_back(BlockPos::new(
            o.x + lp.x as i32,
            lp.y as i32 + MIN_Y,
            o.z + lp.z as i32,
        ));
    }
    let mut one = One(col);
    propagate(&mut one, Kind::Sky, sky_q);
    propagate(&mut one, Kind::Block, blk_q);
}

fn relight_kind(w: &mut impl ChunkAccess, pos: BlockPos, k: Kind) {
    let Some(old) = get_l(w, pos, k) else { return };
    let mut rem = VecDeque::new();
    let mut add = VecDeque::new();
    set_l(w, pos, k, 0);
    rem.push_back((pos, old));
    while let Some((p, pl)) = rem.pop_front() {
        for d in DIRS {
            let n = p.offset(d.0, d.1, d.2);
            let Some(nl) = get_l(w, n, k) else { continue };
            if nl == 0 {
                continue;
            }
            let down_sky = k == Kind::Sky && d == (0, -1, 0) && pl == 15 && nl == 15;
            if nl < pl || down_sky {
                set_l(w, n, k, 0);
                rem.push_back((n, nl));
            } else {
                add.push_back(n);
            }
        }
    }
    // El propio bloque: emisor o luz de cielo directa.
    if let Some(b) = w.block_at(pos) {
        if k == Kind::Block && b.def().emits_light > 0 {
            set_l(w, pos, k, b.def().emits_light);
            add.push_back(pos);
        }
        if k == Kind::Sky && !b.def().opaque {
            let above = pos.offset(0, 1, 0);
            let from_above = if above.y >= MAX_Y {
                Some(15)
            } else {
                get_l(w, above, k)
            };
            if let Some(a) = from_above {
                let v = passes(k, a, (0, -1, 0), b);
                if v > 0 {
                    set_l(w, pos, k, v);
                    add.push_back(pos);
                }
            }
        }
    }
    for d in DIRS {
        add.push_back(pos.offset(d.0, d.1, d.2));
    }
    propagate(w, k, add);
}

/// Une la luz de un chunk recién iluminado con la de sus vecinos ya cargados:
/// propaga desde las celdas de ambos lados de cada borde (luz de cielo y de bloque).
/// Devuelve los chunks cuya luz pudo cambiar (el propio y sus 8 vecinos cargados).
pub fn light_seam(w: &mut impl ChunkAccess, p: ChunkPos) -> Vec<ChunkPos> {
    if w.column(p).is_none() {
        return Vec::new();
    }
    let o = p.origin();
    for k in [Kind::Sky, Kind::Block] {
        let mut q = VecDeque::new();
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = ChunkPos::new(p.x + dx, p.z + dz);
            if w.column(n).is_none() {
                continue;
            }
            for i in 0..16 {
                // Celda propia del borde y celda vecina enfrente.
                let (mine, theirs) = match (dx, dz) {
                    (1, 0) => ((15, i), (16, i)),
                    (-1, 0) => ((0, i), (-1, i)),
                    (0, 1) => ((i, 15), (i, 16)),
                    _ => ((i, 0), (i, -1)),
                };
                for y in MIN_Y..MAX_Y {
                    for (lx, lz) in [mine, theirs] {
                        let b = BlockPos::new(o.x + lx, y, o.z + lz);
                        if get_l(w, b, k).is_some_and(|l| l > 1) {
                            q.push_back(b);
                        }
                    }
                }
            }
        }
        propagate(w, k, q);
    }
    let mut touched = vec![p];
    for dz in -1..=1 {
        for dx in -1..=1 {
            let n = ChunkPos::new(p.x + dx, p.z + dz);
            if (dx, dz) != (0, 0) && w.column(n).is_some() {
                touched.push(n);
            }
        }
    }
    touched
}

/// Reilumina tras cambiar el bloque en `pos` (puede cruzar a chunks vecinos).
pub fn relight_around(w: &mut impl ChunkAccess, pos: BlockPos) {
    relight_kind(w, pos, Kind::Sky);
    relight_kind(w, pos, Kind::Block);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkColumn;
    use std::collections::HashMap;
    use survivalforest_core::block::{AIR, STONE, TORCH};
    use survivalforest_core::{BlockPos, ChunkPos};

    fn set(c: &mut ChunkColumn, x: i32, y: i32, z: i32, b: survivalforest_core::BlockId) {
        c.set(BlockPos::new(x, y, z).local(), b);
    }

    #[test]
    fn open_sky_is_15() {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        for x in 0..16 {
            for z in 0..16 {
                set(&mut c, x, 60, z, STONE);
            }
        }
        light_column(&mut c);
        assert_eq!(c.sky(BlockPos::new(5, 61, 5).local()), 15);
        assert_eq!(c.sky(BlockPos::new(5, 300, 5).local()), 15);
        assert_eq!(c.sky(BlockPos::new(5, 59, 5).local()), 0);
    }

    #[test]
    fn under_roof_falls_off() {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        for x in 0..16 {
            for z in 0..16 {
                set(&mut c, x, 70, z, STONE);
            }
        }
        for x in 6..11 {
            for z in 6..11 {
                set(&mut c, x, 80, z, STONE);
            }
        }
        light_column(&mut c);
        let v = c.sky(BlockPos::new(8, 75, 8).local());
        assert!(v < 15 && v > 0, "luz bajo el techo: {v}");
    }

    fn sealed_cave() -> (HashMap<ChunkPos, ChunkColumn>, BlockPos) {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        for x in 0..16 {
            for z in 0..16 {
                for y in -63..100 {
                    set(&mut c, x, y, z, STONE);
                }
            }
        }
        for x in 4..9 {
            for z in 4..9 {
                for y in 20..25 {
                    set(&mut c, x, y, z, AIR);
                }
            }
        }
        let t = BlockPos::new(6, 22, 6);
        set(&mut c, t.x, t.y, t.z, TORCH);
        light_column(&mut c);
        let mut m = HashMap::new();
        m.insert(c.pos, c);
        (m, t)
    }

    #[test]
    fn torch_lights_dark_cave() {
        let (m, t) = sealed_cave();
        let c = &m[&ChunkPos::new(0, 0)];
        assert_eq!(c.block_light(t.local()), 14);
        assert_eq!(c.block_light(t.offset(1, 0, 0).local()), 13);
        assert_eq!(c.sky(t.local()), 0);
    }

    #[test]
    fn removing_torch_darkens() {
        let (mut m, t) = sealed_cave();
        m.get_mut(&ChunkPos::new(0, 0)).unwrap().set(t.local(), AIR);
        relight_around(&mut m, t);
        let c = &m[&ChunkPos::new(0, 0)];
        for dx in -2..=2 {
            assert_eq!(c.block_light(t.offset(dx, 0, 0).local()), 0);
        }
    }

    #[test]
    fn light_crosses_chunk_border() {
        let mut m = HashMap::new();
        for x in [0, 1] {
            let mut c = ChunkColumn::new(ChunkPos::new(x, 0));
            light_column(&mut c);
            m.insert(c.pos, c);
        }
        let t = BlockPos::new(15, 100, 3);
        m.get_mut(&t.chunk()).unwrap().set(t.local(), TORCH);
        relight_around(&mut m, t);
        let n = BlockPos::new(16, 100, 3);
        assert_eq!(m[&n.chunk()].block_light(n.local()), 13);
    }

    #[test]
    fn placing_block_shades_below_and_removing_restores() {
        let mut m = HashMap::new();
        for x in -1..=1 {
            for z in -1..=1 {
                let mut c = ChunkColumn::new(ChunkPos::new(x, z));
                for lx in 0..16 {
                    for lz in 0..16 {
                        c.set(BlockPos::new(lx, 60, lz).local(), STONE);
                    }
                }
                light_column(&mut c);
                m.insert(c.pos, c);
            }
        }
        // Un techo de 7x7 a y=70: el centro bajo él pierde la luz directa.
        for x in 5..12 {
            for z in 5..12 {
                let p = BlockPos::new(x, 70, z);
                m.get_mut(&p.chunk()).unwrap().set(p.local(), STONE);
                relight_around(&mut m, p);
            }
        }
        let below = BlockPos::new(8, 65, 8);
        let v = m[&below.chunk()].sky(below.local());
        assert!(v < 15 && v > 0, "bajo el techo: {v}");
        let p = BlockPos::new(8, 70, 8);
        m.get_mut(&p.chunk()).unwrap().set(p.local(), AIR);
        relight_around(&mut m, p);
        assert_eq!(m[&below.chunk()].sky(below.local()), 15);
    }

    #[test]
    fn seam_pass_carries_light_across_border() {
        // Antorcha en x=15 del chunk 0; los chunks se iluminan por separado (como al generarlos).
        let mut a = ChunkColumn::new(ChunkPos::new(0, 0));
        let t = BlockPos::new(15, 100, 3);
        a.set(t.local(), TORCH);
        light_column(&mut a);
        let mut b = ChunkColumn::new(ChunkPos::new(1, 0));
        light_column(&mut b);
        let mut m = HashMap::new();
        m.insert(a.pos, a);
        m.insert(b.pos, b);
        let n = BlockPos::new(16, 100, 3);
        assert_eq!(
            m[&n.chunk()].block_light(n.local()),
            0,
            "sin costura todavía"
        );
        let touched = light_seam(&mut m, ChunkPos::new(1, 0));
        assert_eq!(m[&n.chunk()].block_light(n.local()), 13);
        assert!(touched.contains(&ChunkPos::new(1, 0)));
    }

    #[test]
    fn seam_pass_shades_under_overhang_across_border() {
        // Un techo que cubre el borde x=15|16: bajo él, en el chunk vecino, la luz de cielo baja.
        let mut a = ChunkColumn::new(ChunkPos::new(0, 0));
        let mut b = ChunkColumn::new(ChunkPos::new(1, 0));
        for x in 10..22 {
            for z in 0..16 {
                let p = BlockPos::new(x, 90, z);
                if x < 16 {
                    a.set(p.local(), STONE)
                } else {
                    b.set(p.local(), STONE)
                }
            }
        }
        light_column(&mut a);
        light_column(&mut b);
        let mut m = HashMap::new();
        m.insert(a.pos, a);
        m.insert(b.pos, b);
        light_seam(&mut m, ChunkPos::new(1, 0));
        light_seam(&mut m, ChunkPos::new(0, 0));
        // Bajo el techo, a 6 bloques del borde abierto de la izquierda (x=10) y de la derecha (x=21).
        let under = BlockPos::new(16, 85, 8);
        let v = m[&under.chunk()].sky(under.local());
        assert!(v < 15 && v > 0, "luz bajo el techo en el borde: {v}");
    }
}
