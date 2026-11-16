//! Objetos de la etapa 2: bloques que se pueden llevar, herramientas, materiales y comida.
//! Los datos están en `data/items.ron` y `data/block_rules.ron`.

use crate::block::{self, BlockId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Objeto del registro. Se guarda por su clave (no por número) para que añadir objetos
/// no cambie los guardados.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, PartialOrd, Ord)]
pub struct ItemId(pub u16);

impl Serialize for ItemId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&item(*self).key)
    }
}

impl<'de> Deserialize<'de> for ItemId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let k = String::deserialize(d)?;
        item_by_key(&k).ok_or_else(|| serde::de::Error::custom(format!("objeto desconocido: {k}")))
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum ToolClass {
    Pickaxe,
    Axe,
    Shovel,
    Machete,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub class: ToolClass,
    pub tier: u8,
    pub durability: u16,
    pub speed: f32,
    pub repair_base: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Food {
    /// Puntos de hambre (1 jamón = 2 puntos).
    pub hunger: u8,
    pub saturation: f32,
    pub eat_secs: f32,
}

#[derive(Clone, Debug)]
pub struct ItemDef {
    pub id: ItemId,
    pub key: String,
    pub name_es: String,
    pub stack: u16,
    pub places: Option<BlockId>,
    pub tool: Option<Tool>,
    pub food: Option<Food>,
    /// Fundiciones que da como combustible.
    pub fuel_smelts: Option<f32>,
}

#[derive(Deserialize)]
struct ItemRon {
    key: String,
    #[serde(default)]
    name_es: Option<String>,
    #[serde(default)]
    stack: Option<u16>,
    #[serde(default)]
    tool: Option<Tool>,
    #[serde(default)]
    food: Option<Food>,
    #[serde(default)]
    fuel: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Drop {
    pub item: String,
    pub min: u8,
    pub max: u8,
    pub chance: f32,
}

#[derive(Deserialize)]
struct RulesRon {
    hardness: f32,
    tool: Option<ToolClass>,
    min_tier: u8,
    needs_tool_to_drop: bool,
    drops: Option<Vec<Drop>>,
}

/// Reglas de un bloque al romperlo.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockRules {
    pub hardness: f32,
    pub tool: Option<ToolClass>,
    pub min_tier: u8,
    pub needs_tool_to_drop: bool,
    /// Qué suelta (ya resuelto: «el propio bloque» aparece con su clave).
    pub drops: Vec<Drop>,
}

struct Registry {
    items: Vec<ItemDef>,
    by_key: HashMap<String, ItemId>,
    by_block: HashMap<BlockId, ItemId>,
    rules: Vec<BlockRules>,
}

/// ¿El bloque se puede llevar como objeto?
fn carriable(b: &block::BlockDef) -> bool {
    b.breakable && b.id != block::FURNACE_LIT
}

fn registry() -> &'static Registry {
    static R: OnceLock<Registry> = OnceLock::new();
    R.get_or_init(|| {
        let extra: Vec<ItemRon> = ron::from_str(include_str!("../data/items.ron"))
            .unwrap_or_else(|e| panic!("data/items.ron: {e}"));
        let mut items: Vec<ItemDef> = Vec::new();
        for b in block::blocks().iter().filter(|b| carriable(b)) {
            items.push(ItemDef {
                id: ItemId(items.len() as u16),
                key: b.key.into(),
                name_es: b.name_es.into(),
                stack: 64,
                places: Some(b.id),
                tool: None,
                food: None,
                fuel_smelts: None,
            });
        }
        for e in extra {
            if let Some(it) = items.iter_mut().find(|i| i.key == e.key) {
                if let Some(f) = e.fuel {
                    it.fuel_smelts = Some(f);
                }
                continue;
            }
            let stack = e.stack.unwrap_or(if e.tool.is_some() { 1 } else { 64 });
            items.push(ItemDef {
                id: ItemId(items.len() as u16),
                name_es: e
                    .name_es
                    .unwrap_or_else(|| panic!("data/items.ron: {} sin nombre", e.key)),
                key: e.key,
                stack,
                places: None,
                tool: e.tool,
                food: e.food,
                fuel_smelts: e.fuel,
            });
        }
        let by_key = items.iter().map(|i| (i.key.clone(), i.id)).collect();
        let by_block = items
            .iter()
            .filter_map(|i| i.places.map(|b| (b, i.id)))
            .collect();
        let raw: HashMap<String, RulesRon> = ron::from_str(include_str!("../data/block_rules.ron"))
            .unwrap_or_else(|e| panic!("data/block_rules.ron: {e}"));
        let rules = block::blocks()
            .iter()
            .map(|b| {
                let own = || {
                    if carriable(b) {
                        vec![Drop {
                            item: b.key.into(),
                            min: 1,
                            max: 1,
                            chance: 1.0,
                        }]
                    } else {
                        vec![]
                    }
                };
                match raw.get(b.key) {
                    Some(r) => BlockRules {
                        hardness: r.hardness,
                        tool: r.tool,
                        min_tier: r.min_tier,
                        needs_tool_to_drop: r.needs_tool_to_drop,
                        drops: r.drops.clone().unwrap_or_else(own),
                    },
                    None => BlockRules {
                        hardness: 1.0,
                        tool: None,
                        min_tier: 0,
                        needs_tool_to_drop: false,
                        drops: own(),
                    },
                }
            })
            .collect();
        Registry {
            items,
            by_key,
            by_block,
            rules,
        }
    })
}

pub fn items() -> &'static [ItemDef] {
    &registry().items
}

pub fn item(id: ItemId) -> &'static ItemDef {
    &registry().items[id.0 as usize]
}

pub fn item_by_key(key: &str) -> Option<ItemId> {
    registry().by_key.get(key).copied()
}

pub fn item_for_block(b: BlockId) -> Option<ItemId> {
    registry().by_block.get(&b).copied()
}

pub fn block_rules(b: BlockId) -> &'static BlockRules {
    let r = &registry().rules;
    r.get(b.0 as usize).unwrap_or(&r[0])
}

impl ItemId {
    pub fn def(self) -> &'static ItemDef {
        item(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{self, BEDROCK, COAL_ORE, GOLD_ORE, IRON_ORE, RED_SOIL, STONE};

    fn key(k: &str) -> ItemId {
        item_by_key(k).unwrap_or_else(|| panic!("falta {k}"))
    }

    #[test]
    fn items_load_and_keys_unique() {
        let mut keys: Vec<_> = items().iter().map(|i| i.key.as_str()).collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n);
        for (i, it) in items().iter().enumerate() {
            assert_eq!(it.id.0 as usize, i);
        }
        assert_eq!(item(key("pick_iron")).tool.as_ref().unwrap().tier, 4);
        assert_eq!(item(key("pick_wood")).tool.as_ref().unwrap().durability, 60);
        assert_eq!(item(key("pick_wood")).stack, 1);
        assert_eq!(item(key("stick")).stack, 64);
    }

    #[test]
    fn sixteen_tools_with_spec_tiers() {
        let tiers = [
            ("wood", 1, 60, 2.0),
            ("stone", 2, 130, 4.0),
            ("copper", 3, 190, 5.0),
            ("iron", 4, 260, 6.0),
        ];
        for class in ["pick", "axe", "shovel", "machete"] {
            for (m, tier, dur, speed) in tiers {
                let t = item(key(&format!("{class}_{m}"))).tool.clone().unwrap();
                assert_eq!(
                    (t.tier, t.durability, t.speed),
                    (tier, dur, speed),
                    "{class}_{m}"
                );
                assert_eq!(t.repair_base, tier);
            }
        }
    }

    #[test]
    fn every_breakable_block_has_item_and_rules() {
        for b in block::blocks() {
            if b.breakable && b.id != block::FURNACE_LIT {
                let it = item_for_block(b.id).unwrap_or_else(|| panic!("sin objeto: {}", b.key));
                assert_eq!(item(it).places, Some(b.id));
            }
            let _ = block_rules(b.id);
        }
        assert!(item_for_block(BEDROCK).is_none());
    }

    #[test]
    fn ore_rules_match_spec() {
        assert_eq!(block_rules(COAL_ORE).min_tier, 1);
        assert_eq!(block_rules(IRON_ORE).min_tier, 2);
        assert_eq!(block_rules(GOLD_ORE).min_tier, 3);
        assert_eq!(block_rules(IRON_ORE).tool, Some(ToolClass::Pickaxe));
    }

    #[test]
    fn stone_needs_pick() {
        let r = block_rules(STONE);
        assert!(r.needs_tool_to_drop);
        assert_eq!(r.tool, Some(ToolClass::Pickaxe));
        assert_eq!(r.drops[0].item, "cobble");
        let soil = block_rules(RED_SOIL);
        assert!(!soil.needs_tool_to_drop);
        assert_eq!(soil.min_tier, 0);
    }

    #[test]
    fn food_values() {
        for (k, h) in [
            ("platano", 6),
            ("platano_asado", 12),
            ("guayaba", 4),
            ("mora", 2),
        ] {
            assert_eq!(item(key(k)).food.as_ref().unwrap().hunger, h, "{k}");
        }
        assert_eq!(item(key("mora")).food.as_ref().unwrap().eat_secs, 0.8);
        assert_eq!(item(key("platano")).food.as_ref().unwrap().eat_secs, 1.6);
    }

    #[test]
    fn item_id_serializes_as_key() {
        let id = key("coal");
        let text = ron::to_string(&id).unwrap();
        assert_eq!(text, "\"coal\"");
        assert_eq!(ron::from_str::<ItemId>(&text).unwrap(), id);
        assert!(ron::from_str::<ItemId>("\"nada\"").is_err());
    }
}
