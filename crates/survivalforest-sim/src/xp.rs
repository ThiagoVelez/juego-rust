//! Experiencia: puntos, niveles y orbe al morir.

use serde::{Deserialize, Serialize};
use survivalforest_core::tuning::tuning;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Experience {
    pub level: u32,
    /// Puntos dentro del nivel actual.
    pub points: f32,
    /// Puntos ganados en total (desde la última muerte).
    pub total: f32,
}

impl Experience {
    pub fn add(&mut self, p: f32) {
        if p <= 0.0 {
            return;
        }
        self.points += p;
        self.total += p;
        let t = tuning();
        while self.points >= t.xp_to_next(self.level) {
            self.points -= t.xp_to_next(self.level);
            self.level += 1;
        }
    }

    /// Fracción del nivel actual (0..1), para la barra.
    pub fn progress(&self) -> f32 {
        (self.points / tuning().xp_to_next(self.level)).clamp(0.0, 1.0)
    }

    pub fn spend_levels(&mut self, n: u32) -> bool {
        if self.level < n {
            return false;
        }
        self.level -= n;
        self.points = 0.0;
        true
    }

    /// Al morir: la mitad de los puntos totales queda en el orbe; el resto se pierde.
    pub fn drop_on_death(&mut self) -> f32 {
        let orb = self.total / 2.0;
        *self = Experience::default();
        orb
    }
}
