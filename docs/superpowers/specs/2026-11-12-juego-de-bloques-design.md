# Diseño: juego de bloques de CoffeeWorks Games

Nombre: **SurvivalForest** (elegido por el dueño). Antes de publicar conviene
comprobar en Steam, Google Play y Google que no lo use otro juego o marca.

## 1. Intención

Es el primer juego del apartado **Games** de CoffeeWorks: un juego de
supervivencia y construcción en un mundo de bloques, enorme y procedural,
escrito al 100 % en Rust.

- Se instala, actualiza y desinstala desde el Launcher, igual que las apps de
  la suite.
- Trae de serie herramientas que en otros juegos solo existen como mods:
  minimapa, panel de «qué estoy mirando» y lista de objetos con recetas.
- Es el juego que estrena las tres subcategorías de Games: Mods,
  TexturePacks y Shaders.

**Por qué:** el dueño quiere un juego con la profundidad del género (criaturas,
clima, hambre, crafteo, mundos gigantes) pero propio. Una copia de un juego
existente, aunque sea de código abierto, le daría problemas legales. La
referencia visual aprobada es la demo `docs/concepto-juego-bloques.html` del
repositorio CoffeWorks:
- tierra roja, cafetales y guaduales;
- casas encaladas;
- bloques con borde tallado;
- interfaz propia.

## 2. Objetivos

1. **Mundo procedural gigante.** Horizontalmente es prácticamente infinito, se
   genera a partir de una semilla y es determinista: la misma semilla da
   siempre el mismo mundo. Tiene biomas, montañas, cuevas, ríos, menas y
   estructuras.
2. **Construir y romper bloques**, con herramientas que se desgastan.
3. **Supervivencia:**
   - vida con corazones;
   - hambre con jamones;
   - daño por caída, ahogamiento y fuego;
   - muerte y reaparición.
4. **Inventario y crafteo:**
   - inventario del jugador con rejilla de 2×2;
   - mesa de trabajo de 3×3;
   - horno;
   - cofres;
   - libro de recetas.
5. **Criaturas variadas** (pacíficas, neutrales y hostiles), con IA, sonidos y
   animaciones (andar, atacar, comer, recibir daño, morir). Pueden reproducirse
   y dar objetos al morir.
6. **Clima y tiempo dinámicos:**
   - ciclo de día y noche con el sol y la luna moviéndose;
   - lluvia, tormentas con rayos y niebla de montaña;
   - el clima cambia la luz, el sonido y la aparición de criaturas.
7. **Herramientas integradas:**
   - **Minimapa** en una esquina. Con **M** se abre a pantalla completa, y
     admite marcadores con nombre, color e icono.
   - **Panel «qué estoy mirando»**: nombre del bloque o criatura, su vida,
     la herramienta adecuada y el mod de origen.
   - **Lista de objetos** a la derecha, con buscador. Al hacer clic en un
     objeto se ve su receta, y desde ahí también en qué recetas se usa.
8. **Multijugador por LAN:** un jugador abre su mundo a la red local y hasta
   **10 jugadores** en total juegan juntos.
9. **Modos de juego:**
   - **Supervivencia**: vida, hambre y recursos limitados.
   - **Creativo**: bloques infinitos, vuelo y sin daño.
   - El modo se elige al crear el mundo y se puede cambiar con un comando o
     desde el menú si el mundo lo permite.
10. **Integración con el Launcher** (Instalar, Jugar, Actualizar, Desinstalar)
   y con las subcategorías de Games.
11. **Paleta y estilo** a medio camino entre los colores saturados y alegres
    del género y un acabado más suave y pintado, con bordes tallados y luz
    cálida.

## 3. No objetivos (por ahora)

- **Compatibilidad con Minecraft:** ni su protocolo de red, ni sus mundos
  guardados, ni sus resource packs, mods o datos.
- **Consolas y móviles.** Las plataformas son Windows 10+ y Linux (Ubuntu 22.04+
  y Debian 12+).
- **Versión para navegador.**
- **Cuentas en la nube y tienda de pago.**
- **Juego por internet y servidor dedicado.** Por ahora solo LAN. La red se
  diseña para poder añadirlos después sin rehacerla.
- **Modo extremo** (una sola vida).
- **Final o jefe final.** Por ahora es un mundo abierto sin final.
- **Más dimensiones** (al estilo de un «infierno» o un «más allá»). Se dejan
  para una versión posterior.

## 4. Anti-objetivos (fracaso aunque funcione)

- **Que parezca Minecraft:** mismos nombres de bloques u objetos, mismas
  criaturas (creeper, enderman, zombi con su diseño…), mismas texturas o
  sprites, misma disposición exacta de la interfaz, mismos sonidos. Las
  **mecánicas del género** (corazones, hambre, crafteo en rejilla, día y
  noche) sí se permiten, pero con diseño, arte y valores propios.
- **Copiar código de otros proyectos** sin respetar su licencia. Horizon
  (MIT) solo se usa como **referencia técnica**. Si se reutiliza alguna pieza
  genérica, se conserva su aviso de copyright en `THIRD_PARTY_NOTICES.md`.
  Nunca se toma nada que reproduzca reglas, datos o protocolo de Minecraft.
- **Que no vaya fluido en un portátil normal.** Objetivo: 60 FPS con gráfica
  integrada a 12 chunks de distancia de visión.
- **Que dependa de internet para jugar en solitario.**

## 5. Restricciones

- **Lenguaje:** todo en Rust, incluidos el juego, el servidor, las herramientas
  y el CI.
- **Repositorio:** `ThiagoVelez/juego-rust`. El Launcher vive en
  `ThiagoVelez/CoffeWorks` y solo descarga el juego.
- **Peso:** decenas de MB por descarga (aceptado durante la Alpha de
  CoffeeWorks).
- **Licencias:**
  - todo el arte y el sonido son propios o CC0;
  - las dependencias deben tener licencias permisivas (MIT, Apache-2.0, BSD
    o Zlib);
  - nada GPL dentro del ejecutable.

## 6. Enfoques considerados

| Enfoque | Ventajas | Inconvenientes | Decisión |
|---|---|---|---|
| **A. Bevy + motor de bloques propio** | El ECS sirve de base para criaturas, IA, animación, sonido, interfaz, clima y red. Lo específico de los bloques es nuestro. | El ejecutable pesa más (40–80 MB) y Bevy rompe la API entre versiones. | **Elegido por el dueño.** |
| B. Motor propio sobre `wgpu` (como Horizon) | El más ligero y con control total. | Animación, sonido, interfaz, IA y red se hacen a mano: varias veces más trabajo. | Descartado |
| C. Godot con Rust (gdext) | Editor visual. | No es «todo en Rust», y el núcleo es C++. | Descartado |
| D. Fyrox | Rust con editor. | Ecosistema pequeño para red y criaturas. | Descartado |

## 7. Decisiones tomadas (por el dueño)

- **Género:** supervivencia y construcción en bloques, con mundo gigante
  procedural.
- **Contenido:**
  - criaturas variadas y animadas;
  - clima dinámico completo;
  - corazones y jamones, con arte propio;
  - crafteo;
  - minimapa con **M** y marcadores de color e icono;
  - panel de qué se está mirando;
  - lista de objetos con recetas.
- **Estilo:**
  - la dirección de la demo: paisaje de montaña cafetera, tierra roja,
    guadua, casas encaladas y bordes tallados;
  - paleta entre el género clásico y un acabado pintado más suave.
- **Motor:** Bevy (enfoque A), en un repositorio propio.
- **Instalación:** desde el Launcher, como las apps.
- **Orden de trabajo:** por etapas jugables (sección 9), empezando ya.
- **Modos:** supervivencia y creativo. Sin modo extremo.
- **Final:** ninguno por ahora; mundo abierto.
- **Multijugador:** solo LAN, como máximo 10 jugadores por mundo.
- **Criaturas:** animales reales de la región junto con criaturas de leyenda:
  - **Pacíficas:** vaca de páramo, gallina, cabra, guacamaya, tortuga.
  - **Neutrales:** oso de anteojos, jaguar, abejas.
  - **Hostiles:** fantasmas (de noche), wendigo (bosques y nevados, de noche),
    espantos del cafetal, arañas gigantes (cuevas).

  Los diseños, nombres en el juego, sonidos y comportamientos son propios.
  La lista se puede ampliar en cada etapa.

## 8. Arquitectura propuesta

Es una propuesta del diseñador, no una decisión del dueño, y se puede ajustar.

### 8.1 Workspace de Cargo

```
crates/
  survivalforest-core      tipos compartidos: coordenadas, bloques, objetos, recetas, ids
  survivalforest-world     chunks, generación procedural, iluminación, guardado
  survivalforest-sim       reglas del juego sin gráficos: física, criaturas, IA,
                    supervivencia, crafteo, clima. Corre igual en cliente y servidor
  survivalforest-net       protocolo propio y sincronización
  survivalforest-client    Bevy: render, mallas, animación, interfaz, sonido, entrada
  survivalforest-server    (futuro) servidor dedicado sin gráficos
  survivalforest-modapi    API de mods WebAssembly (wasmtime)
assets/             texturas, modelos (Blockbench → glTF), sonidos, datos (RON)
tools/              empaquetado de versiones y manifiesto para el Launcher
```

`survivalforest-sim` es independiente del render. Así el mismo código corre en el
modo de un jugador (servidor integrado), en LAN y en el servidor dedicado.

### 8.2 Mundo

- **Tamaño:**
  - columnas de chunks de 16×16 bloques, divididas en secciones de 16³;
  - altura de −64 a 320;
  - en horizontal, límite jugable de ±30 millones de bloques. El render usa
    coordenadas relativas al chunk para no perder precisión lejos del origen.
- **Generación:**
  - ruido determinista por semilla (continentalidad, erosión, temperatura,
    humedad y «rareza»);
  - biomas propios, cuevas de ruido 3D, ríos, menas por altura;
  - estructuras, como veredas con casas encaladas, ruinas y minas;
  - se genera en hilos aparte, por prioridad de cercanía.
- **Biomas iniciales (propuesta):**
  - montaña cafetera;
  - páramo con frailejones;
  - selva húmeda;
  - valle de guaduales;
  - sabana;
  - desierto rojo;
  - nevado;
  - costa y océano;
  - manglar;
  - bosque de niebla.
- **Mallas:**
  - mallado por caras visibles, con unión de caras iguales (greedy) donde no
    se pierda la oclusión ambiental;
  - mallas separadas para lo opaco, lo transparente y el agua;
  - niveles de detalle para los chunks lejanos.
- **Luz:**
  - luz de cielo y luz de bloque (antorchas, fuego, lava) propagadas por BFS
    por sección;
  - oclusión ambiental por vértice;
  - sombra del sol en los shaders opcionales.
- **Guardado:**
  - archivos de región propios (32×32 chunks), comprimidos con `zstd`;
  - guardado incremental, más los datos del jugador y del mundo en RON.

### 8.3 Simulación

- **Ritmo:** 20 ticks por segundo, con física de jugador y criaturas por cajas
  de colisión contra la malla de bloques.
- **Criaturas:**
  - ECS con componentes de IA: deambular, huir, perseguir, atacar a distancia,
    reproducirse, tentarse con comida;
  - búsqueda de caminos A* sobre bloques;
  - aparición según bioma, luz, hora y clima.
- **Supervivencia:**
  - vida (20 puntos = 10 corazones) y hambre (20 puntos = 10 jamones), con
    saturación;
  - el hambre baja con la actividad, y la vida se regenera si el hambre está
    alta;
  - todos los valores son propios y configurables en datos.
- **Crafteo:** recetas en datos (RON) con o sin forma, horno con combustible y
  progreso, y libro de recetas. Los mods pueden añadir recetas.
- **Clima:**
  - estado del mundo con duración aleatoria;
  - la lluvia se convierte en nieve según el bioma;
  - los rayos pueden prender fuego;
  - el día completo dura 20 minutos (ajustable).

### 8.4 Cliente (Bevy)

- **Render:**
  - texturas de bloques de 16×16 con borde tallado, en un array de texturas;
  - modelos de criaturas creados en Blockbench y exportados a glTF, con
    animación por esqueleto;
  - partículas;
  - cielo con sol, luna, estrellas y nubes;
  - niebla por distancia.
- **Interfaz:**
  - barra de objetos, corazones y jamones, con iconos dibujados por nosotros;
  - inventario, mesa de trabajo, horno y cofres;
  - minimapa: la textura se genera a partir de los colores de la superficie
    de los chunks cargados, y se guarda en caché;
  - panel «qué estoy mirando», con un raycast cada frame;
  - lista de objetos con su receta.
- **Ajustes:**
  - distancia de visión, campo de visión, sensibilidad, controles, sonido,
    idioma y mando;
  - textos en español primero e inglés después.

### 8.5 Red

- **Alcance:** solo LAN, con un máximo de 10 jugadores.
- **Modelo:** servidor con autoridad. Quien abre el mundo ejecuta el servidor
  integrado en su propio juego. Un jugador solo usa ese mismo servidor
  integrado, sin red.
- **Transporte:** UDP fiable y no fiable (por ejemplo, `renet` o `lightyear`;
  lo decide el constructor).
- **Contenido de la red:** se envían los chunks comprimidos y las entidades se
  interpolan.
- **LAN:** se anuncia por broadcast, y la lista de partidas se ve en
  «Multijugador».
- **Futuro, no en este alcance:** servidor dedicado (`survivalforest-server`) y juego
  por internet. El protocolo y `survivalforest-sim` no deben impedirlo.

### 8.6 Contenido extensible (Games)

- **TexturePacks:** `.zip` con `pack.ron` y PNG que sustituyen texturas
  por nombre.
- **Shaders:** packs WGSL que reemplazan pasos del render, como sombras, agua,
  cielo o posprocesado.
- **Mods:** módulos WebAssembly que se ejecutan con `wasmtime` en un sandbox.
  - Pueden registrar bloques, objetos, recetas y criaturas, y escuchar
    eventos.
  - No tienen acceso a archivos ni a la red.

### 8.7 Integración con el Launcher (repo CoffeWorks)

- **Release del juego:** cada versión publica en GitHub Releases:
  - `survivalforest-<v>-windows-x64.zip`;
  - `survivalforest-<v>-linux-x64.tar.gz`;
  - `manifest.json` con la versión, los archivos, el tamaño, el SHA-256 y la
    versión mínima del Launcher.
- **Launcher:**
  - lee el manifiesto;
  - instala en `%LOCALAPPDATA%\CoffeeWorks\Games\survivalforest` o en
    `~/.local/share/CoffeeWorks/Games/survivalforest`;
  - verifica el SHA-256 y crea accesos directos;
  - ofrece Jugar, Actualizar y Desinstalar, y lista los Mods, TexturePacks y
    Shaders del juego.

## 9. Etapas

Cada etapa termina en una versión jugable y con CI en verde.

1. **Mundo:**
   - motor de chunks, generación procedural con biomas, cuevas y menas;
   - mallado, luz y guardado;
   - cámara en primera persona, moverse y saltar;
   - modo creativo (volar, bloques infinitos) para probar el mundo desde el
     principio;
   - romper y poner bloques;
   - ciclo de día y noche.
2. **Supervivencia:**
   - vida y hambre (corazones y jamones), daño y muerte;
   - inventario, barra de objetos, herramientas y desgaste;
   - crafteo (2×2, 3×3 y horno), cofres;
   - comida.
3. **Vida y clima:**
   - criaturas pacíficas, neutrales y hostiles con animaciones, IA,
     aparición, botín y reproducción;
   - lluvia, tormentas, nieve y niebla;
   - estructuras (veredas).
4. **Herramientas y multijugador:**
   - minimapa con **M** y marcadores;
   - panel de qué se mira;
   - lista de objetos con recetas;
   - multijugador por LAN (hasta 10 jugadores).
5. **Plataforma:**
   - publicación de versiones y manifiesto;
   - integración en el Launcher;
   - TexturePacks, Shaders y Mods (WASM);
   - pulido y ajustes.

## 10. Queda a criterio del constructor

- Las librerías concretas de red, ruido, compresión y pathfinding, dentro de
  las licencias permitidas.
- Los parámetros exactos de generación, los valores de daño, el hambre y los
  tiempos, siempre que estén en archivos de datos y se puedan ajustar.
- El formato interno de guardado y del protocolo, con versión para poder
  migrarlos.
- La lista final de bloques, objetos y recetas de cada etapa, con nombres y
  arte propios.
- La estrategia de pruebas:
  - determinismo de la generación;
  - mallado;
  - crafteo;
  - serialización;
  - un modo de prueba sin ventana para CI.

## 11. Decisiones del diseñador para revisar

- Altura del mundo de −64 a 320 y límite horizontal de ±30 millones de
  bloques.
- Día de 20 minutos, 20 ticks por segundo, y vida y hambre de 20 puntos.
- Biomas iniciales de la sección 8.2.
- Texturas de 16×16 con borde tallado. Modelos de criaturas hechos en
  Blockbench.
