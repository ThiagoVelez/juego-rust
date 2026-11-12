# SurvivalForest

Juego de supervivencia y construcción en un mundo de bloques, enorme y procedural, escrito al 100 % en Rust con [Bevy](https://bevy.org). Es el primer juego del apartado **Games** de CoffeeWorks.

> Estado: **etapa 1 de 5** (mundo). Vida, hambre, inventario y crafteo llegan en la etapa 2; criaturas y clima en la 3; minimapa, panel de información, lista de recetas y LAN en la 4; integración con el Launcher, TexturePacks, Shaders y Mods en la 5. El diseño completo está en [`docs/superpowers/specs`](docs/superpowers/specs/2026-11-12-juego-de-bloques-design.md).

## Qué hay en la etapa 1

- **Mundo procedural prácticamente infinito** (±30 millones de bloques, altura de −64 a 320), determinista por semilla, con 11 biomas: montaña cafetera, páramo, selva húmeda, valle de guaduales, sabana, desierto rojo, nevado, costa, océano, manglar y bosque de niebla.
- Cuevas, menas (carbón, cobre, hierro, oro y cuarzo), árboles, guaduales, cafetales en hileras y frailejones.
- Texturas propias de 32×32 (los TexturePacks pueden ser de 32×32 o de 16×16, que se amplía sin suavizar).
- Luz de cielo y de antorchas, oclusión ambiental y niebla.
- **Modos supervivencia y creativo** (en creativo, doble espacio para volar).
- Romper y poner bloques.
- **Ciclo de día y noche** (20 minutos) con sol, luna, estrellas y nubes.
- Menús en español: crear mundo (nombre, semilla y modo), lista de mundos, ajustes y pausa. Autoguardado cada minuto y al salir.

## Controles

| Tecla | Acción |
|---|---|
| W A S D | Moverse |
| Ratón | Mirar |
| Espacio | Saltar (doble espacio en creativo: volar) |
| Shift | Bajar volando |
| Ctrl | Correr |
| Clic izquierdo / derecho | Romper / poner bloque |
| 1–9, rueda | Elegir bloque (creativo) |
| Esc | Pausa |
| F3 | Información (posición, bioma, FPS, hora) |
| F3 + T | Tiempo ×100 |

## Compilar y jugar

Requisitos: Rust 1.97 (lo fija `rust-toolchain.toml`). En Linux, además:

```bash
sudo apt-get install -y libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
```

```bash
cargo run --release -p survivalforest-client
```

Las partidas se guardan en `%APPDATA%\SurvivalForest\saves` (Windows) o `~/.local/share/SurvivalForest/saves` (Linux).

Pruebas:

```bash
cargo test --workspace
```

Prueba de arranque (crea un mundo temporal, entra, guarda `smoke.png` y sale):

```bash
cargo run -p survivalforest-client -- --smoke 30
```

## Estructura

| Crate | Qué hace |
|---|---|
| `survivalforest-core` | Coordenadas, bloques y modos de juego |
| `survivalforest-world` | Chunks, generación procedural, luz, mallado y guardado |
| `survivalforest-sim` | Física, rayos, reglas de edición y hora del día (sin gráficos) |
| `survivalforest-client` | El juego con Bevy: render, jugador, menús e interfaz |

## Licencias

El código es MIT. Todo el arte (texturas de 32×32 pintadas por código, cielo y nubes) es propio; la fuente Nunito usa la licencia SIL OFL 1.1 (`assets/fonts/OFL-Nunito.txt`). Las licencias de las dependencias están en [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

SurvivalForest es un juego original: no usa nombres, texturas, sonidos ni código de otros juegos.
