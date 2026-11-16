//! Guardado de la supervivencia: datos del jugador (`survival.ron`) y, por región,
//! baúles, hornos y objetos en el suelo (`region/r.X.Z.extra.ron`).

use crate::containers::BlockEntities;
use crate::drops::GroundItems;
use crate::inventory::Inventory;
use crate::vitals::Vitals;
use crate::xp::Experience;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use survivalforest_core::BlockPos;
use survivalforest_world::save::{ron_read, ron_write};

pub const SURVIVAL_VERSION: u32 = 1;
pub const EXTRAS_VERSION: u32 = 1;

/// Lo del jugador que añade la etapa 2.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurvivalSave {
    pub version: u32,
    #[serde(default)]
    pub inventory: Inventory,
    #[serde(default)]
    pub vitals: Vitals,
    #[serde(default)]
    pub xp: Experience,
    /// Recetas ya hechas alguna vez (para la experiencia de la primera vez).
    #[serde(default)]
    pub crafted: Vec<String>,
    #[serde(default)]
    pub spawn: Option<[i32; 3]>,
}

impl Default for SurvivalSave {
    fn default() -> Self {
        Self {
            version: SURVIVAL_VERSION,
            inventory: Inventory::default(),
            vitals: Vitals::default(),
            xp: Experience::default(),
            crafted: vec![],
            spawn: None,
        }
    }
}

/// Guarda una copia `.bak` de un archivo que no se pudo leer (para no perderlo al reescribirlo).
fn backup(path: &Path) {
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    let _ = std::fs::copy(path, PathBuf::from(bak));
}

/// Lee `survival.ron`. Si no existe (mundo de la etapa 1), empieza de cero. Si está dañado,
/// también, pero antes guarda una copia `survival.ron.bak`.
pub fn load_survival(world_dir: &Path) -> SurvivalSave {
    let path = world_dir.join("survival.ron");
    match ron_read(&path) {
        Ok(s) => s,
        Err(e) => {
            if path.exists() {
                eprintln!("survival.ron dañado ({e}); se guarda una copia en survival.ron.bak");
                backup(&path);
            }
            SurvivalSave::default()
        }
    }
}

pub fn save_survival(world_dir: &Path, s: &SurvivalSave) -> io::Result<()> {
    ron_write(&world_dir.join("survival.ron"), s)
}

/// Región (32×32 chunks) de un bloque, como la de los archivos de región.
pub fn region_of(p: BlockPos) -> (i32, i32) {
    let c = p.chunk();
    (c.x.div_euclid(32), c.z.div_euclid(32))
}

fn extras_file(world_dir: &Path, r: (i32, i32)) -> PathBuf {
    world_dir
        .join("region")
        .join(format!("r.{}.{}.extra.ron", r.0, r.1))
}

#[derive(Serialize, Deserialize)]
struct ExtrasFile {
    version: u32,
    block_entities: Vec<(BlockPos, crate::containers::BlockEntity)>,
    ground_items: GroundItems,
}

/// Guarda los baúles, hornos y objetos de una región (los demás se ignoran).
pub fn save_extras(
    world_dir: &Path,
    r: (i32, i32),
    be: &BlockEntities,
    items: &GroundItems,
) -> io::Result<()> {
    let mut block_entities: Vec<_> =
        be.0.iter()
            .filter(|(p, _)| region_of(**p) == r)
            .map(|(p, e)| (*p, e.clone()))
            .collect();
    block_entities.sort_by_key(|(p, _)| (p.x, p.y, p.z));
    let ground_items = GroundItems(
        items
            .0
            .iter()
            .filter(|i| {
                region_of(BlockPos::new(
                    i.pos.x.floor() as i32,
                    i.pos.y.floor() as i32,
                    i.pos.z.floor() as i32,
                )) == r
            })
            .cloned()
            .collect(),
    );
    let path = extras_file(world_dir, r);
    if block_entities.is_empty() && ground_items.0.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }
    ron_write(
        &path,
        &ExtrasFile {
            version: EXTRAS_VERSION,
            block_entities,
            ground_items,
        },
    )
}

/// Regiones que tienen algo que guardar.
pub fn regions_in(be: &BlockEntities, items: &GroundItems) -> BTreeSet<(i32, i32)> {
    let mut out: BTreeSet<_> = be.0.keys().map(|p| region_of(*p)).collect();
    out.extend(items.0.iter().map(|i| {
        region_of(BlockPos::new(
            i.pos.x.floor() as i32,
            i.pos.y.floor() as i32,
            i.pos.z.floor() as i32,
        ))
    }));
    out
}

/// Guarda todas las regiones con contenido.
pub fn save_all_extras(
    world_dir: &Path,
    be: &BlockEntities,
    items: &GroundItems,
) -> io::Result<()> {
    for r in regions_in(be, items) {
        save_extras(world_dir, r, be, items)?;
    }
    Ok(())
}

/// Lee los baúles, hornos y objetos de una región. Sin archivo: vacío. Archivo dañado:
/// error (y una copia `.bak`); quien llama no debe volver a escribir esa región.
pub fn try_load_extras(
    world_dir: &Path,
    r: (i32, i32),
) -> Result<(BlockEntities, GroundItems), String> {
    let path = extras_file(world_dir, r);
    if !path.exists() {
        return Ok((BlockEntities::default(), GroundItems::default()));
    }
    match ron_read::<ExtrasFile>(&path) {
        Ok(f) => Ok((
            BlockEntities(f.block_entities.into_iter().collect()),
            f.ground_items,
        )),
        Err(e) => {
            backup(&path);
            Err(format!("{}: {e}", path.display()))
        }
    }
}

/// Como `try_load_extras`, pero un archivo dañado cuenta como vacío.
pub fn load_extras(world_dir: &Path, r: (i32, i32)) -> (BlockEntities, GroundItems) {
    try_load_extras(world_dir, r).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::containers::BlockEntity;
    use crate::furnace::{Chest, Furnace};
    use crate::inventory::ItemStack;
    use glam::DVec3;
    use survivalforest_core::BlockPos;
    use survivalforest_core::item::item_by_key;

    fn st(k: &str, n: u16) -> ItemStack {
        ItemStack::new(item_by_key(k).unwrap(), n)
    }

    #[test]
    fn stage1_world_loads_with_defaults() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("player.ron"),
            "(pos: (1.0, 70.0, 2.0), yaw: 0.0, pitch: 0.0, flying: false)",
        )
        .unwrap();
        let s = load_survival(d.path());
        assert!(s.inventory.slots.is_empty());
        assert_eq!(s.vitals.health, 20.0);
        assert_eq!(s.vitals.hunger, 20.0);
        assert_eq!(s.xp.level, 0);
        assert!(survivalforest_world::save::load_player(d.path()).is_ok());
    }

    #[test]
    fn survival_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let mut s = SurvivalSave::default();
        s.inventory.slots.0[3] = Some(ItemStack {
            wear: 7,
            ..st("pick_stone", 1)
        });
        s.inventory.selected = 3;
        s.vitals.health = 9.0;
        s.xp.add(30.0);
        s.crafted = vec!["planks".into()];
        s.spawn = Some([1, 70, 2]);
        save_survival(d.path(), &s).unwrap();
        assert_eq!(load_survival(d.path()), s);
    }

    #[test]
    fn extras_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("region")).unwrap();
        let mut be = BlockEntities::default();
        let mut chest = Chest::default();
        chest.slots.0[0] = Some(st("coal", 5));
        chest.slots.0[1] = Some(st("stick", 2));
        chest.slots.0[26] = Some(st("platano", 3));
        be.0.insert(BlockPos::new(3, 70, 4), BlockEntity::Chest(chest));
        let f = Furnace {
            input: Some(st("sand", 4)),
            fuel: Some(st("coal", 1)),
            burn_left: 12.0,
            burn_total: 64.0,
            progress: 3.5,
            ..Default::default()
        };
        be.0.insert(BlockPos::new(-600, 70, 4), BlockEntity::Furnace(f));
        let mut items = GroundItems::default();
        items.spawn(DVec3::new(5.5, 71.0, 5.5), st("cobble", 9), 300.0);
        items.spawn_orb(DVec3::new(6.5, 71.0, 5.5), 4.0, 600.0);
        save_all_extras(d.path(), &be, &items).unwrap();
        let (be0, it0) = load_extras(d.path(), (0, 0));
        let (be1, _) = load_extras(d.path(), (-2, 0));
        assert_eq!(be0.0.len(), 1);
        assert_eq!(be1.0.len(), 1);
        assert_eq!(it0, items);
        let mut all = be0.0.clone();
        all.extend(be1.0);
        assert_eq!(all, be.0);
    }

    #[test]
    fn death_items_survive_save_and_load() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("region")).unwrap();
        let mut inv = crate::inventory::Inventory::default();
        inv.slots.0[0] = Some(st("iron_ingot", 3));
        let mut xp = crate::xp::Experience::default();
        xp.add(10.0);
        let mut g = GroundItems::default();
        crate::death::die(&mut inv, &mut xp, &mut g, DVec3::new(100.0, 64.0, 100.0));
        save_all_extras(d.path(), &BlockEntities::default(), &g).unwrap();
        let (_, back) = load_extras(d.path(), region_of(BlockPos::new(100, 64, 100)));
        assert_eq!(back, g);
    }

    #[test]
    fn corrupt_extras_are_kept_with_backup() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("region")).unwrap();
        let f = d.path().join("region").join("r.0.0.extra.ron");
        std::fs::write(&f, "(version: 1, block_entities: [((x: 1, y: 2, z: 3), Chest((slots: [Some((item: \"no_existe\", count: 1))])))], ground_items: ([]))").unwrap();
        assert!(try_load_extras(d.path(), (0, 0)).is_err());
        assert!(f.exists(), "el original no se borra");
        assert!(d.path().join("region").join("r.0.0.extra.ron.bak").exists());
        assert!(try_load_extras(d.path(), (5, 5)).unwrap().0.0.is_empty());
    }

    #[test]
    fn corrupt_survival_is_backed_up() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("survival.ron"), "(version: 1, inventory: ((slots: ([Some((item: \"no_existe\", count: 1))]), selected: 0)))").unwrap();
        let s = load_survival(d.path());
        assert!(s.inventory.slots.is_empty());
        assert!(d.path().join("survival.ron.bak").exists());
    }

    #[test]
    fn missing_extras_file_is_empty() {
        let d = tempfile::tempdir().unwrap();
        let (be, it) = load_extras(d.path(), (7, 7));
        assert!(be.0.is_empty() && it.0.is_empty());
    }
}
