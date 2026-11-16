//! Piedra de amolar: repara herramientas gastando niveles de experiencia.

use crate::inventory::ItemStack;
use crate::xp::Experience;
use survivalforest_core::item::item;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepairError {
    NotATool,
    NotWorn,
    NotEnoughLevels { need: u32 },
}

/// Niveles que cuesta dejar la herramienta como nueva (None si no hay nada que reparar).
pub fn repair_cost(stack: &ItemStack) -> Option<u32> {
    let t = item(stack.item).tool.as_ref()?;
    if stack.wear == 0 {
        return None;
    }
    let frac = stack.wear as f32 / t.durability as f32;
    Some(((frac * t.repair_base as f32) - 1e-4).ceil().max(1.0) as u32)
}

pub fn repair(stack: &mut ItemStack, xp: &mut Experience) -> Result<u32, RepairError> {
    if item(stack.item).tool.is_none() {
        return Err(RepairError::NotATool);
    }
    let need = repair_cost(stack).ok_or(RepairError::NotWorn)?;
    if !xp.spend_levels(need) {
        return Err(RepairError::NotEnoughLevels { need });
    }
    stack.wear = 0;
    Ok(need)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ItemStack;
    use crate::xp::Experience;
    use survivalforest_core::item::item_by_key;

    fn tool(k: &str, wear: u16) -> ItemStack {
        ItemStack {
            item: item_by_key(k).unwrap(),
            count: 1,
            wear,
        }
    }

    #[test]
    fn repair_cost_iron_half_worn_is_2() {
        assert_eq!(repair_cost(&tool("pick_iron", 130)), Some(2));
        assert_eq!(repair_cost(&tool("pick_wood", 1)), Some(1));
        assert_eq!(repair_cost(&tool("pick_iron", 0)), None);
        assert_eq!(
            repair_cost(&ItemStack::new(item_by_key("coal").unwrap(), 1)),
            None
        );
    }

    #[test]
    fn repair_fails_without_levels() {
        let mut t = tool("pick_iron", 260);
        let mut xp = Experience {
            level: 3,
            ..Default::default()
        };
        assert_eq!(
            repair(&mut t, &mut xp),
            Err(RepairError::NotEnoughLevels { need: 4 })
        );
        assert_eq!(t.wear, 260);
        xp.level = 5;
        assert_eq!(repair(&mut t, &mut xp), Ok(4));
        assert_eq!((t.wear, xp.level), (0, 1));
    }
}
