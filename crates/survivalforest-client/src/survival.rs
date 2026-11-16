//! Supervivencia en el cliente: estado del jugador (inventario, vida, experiencia), objetos
//! y bloques con contenido del mundo, y el paso de simulación de cada tick.

use bevy::math::DVec3;
use bevy::prelude::*;
use std::collections::HashSet;
use survivalforest_core::GameMode;
use survivalforest_core::tuning::tuning;
use survivalforest_sim::containers::BlockEntities;
use survivalforest_sim::death::die;
use survivalforest_sim::drops::GroundItems;
use survivalforest_sim::inventory::{Inventory, ItemStack};
use survivalforest_sim::persist::SurvivalSave;
use survivalforest_sim::vitals::Vitals;
pub use survivalforest_sim::vitals::VitalsCtx;
use survivalforest_sim::xp::Experience;
use survivalforest_world::ChunkAccess;

/// Estado de supervivencia del jugador mientras juega.
#[derive(Resource, Clone, Debug, Default)]
pub struct PlayerState {
    pub inv: Inventory,
    pub vitals: Vitals,
    pub xp: Experience,
    pub crafted: HashSet<String>,
    /// Lo que lleva el cursor en las pantallas.
    pub cursor: Option<ItemStack>,
    /// Bloque que se está rompiendo y segundos acumulados.
    pub breaking: Option<(survivalforest_core::BlockPos, f32)>,
    /// Segundos comiendo (con el clic derecho mantenido).
    pub eating: Option<f32>,
    pub dead: bool,
    pub death_pos: Option<DVec3>,
    /// Segundos que quedan del destello rojo de daño.
    pub hurt_flash: f32,
    pub spawn: Option<[i32; 3]>,
}

impl PlayerState {
    pub fn from_save(s: SurvivalSave) -> Self {
        Self {
            inv: s.inventory,
            vitals: s.vitals,
            xp: s.xp,
            crafted: s.crafted.into_iter().collect(),
            spawn: s.spawn,
            ..default()
        }
    }

    pub fn to_save(&self) -> SurvivalSave {
        let mut crafted: Vec<String> = self.crafted.iter().cloned().collect();
        crafted.sort();
        SurvivalSave {
            inventory: self.inv.clone(),
            vitals: self.vitals.clone(),
            xp: self.xp.clone(),
            crafted,
            spawn: self.spawn,
            ..default()
        }
    }
}

/// Baúles, hornos y objetos en el suelo del mundo abierto, con las regiones ya leídas.
#[derive(Resource, Clone, Debug, Default)]
pub struct WorldExtras {
    pub be: BlockEntities,
    pub items: GroundItems,
    pub loaded: HashSet<(i32, i32)>,
}

fn hurt(st: &mut PlayerState, dmg: f32) {
    if dmg > 0.0 {
        st.hurt_flash = 0.3;
    }
}

/// El jugador toca el suelo (o el agua): daño por caída.
pub fn land(st: &mut PlayerState, y: f64, in_water: bool, mode: GameMode) -> f32 {
    let d = st.vitals.on_ground(y, in_water, mode);
    hurt(st, d);
    d
}

/// Un tick de supervivencia: vida, hambre y aire; objetos del suelo; recoger; morir.
pub fn survival_tick(
    st: &mut PlayerState,
    ex: &mut WorldExtras,
    w: &impl ChunkAccess,
    feet: DVec3,
    ctx: VitalsCtx,
    dt: f32,
) {
    st.hurt_flash = (st.hurt_flash - dt).max(0.0);
    ex.items.tick(w, dt);
    if st.dead {
        return;
    }
    let d = st.vitals.tick(dt, ctx);
    hurt(st, d);
    if st.vitals.is_dead() && ctx.mode == GameMode::Survival {
        st.dead = true;
        st.death_pos = Some(feet);
        st.breaking = None;
        st.eating = None;
        if let Some(c) = st.cursor.take()
            && let Some(rest) = st.inv.add(c)
        {
            ex.items.spawn(feet, rest, tuning().death_item_ttl);
        }
        die(
            &mut st.inv,
            &mut st.xp,
            &mut ex.items,
            feet + DVec3::new(0.0, 0.5, 0.0),
        );
        return;
    }
    ex.items.pickup(feet, &mut st.inv, &mut st.xp);
}

/// Volver a la vida con todo lleno (la posición la pone quien llama).
pub fn respawn(st: &mut PlayerState) {
    st.dead = false;
    st.vitals = Vitals::default();
    st.hurt_flash = 0.0;
}

// ---------- Sistemas de Bevy ----------

use crate::player::{MoveInput, Player, WorldPos};
use crate::streaming::{RemeshRequest, World, set_block};
use survivalforest_core::BlockPos;
use survivalforest_core::block::{FURNACE, FURNACE_LIT};
use survivalforest_sim::persist::{load_extras, region_of};

/// Lee del disco los baúles, hornos y objetos de las regiones cercanas que falten.
pub fn load_nearby_regions(dir: &std::path::Path, ex: &mut WorldExtras, feet: DVec3) {
    let r = region_of(BlockPos::new(
        feet.x.floor() as i32,
        0,
        feet.z.floor() as i32,
    ));
    for dz in -1..=1 {
        for dx in -1..=1 {
            let k = (r.0 + dx, r.1 + dz);
            if ex.loaded.insert(k) {
                let (be, items) = load_extras(dir, k);
                ex.be.0.extend(be.0);
                ex.items.0.extend(items.0);
            }
        }
    }
}

/// Tick de 20 Hz: caídas, vida, hambre, aire, objetos, hornos y regiones.
#[allow(clippy::too_many_arguments)]
pub fn survival_step(
    time: Res<Time>,
    mut world: ResMut<World>,
    mode: Res<crate::player::Mode>,
    input: Res<MoveInput>,
    mut st: ResMut<PlayerState>,
    mut ex: ResMut<WorldExtras>,
    mut remesh: MessageWriter<RemeshRequest>,
    q: Single<(&Player, &WorldPos)>,
) {
    let dt = time.delta_secs();
    let (p, wp) = *q;
    let mode = mode.0;
    if !p.flying && !st.dead {
        if p.on_ground || p.in_water.0 {
            land(&mut st, wp.pos.y, p.in_water.0, mode);
        } else {
            st.vitals.airborne(wp.pos.y);
        }
    }
    let ctx = VitalsCtx {
        mode,
        moving: input.dir != Vec2::ZERO,
        sprinting: input.sprint,
        jumped: p.jumped,
        head_in_water: p.in_water.1,
    };
    let dir = world.dir.clone();
    load_nearby_regions(&dir, &mut ex, wp.pos);
    survival_tick(&mut st, &mut ex, &*world, wp.pos, ctx, dt);
    for (pos, lit) in ex.be.tick_furnaces(dt) {
        set_block(
            &mut world,
            pos,
            if lit { FURNACE_LIT } else { FURNACE },
            &mut remesh,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use survivalforest_core::block::STONE;
    use survivalforest_core::item::item_by_key;
    use survivalforest_core::{BlockPos, ChunkPos, GameMode};
    use survivalforest_sim::inventory::ItemStack;
    use survivalforest_world::ChunkColumn;

    fn flat() -> HashMap<ChunkPos, ChunkColumn> {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        for x in 0..16 {
            for z in 0..16 {
                c.set(BlockPos::new(x, 60, z).local(), STONE);
            }
        }
        HashMap::from([(c.pos, c)])
    }

    fn ctx(mode: GameMode) -> VitalsCtx {
        VitalsCtx {
            mode,
            moving: true,
            sprinting: true,
            jumped: false,
            head_in_water: false,
        }
    }

    #[test]
    fn survival_tick_drains_hunger_and_picks_up() {
        let w = flat();
        let mut st = PlayerState::default();
        st.vitals.saturation = 0.0;
        let mut ex = WorldExtras::default();
        ex.items.spawn(
            DVec3::new(5.5, 61.0, 6.0),
            ItemStack::new(item_by_key("coal").unwrap(), 2),
            300.0,
        );
        let feet = DVec3::new(5.5, 61.0, 5.5);
        for _ in 0..(20 * 200) {
            survival_tick(&mut st, &mut ex, &w, feet, ctx(GameMode::Survival), 0.05);
        }
        assert!(st.vitals.hunger < 20.0);
        assert_eq!(st.inv.slots.count(item_by_key("coal").unwrap()), 2);
        assert!(ex.items.0.is_empty());
    }

    #[test]
    fn death_drops_inventory_once() {
        let w = flat();
        let mut st = PlayerState::default();
        st.inv.slots.0[0] = Some(ItemStack::new(item_by_key("coal").unwrap(), 5));
        st.vitals.health = 0.5;
        st.vitals.hunger = 0.0;
        st.vitals.airborne(80.0);
        let mut ex = WorldExtras::default();
        let feet = DVec3::new(5.5, 61.0, 5.5);
        let dmg = land(&mut st, feet.y, false, GameMode::Survival);
        assert!(dmg > 0.0);
        survival_tick(&mut st, &mut ex, &w, feet, ctx(GameMode::Survival), 0.05);
        assert!(st.dead);
        assert_eq!(st.death_pos, Some(feet));
        assert!(st.inv.slots.is_empty());
        let n = ex.items.0.len();
        survival_tick(&mut st, &mut ex, &w, feet, ctx(GameMode::Survival), 0.05);
        assert_eq!(
            ex.items.0.len(),
            n,
            "no recoge ni vuelve a morir estando muerto"
        );
        respawn(&mut st);
        assert!(!st.dead && st.vitals.health == 20.0 && st.vitals.hunger == 20.0);
    }

    #[test]
    fn creative_ignores_damage() {
        let w = flat();
        let mut st = PlayerState::default();
        st.vitals.airborne(200.0);
        assert_eq!(land(&mut st, 61.0, false, GameMode::Creative), 0.0);
        let mut ex = WorldExtras::default();
        for _ in 0..100 {
            survival_tick(
                &mut st,
                &mut ex,
                &w,
                DVec3::new(1.0, 61.0, 1.0),
                ctx(GameMode::Creative),
                0.05,
            );
        }
        assert_eq!(st.vitals.health, 20.0);
    }

    #[test]
    fn save_and_restore_state() {
        let mut st = PlayerState::default();
        st.inv.slots.0[2] = Some(ItemStack::new(item_by_key("stick").unwrap(), 9));
        st.crafted.insert("planks".into());
        st.spawn = Some([1, 2, 3]);
        let back = PlayerState::from_save(st.to_save());
        assert_eq!(back.inv, st.inv);
        assert_eq!(back.crafted, st.crafted);
        assert_eq!(back.spawn, st.spawn);
    }
}
