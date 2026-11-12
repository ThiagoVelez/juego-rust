//! Biomas de SurvivalForest.

use survivalforest_core::block::{self, BlockId};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Biome {
    CoffeeMountain,
    Paramo,
    Rainforest,
    BambooValley,
    Savanna,
    RedDesert,
    SnowPeak,
    Coast,
    Ocean,
    Mangrove,
    CloudForest,
}

impl Biome {
    pub const ALL: [Biome; 11] = [
        Biome::CoffeeMountain,
        Biome::Paramo,
        Biome::Rainforest,
        Biome::BambooValley,
        Biome::Savanna,
        Biome::RedDesert,
        Biome::SnowPeak,
        Biome::Coast,
        Biome::Ocean,
        Biome::Mangrove,
        Biome::CloudForest,
    ];

    pub fn name_es(self) -> &'static str {
        match self {
            Biome::CoffeeMountain => "Montaña cafetera",
            Biome::Paramo => "Páramo",
            Biome::Rainforest => "Selva húmeda",
            Biome::BambooValley => "Valle de guaduales",
            Biome::Savanna => "Sabana",
            Biome::RedDesert => "Desierto rojo",
            Biome::SnowPeak => "Nevado",
            Biome::Coast => "Costa",
            Biome::Ocean => "Océano",
            Biome::Mangrove => "Manglar",
            Biome::CloudForest => "Bosque de niebla",
        }
    }

    /// (superficie, subsuelo)
    pub fn surface(self) -> (BlockId, BlockId) {
        use block::*;
        match self {
            Biome::RedDesert => (RED_SAND, RED_SAND),
            Biome::SnowPeak => (SNOW, STONE),
            Biome::Coast => (SAND, SAND),
            Biome::Ocean => (SAND, SAND),
            Biome::Mangrove => (MUD, MUD),
            _ => (GRASS, RED_SOIL),
        }
    }

    /// Selección por altura y clima.
    pub fn pick(height: i32, sea: i32, temperature: f64, humidity: f64, weird: f64) -> Biome {
        if height < sea - 3 {
            return Biome::Ocean;
        }
        if height <= sea + 2 {
            return if temperature > 0.1 && humidity > 0.1 {
                Biome::Mangrove
            } else {
                Biome::Coast
            };
        }
        if height > 150 {
            return if temperature < -0.15 {
                Biome::SnowPeak
            } else {
                Biome::Paramo
            };
        }
        if height > 100 && humidity > 0.2 {
            return Biome::CloudForest;
        }
        if temperature > 0.3 && humidity < -0.15 {
            return Biome::RedDesert;
        }
        if temperature > 0.15 && humidity < 0.1 {
            return Biome::Savanna;
        }
        if humidity > 0.3 && temperature > 0.05 {
            return Biome::Rainforest;
        }
        if humidity > 0.0 && weird > 0.15 {
            return Biome::BambooValley;
        }
        if temperature < -0.35 {
            return Biome::Paramo;
        }
        Biome::CoffeeMountain
    }
}
