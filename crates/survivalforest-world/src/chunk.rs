//! Secciones de 16³ con paleta y columnas de chunk completas.

use serde::{Deserialize, Serialize};
use survivalforest_core::block::{AIR, BlockId};
use survivalforest_core::{CHUNK, ChunkPos, LocalPos, MIN_Y, SECTIONS};

const VOL: usize = 16 * 16 * 16;
pub const COLUMN_VOL: usize = VOL * SECTIONS;

#[inline]
fn sidx(x: usize, y: usize, z: usize) -> usize {
    (y * 16 + z) * 16 + x
}

/// Datos empaquetados en bits de una sección no uniforme.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Packed {
    bits: u32,
    words: Vec<u64>,
}

impl Packed {
    fn new(bits: u32) -> Self {
        let per = (64 / bits) as usize;
        Packed {
            bits,
            words: vec![0; VOL.div_ceil(per)],
        }
    }
    #[inline]
    fn get(&self, i: usize) -> usize {
        let per = (64 / self.bits) as usize;
        let shift = (i % per) as u32 * self.bits;
        ((self.words[i / per] >> shift) & ((1u64 << self.bits) - 1)) as usize
    }
    #[inline]
    fn set(&mut self, i: usize, v: usize) {
        let per = (64 / self.bits) as usize;
        let shift = (i % per) as u32 * self.bits;
        let mask = ((1u64 << self.bits) - 1) << shift;
        let w = &mut self.words[i / per];
        *w = (*w & !mask) | ((v as u64) << shift);
    }
}

/// 16³ bloques con paleta: un único valor si es uniforme o índices de 4, 8 o 16 bits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    palette: Vec<BlockId>,
    data: Option<Packed>,
}

impl Default for Section {
    fn default() -> Self {
        Section::uniform(AIR)
    }
}

impl Section {
    pub fn uniform(b: BlockId) -> Self {
        Section {
            palette: vec![b],
            data: None,
        }
    }
    pub fn is_uniform(&self) -> bool {
        self.data.is_none()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_none() && self.palette[0] == AIR
    }
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        match &self.data {
            None => self.palette[0],
            Some(p) => self.palette[p.get(sidx(x, y, z))],
        }
    }
    pub fn set(&mut self, x: usize, y: usize, z: usize, b: BlockId) {
        if self.data.is_none() {
            if self.palette[0] == b {
                return;
            }
            self.data = Some(Packed::new(4));
        }
        let pi = match self.palette.iter().position(|&p| p == b) {
            Some(i) => i,
            None => {
                self.palette.push(b);
                let need = self.palette.len();
                let bits = self.data.as_ref().map(|d| d.bits).unwrap_or(4);
                if need > (1usize << bits) {
                    let nb = if need <= 256 { 8 } else { 16 };
                    let old = self.data.take().expect("datos");
                    let mut np = Packed::new(nb);
                    for i in 0..VOL {
                        np.set(i, old.get(i));
                    }
                    self.data = Some(np);
                }
                need - 1
            }
        };
        if let Some(d) = self.data.as_mut() {
            d.set(sidx(x, y, z), pi);
        }
    }
    /// ¿Contiene algún bloque distinto de aire?
    pub fn has_blocks(&self) -> bool {
        !self.is_empty()
    }
}

/// Columna completa de un chunk: 24 secciones, luz y mapa de alturas.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkColumn {
    pub pos: ChunkPos,
    pub sections: Vec<Section>,
    /// 4 bits por bloque.
    pub sky_light: Vec<u8>,
    /// 4 bits por bloque.
    pub block_light: Vec<u8>,
    /// `heightmap[z*16+x]`: y absoluta del bloque opaco más alto (`MIN_Y - 1` si no hay).
    pub heightmap: Vec<i16>,
    #[serde(skip)]
    pub dirty: bool,
}

#[inline]
fn lidx(p: LocalPos) -> usize {
    (p.y as usize * 16 + p.z as usize) * 16 + p.x as usize
}

impl ChunkColumn {
    pub fn new(pos: ChunkPos) -> Self {
        ChunkColumn {
            pos,
            sections: (0..SECTIONS).map(|_| Section::default()).collect(),
            sky_light: vec![0; COLUMN_VOL / 2],
            block_light: vec![0; COLUMN_VOL / 2],
            heightmap: vec![(MIN_Y - 1) as i16; 256],
            dirty: false,
        }
    }
    #[inline]
    pub fn get(&self, p: LocalPos) -> BlockId {
        let s = p.y as usize / 16;
        match self.sections.get(s) {
            Some(sec) => sec.get(p.x as usize, p.y as usize % 16, p.z as usize),
            None => AIR,
        }
    }
    /// Escribe sin tocar el mapa de alturas (para la generación; llama a `rebuild_heightmap` al final).
    #[inline]
    pub fn set_raw(&mut self, p: LocalPos, b: BlockId) {
        let s = p.y as usize / 16;
        if let Some(sec) = self.sections.get_mut(s) {
            sec.set(p.x as usize, p.y as usize % 16, p.z as usize, b);
        }
    }
    pub fn set(&mut self, p: LocalPos, b: BlockId) {
        if p.y as usize >= SECTIONS * 16 {
            return;
        }
        self.set_raw(p, b);
        self.dirty = true;
        let hi = (p.z as usize) * 16 + p.x as usize;
        let y = p.y as i32 + MIN_Y;
        let h = self.heightmap[hi] as i32;
        if b.def().opaque {
            if y > h {
                self.heightmap[hi] = y as i16;
            }
        } else if y == h {
            self.heightmap[hi] = self.scan_height(p.x, p.z, p.y as i32 - 1) as i16;
        }
    }
    fn scan_height(&self, x: u8, z: u8, from_local: i32) -> i32 {
        let mut ly = from_local;
        while ly >= 0 {
            if self.get(LocalPos::new(x, ly as u16, z)).def().opaque {
                return ly + MIN_Y;
            }
            ly -= 1;
        }
        MIN_Y - 1
    }
    pub fn rebuild_heightmap(&mut self) {
        for z in 0..CHUNK as u8 {
            for x in 0..CHUNK as u8 {
                let h = self.scan_height(x, z, (SECTIONS * 16) as i32 - 1);
                self.heightmap[z as usize * 16 + x as usize] = h as i16;
            }
        }
    }
    /// Altura del bloque opaco más alto en (x, z) locales.
    pub fn height(&self, x: u8, z: u8) -> i32 {
        self.heightmap[z as usize * 16 + x as usize] as i32
    }
    #[inline]
    fn nib(v: &[u8], i: usize) -> u8 {
        (v[i >> 1] >> ((i & 1) * 4)) & 0xF
    }
    #[inline]
    fn set_nib(v: &mut [u8], i: usize, val: u8) {
        let sh = (i & 1) * 4;
        v[i >> 1] = (v[i >> 1] & !(0xF << sh)) | ((val & 0xF) << sh);
    }
    #[inline]
    pub fn sky(&self, p: LocalPos) -> u8 {
        Self::nib(&self.sky_light, lidx(p))
    }
    #[inline]
    pub fn block_light(&self, p: LocalPos) -> u8 {
        Self::nib(&self.block_light, lidx(p))
    }
    #[inline]
    pub fn set_sky(&mut self, p: LocalPos, v: u8) {
        Self::set_nib(&mut self.sky_light, lidx(p), v)
    }
    #[inline]
    pub fn set_block_light(&mut self, p: LocalPos, v: u8) {
        Self::set_nib(&mut self.block_light, lidx(p), v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use survivalforest_core::block::STONE;

    #[test]
    fn section_uniform_until_written() {
        let mut s = Section::default();
        assert!(s.is_uniform());
        assert!(s.is_empty());
        s.set(1, 2, 3, STONE);
        assert!(!s.is_uniform());
        assert_eq!(s.get(1, 2, 3), STONE);
        assert_eq!(s.get(0, 0, 0), AIR);
    }

    #[test]
    fn palette_grows_past_16_kinds() {
        let mut s = Section::default();
        for i in 0..40u16 {
            s.set(i as usize % 16, i as usize / 16, 5, BlockId(i));
        }
        for i in 0..40u16 {
            assert_eq!(s.get(i as usize % 16, i as usize / 16, 5), BlockId(i));
        }
        for i in 0..300u16 {
            s.set(
                i as usize % 16,
                8 + (i as usize / 16) % 8,
                i as usize / 128,
                BlockId(i + 1000),
            );
        }
        for i in 0..300u16 {
            assert_eq!(
                s.get(i as usize % 16, 8 + (i as usize / 16) % 8, i as usize / 128),
                BlockId(i + 1000)
            );
        }
    }

    #[test]
    fn heightmap_tracks_highest_opaque() {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        let at = |y: i32| LocalPos::new(3, (y - MIN_Y) as u16, 4);
        c.set(at(10), STONE);
        c.set(at(70), STONE);
        assert_eq!(c.height(3, 4), 70);
        c.set(at(70), AIR);
        assert_eq!(c.height(3, 4), 10);
        c.set(at(10), AIR);
        assert_eq!(c.height(3, 4), MIN_Y - 1);
        assert!(c.dirty);
    }

    #[test]
    fn light_nibbles_roundtrip() {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        for v in 0..=15u8 {
            let p = LocalPos::new(v, 100, 0);
            c.set_sky(p, v);
            c.set_block_light(p, 15 - v);
        }
        for v in 0..=15u8 {
            let p = LocalPos::new(v, 100, 0);
            assert_eq!(c.sky(p), v);
            assert_eq!(c.block_light(p), 15 - v);
        }
    }
}
