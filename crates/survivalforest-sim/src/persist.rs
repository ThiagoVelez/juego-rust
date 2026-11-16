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

/// Lee `survival.ron`; si no existe (mundo de la etapa 1) o está dañado, empieza de cero.
pub fn load_survival(world_dir: &Path) -> SurvivalSave {
    ron_read(&world_dir.join("survival.ron")).unwrap_or_default()
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

pub fn load_extras(world_dir: &Path, r: (i32, i32)) -> (BlockEntities, GroundItems) {
    match ron_read::<ExtrasFile>(&extras_file(world_dir, r)) {
        Ok(f) => (
            BlockEntities(f.block_entities.into_iter().collect()),
            f.ground_items,
        ),
        Err(_) => (BlockEntities::default(), GroundItems::default()),
    }
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
    fn missing_extras_file_is_empty() {
        let d = tempfile::tempdir().unwrap();
        let (be, it) = load_extras(d.path(), (7, 7));
        assert!(be.0.is_empty() && it.0.is_empty());
    }
}
