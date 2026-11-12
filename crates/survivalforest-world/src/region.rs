//! Archivos de región: 32×32 chunks por archivo, comprimidos con LZ4.
//!
//! Formato `r.<rx>.<rz>.sfr`: `SFR1`, tabla de 1024 entradas
//! `(offset u32, len u32, versión u16)` en little endian y después los datos.

use crate::ChunkColumn;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use survivalforest_core::ChunkPos;

const MAGIC: &[u8; 4] = b"SFR1";
const ENTRIES: usize = 1024;
const HEADER: usize = 4 + ENTRIES * 10;
pub const CHUNK_FORMAT: u16 = 1;

struct Region {
    /// Datos comprimidos de cada chunk (índice = (z & 31) * 32 + (x & 31)).
    chunks: Vec<Option<(u16, Vec<u8>)>>,
    dirty: bool,
}

impl Region {
    fn empty() -> Self {
        Region {
            chunks: vec![None; ENTRIES],
            dirty: false,
        }
    }

    fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEADER || &bytes[..4] != MAGIC {
            return None;
        }
        let mut r = Region::empty();
        for i in 0..ENTRIES {
            let e = &bytes[4 + i * 10..4 + i * 10 + 10];
            let off = u32::from_le_bytes(e[0..4].try_into().ok()?) as usize;
            let len = u32::from_le_bytes(e[4..8].try_into().ok()?) as usize;
            let ver = u16::from_le_bytes(e[8..10].try_into().ok()?);
            if len == 0 {
                continue;
            }
            // Una entrada fuera del archivo se descarta: ese chunk se regenerará.
            if let Some(data) = off.checked_add(len).and_then(|end| bytes.get(off..end)) {
                r.chunks[i] = Some((ver, data.to_vec()));
            }
        }
        Some(r)
    }

    fn encode(&self) -> Vec<u8> {
        let mut head = Vec::with_capacity(HEADER);
        head.extend_from_slice(MAGIC);
        let mut body = Vec::new();
        for c in &self.chunks {
            match c {
                Some((ver, data)) => {
                    head.extend_from_slice(&((HEADER + body.len()) as u32).to_le_bytes());
                    head.extend_from_slice(&(data.len() as u32).to_le_bytes());
                    head.extend_from_slice(&ver.to_le_bytes());
                    body.extend_from_slice(data);
                }
                None => head.extend_from_slice(&[0u8; 10]),
            }
        }
        head.extend_from_slice(&body);
        head
    }
}

/// FNV-1a de 32 bits: detecta datos dañados antes de descomprimir.
fn checksum(d: &[u8]) -> u32 {
    d.iter().fold(0x811C_9DC5u32, |h, &b| {
        (h ^ b as u32).wrapping_mul(0x0100_0193)
    })
}

/// Decodifica los bytes de un chunk (suma de verificación, LZ4 y postcard).
/// Se puede llamar desde cualquier hilo. `None` si están dañados.
pub fn decode_chunk(p: ChunkPos, data: &[u8]) -> Option<ChunkColumn> {
    let col = (|| {
        let (sum, body) = data.split_at_checked(4)?;
        if u32::from_le_bytes(sum.try_into().ok()?) != checksum(body) {
            return None;
        }
        let raw = lz4_flex::decompress_size_prepended(body).ok()?;
        let col: ChunkColumn = postcard::from_bytes(&raw).ok()?;
        (col.pos == p && col.sections.len() == survivalforest_core::SECTIONS).then_some(col)
    })();
    if col.is_none() {
        log::warn!("Chunk ({}, {}) ilegible: se regenerará.", p.x, p.z);
    }
    col
}

/// Almacén de chunks de un mundo (carpeta `region/`).
pub struct RegionStore {
    dir: PathBuf,
    open: HashMap<(i32, i32), Region>,
}

fn region_of(p: ChunkPos) -> ((i32, i32), usize) {
    (
        (p.x.div_euclid(32), p.z.div_euclid(32)),
        (p.z.rem_euclid(32) * 32 + p.x.rem_euclid(32)) as usize,
    )
}

/// Escribe en `*.tmp` y renombra: un cierre brusco nunca deja el archivo a medias.
/// Los datos se fuerzan a disco (fsync) antes de renombrar, y en Unix también la carpeta,
/// para que un corte de luz no deje el archivo vacío.
pub fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    #[cfg(unix)]
    if let Some(dir) = path.parent()
        && let Ok(d) = fs::File::open(dir)
    {
        let _ = d.sync_all();
    }
    Ok(())
}

impl RegionStore {
    pub fn open(world_dir: impl AsRef<Path>) -> io::Result<Self> {
        let dir = world_dir.as_ref().join("region");
        fs::create_dir_all(&dir)?;
        Ok(RegionStore {
            dir,
            open: HashMap::new(),
        })
    }

    fn file(&self, r: (i32, i32)) -> PathBuf {
        self.dir.join(format!("r.{}.{}.sfr", r.0, r.1))
    }

    fn region(&mut self, r: (i32, i32)) -> &mut Region {
        if !self.open.contains_key(&r) {
            let path = self.file(r);
            let region = match fs::read(&path) {
                Ok(bytes) => Region::parse(&bytes).unwrap_or_else(|| {
                    log::warn!(
                        "Archivo de región dañado: {}. Se guarda una copia y se regenera.",
                        path.display()
                    );
                    let _ = fs::rename(&path, path.with_extension("sfr.corrupt"));
                    Region::empty()
                }),
                Err(_) => Region::empty(),
            };
            self.open.insert(r, region);
        }
        self.open.get_mut(&r).expect("región abierta")
    }

    /// Bytes guardados de un chunk (sin decodificar): rápido, para el hilo principal.
    pub fn load_raw(&mut self, p: ChunkPos) -> Option<Vec<u8>> {
        let (r, i) = region_of(p);
        self.region(r).chunks[i]
            .as_ref()
            .filter(|(ver, _)| *ver == CHUNK_FORMAT)
            .map(|(_, d)| d.clone())
    }

    /// Devuelve `None` si el chunk no existe o está dañado (el llamador lo regenera).
    pub fn load(&mut self, p: ChunkPos) -> Option<ChunkColumn> {
        let raw = self.load_raw(p)?;
        decode_chunk(p, &raw)
    }

    pub fn save(&mut self, col: &ChunkColumn) -> io::Result<()> {
        let raw = postcard::to_stdvec(col).map_err(io::Error::other)?;
        let body = lz4_flex::compress_prepend_size(&raw);
        let mut data = checksum(&body).to_le_bytes().to_vec();
        data.extend_from_slice(&body);
        let (r, i) = region_of(col.pos);
        let reg = self.region(r);
        reg.chunks[i] = Some((CHUNK_FORMAT, data));
        reg.dirty = true;
        Ok(())
    }

    /// Escribe a disco las regiones modificadas.
    pub fn flush(&mut self) -> io::Result<()> {
        let dirty: Vec<(i32, i32)> = self
            .open
            .iter()
            .filter(|(_, r)| r.dirty)
            .map(|(k, _)| *k)
            .collect();
        for k in dirty {
            let path = self.file(k);
            let reg = self.open.get_mut(&k).expect("región");
            write_atomic(&path, &reg.encode())?;
            reg.dirty = false;
        }
        // Libera las regiones limpias para no acumular memoria.
        self.open.retain(|_, r| r.dirty);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkColumn;
    use survivalforest_core::block::STONE;
    use survivalforest_core::{BlockPos, ChunkPos};

    fn edited(x: i32, z: i32) -> ChunkColumn {
        let mut c = ChunkColumn::new(ChunkPos::new(x, z));
        c.set(BlockPos::new(x * 16 + 3, 70, z * 16 + 4).local(), STONE);
        c.set_sky(BlockPos::new(0, 100, 0).local(), 9);
        c
    }

    #[test]
    fn region_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let cols = [edited(0, 0), edited(5, -3), edited(40, 2)];
        {
            let mut s = RegionStore::open(d.path()).unwrap();
            for c in &cols {
                s.save(c).unwrap();
            }
            s.flush().unwrap();
        }
        let mut s = RegionStore::open(d.path()).unwrap();
        for c in &cols {
            let l = s.load(c.pos).expect("cargado");
            assert_eq!(l.sections, c.sections);
            assert_eq!(l.sky_light, c.sky_light);
            assert_eq!(l.heightmap, c.heightmap);
        }
        assert!(s.load(ChunkPos::new(1, 1)).is_none());
    }

    #[test]
    fn corrupt_chunk_returns_none() {
        let d = tempfile::tempdir().unwrap();
        let mut s = RegionStore::open(d.path()).unwrap();
        s.save(&edited(0, 0)).unwrap();
        s.save(&edited(1, 0)).unwrap();
        s.flush().unwrap();
        drop(s);
        let f = d.path().join("region").join("r.0.0.sfr");
        let mut bytes = std::fs::read(&f).unwrap();
        let n = bytes.len();
        for b in &mut bytes[n - 40..] {
            *b ^= 0xA5;
        }
        // y un byte suelto dentro de los datos del primer chunk
        bytes[super::HEADER + 30] ^= 0x01;
        std::fs::write(&f, bytes).unwrap();
        let mut s = RegionStore::open(d.path()).unwrap();
        let a = s.load(ChunkPos::new(0, 0)).is_some();
        let b = s.load(ChunkPos::new(1, 0)).is_some();
        assert!(!a && !b, "los dos chunks estaban dañados");
    }

    #[test]
    fn truncated_region_file_is_tolerated() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("region")).unwrap();
        std::fs::write(d.path().join("region").join("r.0.0.sfr"), b"SFR1\x01\x02").unwrap();
        let mut s = RegionStore::open(d.path()).unwrap();
        assert!(s.load(ChunkPos::new(0, 0)).is_none());
        s.save(&edited(0, 0)).unwrap();
        s.flush().unwrap();
        let mut s = RegionStore::open(d.path()).unwrap();
        assert!(s.load(ChunkPos::new(0, 0)).is_some());
    }

    #[test]
    fn negative_chunks_go_to_negative_regions() {
        let d = tempfile::tempdir().unwrap();
        let mut s = RegionStore::open(d.path()).unwrap();
        s.save(&edited(-1, -1)).unwrap();
        s.flush().unwrap();
        assert!(d.path().join("region").join("r.-1.-1.sfr").exists());
    }

    #[test]
    fn raw_bytes_decode_off_thread() {
        let d = tempfile::tempdir().unwrap();
        let mut s = RegionStore::open(d.path()).unwrap();
        let c = edited(2, -5);
        s.save(&c).unwrap();
        s.flush().unwrap();
        let mut s = RegionStore::open(d.path()).unwrap();
        let raw = s.load_raw(c.pos).expect("bytes");
        let col = std::thread::spawn(move || decode_chunk(ChunkPos::new(2, -5), &raw))
            .join()
            .unwrap()
            .expect("decodifica");
        assert_eq!(col.sections, c.sections);
        assert!(s.load_raw(ChunkPos::new(9, 9)).is_none());
        assert!(
            decode_chunk(ChunkPos::new(3, -5), &s.load_raw(c.pos).unwrap()).is_none(),
            "posición equivocada"
        );
    }
}
