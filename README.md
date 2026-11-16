# SurvivalForest

Juego de supervivencia y construcción en un mundo de bloques, enorme y procedural, escrito al 100 % en Rust con [Bevy](https://bevy.org). Es el primer juego del apartado **Games** de CoffeeWorks.

> Estado: **etapa 2 de 5** (supervivencia), versión 0.2.0. Criaturas y clima llegan en la etapa 3; minimapa, panel de información, lista de recetas y LAN en la 4; integración con el Launcher, TexturePacks, Shaders y Mods en la 5. El diseño completo está en [`docs/superpowers/specs`](docs/superpowers/specs/2026-11-12-juego-de-bloques-design.md).

## Qué hay en la etapa 2 (supervivencia)

- **Vida y hambre**: 10 corazones y 10 jamones. La vida se regenera con el hambre alta; con el hambre a cero baja hasta medio corazón. Daño por caída (más de 3 bloques) y por ahogarse (10 burbujas de aire). Se puede **nadar** (Espacio sube).
- **Morir**: todo el inventario cae donde moriste y dura 10 minutos; la mitad de la experiencia queda en un orbe. Se reaparece en el punto de aparición.
- **Inventario** de 36 casillas (barra de 9 + mochila) con crafteo de 2×2; **mesa de trabajo** de 3×3, **horno de barro** con combustible y **baúl** de 27 casillas.
- **Herramientas**: pico, hacha, pala y machete en madera, piedra, cobre y hierro. Cada bloque tarda según su dureza y la herramienta; las menas piden un pico mínimo (carbón y cobre: madera; hierro y cuarzo: piedra; oro: cobre) y la piedra necesita al menos un pico de madera. Las herramientas se gastan y se rompen.
- **Experiencia**: picando menas, fundiendo y haciendo cada receta por primera vez; la barra muestra el nivel en un grano de café. La **piedra de amolar** gasta niveles para dejar una herramienta como nueva.
- **Comida**: plátano (plataneras de las zonas cálidas), guayaba (guayabos de sabana y guadual), mora (arbustos de páramo y bosque de niebla) y plátano asado en el horno.
- **Recetas propias** (`crates/survivalforest-core/data/recipes.ron`): una prueba comprueba que ninguna repite la forma de la receta de otro juego. Todos los valores (vida, hambre, dureza, experiencia, tiempos) están en archivos de datos RON.
- Los mundos de la etapa 1 se abren sin perder nada (empiezan con el inventario vacío).

## Qué hay en la etapa 1 (mundo)

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
| Clic izquierdo (mantener) | Romper |
| Clic derecho | Poner bloque, abrir mesa / horno / baúl / piedra de amolar, comer (mantener) |
| Shift + clic derecho | Poner un bloque encima de una mesa, horno o baúl |
| 1–9, rueda | Elegir casilla de la barra |
| E | Inventario (en creativo: todos los objetos) |
| Q / Ctrl+Q | Tirar uno / tirar la pila |
| En las pantallas | Clic: tomar o dejar · clic derecho: la mitad o uno · Shift+clic: mover entre partes |
| Esc | Cerrar la pantalla abierta o pausa |
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

Con `--shot-screen <hud|inventario|mesa|horno|baul|amoladora|muerte|creativo>` abre esa pantalla con objetos de ejemplo antes de la captura.

## Estructura

| Crate | Qué hace |
|---|---|
| `survivalforest-core` | Coordenadas, bloques, objetos, recetas y valores (datos en `data/*.ron`) |
| `survivalforest-world` | Chunks, generación procedural, luz, mallado y guardado |
| `survivalforest-sim` | Física, nado, rayos, edición, hora del día e inventario, crafteo, horno, vida, hambre, experiencia, objetos y guardado de la supervivencia (sin gráficos) |
| `survivalforest-client` | El juego con Bevy: render, jugador, menús, HUD y pantallas |

## Licencias

El código es MIT. Todo el arte (texturas de 32×32 pintadas por código, cielo y nubes) es propio; la fuente Nunito usa la licencia SIL OFL 1.1 (`assets/fonts/OFL-Nunito.txt`). Las licencias de las dependencias están en [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

SurvivalForest es un juego original: no usa nombres, texturas, sonidos ni código de otros juegos.
