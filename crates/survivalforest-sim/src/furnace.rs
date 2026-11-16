//! Horno de barro y baúl.

use crate::inventory::{ItemStack, Slots};
use serde::{Deserialize, Serialize};
use survivalforest_core::item::{item, item_by_key};
use survivalforest_core::recipe::{SmeltRecipe, smelt_recipes};
use survivalforest_core::tuning::tuning;

pub const CHEST_SLOTS: usize = 27;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chest {
    pub slots: Slots,
}

impl Default for Chest {
    fn default() -> Self {
        Self {
            slots: Slots::new(CHEST_SLOTS),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Furnace {
    pub input: Option<ItemStack>,
    pub fuel: Option<ItemStack>,
    pub output: Option<ItemStack>,
    /// Segundos de fuego que quedan y que daba el último combustible.
    pub burn_left: f32,
    pub burn_total: f32,
    /// Segundos de la fundición en curso.
    pub progress: f32,
    pub xp_stored: f32,
}

impl Furnace {
    fn recipe(&self) -> Option<&'static SmeltRecipe> {
        let k = &item(self.input.as_ref()?.item).key;
        smelt_recipes().iter().find(|r| &r.input == k)
    }

    /// ¿Hay algo que fundir y sitio para el resultado?
    fn can_smelt(&self) -> bool {
        let Some(r) = self.recipe() else { return false };
        let Some(out) = item_by_key(&r.output) else {
            return false;
        };
        match &self.output {
            None => true,
            Some(o) => o.item == out && o.count < o.max(),
        }
    }

    fn finish_one(&mut self) {
        let r = self.recipe().expect("receta");
        let out = item_by_key(&r.output).expect("salida");
        match &mut self.output {
            Some(o) => o.count += 1,
            None => self.output = Some(ItemStack::new(out, 1)),
        }
        self.xp_stored += r.xp;
        let i = self.input.as_mut().expect("entrada");
        i.count -= 1;
        if i.count == 0 {
            self.input = None;
        }
    }

    /// Avanza el horno `dt` segundos. Devuelve si está encendido.
    pub fn tick(&mut self, mut dt: f32) -> bool {
        let smelt = tuning().smelt_secs;
        while dt > 0.0 {
            if self.burn_left <= 0.0 && self.can_smelt() {
                let fuel = self.fuel.as_ref().and_then(|f| item(f.item).fuel_smelts);
                if let Some(n) = fuel {
                    self.burn_total = n * smelt;
                    self.burn_left = self.burn_total;
                    let f = self.fuel.as_mut().expect("combustible");
                    f.count -= 1;
                    if f.count == 0 {
                        self.fuel = None;
                    }
                }
            }
            if self.burn_left <= 0.0 {
                self.progress = 0.0;
                break;
            }
            // Avanza hasta el siguiente suceso: se acaba el fuego o termina una fundición.
            let to_done = if self.can_smelt() {
                smelt - self.progress
            } else {
                f32::INFINITY
            };
            let step = dt.min(self.burn_left).min(to_done);
            self.burn_left -= step;
            dt -= step;
            if self.can_smelt() {
                self.progress += step;
                if self.progress >= smelt - 1e-4 {
                    self.progress = 0.0;
                    self.finish_one();
                }
            } else {
                self.progress = 0.0;
            }
            if self.burn_left < 1e-4 {
                self.burn_left = 0.0;
            }
        }
        self.burn_left > 0.0
    }

    /// Saca el resultado y la experiencia acumulada (en puntos enteros; el resto se guarda).
    pub fn take_output(&mut self) -> (Option<ItemStack>, u32) {
        let out = self.output.take();
        if out.is_none() {
            return (None, 0);
        }
        let xp = self.xp_stored.floor();
        self.xp_stored -= xp;
        (out, xp as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ItemStack;
    use survivalforest_core::item::item_by_key;

    fn st(k: &str, n: u16) -> Option<ItemStack> {
        Some(ItemStack::new(item_by_key(k).unwrap(), n))
    }

    #[test]
    fn furnace_smelts_in_8s_with_coal_for_8_items() {
        let mut f = Furnace {
            input: st("iron_ore_item", 8),
            fuel: st("coal", 1),
            ..Default::default()
        };
        for _ in 0..64 {
            f.tick(1.0);
        }
        assert_eq!(f.output, st("iron_ingot", 8));
        assert!(f.input.is_none() && f.fuel.is_none());
        assert_eq!(f.burn_left, 0.0);
    }

    #[test]
    fn furnace_without_fuel_does_nothing() {
        let mut f = Furnace {
            input: st("sand", 2),
            ..Default::default()
        };
        assert!(!f.tick(20.0));
        assert!(f.output.is_none());
    }

    #[test]
    fn furnace_stops_when_output_full() {
        let mut f = Furnace {
            input: st("sand", 5),
            fuel: st("coal", 1),
            output: st("glass", 63),
            ..Default::default()
        };
        for _ in 0..40 {
            f.tick(1.0);
        }
        assert_eq!(f.output.as_ref().unwrap().count, 64);
        assert_eq!(f.input.as_ref().unwrap().count, 4);
    }

    #[test]
    fn furnace_xp_on_take() {
        let mut f = Furnace {
            input: st("iron_ore_item", 3),
            fuel: st("coal", 1),
            ..Default::default()
        };
        for _ in 0..24 {
            f.tick(1.0);
        }
        let (out, xp) = f.take_output();
        assert_eq!(out, st("iron_ingot", 3));
        assert_eq!(xp, 3);
        assert_eq!(f.take_output(), (None, 0));
    }

    #[test]
    fn lit_only_while_burning() {
        let mut f = Furnace {
            input: st("platano", 1),
            fuel: st("stick", 1),
            ..Default::default()
        };
        assert!(f.tick(1.0));
        for _ in 0..10 {
            f.tick(1.0);
        }
        assert!(!f.tick(1.0));
    }

    #[test]
    fn chest_has_27_slots() {
        assert_eq!(Chest::default().slots.len(), 27);
    }
}
