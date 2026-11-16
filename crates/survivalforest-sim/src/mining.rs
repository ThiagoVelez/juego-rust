//! Romper bloques con herramientas: tiempo, qué sueltan, experiencia y desgaste.

use crate::inventory::ItemStack;
use survivalforest_core::GameMode;
use survivalforest_core::block::{BlockId, block};
use survivalforest_core::item::{block_rules, item, item_by_key};
use survivalforest_core::tuning::tuning;

/// ¿La herramienta en la mano sirve para soltar este bloque (clase y nivel)?
fn tool_ok(b: BlockId, held: Option<&ItemStack>) -> bool {
    let r = block_rules(b);
    let Some(class) = r.tool else { return true };
    let t = held.and_then(|h| item(h.item).tool.as_ref());
    match t {
        Some(t) => t.class == class && t.tier >= r.min_tier,
        None => r.min_tier == 0,
    }
}

/// Segundos para romper el bloque (None si no se puede romper).
pub fn break_time(b: BlockId, held: Option<&ItemStack>, mode: GameMode) -> Option<f32> {
    if !block(b).breakable {
        return None;
    }
    if mode == GameMode::Creative {
        return Some(0.0);
    }
    let r = block_rules(b);
    if r.min_tier > 0 && !tool_ok(b, held) {
        return Some(r.hardness * tuning().reach_hand_break_mult);
    }
    let speed = held
        .and_then(|h| item(h.item).tool.as_ref())
        .filter(|t| Some(t.class) == r.tool)
        .map_or(1.0, |t| t.speed);
    Some(r.hardness * 1.5 / speed)
}

/// Número pseudoaleatorio en [0, 1) a partir de una semilla (determinista).
fn roll(seed: u64) -> f64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Lo que suelta el bloque y la experiencia que da.
pub fn drops_for(
    b: BlockId,
    held: Option<&ItemStack>,
    mode: GameMode,
    rng_seed: u64,
) -> (Vec<ItemStack>, u32) {
    if mode == GameMode::Creative {
        return (vec![], 0);
    }
    let r = block_rules(b);
    if (r.needs_tool_to_drop || r.min_tier > 0) && !tool_ok(b, held) {
        return (vec![], 0);
    }
    let mut out = vec![];
    for (i, d) in r.drops.iter().enumerate() {
        let s = rng_seed.wrapping_mul(31).wrapping_add(i as u64 * 7919);
        if roll(s) >= d.chance as f64 {
            continue;
        }
        let span = (d.max - d.min) as f64 + 1.0;
        let n = d.min as u16 + (roll(s ^ 0xABCD) * span).floor() as u16;
        if let Some(it) = item_by_key(&d.item)
            && n > 0
        {
            out.push(ItemStack::new(it, n));
        }
    }
    let xp = tuning().xp_for_ore(block(b).key) as u32;
    (out, xp)
}

/// Gasta la herramienta de la mano al romper `b`. Devuelve si se rompió.
pub fn wear_tool(held: &mut Option<ItemStack>, b: BlockId) -> bool {
    let Some(s) = held else { return false };
    let Some(t) = item(s.item).tool.as_ref() else {
        return false;
    };
    s.wear += if Some(t.class) == block_rules(b).tool {
        1
    } else {
        2
    };
    if s.wear >= t.durability {
        *held = None;
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ItemStack;
    use survivalforest_core::GameMode::{Creative, Survival};
    use survivalforest_core::block::{COAL_ORE, IRON_ORE, PLATANERA, RED_SOIL, STONE};
    use survivalforest_core::item::item_by_key;

    fn tool(k: &str) -> ItemStack {
        ItemStack::new(item_by_key(k).unwrap(), 1)
    }
    fn keys(v: &[ItemStack]) -> Vec<(&str, u16)> {
        v.iter()
            .map(|s| {
                (
                    survivalforest_core::item::item(s.item).key.as_str(),
                    s.count,
                )
            })
            .collect()
    }

    #[test]
    fn stone_by_hand_slow_and_drops_nothing() {
        let hand = break_time(STONE, None, Survival).unwrap();
        let pick = break_time(STONE, Some(&tool("pick_wood")), Survival).unwrap();
        assert!(hand > pick * 2.0, "{hand} {pick}");
        assert!(drops_for(STONE, None, Survival, 1).0.is_empty());
    }

    #[test]
    fn stone_with_wood_pick_drops_cobble() {
        assert_eq!(
            keys(&drops_for(STONE, Some(&tool("pick_wood")), Survival, 1).0),
            vec![("cobble", 1)]
        );
    }

    #[test]
    fn soil_by_hand_drops_itself() {
        assert_eq!(
            keys(&drops_for(RED_SOIL, None, Survival, 1).0),
            vec![("red_soil", 1)]
        );
    }

    #[test]
    fn iron_ore_needs_stone_pick() {
        assert!(
            drops_for(IRON_ORE, Some(&tool("pick_wood")), Survival, 1)
                .0
                .is_empty()
        );
        let (d, xp) = drops_for(IRON_ORE, Some(&tool("pick_stone")), Survival, 1);
        assert_eq!(keys(&d), vec![("iron_ore_item", 1)]);
        assert_eq!(xp, 3);
    }

    #[test]
    fn iron_pick_faster_than_wood() {
        let w = break_time(STONE, Some(&tool("pick_wood")), Survival).unwrap();
        let i = break_time(STONE, Some(&tool("pick_iron")), Survival).unwrap();
        assert!((w / i - 3.0).abs() < 1e-4);
        // Herramienta equivocada: como a mano.
        assert_eq!(
            break_time(STONE, Some(&tool("axe_iron")), Survival),
            break_time(STONE, None, Survival)
        );
    }

    #[test]
    fn coal_ore_gives_coal_and_1_xp() {
        let (d, xp) = drops_for(COAL_ORE, Some(&tool("pick_wood")), Survival, 9);
        assert_eq!(keys(&d), vec![("coal", 1)]);
        assert_eq!(xp, 1);
    }

    #[test]
    fn wear_right_tool_1_wrong_tool_2_breaks_at_zero() {
        let mut held = Some(tool("pick_wood"));
        assert!(!wear_tool(&mut held, STONE));
        assert_eq!(held.as_ref().unwrap().wear, 1);
        assert!(!wear_tool(&mut held, RED_SOIL));
        assert_eq!(held.as_ref().unwrap().wear, 3);
        held.as_mut().unwrap().wear = 59;
        assert!(wear_tool(&mut held, STONE));
        assert!(held.is_none());
        let mut food = Some(ItemStack::new(item_by_key("platano").unwrap(), 2));
        assert!(!wear_tool(&mut food, STONE));
        assert_eq!(food.unwrap().count, 2);
    }

    #[test]
    fn creative_instant_no_drops() {
        assert_eq!(break_time(STONE, None, Creative), Some(0.0));
        assert_eq!(drops_for(STONE, None, Creative, 1), (vec![], 0));
    }

    #[test]
    fn bedrock_unbreakable() {
        assert_eq!(
            break_time(
                survivalforest_core::block::BEDROCK,
                Some(&tool("pick_iron")),
                Survival
            ),
            None
        );
    }

    #[test]
    fn platanera_drops_1_to_3_deterministic() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..50u64 {
            let a = drops_for(PLATANERA, None, Survival, seed).0;
            assert_eq!(a, drops_for(PLATANERA, None, Survival, seed).0);
            let n = a[0].count;
            assert!((1..=3).contains(&n));
            seen.insert(n);
        }
        assert_eq!(seen.len(), 3);
    }
}
