//! Objetos y orbes de experiencia en el suelo.

use crate::inventory::{Inventory, ItemStack};
use crate::physics::{Aabb, move_and_collide};
use crate::xp::Experience;
use glam::DVec3;
use serde::{Deserialize, Serialize};
use survivalforest_core::tuning::tuning;
use survivalforest_world::ChunkAccess;

const ITEM_SIZE: f64 = 0.25;
const ITEM_GRAVITY: f64 = 20.0;

/// Un objeto (o un orbe, si `stack` es None y `xp` > 0) tirado en el mundo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroundItem {
    pub pos: DVec3,
    pub vel: DVec3,
    pub stack: Option<ItemStack>,
    pub xp: f32,
    pub age: f32,
    pub ttl: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroundItems(pub Vec<GroundItem>);

fn aabb(p: DVec3) -> Aabb {
    let h = ITEM_SIZE / 2.0;
    Aabb {
        min: p - DVec3::new(h, 0.0, h),
        max: p + DVec3::new(h, ITEM_SIZE, h),
    }
}

impl GroundItems {
    pub fn spawn(&mut self, pos: DVec3, stack: ItemStack, ttl: f32) {
        self.0.push(GroundItem {
            pos,
            vel: DVec3::ZERO,
            stack: Some(stack),
            xp: 0.0,
            age: 0.0,
            ttl,
        });
    }

    pub fn spawn_orb(&mut self, pos: DVec3, xp: f32, ttl: f32) {
        if xp > 0.0 {
            self.0.push(GroundItem {
                pos,
                vel: DVec3::ZERO,
                stack: None,
                xp,
                age: 0.0,
                ttl,
            });
        }
    }

    /// Gravedad, colisiones, juntar pilas iguales y caducar.
    pub fn tick(&mut self, w: &impl ChunkAccess, dt: f32) {
        let dt64 = dt as f64;
        for it in &mut self.0 {
            it.age += dt;
            it.vel.y = (it.vel.y - ITEM_GRAVITY * dt64).max(-30.0);
            it.vel.x *= 0.9;
            it.vel.z *= 0.9;
            let (b, moved, ground) = move_and_collide(w, aabb(it.pos), it.vel * dt64);
            it.pos = b.feet();
            if ground || (moved.y - it.vel.y * dt64).abs() > 1e-6 {
                it.vel.y = 0.0;
            }
        }
        self.0.retain(|i| i.age < i.ttl);
        let r = tuning().merge_radius as f64;
        let mut i = 0;
        while i < self.0.len() {
            let mut j = i + 1;
            while j < self.0.len() {
                let near = self.0[i].pos.distance(self.0[j].pos) < r;
                let (a, b) = (&self.0[i], &self.0[j]);
                let fits = match (&a.stack, &b.stack) {
                    (Some(x), Some(y)) => x.stacks_with(y) && x.count + y.count <= x.max(),
                    (None, None) => true,
                    _ => false,
                };
                if near && fits {
                    let other = self.0.remove(j);
                    let keep = &mut self.0[i];
                    if let (Some(x), Some(y)) = (&mut keep.stack, &other.stack) {
                        x.count += y.count;
                    }
                    keep.xp += other.xp;
                    keep.ttl = keep.ttl.max(other.ttl);
                    keep.age = keep.age.min(other.age);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    /// Recoge lo que esté cerca de los pies; lo que no cabe se queda en el suelo.
    pub fn pickup(&mut self, feet: DVec3, inv: &mut Inventory, xp: &mut Experience) {
        let t = tuning();
        let center = feet + DVec3::new(0.0, 0.9, 0.0);
        for it in &mut self.0 {
            if it.age < t.pickup_delay
                || it.pos.distance(center).min(it.pos.distance(feet)) > t.pickup_radius as f64
            {
                continue;
            }
            if it.xp > 0.0 {
                xp.add(it.xp);
                it.xp = 0.0;
            }
            if let Some(s) = it.stack.take() {
                it.stack = inv.add(s);
            }
        }
        self.0.retain(|i| i.stack.is_some() || i.xp > 0.0);
    }

    /// Tira un objeto desde los ojos del jugador hacia donde mira.
    pub fn throw(&mut self, eye: DVec3, dir: DVec3, stack: ItemStack) {
        self.0.push(GroundItem {
            pos: eye - DVec3::new(0.0, 0.3, 0.0),
            vel: dir * 6.0 + DVec3::new(0.0, 2.0, 0.0),
            stack: Some(stack),
            xp: 0.0,
            age: 0.0,
            ttl: tuning().ground_item_ttl,
        });
    }
}

/// Suelta pilas alrededor de un punto (romper bloques, cerrar pantallas, morir).
pub fn scatter(g: &mut GroundItems, at: DVec3, stacks: Vec<ItemStack>, ttl: f32) {
    for (i, s) in stacks.into_iter().enumerate() {
        let a = i as f64 * 2.39996;
        g.0.push(GroundItem {
            pos: at,
            vel: DVec3::new(a.cos() * 1.5, 3.0, a.sin() * 1.5),
            stack: Some(s),
            xp: 0.0,
            age: 0.0,
            ttl,
        });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::inventory::{Inventory, ItemStack};
    use crate::xp::Experience;
    use std::collections::HashMap;
    use survivalforest_core::block::STONE;
    use survivalforest_core::item::item_by_key;
    use survivalforest_core::{BlockPos, ChunkPos};
    use survivalforest_world::ChunkColumn;

    /// Suelo de piedra en y=60 (bloque superior en 61) en 3×3 chunks.
    pub(crate) fn flat() -> HashMap<ChunkPos, ChunkColumn> {
        let mut m = HashMap::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                let mut c = ChunkColumn::new(ChunkPos::new(cx, cz));
                for x in 0..16 {
                    for z in 0..16 {
                        c.set(BlockPos::new(cx * 16 + x, 60, cz * 16 + z).local(), STONE);
                    }
                }
                m.insert(c.pos, c);
            }
        }
        m
    }

    fn st(k: &str, n: u16) -> ItemStack {
        ItemStack::new(item_by_key(k).unwrap(), n)
    }

    #[test]
    fn item_falls_and_rests_on_ground() {
        let w = flat();
        let mut g = GroundItems::default();
        g.spawn(DVec3::new(2.5, 66.0, 2.5), st("coal", 1), 300.0);
        for _ in 0..60 {
            g.tick(&w, 0.05);
        }
        let y = g.0[0].pos.y;
        assert!((y - 61.0).abs() < 0.01, "{y}");
    }

    #[test]
    fn pickup_within_radius_only() {
        let mut g = GroundItems::default();
        g.spawn(DVec3::new(0.0, 61.0, 1.0), st("coal", 2), 300.0);
        g.spawn(DVec3::new(0.0, 61.0, 4.0), st("stick", 1), 300.0);
        for it in &mut g.0 {
            it.age = 1.0;
        }
        let (mut inv, mut xp) = (Inventory::default(), Experience::default());
        g.pickup(DVec3::new(0.0, 61.0, 0.0), &mut inv, &mut xp);
        assert_eq!(inv.slots.count(item_by_key("coal").unwrap()), 2);
        assert_eq!(g.0.len(), 1);
    }

    #[test]
    fn freshly_dropped_waits_before_pickup() {
        let mut g = GroundItems::default();
        g.spawn(DVec3::new(0.0, 61.0, 0.5), st("coal", 1), 300.0);
        let (mut inv, mut xp) = (Inventory::default(), Experience::default());
        g.pickup(DVec3::new(0.0, 61.0, 0.0), &mut inv, &mut xp);
        assert_eq!(g.0.len(), 1);
    }

    #[test]
    fn pickup_full_inventory_leaves_rest() {
        let mut inv = Inventory::default();
        for i in 0..36 {
            inv.slots.0[i] = Some(st("stick", 64));
        }
        inv.slots.0[0] = Some(st("coal", 60));
        let mut g = GroundItems::default();
        g.spawn(DVec3::new(0.0, 61.0, 0.5), st("coal", 10), 300.0);
        g.0[0].age = 1.0;
        g.pickup(
            DVec3::new(0.0, 61.0, 0.0),
            &mut inv,
            &mut Experience::default(),
        );
        assert_eq!(inv.slots.count(item_by_key("coal").unwrap()), 64);
        assert_eq!(g.0[0].stack.as_ref().unwrap().count, 6);
    }

    #[test]
    fn same_items_merge() {
        let w = flat();
        let mut g = GroundItems::default();
        g.spawn(DVec3::new(3.5, 61.0, 3.5), st("coal", 3), 300.0);
        g.spawn(DVec3::new(3.7, 61.0, 3.6), st("coal", 4), 300.0);
        g.spawn(DVec3::new(3.6, 61.0, 3.5), st("stick", 1), 300.0);
        g.tick(&w, 0.05);
        assert_eq!(g.0.len(), 2);
        assert_eq!(
            g.0.iter()
                .filter_map(|i| i.stack.as_ref())
                .map(|s| s.count)
                .sum::<u16>(),
            8
        );
    }

    #[test]
    fn items_expire_after_ttl() {
        let w = flat();
        let mut g = GroundItems::default();
        g.spawn(DVec3::new(0.5, 61.0, 0.5), st("coal", 1), 300.0);
        g.spawn(DVec3::new(9.5, 61.0, 9.5), st("stick", 1), 600.0);
        for _ in 0..301 {
            g.tick(&w, 1.0);
        }
        assert_eq!(g.0.len(), 1);
        for _ in 0..300 {
            g.tick(&w, 1.0);
        }
        assert!(g.0.is_empty());
    }

    #[test]
    fn orb_gives_xp() {
        let mut g = GroundItems::default();
        g.spawn_orb(DVec3::new(0.0, 61.0, 0.5), 9.0, 600.0);
        g.0[0].age = 1.0;
        let mut xp = Experience::default();
        g.pickup(
            DVec3::new(0.0, 61.0, 0.0),
            &mut Inventory::default(),
            &mut xp,
        );
        assert_eq!(xp.total, 9.0);
        assert!(g.0.is_empty());
    }
}
