//! Vida, hambre, aire y daño por caída.

use serde::{Deserialize, Serialize};
use survivalforest_core::GameMode;
use survivalforest_core::item::Food;
use survivalforest_core::tuning::tuning;

/// Estado de supervivencia del jugador (puntos: 1 corazón = 2 de vida, 1 jamón = 2 de hambre).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vitals {
    pub health: f32,
    pub hunger: f32,
    pub saturation: f32,
    pub exhaustion: f32,
    /// Segundos de aire que quedan bajo el agua.
    pub air: f32,
    pub regen_timer: f32,
    pub starve_timer: f32,
    pub drown_timer: f32,
    /// Altura más alta desde que se dejó el suelo (para el daño por caída).
    pub fall_start_y: Option<f64>,
}

impl Default for Vitals {
    fn default() -> Self {
        let t = tuning();
        Self {
            health: t.max_health,
            hunger: t.max_hunger,
            saturation: 5.0,
            exhaustion: 0.0,
            air: t.air_secs,
            regen_timer: 0.0,
            starve_timer: 0.0,
            drown_timer: 0.0,
            fall_start_y: None,
        }
    }
}

/// Lo que hizo el jugador este tick.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VitalsCtx {
    pub mode: GameMode,
    pub moving: bool,
    pub sprinting: bool,
    pub jumped: bool,
    pub head_in_water: bool,
}

impl Vitals {
    /// Avanza `dt` segundos. Devuelve el daño recibido en este tick.
    pub fn tick(&mut self, dt: f32, c: VitalsCtx) -> f32 {
        let t = tuning();
        if c.mode == GameMode::Creative {
            self.air = t.air_secs;
            self.drown_timer = 0.0;
            return 0.0;
        }
        // Hambre: el esfuerzo se acumula y gasta primero la saciedad.
        self.exhaustion += dt
            * if c.sprinting && c.moving {
                t.exhaustion_sprint_per_sec
            } else if c.moving {
                t.exhaustion_walk_per_sec
            } else {
                0.0
            };
        if c.jumped {
            self.exhaustion += t.exhaustion_jump;
        }
        self.burn_exhaustion();
        let mut dmg = 0.0;
        // Regeneración con el hambre alta.
        if self.hunger >= t.regen_min_hunger && self.health < t.max_health {
            self.regen_timer += dt;
            while self.regen_timer >= t.regen_every_secs - 1e-4 {
                self.regen_timer -= t.regen_every_secs;
                self.health = (self.health + 1.0).min(t.max_health);
                self.exhaustion += t.exhaustion_regen;
            }
            self.burn_exhaustion();
        } else {
            self.regen_timer = 0.0;
        }
        // Hambre a cero: quita vida, pero no mata.
        if self.hunger <= 0.0 {
            self.starve_timer += dt;
            while self.starve_timer >= t.starve_every_secs - 1e-4 {
                self.starve_timer -= t.starve_every_secs;
                let d = (self.health - t.starve_floor).clamp(0.0, 1.0);
                self.health -= d;
                dmg += d;
            }
        } else {
            self.starve_timer = 0.0;
        }
        // Aire bajo el agua.
        if c.head_in_water {
            let used = self.air.min(dt);
            self.air -= used;
            self.drown_timer += dt - used;
            while self.drown_timer >= 1.0 - 1e-4 {
                self.drown_timer -= 1.0;
                self.health -= t.drown_damage_per_sec;
                dmg += t.drown_damage_per_sec;
            }
        } else {
            self.air = (self.air + dt * t.air_secs / 3.0).min(t.air_secs);
            self.drown_timer = 0.0;
        }
        self.health = self.health.max(0.0);
        dmg
    }

    fn burn_exhaustion(&mut self) {
        let per = tuning().exhaustion_per_hunger;
        while self.exhaustion >= per {
            self.exhaustion -= per;
            if self.saturation > 0.0 {
                self.saturation = (self.saturation - 1.0).max(0.0);
            } else {
                self.hunger = (self.hunger - 1.0).max(0.0);
            }
        }
    }

    /// El jugador está en el aire a la altura `y` (se recuerda la más alta).
    pub fn airborne(&mut self, y: f64) {
        self.fall_start_y = Some(self.fall_start_y.map_or(y, |s| s.max(y)));
    }

    /// El jugador toca el suelo (o el agua) a la altura `y`. Devuelve el daño por caída.
    pub fn on_ground(&mut self, y: f64, in_water: bool, mode: GameMode) -> f32 {
        let Some(start) = self.fall_start_y.take() else {
            return 0.0;
        };
        if in_water || mode == GameMode::Creative {
            return 0.0;
        }
        let d = ((start - y) as f32 - tuning().fall_safe_blocks)
            .floor()
            .max(0.0);
        self.health = (self.health - d).max(0.0);
        d
    }

    pub fn eat(&mut self, f: &Food) {
        let t = tuning();
        self.hunger = (self.hunger + f.hunger as f32).min(t.max_hunger);
        self.saturation = (self.saturation + f.saturation).min(self.hunger);
    }

    pub fn is_dead(&self) -> bool {
        self.health <= 0.0
    }

    /// Burbujas de aire que se dibujan (0..=10).
    pub fn air_bubbles(&self) -> u32 {
        let t = tuning();
        ((self.air / t.air_secs) * t.air_bubbles as f32).ceil() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use survivalforest_core::GameMode;
    use survivalforest_core::item::{item, item_by_key};

    fn ctx() -> VitalsCtx {
        VitalsCtx {
            mode: GameMode::Survival,
            moving: false,
            sprinting: false,
            jumped: false,
            head_in_water: false,
        }
    }

    fn run(v: &mut Vitals, secs: u32, c: VitalsCtx) -> f32 {
        (0..secs).map(|_| v.tick(1.0, c)).sum()
    }

    #[test]
    fn regenerates_when_fed() {
        let mut v = Vitals {
            health: 10.0,
            ..Default::default()
        };
        run(&mut v, 4, ctx());
        assert_eq!(v.health, 11.0);
    }

    #[test]
    fn starving_stops_at_one_point() {
        let mut v = Vitals {
            health: 3.0,
            hunger: 0.0,
            saturation: 0.0,
            ..Default::default()
        };
        run(&mut v, 60, ctx());
        assert_eq!(v.health, 1.0);
        assert!(!v.is_dead());
    }

    #[test]
    fn fall_3_blocks_no_damage_10_blocks_7() {
        let mut v = Vitals::default();
        v.airborne(73.0);
        assert_eq!(v.on_ground(70.0, false, GameMode::Survival), 0.0);
        v.airborne(80.0);
        v.airborne(75.0);
        assert_eq!(v.on_ground(70.0, false, GameMode::Survival), 7.0);
        assert_eq!(v.health, 13.0);
    }

    #[test]
    fn fall_into_water_no_damage() {
        let mut v = Vitals::default();
        v.airborne(90.0);
        assert_eq!(v.on_ground(70.0, true, GameMode::Survival), 0.0);
    }

    #[test]
    fn drowning_after_15s_one_per_second() {
        let mut v = Vitals::default();
        let wet = VitalsCtx {
            head_in_water: true,
            ..ctx()
        };
        assert_eq!(run(&mut v, 15, wet), 0.0);
        assert_eq!(v.air, 0.0);
        assert_eq!(run(&mut v, 3, wet), 3.0);
    }

    #[test]
    fn air_refills_out_of_water() {
        let mut v = Vitals {
            air: 0.0,
            ..Default::default()
        };
        run(&mut v, 10, ctx());
        assert_eq!(v.air, 15.0);
        assert_eq!(v.air_bubbles(), 10);
    }

    #[test]
    fn sprinting_drains_hunger_faster() {
        let mut walk = Vitals {
            saturation: 0.0,
            ..Default::default()
        };
        let mut sprint = walk.clone();
        run(
            &mut walk,
            120,
            VitalsCtx {
                moving: true,
                ..ctx()
            },
        );
        run(
            &mut sprint,
            120,
            VitalsCtx {
                moving: true,
                sprinting: true,
                ..ctx()
            },
        );
        assert!(
            sprint.hunger < walk.hunger,
            "{} < {}",
            sprint.hunger,
            walk.hunger
        );
    }

    #[test]
    fn eat_platano_adds_6_capped_at_20() {
        let food = item(item_by_key("platano").unwrap()).food.clone().unwrap();
        let mut v = Vitals {
            hunger: 10.0,
            ..Default::default()
        };
        v.eat(&food);
        assert_eq!(v.hunger, 16.0);
        v.eat(&food);
        assert_eq!(v.hunger, 20.0);
    }

    #[test]
    fn creative_never_damaged() {
        let mut v = Vitals {
            hunger: 0.0,
            health: 5.0,
            ..Default::default()
        };
        let c = VitalsCtx {
            mode: GameMode::Creative,
            head_in_water: true,
            sprinting: true,
            moving: true,
            ..ctx()
        };
        assert_eq!(run(&mut v, 100, c), 0.0);
        v.airborne(200.0);
        assert_eq!(v.on_ground(60.0, false, GameMode::Creative), 0.0);
        assert_eq!(v.health, 5.0);
    }

    #[test]
    fn dies_at_zero() {
        let mut v = Vitals {
            health: 2.0,
            ..Default::default()
        };
        v.airborne(80.0);
        v.on_ground(70.0, false, GameMode::Survival);
        assert!(v.is_dead());
    }
}
