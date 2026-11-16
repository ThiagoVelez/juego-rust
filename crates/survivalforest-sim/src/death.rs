//! Morir: todo el inventario cae donde murió el jugador, con la mitad de la experiencia en un orbe.

use crate::drops::{GroundItems, scatter};
use crate::inventory::Inventory;
use crate::xp::Experience;
use glam::DVec3;
use survivalforest_core::tuning::tuning;

pub fn die(inv: &mut Inventory, xp: &mut Experience, ground: &mut GroundItems, at: DVec3) {
    let ttl = tuning().death_item_ttl;
    scatter(ground, at, inv.slots.drain_all(), ttl);
    let orb = xp.drop_on_death();
    ground.spawn_orb(at, orb, ttl);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ItemStack;
    use survivalforest_core::item::item_by_key;

    #[test]
    fn death_drops_everything_and_half_xp_orb() {
        let mut inv = Inventory::default();
        inv.slots.0[0] = Some(ItemStack::new(item_by_key("coal").unwrap(), 5));
        inv.slots.0[20] = Some(ItemStack::new(item_by_key("pick_wood").unwrap(), 1));
        let mut xp = Experience::default();
        xp.add(20.0);
        let mut g = GroundItems::default();
        die(&mut inv, &mut xp, &mut g, DVec3::new(1.0, 70.0, 1.0));
        assert!(inv.slots.is_empty());
        assert_eq!(xp, Experience::default());
        let items: Vec<_> = g.0.iter().filter_map(|i| i.stack.as_ref()).collect();
        assert_eq!(items.len(), 2);
        assert!(g.0.iter().all(|i| i.ttl == 600.0));
        assert_eq!(g.0.iter().map(|i| i.xp).sum::<f32>(), 10.0);
    }
}
