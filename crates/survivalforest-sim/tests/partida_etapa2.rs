//! Partida de prueba de la etapa 2 sin ventana: talar, mesa y pico, picar, fundir, comer,
//! caer, morir y recuperar las cosas.

use glam::DVec3;
use survivalforest_core::GameMode::Survival;
use survivalforest_core::block::{IRON_ORE, LOG, STONE};
use survivalforest_core::item::item_by_key;
use survivalforest_sim::crafting::craft;
use survivalforest_sim::death::die;
use survivalforest_sim::drops::GroundItems;
use survivalforest_sim::furnace::Furnace;
use survivalforest_sim::inventory::{Inventory, ItemStack};
use survivalforest_sim::mining::{break_time, drops_for, wear_tool};
use survivalforest_sim::vitals::Vitals;
use survivalforest_sim::xp::Experience;

fn id(k: &str) -> survivalforest_core::item::ItemId {
    item_by_key(k).unwrap()
}

fn grid(w: usize, cells: &[(usize, &str, u16)]) -> Vec<Option<ItemStack>> {
    let mut g = vec![None; w * w];
    for (i, k, n) in cells {
        g[*i] = Some(ItemStack::new(id(k), *n));
    }
    g
}

#[test]
fn partida_completa() {
    let mut inv = Inventory::default();
    let mut xp = Experience::default();
    let mut v = Vitals::default();
    // Talar a mano: 4 troncos.
    for s in 0..4 {
        assert!(break_time(LOG, None, Survival).unwrap() < 5.0);
        for d in drops_for(LOG, None, Survival, s).0 {
            inv.add(d);
        }
    }
    assert_eq!(inv.slots.count(id("log")), 4);
    // Tablones, varas, mesa y pico de madera.
    let mut g = grid(2, &[(0, "log", 3)]);
    for _ in 0..3 {
        inv.add(craft(&mut g, 2).unwrap());
    }
    assert_eq!(inv.slots.count(id("planks")), 9);
    let table = craft(
        &mut grid(
            2,
            &[
                (0, "planks", 1),
                (1, "planks", 1),
                (2, "log", 1),
                (3, "log", 1),
            ],
        ),
        2,
    )
    .unwrap();
    assert_eq!(table.item, id("crafting_table"));
    let sticks = craft(&mut grid(2, &[(0, "planks", 1)]), 2).unwrap();
    assert_eq!(sticks.count, 3);
    let pick = craft(
        &mut grid(
            3,
            &[
                (0, "planks", 1),
                (2, "planks", 1),
                (3, "planks", 1),
                (4, "stick", 1),
                (5, "planks", 1),
                (7, "stick", 1),
            ],
        ),
        3,
    )
    .unwrap();
    assert_eq!(pick.item, id("pick_wood"));
    inv.slots.0[0] = Some(pick);
    // Picar piedra con el pico (se gasta) y hacer un pico de piedra.
    let mut cobble = 0;
    for s in 0..3 {
        let held = inv.held().cloned();
        let (d, _) = drops_for(STONE, held.as_ref(), Survival, s);
        cobble += d.iter().map(|x| x.count).sum::<u16>();
        wear_tool(inv.held_mut(), STONE);
    }
    assert_eq!(cobble, 3);
    assert_eq!(inv.held().unwrap().wear, 3);
    let stone_pick = craft(
        &mut grid(
            3,
            &[
                (0, "cobble", 1),
                (2, "cobble", 1),
                (3, "cobble", 1),
                (4, "stick", 1),
                (5, "cobble", 1),
                (7, "stick", 1),
            ],
        ),
        3,
    )
    .unwrap();
    inv.slots.0[1] = Some(stone_pick);
    inv.selected = 1;
    // Mena de hierro con pico de piedra: experiencia y mena.
    let (d, ore_xp) = drops_for(IRON_ORE, inv.held(), Survival, 7);
    xp.add(ore_xp as f32);
    assert_eq!(d[0].item, id("iron_ore_item"));
    // Fundir en el horno con carbón.
    let mut f = Furnace {
        input: Some(ItemStack::new(id("iron_ore_item"), 2)),
        fuel: Some(ItemStack::new(id("coal"), 1)),
        ..Default::default()
    };
    for _ in 0..16 {
        f.tick(1.0);
    }
    let (out, fx) = f.take_output();
    xp.add(fx as f32);
    assert_eq!(out.unwrap().count, 2);
    assert!(xp.total >= 5.0);
    // Comer un plátano con hambre.
    v.hunger = 10.0;
    v.eat(
        survivalforest_core::item::item(id("platano"))
            .food
            .as_ref()
            .unwrap(),
    );
    assert_eq!(v.hunger, 16.0);
    // Caer 10 bloques quita 7 puntos.
    v.airborne(80.0);
    assert_eq!(v.on_ground(70.0, false, Survival), 7.0);
    // Morir: todo cae y se recupera al volver.
    let total_before: u32 = inv.slots.0.iter().flatten().map(|s| s.count as u32).sum();
    let mut ground = GroundItems::default();
    die(&mut inv, &mut xp, &mut ground, DVec3::new(0.0, 70.0, 0.0));
    assert!(inv.slots.is_empty());
    for it in &mut ground.0 {
        it.age = 1.0;
        it.pos = DVec3::new(0.0, 70.0, 0.0);
    }
    ground.pickup(DVec3::new(0.0, 70.0, 0.0), &mut inv, &mut xp);
    let total_after: u32 = inv.slots.0.iter().flatten().map(|s| s.count as u32).sum();
    assert_eq!(total_after, total_before);
    assert!(xp.total > 0.0);
}
