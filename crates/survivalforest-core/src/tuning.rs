//! Valores ajustables de la supervivencia (`data/tuning.ron`).

use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Tuning {
    pub max_health: f32,
    pub max_hunger: f32,
    pub air_secs: f32,
    pub air_bubbles: u32,
    pub regen_min_hunger: f32,
    pub regen_every_secs: f32,
    pub starve_every_secs: f32,
    pub starve_floor: f32,
    pub drown_damage_per_sec: f32,
    pub fall_safe_blocks: f32,
    pub exhaustion_walk_per_sec: f32,
    pub exhaustion_sprint_per_sec: f32,
    pub exhaustion_jump: f32,
    pub exhaustion_regen: f32,
    pub exhaustion_per_hunger: f32,
    pub xp_base: f32,
    pub xp_per_level: f32,
    pub xp_first_craft: f32,
    pub xp_ores: Vec<(String, f32)>,
    pub smelt_secs: f32,
    pub ground_item_ttl: f32,
    pub death_item_ttl: f32,
    pub pickup_radius: f32,
    pub pickup_delay: f32,
    pub merge_radius: f32,
    /// Multiplicador del tiempo de rotura de una mena sin el pico necesario.
    pub reach_hand_break_mult: f32,
}

impl Tuning {
    pub fn xp_for_ore(&self, block_key: &str) -> f32 {
        self.xp_ores
            .iter()
            .find(|(k, _)| k == block_key)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    }
    /// Puntos para pasar del nivel `level` al siguiente.
    pub fn xp_to_next(&self, level: u32) -> f32 {
        self.xp_base + self.xp_per_level * level as f32
    }
}

pub fn tuning() -> &'static Tuning {
    static T: OnceLock<Tuning> = OnceLock::new();
    T.get_or_init(|| {
        ron::from_str(include_str!("../data/tuning.ron"))
            .unwrap_or_else(|e| panic!("data/tuning.ron: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_matches_spec() {
        let t = tuning();
        assert_eq!(
            (t.max_health, t.max_hunger, t.air_secs, t.air_bubbles),
            (20.0, 20.0, 15.0, 10)
        );
        assert_eq!(
            (
                t.regen_min_hunger,
                t.regen_every_secs,
                t.starve_every_secs,
                t.starve_floor
            ),
            (18.0, 4.0, 4.0, 1.0)
        );
        assert_eq!((t.fall_safe_blocks, t.drown_damage_per_sec), (3.0, 1.0));
        assert_eq!(
            (
                t.smelt_secs,
                t.ground_item_ttl,
                t.death_item_ttl,
                t.pickup_radius
            ),
            (8.0, 300.0, 600.0, 1.5)
        );
        assert_eq!(t.xp_for_ore("iron_ore"), 3.0);
        assert_eq!(t.xp_for_ore("gold_ore"), 5.0);
        assert_eq!(t.xp_for_ore("stone"), 0.0);
        assert_eq!(t.xp_to_next(0), 7.0);
        assert_eq!(t.xp_to_next(2), 13.0);
    }
}
