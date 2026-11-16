# SurvivalForest · Etapa 2 (Supervivencia): diseño

Fecha: 2026-11-16 · Estado: pendiente de aprobación del dueño

Diseño general del juego: `docs/superpowers/specs/2026-11-12-juego-de-bloques-design.md` (sección 9, «Etapas»).

## Para qué

La etapa 1 dejó un mundo para explorar y construir. La etapa 2 lo convierte en un juego de **supervivencia**: el jugador empieza con las manos vacías, tiene que conseguir comida y herramientas, y morir tiene consecuencias.

Al terminar, la versión jugable permite:

- crear un mundo en supervivencia;
- talar con la mano y hacer tablones, una mesa de trabajo y herramientas;
- picar piedra y menas;
- fundir en el horno, guardar en un baúl y comer;
- reparar las herramientas con la experiencia;
- morir y volver a por las cosas.

## Decisiones del dueño

1. Contenido con sabor de la región:
   - herramientas: pico, hacha, pala y **machete**, en madera, piedra, cobre y hierro;
   - bloques: mesa de trabajo, **horno de barro**, **baúl**, piedra partida, ladrillo y vidrio;
   - comida de plantas: **plátano** de plataneras, **guayaba** y **mora**, y **plátano asado** en el horno.
2. **Al morir, todos los objetos caen donde murió el jugador**, que tiene unos minutos para volver por ellos.
3. **Barra de experiencia** con mecánica del género y diseño propio.
4. La experiencia sirve para **reparar herramientas en la piedra de amolar**. Las mejoras para herramientas quedan para una etapa posterior.
5. Arquitectura:
   - los datos (objetos, recetas, comida y valores) van en archivos;
   - las reglas van en la simulación, sin depender de Bevy;
   - el cliente solo dibuja y manda acciones.

## Objetivos

1. **Vida:** 10 corazones (20 puntos).
2. **Daño:** por caída y por ahogamiento.
3. **Hambre:** 10 jamones (20 puntos) que bajan con el tiempo y el esfuerzo. Con el hambre alta la vida se regenera; con el hambre a cero la vida baja.
4. **Muerte:** pantalla de muerte, reaparición en el punto de aparición del mundo y objetos caídos.
5. **Objetos:** romper un bloque suelta un objeto en el suelo que se recoge al pasar cerca. **Q** tira el objeto de la mano.
6. **Inventario:** 9 casillas de barra más 27 de mochila, y rejilla de crafteo de 2×2.
7. **Crafteo:** mesa de 3×3, horno con combustible y baúl de 27 casillas.
8. **Herramientas:**
   - cada bloque tarda en romperse según su dureza y la herramienta usada;
   - las menas exigen un pico de nivel mínimo;
   - cada uso gasta durabilidad, y la herramienta se rompe al llegar a cero.
9. **Experiencia:**
   - se gana picando menas, fundiendo y crafteando cada receta por primera vez;
   - la barra muestra el nivel;
   - la piedra de amolar gasta niveles para reparar.
10. **Plantas nuevas** en la generación del mundo: platanera, arbusto de mora y guayabo. Son deterministas con la semilla, como el resto del mundo.
11. **Guardado:** se guardan el inventario, la vida, el hambre, la experiencia y el contenido de baúles y hornos. Los mundos de la etapa 1 se abren sin perder nada.
12. **Modo creativo:** sin daño ni hambre. El inventario muestra todos los objetos para tomarlos.

## No objetivos (llegan en otras etapas)

- Criaturas, carne, combate contra criaturas y botín: etapa 3. El machete ya existe y su daño queda definido para entonces.
- Daño por fuego y lava, camas, cultivos y armaduras.
- Mejoras para herramientas.
- Minimapa, panel de qué se mira y lista de recetas: etapa 4.
- Multijugador: etapa 4. Las reglas se escriben pensando en que luego las ejecute el servidor.

## Anti-objetivos

- **Que se parezca a Minecraft:**
  - mismos nombres, iconos o texturas;
  - la misma disposición exacta de la interfaz;
  - **las mismas formas de receta** que en Minecraft.

  Las recetas tienen formas propias (ver «Recetas»).
- **Que un mundo de la etapa 1 deje de abrir** o pierda bloques.
- **Que las reglas vivan en el cliente**, porque luego no servirían para el multijugador.
- **Que baje el rendimiento:** el objetivo sigue siendo 60 FPS con gráfica integrada a 12 chunks.

## Restricciones

- Todo en Rust con Bevy 0.20, en el repositorio `ThiagoVelez/juego-rust`.
- Arte propio pintado por código, como las texturas actuales en `textures.rs`. La paleta es la del juego: tierra roja, verdes cálidos y bordes tallados.
- Los valores ajustables van en archivos de datos RON dentro de `survivalforest-core`, cargados con `include_str!` para que funcionen en las pruebas y en el juego. Los mods de la etapa 5 podrán añadir más.
- CI actual en verde: `cargo fmt`, `clippy -D warnings`, `cargo test --workspace` y la prueba de humo del cliente con xvfb.

## Enfoques considerados

- **A. Datos en RON y reglas en la simulación** (elegido por el dueño): inventario, crafteo, vida, hambre, daño, horno, baúl y experiencia son código puro en `survivalforest-sim`, con pruebas. El cliente traduce teclas y ratón en acciones y dibuja el estado. Sirve tal cual para el servidor LAN de la etapa 4.
- **B. Todo como sistemas de Bevy en el cliente.** Es más rápido de empezar, pero difícil de probar y habría que rehacerlo para el multijugador. Descartado.

## Diseño

### Datos (`survivalforest-core`)

- **`ItemId` y registro de objetos** (`items.ron`):
  - todos los bloques que se pueden poner son también objetos;
  - además hay herramientas, materiales y comida;
  - cada objeto tiene clave, nombre en español, tamaño de pila (64; las herramientas, 1), textura o icono, y según su tipo: bloque que coloca, herramienta (clase, nivel, durabilidad y velocidad), comida (hambre y saciedad) y combustible (segundos).
- **Recetas** (`recipes.ron`):
  - con forma (rejilla) y sin forma;
  - las de 2×2 se hacen en el inventario y en la mesa, y las de 3×3 solo en la mesa;
  - las recetas del horno van aparte: entrada, salida, tiempo y experiencia.
- **Valores** (`tuning.ron`):
  - vida y hambre máximas;
  - ritmos de hambre;
  - daño por caída;
  - aire bajo el agua;
  - curva de experiencia;
  - tiempo que duran los objetos caídos;
  - coste de la piedra de amolar.
- **Bloques nuevos** en el registro actual, con texturas nuevas en `TILE_NAMES`:
  - mesa de trabajo, horno de barro (apagado y encendido; el encendido da luz 13), baúl y piedra de amolar;
  - piedra partida, ladrillo y vidrio (no opaco);
  - platanera (planta, no sólida), arbusto de mora y hojas de guayabo.

  Los ids existentes no cambian; los nuevos van al final.
- **Dureza y herramienta adecuada por bloque**: forman parte de los datos del bloque.

### Contenido

| Grupo | Objetos |
|---|---|
| Herramientas | pico, hacha, pala y machete, en madera, piedra, cobre y hierro (16 en total) |
| Materiales | vara, carbón, carbón vegetal, mena de cobre, mena de hierro, mena de oro (se funden), lingote de cobre, lingote de hierro, lingote de oro, cuarzo, ladrillo (objeto), guadua |
| Comida | plátano (3 jamones de hambre), plátano asado (6), guayaba (2), mora (1, se come rápido) |
| Bloques nuevos | mesa de trabajo, horno de barro, baúl, piedra de amolar, piedra partida, bloque de ladrillo, vidrio |

**Niveles de herramienta** (valores iniciales, ajustables en datos):

| Material | Nivel | Durabilidad | Velocidad |
|---|---|---|---|
| Madera | 1 | 60 | ×2 |
| Piedra | 2 | 130 | ×4 |
| Cobre | 3 | 190 | ×5 |
| Hierro | 4 | 260 | ×6 |

**Nivel de pico mínimo por mena:**
- carbón y cobre: madera;
- hierro y cuarzo: piedra;
- oro: cobre.

Sin el pico necesario, la mena se rompe muy despacio y no suelta nada.

**Piedra a mano** (decisión de Claude): la piedra y la piedra partida también necesitan al menos un pico de madera para soltar algo. Así el camino es mano → madera → piedra → cobre → hierro. El resto de bloques (tierra, arena, madera, hojas, plantas…) se recogen a mano, solo que más despacio.

**Qué suelta cada bloque al romperse:**

| Bloque | Suelta |
|---|---|
| Piedra | piedra partida |
| Hierba | tierra roja |
| Mena de carbón | carbón |
| Mena de cuarzo | cuarzo |
| Mena de cobre, de hierro y de oro | su mena, que se funde en el horno |
| Hojas | a veces una vara |
| Hojas de guayabo | a veces una guayaba |
| Platanera | 1–3 plátanos |
| Arbusto de mora | 1–2 moras |
| Cafeto | nada nuevo en esta etapa |
| Resto | el propio bloque |

### Recetas

Las recetas usan **formas propias**: ninguna repite la forma de la receta equivalente de Minecraft. La lista exacta y sus formas las decide quien construye, siempre con esta regla y con estos ingredientes:

| Qué se hace | Con qué |
|---|---|
| Tablones | tronco (2×2) |
| Varas | tablones (2×2) |
| Mesa de trabajo | tablones (2×2) |
| Antorchas | carbón o carbón vegetal y vara (2×2) |
| Herramientas | 2 o 3 del material y varas (mesa) |
| Horno de barro | arcilla y piedra partida (mesa) |
| Baúl | tablones y un lingote de cobre como cierre (mesa) |
| Piedra de amolar | piedra partida, cuarzo y varas (mesa) |
| Bloque de ladrillo | 4 ladrillos |
| Teja y pared encalada | arcilla y arena (para construir casas como las de la demo) |

**Horno:**

| Entra | Sale |
|---|---|
| Menas | lingotes |
| Arena y arena roja | vidrio |
| Arcilla | ladrillo |
| Plátano | plátano asado |
| Tronco | carbón vegetal |
| Piedra partida | piedra |

Cada fundición tarda 8 s. Combustibles:
- carbón y carbón vegetal: 8 fundiciones;
- tronco y tablones: 1,5;
- vara y guadua: 0,5.

### Reglas (`survivalforest-sim`)

- **`Inventory`:**
  - 36 casillas;
  - operaciones de las casillas: tomar, dejar, partir pilas y mover con Mayús;
  - añadir, que llena primero las pilas existentes;
  - soltar y vaciar.
- **`Crafting`:** busca la receta de una rejilla de 2×2 o de 3×3, ignorando el espacio vacío alrededor, y la ejecuta.
- **`Furnace`:**
  - estado con entrada, combustible, salida, combustible restante y progreso;
  - avanza con `tick(dt)` y da experiencia al sacar el resultado.
- **`Chest`:** 27 casillas.
- **`Vitals`:**
  - **Vida.**
  - **Hambre y saciedad:** el hambre baja con el tiempo, más rápido al correr y al saltar.
  - **Regeneración:** si el hambre es de 18 o más, se recupera medio corazón cada 4 s.
  - **Hambre a cero:** quita medio corazón cada 4 s, pero no mata; deja medio corazón (decisión de Claude).
  - **Caída:** quita la distancia de la caída menos 3 bloques, en puntos de vida; caer en agua no hace daño.
  - **Aire bajo el agua:** 10 burbujas (15 s); después, medio corazón por segundo.
- **`Experience`:**
  - puntos y nivel, con la curva propia definida en los datos;
  - fuentes:
    - menas: carbón 1, cobre 2, hierro y cuarzo 3, oro 5;
    - horno: según la receta;
    - primera vez que se craftea cada receta: 2.
- **Piedra de amolar:**
  - coste en niveles = redondeo hacia arriba de (desgaste de la herramienta × coste base de su material);
  - coste base: madera 1, piedra 2, cobre 3, hierro 4;
  - deja la herramienta como nueva;
  - sin niveles suficientes no deja repararla y lo dice.
- **Nadar** (decisión de Claude: sin esto el agua sería una trampa mortal, porque en la etapa 1 el jugador se hunde sin poder subir):
  - en el agua el jugador se hunde despacio y Espacio lo hace subir;
  - se mueve más lento y puede salir a una orilla de un bloque de alto.
- **Romper bloques:**
  - el tiempo sale de la dureza, la herramienta, si es la adecuada y su nivel;
  - mantener el clic avanza el progreso, y soltarlo o cambiar de bloque lo reinicia;
  - al terminar, gasta 1 de durabilidad si se usó una herramienta adecuada (2 si no lo era), y suelta el objeto y la experiencia.
- **Objetos en el suelo:**
  - posición, velocidad y gravedad sencillas (usan las colisiones actuales);
  - se recogen solos a 1,5 bloques y se juntan con otros iguales cercanos;
  - desaparecen a los 5 minutos;
  - los de la muerte duran 10 minutos.
- **Muerte:**
  - todo el inventario cae en el sitio de la muerte;
  - la mitad de los puntos de experiencia queda en un orbe que se recoge como los objetos; la otra mitad se pierde;
  - se reaparece con vida y hambre al máximo.
- **Acciones** (lo que el cliente envía):
  - romper (empezar, seguir y cancelar);
  - poner o usar (bloque, comer o abrir un bloque);
  - movimientos de casillas, craftear, tirar, reaparecer y elegir la casilla de la barra.

  Así la etapa 4 solo tendrá que enviarlas por la red.

### Interfaz (cliente)

Abajo, en el centro, durante el juego:

```
      ♥♥♥♥♥♥♥♥♡♡          ◖◗◖◗◖◗◖◗◖◗   (corazones · jamones)
  ▕████████████(☕ 7)░░░░░░░░░▏          (experiencia: barra café/verde, nivel en un grano de café)
  [⛏][🪓][▦][▦][🍌][ ][ ][ ][ ]          (barra de 9 casillas, la elegida con borde tallado claro)
```

Pantallas:
- **Inventario (E):**
  - en la parte de arriba, a la izquierda, la figura del jugador;
  - a su derecha, la rejilla de crafteo 2×2 → resultado;
  - debajo, la mochila de 27 casillas y la barra de 9.
- **Mesa de trabajo:** rejilla de 3×3 → resultado, y debajo el inventario.
- **Horno de barro:** entrada, combustible, una flecha de progreso con llama y la salida.
- **Baúl:** 27 casillas, y debajo el inventario.
- **Piedra de amolar:** casilla de herramienta → casilla de resultado, con el coste en niveles y el aviso «Te faltan N niveles».
- **Pantalla de muerte:** «Te has quedado en el cafetal», con las coordenadas donde quedaron las cosas y el botón «Reaparecer».

Estilo de todas las pantallas:
- madera de guadua con bordes tallados;
- casillas en tono tierra;
- textos en la fuente del juego;
- la burbuja de información muestra el nombre, la durabilidad y el hambre que quita cada comida.

Los iconos se dibujan por código, como las texturas.

Otros detalles:
- Al romper un bloque, se ven grietas en 6 etapas.
- Al recibir daño, la pantalla se pone roja.
- La burbuja de aire aparece bajo el agua.

**Controles:**
- clic izquierdo mantenido: romper;
- clic derecho: poner, usar o comer (comer se mantiene 1,6 s);
- E: inventario;
- Q: tirar (Ctrl+Q tira la pila);
- 1–9 y rueda: casillas.
- **En las pantallas:**
  - clic toma o deja la pila;
  - clic derecho toma la mitad o deja una;
  - Mayús+clic mueve entre partes.

### Mundo

- Plantas nuevas en `worldgen/features.rs`, deterministas con la semilla:
  - plataneras en Selva, Costa, Manglar y Montaña cafetera baja;
  - arbustos de mora en Páramo y Bosque de niebla;
  - guayabos (árbol pequeño con hojas de guayabo) en Sabana y Valle de guadua.
- **Punto de aparición** (decisión de Claude): la búsqueda del punto de aparición evita también Desierto rojo, Nevado y Páramo, para empezar siempre cerca de árboles y comida. En mundos de la etapa 1 el punto puede moverse un poco.
- Los chunks que un mundo de la etapa 1 ya generó no reciben plantas nuevas; sí aparecen en todo lo que se explore después. Es aceptable porque esos mundos eran de prueba.

### Guardado

- `PlayerSave` gana inventario, vida, hambre, saciedad, aire, experiencia y recetas ya hechas. Los campos usan `#[serde(default)]`, así que el `player.ron` de la etapa 1 abre con el inventario vacío y la vida llena.
- `META_VERSION` pasa a 2.
- **Contenido de los bloques** (baúl y horno): se guarda en un archivo por región junto a la región, con su propia versión. Romper un baúl o un horno suelta su contenido.
- **Objetos en el suelo:** se guardan con su chunk, para que no se pierdan las cosas de una muerte al salir del juego.

### Creativo

Sin vida, hambre, aire ni daño. El inventario muestra la lista de todos los objetos para tomarlos sin límite, en lugar de la mochila. Romper es instantáneo y no suelta nada (como ahora). La barra creativa de la etapa 1 pasa a ser la barra de 9 normal, con los bloques de antes al empezar.

## Pruebas (lo mínimo)

- **`survivalforest-core`:**
  - los RON cargan;
  - las claves son únicas;
  - toda receta usa objetos que existen;
  - toda herramienta tiene nivel y durabilidad;
  - toda comida da hambre;
  - **ninguna receta con forma coincide con una lista de formas prohibidas**: las de Minecraft para los objetos equivalentes, escritas a mano en la prueba.
- **`survivalforest-sim`:**
  - inventario: pilas, partir y llenar;
  - crafteo de 2×2 y 3×3, desplazado y sin forma;
  - horno, con combustible y tiempo;
  - vida y hambre en el tiempo, con regeneración y daño por hambre hasta medio corazón;
  - caída: 3 bloques sin daño, 10 bloques quitan 7, sin daño en agua;
  - ahogamiento;
  - tiempos de rotura y nivel mínimo de las menas;
  - desgaste;
  - muerte: los objetos caen y se recuperan, con la mitad de la experiencia;
  - coste de la piedra de amolar;
  - curva de experiencia.
- **`survivalforest-world`:**
  - las plantas nuevas son deterministas con la semilla y aparecen en sus biomas;
  - un mundo de la etapa 1 se abre con el código nuevo;
  - el contenido de los baúles se guarda y se recupera.
- **Cliente:**
  - la prueba de humo de CI sigue en verde;
  - capturas de la interfaz (barra, inventario, mesa, horno, baúl, piedra de amolar y muerte) con el mecanismo de capturas que ya existe, para revisarlas.

## Entrega

- Una sola versión jugable, **0.2.0**, con su PR en `juego-rust`, en varios commits por partes.
- El Launcher no cambia: la nueva versión se publica con el flujo de versiones que ya existe.

## Lo que decide quien construye

- Las formas exactas de las recetas, siempre propias y comprobadas por la prueba de formas prohibidas.
- Los valores finos de los datos: dureza por bloque, saciedad, curva de experiencia y velocidad del hambre.
- Cómo dibujar los iconos y las texturas nuevas, dentro del estilo.
- La estructura interna de los módulos de la simulación y de la interfaz en Bevy.
- El formato del archivo de bloques con contenido, siempre con versión.

## Decisiones de Claude marcadas para revisar

- El hambre a cero no mata: deja medio corazón.
- Se reaparece en el punto de aparición del mundo, porque aún no hay camas.
- De la experiencia, la mitad queda en un orbe al morir y la otra mitad se pierde.
- Los objetos de la muerte duran 10 minutos y los demás objetos caídos, 5.
- Hay tres menas que hay que fundir: cobre, hierro y oro. El oro solo da lingotes por ahora, y su uso llega en otra etapa.
- El cobre es un nivel entre la piedra y el hierro.
- La piedra de amolar repara por completo y cobra según el desgaste.
- Las recetas de teja y pared encalada sirven para construir las casas de la demo.
- Se puede nadar (Espacio sube en el agua).
- La piedra necesita un pico de madera para soltar algo; el resto de bloques sale a mano.
- Se aparece lejos de Desierto rojo, Nevado y Páramo; los chunks ya generados de mundos viejos no reciben plantas nuevas.
