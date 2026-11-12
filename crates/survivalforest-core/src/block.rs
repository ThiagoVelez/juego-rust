//! Registro de bloques de la etapa 1.

use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(
    Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Serialize, Deserialize, PartialOrd, Ord,
)]
pub struct BlockId(pub u16);

#[derive(Debug)]
pub struct BlockDef {
    pub id: BlockId,
    pub key: &'static str,
    pub name_es: &'static str,
    pub opaque: bool,
    pub solid: bool,
    pub breakable: bool,
    pub emits_light: u8,
    /// Capas de textura: arriba, lado, abajo (índices en `TILE_NAMES`).
    pub tex: [u16; 3],
}

/// Nombres de las capas de textura, en el orden del array de texturas del cliente.
pub const TILE_NAMES: &[&str] = &[
    "grass_top",
    "grass_side",
    "red_soil",
    "stone",
    "sand",
    "red_sand",
    "gravel",
    "clay",
    "mud",
    "snow",
    "ice",
    "water",
    "log_side",
    "log_top",
    "leaves",
    "coffee_bush",
    "bamboo",
    "frailejon",
    "planks",
    "whitewash",
    "roof_tile",
    "coal_ore",
    "iron_ore",
    "copper_ore",
    "gold_ore",
    "quartz_ore",
    "torch",
    "bedrock_floor",
    "snow_side",
];

const fn t(name: &str) -> u16 {
    let mut i = 0;
    while i < TILE_NAMES.len() {
        if eq(TILE_NAMES[i], name) {
            return i as u16;
        }
        i += 1;
    }
    panic!("textura desconocida");
}

const fn eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

macro_rules! blocks {
    ($( $id:ident = $n:expr, $key:literal, $name:literal, op=$op:expr, so=$so:expr, br=$br:expr, li=$li:expr, tex=[$a:literal, $b:literal, $c:literal]; )*) => {
        $( pub const $id: BlockId = BlockId($n); )*
        static BLOCKS: &[BlockDef] = &[
            $( BlockDef { id: BlockId($n), key: $key, name_es: $name, opaque: $op, solid: $so, breakable: $br, emits_light: $li, tex: [t($a), t($b), t($c)] }, )*
        ];
    };
}

blocks! {
    AIR = 0, "air", "Aire", op=false, so=false, br=false, li=0, tex=["stone", "stone", "stone"];
    BEDROCK = 1, "bedrock_floor", "Lecho de roca", op=true, so=true, br=false, li=0, tex=["bedrock_floor", "bedrock_floor", "bedrock_floor"];
    STONE = 2, "stone", "Piedra", op=true, so=true, br=true, li=0, tex=["stone", "stone", "stone"];
    RED_SOIL = 3, "red_soil", "Tierra roja", op=true, so=true, br=true, li=0, tex=["red_soil", "red_soil", "red_soil"];
    GRASS = 4, "grass", "Hierba", op=true, so=true, br=true, li=0, tex=["grass_top", "grass_side", "red_soil"];
    SAND = 5, "sand", "Arena", op=true, so=true, br=true, li=0, tex=["sand", "sand", "sand"];
    RED_SAND = 6, "red_sand", "Arena roja", op=true, so=true, br=true, li=0, tex=["red_sand", "red_sand", "red_sand"];
    GRAVEL = 7, "gravel", "Grava", op=true, so=true, br=true, li=0, tex=["gravel", "gravel", "gravel"];
    CLAY = 8, "clay", "Arcilla", op=true, so=true, br=true, li=0, tex=["clay", "clay", "clay"];
    MUD = 9, "mud", "Barro de manglar", op=true, so=true, br=true, li=0, tex=["mud", "mud", "mud"];
    SNOW = 10, "snow", "Nieve", op=true, so=true, br=true, li=0, tex=["snow", "snow_side", "red_soil"];
    ICE = 11, "ice", "Hielo", op=true, so=true, br=true, li=0, tex=["ice", "ice", "ice"];
    WATER = 12, "water", "Agua", op=false, so=false, br=false, li=0, tex=["water", "water", "water"];
    LOG = 13, "log", "Tronco", op=true, so=true, br=true, li=0, tex=["log_top", "log_side", "log_top"];
    LEAVES = 14, "leaves", "Hojas", op=false, so=true, br=true, li=0, tex=["leaves", "leaves", "leaves"];
    COFFEE_BUSH = 15, "coffee_bush", "Cafeto", op=false, so=true, br=true, li=0, tex=["coffee_bush", "coffee_bush", "coffee_bush"];
    BAMBOO = 16, "bamboo", "Guadua", op=false, so=true, br=true, li=0, tex=["bamboo", "bamboo", "bamboo"];
    FRAILEJON = 17, "frailejon", "Frailejón", op=false, so=true, br=true, li=0, tex=["frailejon", "frailejon", "frailejon"];
    PLANKS = 18, "planks", "Tablones", op=true, so=true, br=true, li=0, tex=["planks", "planks", "planks"];
    WHITEWASH = 19, "whitewash", "Pared encalada", op=true, so=true, br=true, li=0, tex=["whitewash", "whitewash", "whitewash"];
    ROOF_TILE = 20, "roof_tile", "Teja", op=true, so=true, br=true, li=0, tex=["roof_tile", "roof_tile", "roof_tile"];
    COAL_ORE = 21, "coal_ore", "Carbón", op=true, so=true, br=true, li=0, tex=["coal_ore", "coal_ore", "coal_ore"];
    IRON_ORE = 22, "iron_ore", "Hierro", op=true, so=true, br=true, li=0, tex=["iron_ore", "iron_ore", "iron_ore"];
    COPPER_ORE = 23, "copper_ore", "Cobre", op=true, so=true, br=true, li=0, tex=["copper_ore", "copper_ore", "copper_ore"];
    GOLD_ORE = 24, "gold_ore", "Oro", op=true, so=true, br=true, li=0, tex=["gold_ore", "gold_ore", "gold_ore"];
    QUARTZ_ORE = 25, "quartz_ore", "Cuarzo", op=true, so=true, br=true, li=0, tex=["quartz_ore", "quartz_ore", "quartz_ore"];
    TORCH = 26, "torch", "Antorcha", op=false, so=false, br=true, li=14, tex=["torch", "torch", "torch"];
}

pub fn blocks() -> &'static [BlockDef] {
    BLOCKS
}

pub fn block(id: BlockId) -> &'static BlockDef {
    BLOCKS.get(id.0 as usize).unwrap_or(&BLOCKS[0])
}

pub fn by_key(key: &str) -> Option<BlockId> {
    BLOCKS.iter().find(|b| b.key == key).map(|b| b.id)
}

impl BlockId {
    pub fn def(self) -> &'static BlockDef {
        block(self)
    }
    pub fn is_air(self) -> bool {
        self == AIR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_match_indices_and_keys_unique() {
        for (i, b) in blocks().iter().enumerate() {
            assert_eq!(b.id.0 as usize, i);
        }
        let mut keys: Vec<_> = blocks().iter().map(|b| b.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), blocks().len());
        assert!(!block(by_key("bedrock_floor").unwrap()).breakable);
        assert_eq!(block(by_key("torch").unwrap()).emits_light, 14);
    }

    #[test]
    fn transparency_rules() {
        for k in [
            "air",
            "water",
            "leaves",
            "coffee_bush",
            "bamboo",
            "frailejon",
            "torch",
        ] {
            assert!(!block(by_key(k).unwrap()).opaque, "{k}");
        }
        for k in ["air", "water", "torch"] {
            assert!(!block(by_key(k).unwrap()).solid, "{k}");
        }
        assert!(block(STONE).opaque && block(STONE).solid);
    }

    #[test]
    fn textures_in_range() {
        for b in blocks() {
            for t in b.tex {
                assert!((t as usize) < TILE_NAMES.len(), "{}", b.key);
            }
        }
    }
}
