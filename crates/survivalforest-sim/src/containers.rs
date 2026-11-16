//! Bloques con contenido: baúles y hornos.

use crate::drops::{GroundItems, scatter};
use crate::furnace::{Chest, Furnace};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use survivalforest_core::BlockPos;
use survivalforest_core::block::{BlockId, CHEST, FURNACE, FURNACE_LIT};
use survivalforest_core::tuning::tuning;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BlockEntity {
    Chest(Chest),
    Furnace(Furnace),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockEntities(pub HashMap<BlockPos, BlockEntity>);

impl BlockEntities {
    pub fn on_placed(&mut self, pos: BlockPos, b: BlockId) {
        if b == CHEST {
            self.0.insert(pos, BlockEntity::Chest(Chest::default()));
        } else if b == FURNACE || b == FURNACE_LIT {
            self.0
                .entry(pos)
                .or_insert_with(|| BlockEntity::Furnace(Furnace::default()));
        }
    }

    /// Al romper el bloque, su contenido cae al suelo.
    pub fn on_broken(&mut self, pos: BlockPos, ground: &mut GroundItems) {
        let at = DVec3::new(pos.x as f64 + 0.5, pos.y as f64 + 0.5, pos.z as f64 + 0.5);
        let stacks = match self.0.remove(&pos) {
            Some(BlockEntity::Chest(mut c)) => c.slots.drain_all(),
            Some(BlockEntity::Furnace(f)) => {
                [f.input, f.fuel, f.output].into_iter().flatten().collect()
            }
            None => return,
        };
        scatter(ground, at, stacks, tuning().ground_item_ttl);
    }

    /// Avanza los hornos. Devuelve los que cambiaron de apagado a encendido o al revés.
    pub fn tick_furnaces(&mut self, dt: f32) -> Vec<(BlockPos, bool)> {
        let mut out = vec![];
        for (p, be) in &mut self.0 {
            if let BlockEntity::Furnace(f) = be {
                let lit = f.tick(dt);
                if lit != f.lit {
                    f.lit = lit;
                    out.push((*p, lit));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ItemStack;
    use survivalforest_core::block::{CHEST, FURNACE, STONE};
    use survivalforest_core::item::item_by_key;

    fn st(k: &str, n: u16) -> Option<ItemStack> {
        Some(ItemStack::new(item_by_key(k).unwrap(), n))
    }

    #[test]
    fn placing_creates_and_other_blocks_do_not() {
        let mut be = BlockEntities::default();
        be.on_placed(BlockPos::new(1, 2, 3), CHEST);
        be.on_placed(BlockPos::new(4, 2, 3), FURNACE);
        be.on_placed(BlockPos::new(5, 2, 3), STONE);
        assert_eq!(be.0.len(), 2);
        assert!(matches!(
            be.0[&BlockPos::new(1, 2, 3)],
            BlockEntity::Chest(_)
        ));
    }

    #[test]
    fn breaking_chest_drops_contents() {
        let mut be = BlockEntities::default();
        let p = BlockPos::new(1, 61, 1);
        be.on_placed(p, CHEST);
        if let Some(BlockEntity::Chest(c)) = be.0.get_mut(&p) {
            c.slots.0[0] = st("coal", 9);
            c.slots.0[5] = st("stick", 3);
        }
        let mut g = GroundItems::default();
        be.on_broken(p, &mut g);
        assert!(be.0.is_empty());
        assert_eq!(g.0.len(), 2);
    }

    #[test]
    fn breaking_furnace_drops_input_fuel_output() {
        let mut be = BlockEntities::default();
        let p = BlockPos::new(1, 61, 1);
        be.on_placed(p, FURNACE);
        if let Some(BlockEntity::Furnace(f)) = be.0.get_mut(&p) {
            f.input = st("sand", 2);
            f.fuel = st("coal", 1);
            f.output = st("glass", 1);
        }
        let mut g = GroundItems::default();
        be.on_broken(p, &mut g);
        assert_eq!(g.0.len(), 3);
    }

    #[test]
    fn furnace_lit_state_reported() {
        let mut be = BlockEntities::default();
        let p = BlockPos::new(1, 61, 1);
        be.on_placed(p, FURNACE);
        assert_eq!(be.tick_furnaces(1.0), vec![]);
        if let Some(BlockEntity::Furnace(f)) = be.0.get_mut(&p) {
            f.input = st("sand", 1);
            f.fuel = st("coal", 1);
        }
        assert_eq!(be.tick_furnaces(1.0), vec![(p, true)]);
        assert_eq!(be.tick_furnaces(1.0), vec![]);
        for _ in 0..70 {
            be.tick_furnaces(1.0);
        }
        assert!(matches!(&be.0[&p], BlockEntity::Furnace(f) if f.burn_left == 0.0));
    }
}
