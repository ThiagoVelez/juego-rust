# SurvivalForest · Etapa 1 (Mundo) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** una versión jugable de SurvivalForest con estas piezas:
- un mundo de bloques procedural prácticamente infinito, con biomas, cuevas y menas;
- luz y guardado;
- primera persona en modo supervivencia (andar y saltar) y creativo (volar);
- romper y poner bloques;
- ciclo de día y noche;
- menús para crear y abrir mundos.

**Architecture:** el workspace de Cargo separa la lógica pura de Rust de Bevy.
- La lógica pura, con pruebas unitarias sin ventana, va en `survivalforest-core`, `survivalforest-world` y `survivalforest-sim`.
- `survivalforest-client` es la única crate que depende de Bevy. Hace de pegamento: transmisión de chunks, render, entrada, interfaz.
- La generación y el mallado de chunks corren en el pool de tareas asíncronas de Bevy.

**Tech Stack:**
- Rust 1.97 (edición 2024);
- Bevy 0.20.x;
- `noise` 0.9;
- `lz4_flex`, que es Rust puro (no `zstd`, que compila C);
- `serde` y `ron` 0.12;
- `rand_chacha`;
- GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-12-juego-de-bloques-design.md`

## Global Constraints

- **Lenguaje:** todo en Rust, incluidos el juego, el servidor, las herramientas y el CI.
- **Licencias:** las dependencias deben tener licencias permisivas (MIT, Apache-2.0, BSD o Zlib), y nada GPL dentro del ejecutable.
- **Arte y sonido:** propios o CC0.
- **Plataformas:** Windows 10+ y Linux (Ubuntu 22.04+ y Debian 12+).
- **Rendimiento:** 60 FPS con gráfica integrada a 12 chunks de distancia de visión.
- **Sin internet:** el juego en solitario no depende de internet.
- **Identidad propia:** prohibido usar nombres, texturas, sonidos o la disposición exacta de la interfaz de Minecraft.
  - Los nombres de bloques y objetos son propios y en español en la interfaz.
  - Los identificadores internos van en `snake_case` inglés neutro, por ejemplo `red_soil`.
- **Tamaño del mundo:**
  - columnas de 16×16 con secciones de 16³;
  - altura de −64 a 320 (384 bloques, 24 secciones);
  - límite horizontal de ±30 000 000 bloques.
- **Tiempo:** 20 ticks por segundo, y un día completo de 20 minutos (24 000 ticks).
- **Guardado:**
  - regiones de 32×32 chunks, comprimidas;
  - datos del mundo y del jugador en RON;
  - formatos con número de versión.
- **Modos:** supervivencia y creativo. En esta etapa, supervivencia solo significa que no se vuela; vida, hambre e inventario llegan en la etapa 2.
- **Bevy:** 0.20 queda fuera del conocimiento del autor del plan. Antes de escribir código de Bevy, mira los ejemplos de la versión descargada en `~/.cargo/registry/src/*/bevy-0.20*/examples/` (por ejemplo `3d/generate_custom_mesh.rs`, `asset/asset_loading.rs`, `async_tasks/async_compute.rs`, `input/mouse_grab.rs`) y copia la API de ahí.

## Review Focus

1. **Coordenadas negativas y bordes de chunk.** Al pasar de bloque a chunk con x o z negativos se debe usar división hacia abajo: el bloque −1 está en el chunk −1 con índice local 15. Lo prueba la Task 1.
2. **Lejos del origen** (x ≈ 29 999 000). La generación funciona igual, y el render no tiembla: la cámara usa un origen flotante y las mallas se colocan relativas al chunk del jugador. Lo prueban las Tasks 3 y 8.
3. **Poner un bloque dentro del propio jugador**, o fuera de −64…319, se rechaza sin efecto. Lo prueba la Task 10.
4. **Romper o poner en el borde de un chunk** vuelve a mallar también el chunk vecino, para que no queden caras abiertas ni sombras viejas. Lo prueba la Task 10.
5. **Cierre brusco o archivo de región dañado.** Se escribe en un archivo temporal y luego se renombra. Un chunk ilegible se regenera y se avisa en el registro; nunca hace fallar la carga del mundo. Lo prueba la Task 7.

---

## Mapa de archivos

```
Cargo.toml                         workspace (resolver 3), perfiles: dev opt-level=1 y deps opt-level=3
crates/survivalforest-core/src/
  lib.rs, coords.rs                BlockPos, ChunkPos, LocalPos, constantes de tamaño
  block.rs                         BlockId, BlockDef, registro estático de bloques de la etapa 1
  mode.rs                          GameMode
crates/survivalforest-world/src/
  lib.rs
  chunk.rs                         Section (paleta) y ChunkColumn
  gen/mod.rs                       WorldGen (semilla → ChunkColumn)
  gen/climate.rs                   ruido de clima y elección de bioma
  gen/biome.rs                     Biome y reglas de superficie y vegetación
  gen/carve.rs                     cuevas y menas
  gen/features.rs                  árboles, guadua y cafetos
  light.rs                         luz de cielo y de bloque
  mesh.rs                          mallado por caras con AO → MeshData
  region.rs                        archivos de región
  save.rs                          WorldMeta, PlayerSave y carpeta del mundo
crates/survivalforest-sim/src/
  lib.rs
  physics.rs                       Aabb, colisión y movimiento contra el mundo
  raycast.rs                       DDA de bloques
  time.rs                          WorldTime: sol y colores del cielo
  edit.rs                          reglas de romper y poner
crates/survivalforest-client/src/
  main.rs                          App, estados (Menu, Loading, InGame, Paused)
  textures.rs                      texturas 16×16 pintadas por código → array de texturas
  streaming.rs                     carga y descarga de chunks, tareas asíncronas, origen flotante
  render.rs                        material de bloques (WGSL) y cielo
  player.rs                        entrada, cámara, modo creativo
  interact.rs                      apuntar, romper, poner, contorno
  hud.rs                           punto de mira, barra de bloques (creativo), F3
  menu.rs                          título, crear/abrir mundo, pausa, ajustes
  settings.rs                      Settings en RON
assets/shaders/blocks.wgsl, sky.wgsl
.github/workflows/ci.yml
```

---

### Task 1: Workspace, coordenadas y registro de bloques

**Files:**
- Create: `Cargo.toml`, `crates/survivalforest-core/{Cargo.toml,src/lib.rs,src/coords.rs,src/block.rs,src/mode.rs}`, `.github/workflows/ci.yml`, `rust-toolchain.toml` (channel `1.97`), `.gitignore` (`/target`)
- Test: `crates/survivalforest-core/src/coords.rs` (`#[cfg(test)]`), `crates/survivalforest-core/src/block.rs`

**Interfaces:**
- Produces:
  - Constantes:
    - `pub const CHUNK: i32 = 16;`
    - `pub const MIN_Y: i32 = -64;`
    - `pub const MAX_Y: i32 = 320;` (exclusivo)
    - `pub const SECTIONS: usize = 24;`
    - `pub const WORLD_LIMIT: i32 = 30_000_000;`
  - Coordenadas:
    - `#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)] pub struct BlockPos { pub x: i32, pub y: i32, pub z: i32 }`, con `chunk(self) -> ChunkPos`, `local(self) -> LocalPos` y `offset(self, dx, dy, dz) -> BlockPos`;
    - `pub struct ChunkPos { pub x: i32, pub z: i32 }`, con `origin(self) -> BlockPos` (y = `MIN_Y`) y `neighbors(self) -> [ChunkPos; 4]`;
    - `pub struct LocalPos { pub x: u8, pub y: u16, pub z: u8 }`, donde `y` cuenta desde `MIN_Y`.
  - Bloques:
    - `#[repr(transparent)] pub struct BlockId(pub u16)`;
    - `pub struct BlockDef { pub id: BlockId, pub key: &'static str, pub name_es: &'static str, pub opaque: bool, pub solid: bool, pub breakable: bool, pub emits_light: u8, pub tex: [u16; 3] /* arriba, lado, abajo */ }`;
    - `pub fn blocks() -> &'static [BlockDef]`, `pub fn block(id: BlockId) -> &'static BlockDef`, `pub fn by_key(key: &str) -> Option<BlockId>`.
  - `pub enum GameMode { Survival, Creative }` (serde).
  - Los bloques de la etapa 1, en este orden (id = índice):
    - `air`, `bedrock_floor` (Lecho de roca), `stone` (Piedra), `red_soil` (Tierra roja), `grass` (Hierba), `sand` (Arena), `red_sand` (Arena roja), `gravel` (Grava), `clay` (Arcilla), `mud` (Barro de manglar), `snow` (Nieve), `ice` (Hielo), `water` (Agua);
    - `log` (Tronco), `leaves` (Hojas), `coffee_bush` (Cafeto), `bamboo` (Guadua), `frailejon` (Frailejón), `planks` (Tablones), `whitewash` (Pared encalada), `roof_tile` (Teja);
    - `coal_ore` (Carbón), `iron_ore` (Hierro), `copper_ore` (Cobre), `gold_ore` (Oro), `quartz_ore` (Cuarzo), `torch` (Antorcha, `emits_light = 14`).
  - Constantes `pub const AIR: BlockId = BlockId(0)` y `pub const WATER`, `STONE`, `BEDROCK` (u otras) donde las usen otras tasks.
  - `air`, `water`, `leaves`, `coffee_bush`, `bamboo`, `frailejon` y `torch` no son opacos.
  - `air`, `water` y `torch` no son sólidos.
  - `bedrock_floor` no se puede romper.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn negative_block_maps_to_previous_chunk() {
    let p = BlockPos { x: -1, y: 0, z: -17 };
    assert_eq!(p.chunk(), ChunkPos { x: -1, z: -2 });
    assert_eq!(p.local(), LocalPos { x: 15, y: 64, z: 15 });
}
#[test] fn chunk_origin_roundtrip() {
    for (x, z) in [(0, 0), (-1, 5), (1874999, -1874999)] {
        let c = ChunkPos { x, z };
        assert_eq!(c.origin().chunk(), c);
    }
}
#[test] fn registry_ids_match_indices_and_keys_unique() {
    for (i, b) in blocks().iter().enumerate() { assert_eq!(b.id.0 as usize, i); }
    let mut keys: Vec<_> = blocks().iter().map(|b| b.key).collect(); keys.sort(); keys.dedup();
    assert_eq!(keys.len(), blocks().len());
    assert!(!block(by_key("bedrock_floor").unwrap()).breakable);
    assert_eq!(block(by_key("torch").unwrap()).emits_light, 14);
}
```

- [ ] **Step 2:** Run `cargo test -p survivalforest-core`. Expected: FAIL (no compila).
- [ ] **Step 3: Implement.** Usa `div_euclid` y `rem_euclid`. El registro es un `static` slice. Los índices de textura `tex` apuntan a las capas que pinta la Task 8, en el mismo orden en que esa task las declara en `TILES`; si se definen allí, coloca aquí la tabla compartida `pub const TILE_NAMES: &[&str]`.
- [ ] **Step 4:** Crea `ci.yml`, que corre en ubuntu-22.04 y en windows-latest:
  - `cargo fmt --check`;
  - `cargo clippy --workspace -- -D warnings`;
  - `cargo test --workspace`.

  En Linux instala antes las dependencias de Bevy: `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`. Usa `Swatinem/rust-cache@v2`.
- [ ] **Step 5:** Run `cargo test -p survivalforest-core`. Expected: PASS.
- [ ] **Step 6:** Commit `feat(core): coordenadas, registro de bloques y CI`.

### Task 2: Almacenamiento de chunks

**Files:**
- Create: `crates/survivalforest-world/{Cargo.toml,src/lib.rs,src/chunk.rs}`

**Interfaces:**
- Consumes: `BlockId`, `LocalPos`, `SECTIONS`.
- Produces:
  - `pub struct Section`: paleta de `Vec<BlockId>` más índices empaquetados en bits, o un único valor si es uniforme. Métodos `get(x,y,z: usize) -> BlockId`, `set(..)`, `is_empty() -> bool`.
  - `pub struct ChunkColumn { pub pos: ChunkPos, pub sections: [Section; SECTIONS], pub sky_light: Vec<u8>, pub block_light: Vec<u8>, pub heightmap: [i16; 256], pub dirty: bool }`.
    - `sky_light` y `block_light` usan 4 bits por bloque, empaquetados.
    - Métodos: `get(LocalPos) -> BlockId`, `set(LocalPos, BlockId)` (marca `dirty` y actualiza `heightmap`), `sky(LocalPos) -> u8`, `block_light(LocalPos) -> u8` y sus setters.
  - `heightmap[z*16+x]` = la y absoluta del bloque opaco más alto.

- [ ] **Step 1: Write the failing tests:**
  - `section_uniform_until_written`: una sección nueva es uniforme; tras un `set` deja de serlo, y `get` devuelve lo escrito.
  - `palette_grows_past_16_kinds`: escribe 40 ids distintos y lee los 40.
  - `heightmap_tracks_highest_opaque`: pon piedra en y=10 y en y=70, y luego aire en y=70; el heightmap pasa de 70 a 10.
  - `light_nibbles_roundtrip`: escribe los valores 0..=15 en posiciones vecinas y lee los mismos.
- [ ] **Step 2:** `cargo test -p survivalforest-world chunk`. Expected: FAIL.
- [ ] **Step 3: Implement** con índices de 4 bits mientras la paleta tenga ≤16 entradas y de 8 bits hasta 256; por encima, `u16`.
- [ ] **Step 4:** Mismo comando. Expected: PASS.
- [ ] **Step 5:** Commit `feat(world): secciones con paleta y columnas de chunk`.

### Task 3: Generación del terreno y biomas

**Files:**
- Create: `crates/survivalforest-world/src/gen/{mod.rs,climate.rs,biome.rs}`

**Interfaces:**
- Consumes: `ChunkColumn`, registro de bloques.
- Produces:
  - `pub struct WorldGen { pub seed: u64, .. }`, con:
    - `fn new(seed: u64) -> Self`;
    - `fn generate(&self, pos: ChunkPos) -> ChunkColumn`, que debe ser `Send + Sync`;
    - `fn biome_at(&self, x: i32, z: i32) -> Biome`;
    - `fn surface_height(&self, x: i32, z: i32) -> i32`.
  - `pub enum Biome { CoffeeMountain, Paramo, Rainforest, BambooValley, Savanna, RedDesert, SnowPeak, Coast, Ocean, Mangrove, CloudForest }`, con `name_es()` y `surface(&self) -> (BlockId /*superficie*/, BlockId /*subsuelo*/)` y vegetación por densidad.
- **Clima:** cinco campos de ruido Perlin fBm independientes, sembrados con `seed ^ CONST_i`: continentalidad, erosión, temperatura, humedad y rareza.
- **Elección de bioma:**
  - continentalidad < −0.25 → Ocean; < −0.12 → Coast;
  - después, por temperatura y humedad;
  - Paramo y SnowPeak exigen altura > 150; CloudForest, 100–160; Mangrove es costa húmeda y cálida.
- **Altura:** `64 + continentalidad*… + montaña(erosión)`, limitada a `[MIN_Y+8, 300]`. Nivel del mar: y = 62.
- **Columna:** `bedrock_floor` en y = −64; piedra hasta superficie−4; subsuelo de 3 bloques; superficie; agua hasta 62.

- [ ] **Step 1: Write the failing tests:**
  - `same_seed_same_chunk`: `generate` dos veces con la misma semilla da columnas iguales; compara con `PartialEq` o un hash del contenido.
  - `different_seed_differs`.
  - `border_continuity`: para 64 posiciones a lo largo del borde x=15|16 entre dos chunks, `|surface_height(15,z) - surface_height(16,z)| <= 6`.
  - `far_lands_still_generate`: genera `ChunkPos{x:1_874_937, z:-1_874_937}` (≈ ±29 999 000 bloques). Comprueba que hay `bedrock_floor` en el fondo y una superficie entre −56 y 300.
  - `biome_variety`: muestrea 4096 posiciones en una rejilla de 8 192 × 8 192 con la semilla 42; aparecen al menos 8 de los 11 biomas.
  - `ocean_has_water_at_sea_level`.
- [ ] **Step 2:** `cargo test -p survivalforest-world gen`. Expected: FAIL.
- [ ] **Step 3: Implement.** Usa coordenadas `f64` del mundo divididas por la escala de cada campo; la continentalidad va a escala 1/2048. Las semillas de cada campo salen de `rand_chacha::ChaCha8Rng::seed_from_u64(seed)`.
- [ ] **Step 4:** Expected: PASS. Además, `cargo test --release -p survivalforest-world gen_speed`: una prueba `#[ignore]`-free que genere 64 chunks en < 2 s en release; marca la prueba `#[cfg(not(debug_assertions))]`.
- [ ] **Step 5:** Commit `feat(world): terreno procedural y 11 biomas`.

### Task 4: Cuevas, menas y vegetación

**Files:**
- Create: `crates/survivalforest-world/src/gen/{carve.rs,features.rs}`
- Modify: `gen/mod.rs` (llama a `carve` y luego a `features` dentro de `generate`)

**Interfaces:**
- Produces:
  - `fn carve_caves(col: &mut ChunkColumn, gen: &WorldGen)`: ruido 3D tipo «queso» más túneles «espagueti». Nunca toca `bedrock_floor`, y no abre cuevas a menos de 4 bloques del fondo del océano.
  - `fn place_ores(col: &mut ChunkColumn, seed: u64)`, con vetas por rango de altura:

    | Mena | Altura | Vetas por chunk | Tamaño |
    |---|---|---|---|
    | carbón | 0..192 | 20 | ≤17 |
    | cobre | −16..112 | 10 | — |
    | hierro | −64..72 | 12 | ≤9 |
    | oro | −64..32 | 4 | — |
    | cuarzo | −64..0 | 3 | — |

    Solo sustituyen a `stone`.
  - `fn decorate(col: &mut ChunkColumn, gen: &WorldGen)`:
    - árboles de copa redonda;
    - matas de guadua de 5 a 10 de alto;
    - hileras de cafetos en CoffeeMountain;
    - frailejones en Paramo.

    Solo se coloca lo que cabe dentro del propio chunk; la vegetación que cruza chunks queda fuera de esta etapa. Es determinista por posición del mundo.

- [ ] **Step 1: Write the failing tests:**
  - `caves_exist`: en 16 chunks de la semilla 7, al menos el 2 % de los bloques bajo y=50 son aire.
  - `ores_respect_ranges`: en 32 chunks no hay oro por encima de y=32 ni hierro por encima de y=72, y hay al menos 1 de carbón por chunk.
  - `bedrock_intact`: y=−64 es siempre `bedrock_floor`.
  - `coffee_rows_in_coffee_mountain`: busca un chunk CoffeeMountain y comprueba que tiene ≥ 4 `coffee_bush`.
  - `generation_still_deterministic`: vuelve a ejecutar la prueba de determinismo de la Task 3 con todo activado.
- [ ] **Step 2:** Run `cargo test -p survivalforest-world`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Expected: PASS.
- [ ] **Step 5:** Commit `feat(world): cuevas, menas y vegetación`.

### Task 5: Iluminación

**Files:**
- Create: `crates/survivalforest-world/src/light.rs`

**Interfaces:**
- Produces:
  - `pub fn light_column(col: &mut ChunkColumn)`: luz de cielo 15 desde arriba hasta el primer opaco. Las hojas y el agua restan 1 por bloque. Después, propagación BFS lateral y hacia abajo (−1 por paso), más la luz de bloque desde los `emits_light`.
  - `pub fn relight_around(world: &mut impl ChunkAccess, pos: BlockPos)`: reiluminación incremental tras un cambio. Primero quita la luz por BFS y después la vuelve a propagar. Puede cruzar a chunks vecinos.
  - `pub trait ChunkAccess { fn column(&self, p: ChunkPos) -> Option<&ChunkColumn>; fn column_mut(&mut self, p: ChunkPos) -> Option<&mut ChunkColumn>; }`. Lo implementa `HashMap<ChunkPos, ChunkColumn>`.

- [ ] **Step 1: Write the failing tests:**
  - `open_sky_is_15`.
  - `under_roof_falls_off`: con un techo de piedra 5×5 a y=80 sobre el suelo y=70, la luz de cielo en el centro a y=75 es < 15 y > 0.
  - `torch_lights_dark_cave`: una antorcha en una cavidad sellada da 14 en su bloque y 13 al lado.
  - `removing_torch_darkens`: `relight_around` tras quitar la antorcha deja 0 en la cavidad.
  - `light_crosses_chunk_border`: una antorcha en x=15 ilumina x=16 del chunk vecino con 13.
- [ ] **Step 2:** Run `cargo test -p survivalforest-world light`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Expected: PASS.
- [ ] **Step 5:** Commit `feat(world): luz de cielo y de bloque`.

### Task 6: Mallado

**Files:**
- Create: `crates/survivalforest-world/src/mesh.rs`

**Interfaces:**
- Consumes: `ChunkColumn` y los 4 vecinos (opcionales; si falta un vecino, su borde se trata como opaco para no crear caras ocultas).
- Produces:
  - `pub struct MeshData { pub positions: Vec<[f32; 3]>, pub normals: Vec<[f32; 3]>, pub uvs: Vec<[f32; 2]>, pub layer: Vec<u32>, pub light: Vec<[f32; 2]> /* cielo, bloque 0..1 */, pub ao: Vec<f32>, pub indices: Vec<u32> }`.
  - `pub struct ChunkMeshes { pub opaque: MeshData, pub cutout: MeshData /* hojas, plantas */, pub water: MeshData }`.
  - `pub fn mesh_column(col: &ChunkColumn, neighbors: [Option<&ChunkColumn>; 4]) -> ChunkMeshes`. Las posiciones son locales al chunk (0..16, y absoluta).
- **Reglas:**
  - Una cara se dibuja si el vecino no es opaco y (no es el mismo bloque, o el bloque es cutout).
  - El agua solo dibuja las caras que dan al aire, y su cara superior baja 0,125.
  - AO por vértice con la regla de 3 vecinos (lado1, lado2, esquina) y valores {0.5, 0.68, 0.85, 1.0}. Si `ao[0]+ao[2] < ao[1]+ao[3]`, se gira la diagonal.
  - `guadua`, `frailejon`, `coffee_bush` y `torch` se dibujan como cubo (las plantas en cruz llegan en la etapa 3).

- [ ] **Step 1: Write the failing tests:**
  - `single_block_has_6_faces` (24 vértices, 36 índices).
  - `two_adjacent_blocks_have_10_faces`.
  - `water_only_faces_air`.
  - `missing_neighbor_hides_border`.
  - `ao_corner_darkens`: un bloque con dos vecinos en L tiene algún vértice con `ao == 0.5`.
  - `mesh_full_chunk_under_20ms_release` (`#[cfg(not(debug_assertions))]`): una columna generada se malla en < 20 ms.
- [ ] **Step 2:** Run `cargo test -p survivalforest-world mesh`. Expected: FAIL.
- [ ] **Step 3:** Implement. Recorre las secciones no vacías y salta las secciones enteras de aire.
- [ ] **Step 4:** Expected: PASS.
- [ ] **Step 5:** Commit `feat(world): mallado con oclusión ambiental`.

### Task 7: Guardado

**Files:**
- Create: `crates/survivalforest-world/src/{region.rs,save.rs}`

**Interfaces:**
- Produces:
  - Región:
    - `pub struct RegionStore { dir: PathBuf, .. }`, con `fn open(dir) -> io::Result<Self>`, `fn load(&mut self, p: ChunkPos) -> Option<ChunkColumn>`, `fn save(&mut self, col: &ChunkColumn) -> io::Result<()>` y `fn flush(&mut self) -> io::Result<()>`.
    - Archivo `region/r.<rx>.<rz>.sfr`: cabecera mágica `b"SFR1"` y tabla de 1024 entradas `(offset u32, len u32, version u16)`.
    - Cada chunk se serializa con `postcard` o con bincode propio, y se comprime con `lz4_flex`.
  - Mundo:
    - `pub struct WorldMeta { pub version: u32, pub name: String, pub seed: u64, pub mode: GameMode, pub time_ticks: u64, pub created: String }`;
    - `pub struct PlayerSave { pub pos: [f64; 3], pub yaw: f32, pub pitch: f32, pub flying: bool }`;
    - `pub fn worlds_dir() -> PathBuf`: `%APPDATA%\SurvivalForest\saves` en Windows y `~/.local/share/SurvivalForest/saves` en Linux.
    - `pub fn create_world(name, seed, mode) -> io::Result<PathBuf>`, `pub fn list_worlds() -> Vec<(PathBuf, WorldMeta)>`, `pub fn load_meta`, `pub fn save_meta` y `pub fn save_player`/`load_player` (RON).
  - Escritura atómica: se escribe en `*.tmp` y se renombra.

- [ ] **Step 1: Write the failing tests** (en `tempfile::tempdir()`):
  - `region_roundtrip`: guarda 3 columnas editadas, cierra, abre y obtiene las mismas.
  - `corrupt_chunk_returns_none`: estropea los bytes de un chunk en el archivo; `load` devuelve `None` y no entra en pánico.
  - `truncated_region_file_is_tolerated`.
  - `meta_roundtrip_ron`.
  - `list_worlds_skips_garbage_dirs`.
  - `negative_chunks_go_to_negative_regions`: el chunk (−1,−1) cae en `r.-1.-1.sfr`.
- [ ] **Step 2:** Run `cargo test -p survivalforest-world region save`. Expected: FAIL.
- [ ] **Step 3:** Implement. Cuando `load` devuelve `None` en un chunk que estaba en la tabla, se registra `warn!`; el llamador regenera el chunk.
- [ ] **Step 4:** Expected: PASS.
- [ ] **Step 5:** Commit `feat(world): regiones comprimidas y datos del mundo`.

### Task 8: Cliente Bevy: ventana, texturas y transmisión de chunks

**Files:**
- Create:
  - `crates/survivalforest-client/{Cargo.toml,src/main.rs,src/textures.rs,src/streaming.rs,src/render.rs}`;
  - `assets/shaders/blocks.wgsl`.

**Interfaces:**
- Consumes: `WorldGen`, `mesh_column`, `light_column`, `RegionStore`.
- Produces:
  - `#[derive(Resource)] pub struct World { pub gen: Arc<WorldGen>, pub columns: HashMap<ChunkPos, ChunkColumn>, pub store: RegionStore, pub meta: WorldMeta, pub dir: PathBuf }`, que implementa `ChunkAccess`.
  - `#[derive(Resource)] pub struct ViewDistance(pub u32)` (12 por defecto).
  - `#[derive(Resource)] pub struct FloatingOrigin(pub ChunkPos)`.
  - Eventos:
    - `#[derive(Event)] pub struct RemeshRequest(pub ChunkPos)`;
    - `pub fn set_block(world: &mut World, pos: BlockPos, id: BlockId, remesh: &mut EventWriter<RemeshRequest>)`, que relanza la luz y emite el remallado del chunk y de los vecinos tocados.
  - Estados: `#[derive(States)] pub enum AppState { Menu, Loading, InGame, Paused }`.
- **Texturas:**
  - `textures.rs` pinta en Rust una capa de 16×16 por textura, con la misma paleta de la demo: tierra roja `#b5532f`, hierba `#6fb84a`, piedra `#9a9389`, borde tallado (×0,8 en el borde y ×0,93 en el segundo píxel), etc.
  - Si existe `assets/textures/<tile>.png` con 16×16, lo usa en su lugar. Esto prepara los TexturePacks de la etapa 5.
  - Crea un `Image` con `texture_view_dimension = D2Array` y filtro nearest.
- **Shader (`blocks.wgsl`):**
  - color = textura × AO × max(cielo×luz_del_sol, bloque×cálida);
  - niebla por distancia con el color del cielo;
  - descarte alfa en cutout.
- **Transmisión:**
  - Cada frame, pide los chunks dentro de `ViewDistance` en espiral desde el jugador.
  - Cada chunk se carga o genera, se ilumina y se malla en `AsyncComputeTaskPool`, con un máximo de 8 tareas en vuelo.
  - Se descargan (y guardan si están `dirty`) los chunks que pasan de `ViewDistance + 2`.
- **Origen flotante:** cuando el jugador se aleja más de 2 chunks de `FloatingOrigin`, se mueve el origen y se recolocan todas las entidades. Las `Transform` siempre son relativas al origen.

- [ ] **Step 1: Write the failing tests** (lógica pura de `streaming.rs`, sin App):
  - `spiral_order_nearest_first`: `spiral(center, r)` empieza en el centro y tiene `(2r+1)^2` posiciones sin repetir.
  - `unload_set_beyond_margin`.
  - `floating_origin_rebase`: `to_render(BlockPos{x:29_999_000,..}, origin)` devuelve un `Vec3` con |x| < 64.
- [ ] **Step 2:** Run `cargo test -p survivalforest-client`. Expected: FAIL.
- [ ] **Step 3: Implement** la App:
  - ventana con el título «SurvivalForest»;
  - un mundo temporal de semilla fija, con una cámara libre provisional;
  - los sistemas de transmisión y render.
- [ ] **Step 4:** Run `cargo test -p survivalforest-client`. Expected: PASS. Run `cargo run -p survivalforest-client --release`. Expected: se ve terreno con biomas y agua hasta el horizonte, sin huecos en los bordes de chunk.
- [ ] **Step 5:** Añade la opción `--smoke <segundos>`: arranca un mundo nuevo, espera, guarda `smoke.png` con la captura de pantalla de Bevy y sale con código 0. En el CI de Linux, ejecútala con `xvfb-run` (instala `xvfb` y `mesa-vulkan-drivers`) y sube `smoke.png` como artefacto.
- [ ] **Step 6:** Commit `feat(client): ventana, texturas, transmisión de chunks y origen flotante`.

### Task 9: Jugador, física y modos

**Files:**
- Create: `crates/survivalforest-sim/{Cargo.toml,src/lib.rs,src/physics.rs}`, `crates/survivalforest-client/src/player.rs`

**Interfaces:**
- Produces:
  - `pub struct Aabb { pub min: DVec3, pub max: DVec3 }`. El jugador mide 0,6 × 1,8 × 0,6 y los ojos están a 1,62.
  - `pub fn move_and_collide(world: &impl ChunkAccess, aabb: Aabb, vel: DVec3) -> (Aabb, DVec3 /*vel resultante*/, bool /*en_suelo*/)`. Resuelve los ejes en orden Y, X, Z contra los bloques `solid`; un chunk no cargado cuenta como sólido.
  - Constantes en `physics.rs`:
    - gravedad 32 bloques/s²;
    - salto 9 bloques/s;
    - andar 4,3 bloques/s; correr (Ctrl) 5,6 bloques/s;
    - vuelo creativo 11 bloques/s; vuelo esprintando 22 bloques/s.
  - `#[derive(Component)] pub struct Player { pub vel: DVec3, pub on_ground: bool, pub flying: bool, pub yaw: f32, pub pitch: f32 }`. Las posiciones se guardan en `f64` dentro de un componente `WorldPos(DVec3)`.
- **Controles:**
  - WASD, espacio, ratón capturado;
  - en creativo, doble espacio alterna el vuelo; Shift baja volando;
  - en supervivencia no se puede volar.
- La física corre en `FixedUpdate` a 20 Hz, con interpolación visual de la cámara.

- [ ] **Step 1: Write the failing tests** (en sim):
  - `falls_and_lands_on_floor`.
  - `cannot_walk_through_wall`.
  - `steps_into_one_block_gap_without_jitter`.
  - `unloaded_chunk_is_solid`.
  - `diagonal_corner_does_not_tunnel`: una velocidad de 30 bloques/s contra una esquina no atraviesa.
- [ ] **Step 2:** Run `cargo test -p survivalforest-sim`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Expected: PASS. Manual:
  - en supervivencia se anda, se salta y no se vuela;
  - en creativo, el doble espacio vuela.
- [ ] **Step 5:** Commit `feat: jugador con física y modos supervivencia/creativo`.

### Task 10: Romper y poner bloques

**Files:**
- Create: `crates/survivalforest-sim/src/{raycast.rs,edit.rs}`, `crates/survivalforest-client/src/interact.rs`

**Interfaces:**
- Produces:
  - `pub struct Hit { pub block: BlockPos, pub face: IVec3, pub dist: f64 }`.
  - `pub fn raycast(world: &impl ChunkAccess, origin: DVec3, dir: DVec3, max: f64) -> Option<Hit>`, con DDA voxel. Alcance: 5 en supervivencia y 6 en creativo.
  - `pub enum EditError { OutOfWorld, Unbreakable, InsidePlayer, Occupied }`.
  - `pub fn can_break(world, pos) -> Result<(), EditError>`.
  - `pub fn can_place(world, pos, player: &Aabb) -> Result<(), EditError>`. Se rechaza si y ∉ [MIN_Y, MAX_Y), si el bloque se solapa con la caja del jugador, o si en la posición hay algo distinto de aire o agua.
- **Cliente:**
  - clic izquierdo rompe; en creativo es instantáneo, y en supervivencia esta etapa también es instantánea (los tiempos de minado llegan en la etapa 2);
  - clic derecho pone el bloque seleccionado;
  - contorno del bloque apuntado.
  - Cada cambio usa `set_block`, que guarda y relanza la luz con `relight_around` y emite `RemeshRequest` del chunk y de los vecinos si x o z locales es 0 o 15.

- [ ] **Step 1: Write the failing tests:**
  - `ray_hits_first_solid_and_face`.
  - `ray_ignores_water_and_air`.
  - `ray_across_negative_coords`.
  - `cannot_place_inside_player`.
  - `cannot_place_above_world_top` (y=320) and `cannot_place_below_bottom`.
  - `cannot_break_bedrock`.
  - `edge_edit_requests_neighbor_remesh`: en el cliente, editar el bloque local x=0 emite `RemeshRequest` para el chunk y para su vecino oeste. Pruébalo con una función pura `remesh_targets(pos) -> Vec<ChunkPos>`.
- [ ] **Step 2:** Run `cargo test -p survivalforest-sim -p survivalforest-client`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Expected: PASS. Manual: romper en un borde de chunk no deja caras abiertas, y poner una antorcha en una cueva la ilumina.
- [ ] **Step 5:** Commit `feat: romper y poner bloques con remallado de vecinos`.

### Task 11: Ciclo de día y noche

**Files:**
- Create: `crates/survivalforest-sim/src/time.rs`, `crates/survivalforest-client/src/render.rs` (cielo), `assets/shaders/sky.wgsl`

**Interfaces:**
- Produces:
  - `pub struct WorldTime { pub ticks: u64 }`, con:
    - `fn day_fraction(&self) -> f32`: 0 = amanecer, 0,25 = mediodía, 0,5 = atardecer, 0,75 = medianoche;
    - `fn sun_dir(&self) -> Vec3`;
    - `fn sun_light(&self) -> f32`: 1 de día, 0,15 de noche, con transición suave de unos 1 200 ticks;
    - `fn sky_colors(&self) -> (Color /*cénit*/, Color /*horizonte*/)`.
  - Constante `DAY_TICKS: u64 = 24_000`.
- **Paleta del cielo:**
  - día: cénit `#5ea9e6`, horizonte `#d7edf6`;
  - atardecer: horizonte `#f0a868`, cénit `#6a7fc0`;
  - noche: cénit `#0b1230`, horizonte `#1d2a4a`.
- **Cielo:** cúpula con degradado, disco del sol, luna, estrellas de noche y nubes de bloques que se mueven.
- El uniforme `sun_light` alimenta `blocks.wgsl`. La niebla toma el color del horizonte.
- El tiempo avanza en `FixedUpdate` y se guarda en `WorldMeta.time_ticks`.

- [ ] **Step 1: Write the failing tests:**
  - `day_has_24000_ticks`.
  - `noon_is_brightest_midnight_darkest`.
  - `sun_light_continuous`: el salto entre ticks consecutivos es < 0,01.
  - `sky_color_at_noon_matches_palette`.
- [ ] **Step 2:** Run `cargo test -p survivalforest-sim time`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Expected: PASS. Manual: con F3+T a velocidad ×100, el sol sale, se pone y la noche oscurece el terreno y la niebla.
- [ ] **Step 5:** Commit `feat: ciclo de día y noche con cielo y niebla`.

### Task 12: Menús, interfaz y ajustes

**Files:**
- Create: `crates/survivalforest-client/src/{menu.rs,hud.rs,settings.rs}`

**Interfaces:**
- Consumes: `create_world`, `list_worlds`, `load_meta`/`save_meta`, `save_player`/`load_player`, `AppState`.
- Produces: `#[derive(Resource, Serialize, Deserialize)] pub struct Settings { pub view_distance: u32 /*4..=32, por defecto 12*/, pub fov: f32 /*60..=110, 75*/, pub sensitivity: f32, pub language: String /*"es"*/ }`, guardado en RON junto a `saves`.
- **Pantallas (UI de Bevy, textos en español, estilo de la demo con tarjetas oscuras translúcidas y esquinas redondeadas):**
  - Título: Un jugador, Ajustes, Salir.
  - Un jugador: lista de mundos (nombre, modo, última partida), con Jugar, Crear mundo y Borrar (con confirmación).
  - Crear mundo: nombre, semilla (vacía = aleatoria; acepta un número o un texto, que se convierte con hash), modo Supervivencia/Creativo.
  - Cargando: barra con los chunks listos sobre los necesarios. Se entra al juego con el radio 2 alrededor del jugador ya mallado.
  - Pausa (Esc): Volver, Ajustes, Guardar y salir al título.
- **HUD:**
  - punto de mira;
  - en creativo, barra de 9 huecos redondos con los bloques de construcción (1–9 y la rueda del ratón);
  - F3 muestra coordenadas, chunk, bioma, FPS, chunks cargados y hora.
- **Aparición:** el primer lugar seco cercano a (0, 0) sobre la superficie.
- **Guardado:** autoguardado cada 60 s y al salir: chunks `dirty`, jugador y meta.

- [ ] **Step 1: Write the failing tests:**
  - `seed_from_text_is_stable` ("cafetal" siempre da el mismo u64).
  - `settings_clamp` (view_distance 99 → 32).
  - `spawn_point_is_dry_land` (con la semilla 42, el punto devuelto tiene un bloque sólido debajo y aire en los 2 de encima).
- [ ] **Step 2:** Run `cargo test -p survivalforest-client`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Expected: PASS. Manual:
  - crear un mundo, jugar, salir, volver a entrar y encontrar las mismas ediciones y la misma posición;
  - cambiar la distancia de visión aplica sin reiniciar.
- [ ] **Step 5:** Commit `feat: menús, HUD, ajustes y autoguardado`.

### Task 13: Cierre de etapa

**Files:**
- Modify: `README.md`, `.github/workflows/ci.yml`

- [ ] **Step 1:** Añade un job `build` en windows-latest y ubuntu-22.04 que ejecute `cargo build --release -p survivalforest-client` y suba el ejecutable con `assets/` como artefacto `survivalforest-<os>`. El empaquetado de versiones y el manifiesto para el Launcher son de la etapa 5.
- [ ] **Step 2:** README en español:
  - qué es;
  - controles;
  - cómo compilar;
  - estado (etapa 1 de 5);
  - licencia del código;
  - aviso de que todo el arte es propio;
  - `THIRD_PARTY_NOTICES.md` generado con `cargo about` o escrito a mano con las licencias de las dependencias.
- [ ] **Step 3:** Run:
  - `cargo fmt --check`;
  - `cargo clippy --workspace -- -D warnings`;
  - `cargo test --workspace`;
  - `cargo test --workspace --release`.

  Expected: todo PASS.
- [ ] **Step 4:** Ejecuta `--smoke 20` en Linux con xvfb. Expected: `smoke.png` muestra el terreno con cielo y no hay errores en el registro.
- [ ] **Step 5:** Commit `docs: README y CI de compilación de la etapa 1`. Push y PR a `main`.
