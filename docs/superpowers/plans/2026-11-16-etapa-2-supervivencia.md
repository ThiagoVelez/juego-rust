# SurvivalForest · Etapa 2 (Supervivencia): plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Versión jugable 0.2.0 con vida, hambre, aire, nado, objetos, inventario, crafteo (2×2 y 3×3), horno, baúl, herramientas con desgaste, experiencia, piedra de amolar, muerte con objetos caídos y plantas de comida.

**Architecture:**
- Los datos (objetos, reglas de bloques, recetas y valores) van en RON dentro de `survivalforest-core`, con `include_str!`.
- Las reglas son código puro con pruebas en `survivalforest-sim` y no dependen de Bevy. Las acciones del jugador entran por un `Action` que en la etapa 4 viajará por la red.
- `survivalforest-world` añade las plantas y el guardado de los datos nuevos.
- El cliente (Bevy) lee las teclas y el ratón, envía acciones y dibuja el HUD y las pantallas.

**Tech Stack:** Rust 2024, Bevy 0.20, serde y ron 0.12, `cargo test`, CI con fmt, clippy `-D warnings` y prueba de humo con xvfb.

**Spec:** `docs/superpowers/specs/2026-11-16-etapa-2-supervivencia-design.md` (y el diseño general `docs/superpowers/specs/2026-11-12-juego-de-bloques-design.md`).

## Global Constraints

- **Puntos:**
  - Vida máxima 20 puntos (10 corazones) y hambre máxima 20 puntos (10 jamones); 1 jamón son 2 puntos.
  - Aire: 10 burbujas, que equivalen a 15 s.
- **Vida y daño:**
  - Con hambre ≥ 18, se recupera 1 punto cada 4 s.
  - Con hambre 0, se pierde 1 punto cada 4 s, sin bajar de 1 punto.
  - Caída: daño = bloques caídos − 3 (en puntos). Caer en agua no hace daño.
  - Sin aire: 1 punto por segundo.
- **Herramientas:**
  - Niveles: madera 1 (durabilidad 60, velocidad ×2), piedra 2 (130, ×4), cobre 3 (190, ×5), hierro 4 (260, ×6).
  - Mena → pico mínimo: carbón y cobre, madera; hierro y cuarzo, piedra; oro, cobre.
  - Piedra y piedra partida necesitan un pico de madera o mejor para soltar algo; el resto de bloques sale a mano.
  - Desgaste por bloque roto: 1 con la herramienta adecuada, 2 con otra.
- **Experiencia:**
  - Menas: carbón 1, cobre 2, hierro y cuarzo 3, oro 5.
  - Primera vez que se craftea cada receta: 2.
  - Piedra de amolar: niveles = ⌈desgaste/durabilidad × base⌉, con base madera 1, piedra 2, cobre 3 y hierro 4. Repara por completo.
- **Objetos y muerte:**
  - Los objetos en el suelo se recogen solos a 1,5 bloques y duran 300 s; los de la muerte duran 600 s.
  - Al morir cae todo el inventario. La mitad de los puntos de experiencia queda en un orbe y la otra mitad se pierde.
  - Se reaparece en el punto de aparición con la vida y el hambre al máximo.
- **Horno:** cada fundición tarda 8 s. Combustible, en fundiciones: carbón y carbón vegetal 8; tronco y tablones 1,5; vara y guadua 0,5.
- **Comida, en puntos de hambre:** plátano 6, plátano asado 12, guayaba 4, mora 2. Comer tarda 1,6 s (la mora, 0,8 s).
- **Inventario:**
  - Mochila de 36 casillas: 0–8 son la barra y 9–35 la mochila.
  - Baúl de 27 casillas.
  - Pilas de 64; herramientas de 1.
- **Recetas:** **formas propias**; ninguna puede coincidir con la forma de la receta de Minecraft del objeto equivalente (lo comprueba una prueba).
- **Ids:** los ids de bloque actuales (0–26) no cambian; los nuevos van al final.
- **Mundos de la etapa 1:**
  - abren sin errores, con el inventario vacío y la vida llena (`#[serde(default)]`);
  - `META_VERSION = 2`.
- **Nombres y arte:** en español, propios (anti-objetivo: nada que parezca Minecraft). Texturas e iconos pintados por código en `textures.rs`, con la paleta del juego.
- **CI:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo test --workspace` en verde.

## Review Focus

1. Abrir un mundo de la etapa 1 (un `player.ron` y un `meta.ron` de entonces, sin `survival.ron`): carga, con el inventario vacío y la vida y el hambre llenas. Lo prueba la Task 9.
2. Inventario lleno al romper un bloque o al sacar algo del horno: lo que no cabe queda en el suelo (o en la casilla del horno); nunca desaparece. Lo prueban las Tasks 3 y 7.
3. Morir con el inventario lleno, salir del juego y volver: los objetos y el orbe siguen en el suelo, en el sitio de la muerte. Lo prueba la Task 9.
4. Romper un baúl o un horno con cosas dentro: todo cae al suelo. Lo prueba la Task 7.
5. Creativo: no hay daño ni hambre ni ahogo, y romper no suelta objetos. Lo prueba la Task 6.

---

### Task 1: Objetos y reglas de bloques en datos

**Files:**
- Create:
  - `crates/survivalforest-core/src/item.rs`
  - `crates/survivalforest-core/data/items.ron`
  - `crates/survivalforest-core/data/block_rules.ron`
- Modify:
  - `crates/survivalforest-core/src/block.rs` (bloques y texturas nuevos)
  - `crates/survivalforest-core/src/lib.rs`
  - `crates/survivalforest-core/Cargo.toml` (`ron = "0.12"`)

**Interfaces — Produces:**
- **Bloques nuevos**, con texturas nuevas al final de `TILE_NAMES`:

  | Id | Bloque | Notas |
  |---|---|---|
  | 27 | `CRAFTING_TABLE` | |
  | 28 | `FURNACE` | |
  | 29 | `FURNACE_LIT` | luz 13 |
  | 30 | `CHEST` | |
  | 31 | `GRINDSTONE` | |
  | 32 | `COBBLE` («Piedra partida») | |
  | 33 | `BRICKS` | |
  | 34 | `GLASS` | no opaco, sólido |
  | 35 | `PLATANERA` | no opaco, no sólido |
  | 36 | `BLACKBERRY_BUSH` («Arbusto de mora») | no opaco, sólido |
  | 37 | `GUAVA_LEAVES` («Hojas de guayabo») | no opaco, sólido |

- **Tipos de objeto:**
  - `ItemId(pub u16)` (Serialize, Deserialize, Copy, Eq, Hash, Ord);
  - `ToolClass { Pickaxe, Axe, Shovel, Machete }`;
  - `Tool { class: ToolClass, tier: u8, durability: u16, speed: f32, repair_base: u8 }`;
  - `Food { hunger: u8, saturation: f32, eat_secs: f32 }`;
  - `ItemDef { id: ItemId, key: String, name_es: String, stack: u16, places: Option<BlockId>, tool: Option<Tool>, food: Option<Food>, fuel_smelts: Option<f32>, icon: String }`.
- **Registro de objetos:**
  - `items() -> &'static [ItemDef]`, `item(id) -> &'static ItemDef`, `item_by_key(&str) -> Option<ItemId>` y `item_for_block(BlockId) -> Option<ItemId>`;
  - todo bloque con `breakable` tiene objeto;
  - se carga una vez con `OnceLock`, y un RON roto hace `panic!` con un mensaje claro (lo detectan las pruebas).
- **Reglas de bloque:**
  - `BlockRules { hardness: f32, tool: Option<ToolClass>, min_tier: u8, needs_tool_to_drop: bool, drops: Vec<Drop> }`;
  - `Drop { item: String, min: u8, max: u8, chance: f32 }`;
  - `block_rules(BlockId) -> &'static BlockRules`.
- **Contenido mínimo:**
  - todo lo de la tabla «Contenido» del diseño;
  - las 16 herramientas, con claves `pick_wood`, `axe_stone`, `machete_iron`, etc.;
  - materiales: `stick` («Vara»), `coal`, `charcoal`, `copper_ore_item`, `iron_ore_item`, `gold_ore_item`, `copper_ingot`, `iron_ingot`, `gold_ingot`, `quartz`, `brick`, `guadua`;
  - comida: `platano`, `platano_asado`, `guayaba`, `mora`.

- [ ] **Step 1: Escribir las pruebas que fallan** en `item.rs`:
  - `items_load_and_keys_unique`: las claves son únicas, `item(item_by_key("pick_iron").unwrap()).tool.unwrap().tier == 4` y la durabilidad del pico de madera es 60.
  - `every_breakable_block_has_item_and_rules`.
  - `ore_rules_match_spec`: `min_tier` de hierro = 2 y de oro = 3.
  - `stone_needs_pick`: `needs_tool_to_drop` y `tool == Some(Pickaxe)`; la tierra roja suelta a mano.
  - `food_values`: plátano 6, plátano asado 12, guayaba 4, mora 2 y `eat_secs` de la mora 0,8.
  - `new_block_ids_appended`: `CRAFTING_TABLE.0 == 27`, `GUAVA_LEAVES.0 == 37` y `STONE.0 == 2`.
  - `transparency_rules`: amplía la actual con vidrio, platanera, arbusto de mora y hojas de guayabo.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-core`.** Debe fallar porque falta `item`.
- [ ] **Step 3: Implementar** los RON y el código.
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-core`.** Debe pasar. Después, `cargo clippy -p survivalforest-core --all-targets -- -D warnings` sin avisos.
- [ ] **Step 5: Commit** «feat(core): objetos, herramientas, comida y reglas de bloques en datos».

### Task 2: Recetas y valores

**Files:**
- Create:
  - `crates/survivalforest-core/src/recipe.rs`
  - `crates/survivalforest-core/src/tuning.rs`
  - `crates/survivalforest-core/data/recipes.ron`
  - `crates/survivalforest-core/data/tuning.ron`

**Interfaces — Produces:**
- **Recetas de crafteo:** `Recipe { id: String, kind: RecipeKind, output: (String, u16) }`, con `RecipeKind::Shaped { rows: Vec<String>, key: Vec<(char, String)> }` o `RecipeKind::Shapeless(Vec<String>)`. Se consultan con `recipes() -> &'static [Recipe]`.
- **Recetas del horno:** `SmeltRecipe { input: String, output: String, xp: f32 }` y `smelt_recipes() -> &'static [SmeltRecipe]`.
- **Valores:** `Tuning` con todos los valores numéricos de las Global Constraints, más el ritmo del hambre (agotamiento por segundo andando, corriendo y saltando) y la curva de experiencia (`xp_to_next(level) = 7 + 3·level`, decisión del plan). Se leen con `tuning() -> &'static Tuning`.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `recipes_reference_existing_items`: la salida y todos los ingredientes existen.
  - `every_tool_and_station_is_craftable`: las 16 herramientas, la mesa, el horno, el baúl, la piedra de amolar, las antorchas, el bloque de ladrillo, la teja y la pared encalada tienen receta.
  - `smelting_covers_spec`: las menas (cobre, hierro y oro), la arena y la arena roja, la arcilla, el plátano, el tronco y la piedra partida se funden.
  - `no_forbidden_shapes`: hay una lista escrita a mano en la prueba con las formas de Minecraft de pico, hacha, pala, espada (para el machete), mesa, horno, cofre (para el baúl) y antorcha. Para cada receta con forma de esos objetos, su forma normalizada (sin filas ni columnas vacías y con los ingredientes cambiados por letras genéricas por material) no coincide con la prohibida, **ni con su reflejo**.
  - `tuning_matches_spec`: los valores de las Global Constraints.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-core`.** Debe fallar.
- [ ] **Step 3: Implementar.** Las formas propias se eligen aquí.
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-core`.** Debe pasar, y clippy sin avisos.
- [ ] **Step 5: Commit** «feat(core): recetas propias, horno y valores de la supervivencia».

### Task 3: Inventario

**Files:**
- Create: `crates/survivalforest-sim/src/inventory.rs`
- Modify: `crates/survivalforest-sim/src/lib.rs`, `crates/survivalforest-sim/Cargo.toml` (`serde.workspace = true`)

**Interfaces — Produces:**
- `ItemStack { item: ItemId, count: u16, wear: u16 }` (Serialize, Deserialize, Clone, PartialEq).
- `Slots(pub Vec<Option<ItemStack>>)` (Serialize, Deserialize), con estas funciones:
  - `new(n)`;
  - `add(&mut self, s: ItemStack) -> Option<ItemStack>`: llena primero las pilas iguales y después los huecos, y devuelve lo que no cabe;
  - `click(&mut self, i, cursor: &mut Option<ItemStack>, right: bool)`: clic izquierdo toma o deja la pila, o la intercambia; clic derecho toma la mitad o deja una;
  - `take(&mut self, i, n) -> Option<ItemStack>`;
  - `count(item) -> u32`;
  - `drain_all(&mut self) -> Vec<ItemStack>`.
- `quick_move(from: &mut Slots, i, to: &mut Slots) -> bool`: Mayús+clic.
- `Inventory { slots: Slots /*36*/, selected: u8 }`, con `held(&self) -> Option<&ItemStack>`.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `add_fills_existing_stacks_first`;
  - `add_returns_overflow_when_full`, con el inventario lleno y 10 de sobra;
  - `tools_do_not_stack`;
  - `right_click_splits_half_and_places_one`;
  - `click_swaps_different_items`;
  - `quick_move_moves_between_containers`.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-sim inventory`.** Debe fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-sim`.** Debe pasar.
- [ ] **Step 5: Commit** «feat(sim): inventario y casillas».

### Task 4: Crafteo, horno, baúl y piedra de amolar

**Files:** Create `crates/survivalforest-sim/src/crafting.rs`, `crates/survivalforest-sim/src/furnace.rs`, `crates/survivalforest-sim/src/grindstone.rs`.

**Interfaces:**
- **Consume:** Task 2 (`recipes`, `smelt_recipes`, `tuning`) y Task 3 (`ItemStack`, `Slots`).
- **Produce:**
  - **Crafteo:**
    - `match_recipe(grid: &[Option<ItemStack>], w: usize) -> Option<&'static Recipe>`: rejilla de 2×2 o de 3×3 (`w` = 2 o 3); ignora el espacio vacío alrededor, prueba también el reflejo horizontal y las recetas sin forma.
    - `craft(grid: &mut [Option<ItemStack>], w) -> Option<ItemStack>`: consume 1 de cada casilla usada.
  - **Horno:**
    - `Furnace { input: Option<ItemStack>, fuel: Option<ItemStack>, output: Option<ItemStack>, burn_left: f32, burn_total: f32, progress: f32, xp_stored: f32 }` (Serialize, Deserialize);
    - `tick(&mut self, dt: f32) -> bool`: devuelve si está encendido;
    - `take_output(&mut self) -> (Option<ItemStack>, u32 /*xp*/)`;
    - si la salida está llena, no avanza.
  - **Baúl:** `Chest { slots: Slots /*27*/ }`.
  - **Piedra de amolar:**
    - `repair_cost(stack: &ItemStack) -> Option<u32>`: `None` si no es una herramienta o no tiene desgaste;
    - `repair(stack: &mut ItemStack, xp: &mut Experience) -> Result<u32, RepairError>`, con `RepairError::NotEnoughLevels { need: u32 }`;
    - `Experience` viene de la Task 5; esta función se escribe en la Task 5 si conviene, pero se prueba aquí.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `planks_from_log_anywhere_in_2x2`;
  - `pickaxe_needs_3x3_and_shifted_grid_matches`;
  - `mirrored_shape_matches`;
  - `shapeless_matches_any_order`;
  - `craft_consumes_one_each`;
  - `furnace_smelts_in_8s_with_coal_for_8_items`: 8 menas de hierro y 1 carbón dan 8 lingotes en 64 s y queda `burn_left == 0`;
  - `furnace_stops_when_output_full`;
  - `furnace_xp_on_take`;
  - `repair_cost_iron_half_worn_is_2`: ⌈0,5 × 4⌉ = 2;
  - `repair_fails_without_levels`.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-sim`.** Debe fallar.
- [ ] **Step 3: Implementar.** Para que esta tarea compile antes, `Experience` va en `xp.rs` con `level: u32`, `points: f32`, `add(points)` y `spend_levels(n) -> bool`; sus pruebas propias están en la Task 5.
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-sim`.** Debe pasar.
- [ ] **Step 5: Commit** «feat(sim): crafteo, horno, baúl y piedra de amolar».

### Task 5: Vida, hambre, aire, caída y experiencia

**Files:** Create `crates/survivalforest-sim/src/vitals.rs`; Modify `crates/survivalforest-sim/src/xp.rs`.

**Interfaces — Produces:**
- **`Vitals`:** `{ health: f32, hunger: f32, saturation: f32, air: f32, regen_timer: f32, starve_timer: f32, fall_start_y: Option<f64> }` (Serialize, Deserialize, Default con todo lleno), con estas funciones:
  - `tick(&mut self, dt: f32, ctx: VitalsCtx) -> f32`: devuelve el daño de este tick;
  - `VitalsCtx { mode: GameMode, sprinting: bool, jumped: bool, head_in_water: bool }`;
  - `on_ground(&mut self, y: f64, in_water: bool) -> f32`: daño por caída;
  - `eat(&mut self, food: &Food)`;
  - `is_dead()`.
- **`Experience`:** `{ level: u32, points: f32, total: f32 }`, con estas funciones:
  - `add(&mut self, p: f32)`: sube de nivel con `xp_to_next`;
  - `progress() -> f32` (0..1);
  - `spend_levels`;
  - `drop_on_death(&mut self) -> f32`: devuelve la mitad del total para el orbe y deja la experiencia a 0.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `regenerates_when_fed`: hambre 20 y vida 10; tras 4 s, la vida es 11.
  - `starving_stops_at_one_point`: hambre 0 y vida 3; tras 60 s, la vida es 1.
  - `fall_3_blocks_no_damage_10_blocks_7`.
  - `fall_into_water_no_damage`.
  - `drowning_after_15s_one_per_second`: con la cabeza bajo el agua, en el segundo 15 queda el aire a 0, y del segundo 16 al 18 se pierden 3 puntos.
  - `air_refills_out_of_water`.
  - `sprinting_drains_hunger_faster`.
  - `eat_platano_adds_6`, con el límite de 20.
  - `creative_never_damaged`: con `mode = Creative`, `tick` y `on_ground` devuelven 0 siempre.
  - `xp_levels_curve` y `xp_death_keeps_half_in_orb`.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-sim vitals`.** Debe fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-sim`.** Debe pasar.
- [ ] **Step 5: Commit** «feat(sim): vida, hambre, aire, caída y experiencia».

### Task 6: Romper bloques, nadar y acciones

**Files:**
- Create: `crates/survivalforest-sim/src/mining.rs`, `crates/survivalforest-sim/src/swim.rs`, `crates/survivalforest-sim/src/action.rs`
- Modify: `crates/survivalforest-sim/src/physics.rs` (exportar `in_water(w, aabb) -> (bool /*cuerpo*/, bool /*cabeza*/)`)

**Interfaces — Produces:**
- **`break_time(block: BlockId, held: Option<&ItemStack>, mode: GameMode) -> Option<f32>`:**
  - en creativo devuelve `Some(0.0)`;
  - devuelve `None` si el bloque no se puede romper;
  - si no, `hardness × 1,5 / velocidad`, donde la velocidad es la de la herramienta si es de la clase adecuada y de nivel suficiente, o 1;
  - con una mena sin el nivel necesario, el tiempo es `hardness × 5`.
- **`drops_for(block, held, mode, rng_seed: u64) -> (Vec<ItemStack>, u32 /*xp*/)`:**
  - en creativo devuelve `(vec![], 0)`;
  - sin la herramienta exigida (`needs_tool_to_drop` o `min_tier`) devuelve `vec![]`.
- **`wear_tool(held: &mut Option<ItemStack>, block) -> bool`:** devuelve `true` si la herramienta se rompió.
- **`swim_velocity(vel: DVec3, wish: DVec3, jump: bool, dt) -> DVec3`:** en el agua el jugador se hunde a 2 bloques/s como máximo, Espacio lo sube a 3 bloques/s y se mueve a la mitad de velocidad.
- **`Action`:** `enum Action { StartBreak(BlockPos), CancelBreak, Use(BlockPos, Face), SelectSlot(u8), Drop { all: bool }, SlotClick { container: ContainerRef, slot: usize, right: bool, shift: bool }, Craft, Respawn, Eat }`. En esta etapa el cliente la aplica en local; en la etapa 4, el servidor.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `stone_by_hand_slow_and_drops_nothing`.
  - `stone_with_wood_pick_drops_cobble`.
  - `iron_ore_needs_stone_pick`.
  - `iron_pick_faster_than_wood`.
  - `coal_ore_gives_coal_and_1_xp`.
  - `wear_right_tool_1_wrong_tool_2_breaks_at_zero`.
  - `creative_instant_no_drops`.
  - `platanera_drops_1_to_3`, determinista con la semilla.
  - Nadar (con el ayudante de mundo plano de `physics.rs`):
    - `swim_up_with_jump`;
    - `sinks_slowly_without_jump`;
    - `in_water_detects_head`.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-sim`.** Debe fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-sim`.** Debe pasar.
- [ ] **Step 5: Commit** «feat(sim): romper con herramientas, desgaste, nadar y acciones».

### Task 7: Objetos en el suelo, muerte y bloques con contenido

**Files:** Create `crates/survivalforest-sim/src/drops.rs`, `crates/survivalforest-sim/src/death.rs`, `crates/survivalforest-sim/src/containers.rs`.

**Interfaces — Produces:**
- **Objetos en el suelo:**
  - `GroundItem { pos: DVec3, vel: DVec3, stack: Option<ItemStack>, xp: f32, age: f32, ttl: f32 }` (Serialize, Deserialize); un orbe tiene `stack: None` y `xp > 0`.
  - `GroundItems(pub Vec<GroundItem>)`, con:
    - `spawn(pos, stack, ttl)`;
    - `tick(&mut self, w: &impl ChunkAccess, dt)`: gravedad, colisiones, juntar pilas iguales a menos de 0,7 y caducar;
    - `pickup(&mut self, feet: DVec3, inv: &mut Inventory, xp: &mut Experience)`: recoge a 1,5 bloques, deja en el suelo lo que no cabe y respeta 0,5 s de espera tras tirarlo.
- **Muerte:** `die(inv: &mut Inventory, xp: &mut Experience, ground: &mut GroundItems, at: DVec3)`: todo el inventario y el orbe quedan con `ttl` 600.
- **Bloques con contenido:**
  - `BlockEntities(pub HashMap<BlockPos, BlockEntity>)` (Serialize, Deserialize), con `enum BlockEntity { Chest(Chest), Furnace(Furnace) }`;
  - `on_placed(pos, block)` y `on_broken(pos, ground)`: al romperlo, el contenido cae al suelo;
  - `tick_furnaces(dt) -> Vec<(BlockPos, bool /*encendido*/)>`, para que el cliente cambie `FURNACE` ↔ `FURNACE_LIT`.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `item_falls_and_rests_on_ground`.
  - `pickup_within_radius_only`.
  - `pickup_full_inventory_leaves_rest`.
  - `same_items_merge`.
  - `items_expire_after_300s_death_items_600s`.
  - `death_drops_everything_and_half_xp_orb`: el inventario queda vacío, la suma de objetos en el suelo es igual a la de antes y el orbe tiene la mitad de la experiencia total.
  - `breaking_chest_drops_contents`.
  - `breaking_furnace_drops_input_fuel_output`.
  - `furnace_lit_state_reported`.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-sim`.** Debe fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-sim`.** Debe pasar.
- [ ] **Step 5: Commit** «feat(sim): objetos en el suelo, muerte y baúles y hornos».

### Task 8: Plantas de comida y punto de aparición

**Files:**
- Modify:
  - `crates/survivalforest-world/src/worldgen/features.rs`
  - `crates/survivalforest-client/src/session.rs` (`spawn_point`)
- Test: las pruebas de `features.rs` y la de `spawn_point_is_dry_land`, que ya existen.

**Interfaces — Produces:**
- **Generación:**
  - plataneras (`PLATANERA`, 2 bloques de alto) en Rainforest, Coast, Mangrove y CoffeeMountain con altura ≤ nivel del mar + 20;
  - arbustos de mora en Paramo y CloudForest;
  - guayabos (tronco de 3 bloques más `GUAVA_LEAVES`) en Savanna y BambooValley.

  Todo con el `Rng` existente y una sal propia, así que es determinista.
- **`spawn_point`:** salta también `RedDesert`, `SnowPeak` y `Paramo`.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `platanera_in_rainforest_chunks`: algún chunk de una búsqueda en espiral sobre una semilla fija tiene `PLATANERA`.
  - `blackberry_in_paramo`.
  - `guava_in_savanna`.
  - `plants_deterministic`: la misma semilla y el mismo chunk dan los mismos bloques dos veces.
  - `spawn_point_avoids_barren_biomes`: con 20 semillas, el bioma de aparición nunca es RedDesert, SnowPeak ni Paramo.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-world` y `cargo test -p survivalforest-client spawn`.** Deben fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar los mismos comandos.** Deben pasar.
- [ ] **Step 5: Commit** «feat(world): plataneras, moras y guayabos; aparición cerca de comida».

### Task 9: Guardado de la supervivencia

**Files:** Modify `crates/survivalforest-world/src/save.rs` (`META_VERSION` y funciones `pub`); Create `crates/survivalforest-sim/src/persist.rs`.

**Interfaces:**
- **Consume:** los tipos de sim. Para evitar dependencias circulares, `world` no puede depender de `sim`, así que el guardado de lo nuevo va en **`survivalforest-sim/src/persist.rs`** (decisión del plan). `world` solo aporta `write_atomic`, `ron_read` y `ron_write` (que pasan a ser `pub`).
- **Produce:**
  - **`PlayerSave` no cambia** (vive en `world`, que no ve los tipos de `sim`). Lo nuevo del jugador va en un archivo aparte, `survival.ron`, con `SurvivalSave { version: 1, inventory: Slots, selected: u8, vitals: Vitals, xp: Experience, crafted: Vec<String>, spawn: Option<[i32; 3]> }`, leído y escrito por `persist.rs` con `load_survival(world_dir) -> SurvivalSave` (si falta el archivo: inventario vacío y todo lleno) y `save_survival(world_dir, &SurvivalSave)`.
  - **`META_VERSION = 2`**: al cargar se acepta la 1, y al guardar se escribe la 2.
  - **`persist.rs`:**
    - `save_extras(world_dir, region: (i32, i32), be: &BlockEntities, items: &GroundItems)`;
    - `load_extras(world_dir, region) -> (BlockEntities, GroundItems)`;
    - un archivo `region/r.X.Z.extra.ron` con `{ version: 1, block_entities, ground_items }`;
    - los objetos en el suelo se reparten por región según su posición.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `stage1_world_loads_with_defaults`: un mundo con el `player.ron` y el `meta.ron` de la etapa 1 y sin `survival.ron` carga; `load_survival` da el inventario vacío y la vida y el hambre llenas.
  - `stage1_meta_version_1_loads`.
  - `extras_roundtrip`: un baúl con 3 pilas, un horno a medias, un objeto y un orbe guardan y cargan iguales.
  - `death_items_survive_save_and_load`.
  - `missing_extras_file_is_empty`.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-world -p survivalforest-sim persist`.** Debe fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar `cargo test --workspace`.** Debe pasar.
- [ ] **Step 5: Commit** «feat: guardado de inventario, vida, experiencia, baúles, hornos y objetos (mundos de la etapa 1 compatibles)».

### Task 10: Texturas e iconos nuevos

**Files:** Modify `crates/survivalforest-client/src/textures.rs`; Create `crates/survivalforest-client/src/icons.rs`.

**Interfaces — Produces:**
- `paint_tile(name)` cubre las texturas nuevas de la Task 1. Las de la mesa, el horno (apagado y encendido), el baúl y la piedra de amolar tienen arriba, lado y abajo.
- `item_icon(key: &str) -> Tile` (32×32 RGBA) pinta cada objeto que no es bloque: herramientas con mango de vara y cabeza del color del material, lingotes, carbón, vara, comida, cuarzo y ladrillo. Para los bloques, `block_icon(BlockId)` reutiliza el que ya existe.
- `ItemIcons(HashMap<ItemId, Handle<Image>>)` como recurso de Bevy, que reemplaza a `BlockIcons`.

- [ ] **Step 1: Escribir las pruebas que fallan:**
  - `every_tile_name_paints`: ninguna textura queda vacía;
  - `every_item_has_icon_not_blank`: cada icono tiene al menos 40 píxeles opacos;
  - `material_colors_distinct`: el color medio de la cabeza del pico cambia entre madera, piedra, cobre y hierro.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-client textures`.** Debe fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-client`.** Debe pasar.
- [ ] **Step 5: Commit** «feat(client): texturas e iconos de los objetos nuevos».

### Task 11: Juego en el cliente (simulación, HUD, objetos en el mundo)

**Files:**
- Create: `crates/survivalforest-client/src/survival.rs`, `crates/survivalforest-client/src/drops_view.rs`
- Modify:
  - `crates/survivalforest-client/src/interact.rs`
  - `crates/survivalforest-client/src/player.rs`
  - `crates/survivalforest-client/src/hud.rs`
  - `crates/survivalforest-client/src/session.rs`
  - `crates/survivalforest-client/src/main.rs`

**Interfaces:**
- **Consume:** Tasks 3–10.
- **Produce:**
  - **Recursos de Bevy:** `PlayerState { inv: Inventory, vitals: Vitals, xp: Experience, crafted: HashSet<String>, cursor: Option<ItemStack>, breaking: Option<(BlockPos, f32)>, eating: Option<f32>, dead: bool }`, y además `WorldExtras { be: BlockEntities, items: GroundItems }`.
  - **Sistemas en `FixedUpdate`:** `vitals_step`, que usa `in_water`, `swim_velocity` en `physics_step` y `on_ground` al aterrizar; `drops_step`; `furnaces_step`; `pickup_step`.
  - **Interacción (`interact.rs`):**
    - Clic izquierdo mantenido avanza `breaking` con `break_time`. Al terminar:
      - rompe el bloque con `set_block`;
      - suelta objetos con `drops_for` y llama a `wear_tool`;
      - suma la experiencia;
      - si era un bloque con contenido, llama a `on_broken`.
    - Clic derecho, según lo que se apunte y lo que se tenga en la mano:
      - una mesa, un horno, un baúl o una piedra de amolar abren su pantalla (Task 12);
      - si en la mano hay un bloque, lo pone y gasta 1;
      - si en la mano hay comida y el hambre no está llena, come (`eating` durante `eat_secs`).
    - En creativo siguen como ahora: romper instantáneo y poner sin gastar.
  - **Teclas:**
    - Q tira 1 de la mano y Ctrl+Q la pila entera. Para que no choque con correr: si se pulsa Q con Ctrl, Ctrl deja de contar como correr ese tick.
    - 1–9 y la rueda eligen casilla.
    - E abre el inventario.
  - **HUD (`hud.rs`)**, con la disposición del boceto del diseño:
    - corazones (llenos, medios y vacíos);
    - jamones;
    - la barra de experiencia con el nivel dentro de un grano de café;
    - la barra de 9 casillas con iconos, cantidades y barra de durabilidad;
    - las burbujas de aire, solo bajo el agua;
    - un destello rojo de 0,3 s al recibir daño;
    - las grietas sobre el bloque que se rompe (6 etapas, dibujadas con gizmos o con un cubo con textura de grietas).
    - En creativo no hay corazones, jamones ni experiencia.
  - **Objetos en el suelo (`drops_view.rs`):** cubos pequeños con el icono, que giran y flotan; los orbes, como una esfera verde café brillante.
  - **Al morir:** `dead = true`, llama a `die` y muestra la pantalla de muerte (Task 12).
  - **Guardado (`session.rs`):** guarda y carga `PlayerState` y `WorldExtras` con las Tasks 9.

- [ ] **Step 1: Escribir la prueba que falla** (en Rust, sin ventana): `survival_tick_headless`. Monta una `App` mínima de Bevy con `MinimalPlugins`, el `World` de un mundo plano y los sistemas `vitals_step` y `drops_step`. Tras 10 s de tiempo fijo en supervivencia, el hambre ha bajado. Con un `GroundItem` a 1 bloque, `pickup_step` lo mete en el inventario.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-client survival`.** Debe fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar `cargo test -p survivalforest-client` y `cargo build -p survivalforest-client`.** Deben pasar. La prueba de humo `--smoke` sigue funcionando.
- [ ] **Step 5: Commit** «feat(client): supervivencia en el juego: vida, hambre, romper con herramientas, objetos y HUD».

### Task 12: Pantallas (inventario, mesa, horno, baúl, piedra de amolar, muerte, creativo)

**Files:** Create `crates/survivalforest-client/src/screens.rs`; Modify `crates/survivalforest-client/src/main.rs`, `crates/survivalforest-client/src/menu.rs` (estilo compartido).

**Interfaces — Produces:**
- **Estado de pantalla:** `enum Screen { None, Inventory, Table(BlockPos), Furnace(BlockPos), Chest(BlockPos), Grindstone(BlockPos), Death, CreativeList }` como recurso.
  - Con una pantalla abierta se suelta el ratón y el jugador no se mueve.
  - E o Esc la cierran y devuelven al inventario lo que quede en la rejilla de crafteo y en el cursor; lo que no quepa se tira al suelo.
- **Casillas:** cada casilla es un `Button` con `SlotRef { container: ContainerRef, index }`. El clic llama a `Slots::click`, Mayús+clic a `quick_move` y el clic en la salida de crafteo a `craft`, que da la experiencia de la primera vez.
- **Burbuja de información:** nombre, durabilidad «n/m» y «+N jamones» para la comida.
- **Pantallas:**
  - **Inventario:** la figura, la rejilla 2×2 → resultado, y la mochila y la barra.
  - **Mesa:** la rejilla 3×3 → resultado.
  - **Horno:** entrada, combustible, la llama y la flecha de progreso, y la salida.
  - **Baúl:** 27 casillas.
  - **Piedra de amolar:** la casilla de la herramienta, el resultado, el coste y el aviso «Te faltan N niveles».
  - **Muerte:** «Te has quedado en el cafetal», las coordenadas de las cosas y el botón «Reaparecer».
  - **Lista creativa:** todos los objetos; un clic da una pila completa al cursor.
- **Estilo:** madera de guadua con bordes tallados, casillas en tono tierra y la fuente del juego. Se reutilizan los colores de `menu.rs`.

- [ ] **Step 1: Escribir las pruebas que fallan**, de la lógica de pantalla sin ventana, en `screens.rs`:
  - `closing_returns_grid_and_cursor_to_inventory`;
  - `closing_with_full_inventory_drops_to_ground`;
  - `craft_click_gives_first_time_xp_once`.
- [ ] **Step 2: Ejecutar `cargo test -p survivalforest-client screens`.** Debe fallar.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Comprobar.**
  - `cargo test -p survivalforest-client` debe pasar.
  - Capturas con el mecanismo que ya existe en `main.rs` (`Screenshot`), lanzando el cliente con xvfb y una opción de depuración `--shot-screen <nombre>`, que abre la pantalla indicada sobre un mundo de prueba y guarda la captura. Se revisan las de HUD, inventario, mesa, horno, baúl, piedra de amolar y muerte.
- [ ] **Step 5: Commit** «feat(client): inventario, mesa, horno, baúl, piedra de amolar, muerte y lista creativa».

### Task 13: Cierre

**Files:** `Cargo.toml` (versión 0.2.0), `README.md`, `THIRD_PARTY_NOTICES.md` (si hay dependencias nuevas).

- [ ] **Step 1: Actualizar la versión y el README.**
  - Versión del workspace: 0.2.0.
  - README: controles y contenido de la supervivencia, con la tabla de recetas generada a mano a partir de `recipes.ron`.
- [ ] **Step 2: Verificación completa.** Todo en verde:
  - `cargo fmt --all --check`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo test --workspace`;
  - `cargo build --release -p survivalforest-client`;
  - la prueba de humo con xvfb, como en CI.
- [ ] **Step 3: Partida de prueba guiada por capturas.** Se crea un mundo con una semilla fija y se ejecuta el guion de depuración `--script etapa2`: talar, hacer la mesa y un pico, picar piedra, fundir, comer, caer de 10 bloques y morir. Captura de cada paso, para revisarlas.
- [ ] **Step 4: Revisión final** de toda la rama con un revisor independiente. Lo que encuentre se corrige empezando por una prueba que falle.
- [ ] **Step 5: Commit, push y PR a `main`.** Requiere que Claude tenga acceso a GitHub en `juego-rust`; si no, se avisa al dueño.
