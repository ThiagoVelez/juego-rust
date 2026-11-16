//! Inventario y casillas (mochila, baúl, rejillas de crafteo).

use serde::{Deserialize, Serialize};
use survivalforest_core::item::{ItemId, item};

/// Una pila de objetos. `wear` es el desgaste de una herramienta (0 = nueva).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u16,
    #[serde(default)]
    pub wear: u16,
}

impl ItemStack {
    pub fn new(item: ItemId, count: u16) -> Self {
        Self {
            item,
            count,
            wear: 0,
        }
    }
    pub fn max(&self) -> u16 {
        item(self.item).stack
    }
    /// ¿Se pueden juntar en una pila?
    pub fn stacks_with(&self, o: &ItemStack) -> bool {
        self.item == o.item && self.wear == o.wear && self.max() > 1
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slots(pub Vec<Option<ItemStack>>);

impl Slots {
    pub fn new(n: usize) -> Self {
        Self(vec![None; n])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|s| s.is_none())
    }

    /// Mete la pila: primero en las pilas iguales, luego en los huecos. Devuelve lo que no cabe.
    pub fn add(&mut self, mut s: ItemStack) -> Option<ItemStack> {
        self.add_in(0..self.0.len(), &mut s);
        (s.count > 0).then_some(s)
    }

    fn add_in(&mut self, range: std::ops::Range<usize>, s: &mut ItemStack) {
        for i in range.clone() {
            if s.count == 0 {
                return;
            }
            if let Some(t) = &mut self.0[i]
                && t.stacks_with(s)
            {
                let n = (t.max() - t.count).min(s.count);
                t.count += n;
                s.count -= n;
            }
        }
        for i in range {
            if s.count == 0 {
                return;
            }
            if self.0[i].is_none() {
                let n = s.max().min(s.count);
                self.0[i] = Some(ItemStack {
                    count: n,
                    ..s.clone()
                });
                s.count -= n;
            }
        }
    }

    /// Clic en una casilla con lo que lleva el cursor.
    /// Izquierdo: toma, deja, junta o intercambia. Derecho: toma la mitad o deja una.
    pub fn click(&mut self, i: usize, cursor: &mut Option<ItemStack>, right: bool) {
        let slot = &mut self.0[i];
        match (slot.as_mut(), cursor.as_mut()) {
            (None, None) => {}
            (Some(s), None) => {
                if right {
                    let take = s.count.div_ceil(2);
                    *cursor = Some(ItemStack {
                        count: take,
                        ..s.clone()
                    });
                    s.count -= take;
                    if s.count == 0 {
                        *slot = None;
                    }
                } else {
                    *cursor = slot.take();
                }
            }
            (None, Some(c)) => {
                if right {
                    *slot = Some(ItemStack {
                        count: 1,
                        ..c.clone()
                    });
                    c.count -= 1;
                    if c.count == 0 {
                        *cursor = None;
                    }
                } else {
                    *slot = cursor.take();
                }
            }
            (Some(s), Some(c)) => {
                if s.stacks_with(c) {
                    let n = if right { 1 } else { c.count }.min(s.max() - s.count);
                    s.count += n;
                    c.count -= n;
                    if c.count == 0 {
                        *cursor = None;
                    }
                } else if !right {
                    std::mem::swap(slot, cursor);
                }
            }
        }
    }

    /// Saca hasta `n` de la casilla `i`.
    pub fn take(&mut self, i: usize, n: u16) -> Option<ItemStack> {
        let s = self.0[i].as_mut()?;
        let k = n.min(s.count);
        let out = ItemStack {
            count: k,
            ..s.clone()
        };
        s.count -= k;
        if s.count == 0 {
            self.0[i] = None;
        }
        (k > 0).then_some(out)
    }

    pub fn count(&self, it: ItemId) -> u32 {
        self.0
            .iter()
            .flatten()
            .filter(|s| s.item == it)
            .map(|s| s.count as u32)
            .sum()
    }

    pub fn drain_all(&mut self) -> Vec<ItemStack> {
        self.0.iter_mut().filter_map(|s| s.take()).collect()
    }
}

/// Mayús+clic: mueve la pila de `from[i]` a `to`. Devuelve si movió algo.
pub fn quick_move(from: &mut Slots, i: usize, to: &mut Slots) -> bool {
    let Some(s) = from.0[i].take() else {
        return false;
    };
    let before = s.count;
    let rest = to.add(s);
    let moved = rest.as_ref().map(|r| r.count) != Some(before);
    from.0[i] = rest;
    moved
}

/// Mochila del jugador: casillas 0–8 son la barra.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Slots,
    pub selected: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: Slots::new(INVENTORY_SLOTS),
            selected: 0,
        }
    }
}

pub const INVENTORY_SLOTS: usize = 36;
pub const HOTBAR_SLOTS: usize = 9;

impl Inventory {
    pub fn held(&self) -> Option<&ItemStack> {
        self.slots.0[self.selected as usize].as_ref()
    }
    pub fn held_mut(&mut self) -> &mut Option<ItemStack> {
        &mut self.slots.0[self.selected as usize]
    }
    /// Recoger: primero la barra, luego la mochila.
    pub fn add(&mut self, s: ItemStack) -> Option<ItemStack> {
        self.slots.add(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use survivalforest_core::item::item_by_key;

    fn st(k: &str, n: u16) -> ItemStack {
        ItemStack::new(item_by_key(k).unwrap(), n)
    }

    #[test]
    fn add_fills_existing_stacks_first() {
        let mut s = Slots::new(4);
        s.0[2] = Some(st("stick", 60));
        assert_eq!(s.add(st("stick", 10)), None);
        assert_eq!(s.0[2].as_ref().unwrap().count, 64);
        assert_eq!(s.0[0].as_ref().unwrap().count, 6);
    }

    #[test]
    fn add_returns_overflow_when_full() {
        let mut s = Slots::new(2);
        s.0[0] = Some(st("stick", 64));
        s.0[1] = Some(st("coal", 60));
        let rest = s.add(st("coal", 14)).unwrap();
        assert_eq!(rest.count, 10);
        assert_eq!(s.count(item_by_key("coal").unwrap()), 64);
    }

    #[test]
    fn tools_do_not_stack() {
        let mut s = Slots::new(3);
        assert_eq!(s.add(st("pick_wood", 1)), None);
        assert_eq!(s.add(st("pick_wood", 1)), None);
        assert!(s.0[0].is_some() && s.0[1].is_some());
        assert_eq!(s.0[0].as_ref().unwrap().count, 1);
    }

    #[test]
    fn right_click_splits_half_and_places_one() {
        let mut s = Slots::new(2);
        s.0[0] = Some(st("coal", 7));
        let mut cur = None;
        s.click(0, &mut cur, true);
        assert_eq!(cur.as_ref().unwrap().count, 4);
        assert_eq!(s.0[0].as_ref().unwrap().count, 3);
        s.click(1, &mut cur, true);
        assert_eq!(s.0[1].as_ref().unwrap().count, 1);
        assert_eq!(cur.as_ref().unwrap().count, 3);
    }

    #[test]
    fn click_swaps_different_items() {
        let mut s = Slots::new(1);
        s.0[0] = Some(st("coal", 2));
        let mut cur = Some(st("stick", 5));
        s.click(0, &mut cur, false);
        assert_eq!(s.0[0].as_ref().unwrap().count, 5);
        assert_eq!(cur.unwrap().count, 2);
    }

    #[test]
    fn quick_move_moves_between_containers() {
        let mut a = Slots::new(2);
        let mut b = Slots::new(2);
        a.0[1] = Some(st("coal", 20));
        assert!(quick_move(&mut a, 1, &mut b));
        assert!(a.0[1].is_none());
        assert_eq!(b.count(item_by_key("coal").unwrap()), 20);
        assert!(!quick_move(&mut a, 0, &mut b));
    }
}
