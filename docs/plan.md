# Plan: de la maqueta a la GUI real

> Las rutas `examples/`, `tests/fixtures/` y `instatypst-cli/` son del repo `typst-rrss` (`~/Documentos/GitHub/typst-rrss`). Este plan vive aquí, con la GUI; el contrato está en `typst-rrss/docs/contrato-v1.md`.

Estado de partida: `instatypst-gui` (`~/Proyectos/Instatypst-Gui`) es una maqueta de egui de
1 900 líneas con datos de mentira (`Block`, `Template`, `FileRow`, `PRESETS`, `RATIOS` fijos en el
código). Ya tiene el dock (egui_dock) con los paneles Contenido, Archivos, Editor, Vista previa,
Plantilla, Estilo, Formato y Color. No lee ni escribe nada. Lo que tiene el resto del sistema:

- el contrato `instatypst-piece/v1`, congelado, con `check --json` (informe `instatypst-check/v1`);
- las firmas de los estilos (`firmas.json`, embebido en el CLI con `Catalog::embedded()`);
- `instatypst-color`: hex, contraste, esquemas, selector de roles, `from_base`, gradientes y
  duotonos, Material;
- el CLI `instatypst`, que compila lanzando el binario `typst` (`process.rs`, `typst.rs`);
- **Grammata** (`~/Proyectos/grammata`), con un motor de Typst embebido que ya resuelve lo difícil:
  `engine.rs` (`World`, fuentes, paquetes), `render_preview` (de fuente a `egui::ColorImage`),
  `SyncEntry`/`line_for_pos` (clic en la página → línea del código), `export_pages`, la compilación
  en un hilo propio y `typst-ide` para el autocompletado.

## Principio

**La GUI edita el documento y lo previsualiza con el mismo Typst que lo exporta.** Grammata ya paga
el costo de embeber Typst, así que la GUI lo hace también: la vista previa sale de `typst-render`
en memoria, que es el mismo renderizador de `typst compile`, y por eso lo que se ve es lo que se
exporta. Sin subproceso, sin hayro (el PDF solo es una salida para A4 y A5) y con acceso directo al
documento compilado para el modo debug.

El CLI sigue siendo la herramienta de escritura masiva y de CI. La GUI no le pide la vista previa;
sí comparten crates, para que no puedan desincronizarse:

| Crate | Contiene | Lo usan |
| :--- | :--- | :--- |
| `instatypst-piece` | modelo del contrato, `markup`, validación y firmas (`firmas.json`) | CLI, GUI |
| `instatypst-color` | color, contraste, roles, paletas | CLI, GUI |
| `instatypst-engine` | `World` de Typst, fuentes, paquetes, compilar, renderizar, sincronía | GUI; el CLI después |

## Fases

Cada fase termina en algo que se usa y se prueba; ninguna deja la GUI rota.

### 0 · Preparar el terreno (en typst-rrss)
- **`instatypst-piece`**: mover `model`, `markup`, `firmas.json` y la validación a un crate del
  workspace; el CLI depende de él. Es mover código sin cambiar comportamiento: `congelado` y `pieza`
  lo prueban.
- **`instatypst-engine`**: partir de `grammata/src/engine.rs` y quedarse con lo que no es de
  Grammata (`World`, fuentes del sistema y de `Fonts/`, paquetes, compilación a `PagedDocument`,
  render a pixmap, exportación de páginas, mapa posición↔fuente). Quitar lo que depende de egui
  para que el crate no arrastre la interfaz. Versión de Typst fijada (hoy 0.15.1, como en Grammata
  y en `typst-syntax` del CLI).
- **Licencia**: Grammata es GPL-3.0-or-later e Instatypst se declara MIT. Reutilizar tu propio
  código con otra licencia es posible si eres su único autor; confirmarlo antes de copiar
  (y revisar que `engine.rs` no tenga trozos ajenos). Si hubiera dudas, reescribir el `World`
  desde la documentación de Typst.
- La GUI entra al workspace (un solo `cargo test`, un solo CI). Perfil de desarrollo con las
  dependencias optimizadas, como en Grammata, porque el render es lento sin eso.

### 1 · Un documento real
- Sustituir las estructuras de mentira por el modelo del contrato (`Document`, `Piece`, bloques).
- Abrir y guardar `.toml` / `.json` / `.yaml`. Para TOML, **`toml_edit`**: guardar sin destruir
  comentarios ni orden del archivo de la persona.
- Panel Archivos = documentos reales; panel Contenido = la lista de piezas, en orden (el orden es
  la serie). Crear, duplicar, borrar y reordenar piezas.
- Hecho cuando: abrir `examples/*.toml`, tocar una pieza y guardar deja un diff mínimo.

### 2 · Diagnóstico vivo
- Tras cada cambio (con pausa de ~300 ms), validar en un hilo con las mismas funciones que usa
  `check` (modelo, semántica, puente, render a Typst): ya no hay que lanzar un proceso, y el
  informe es el de `instatypst-check/v1`.
- Un panel de incidencias; cada una lleva `path` (`post[2].content[1]`), así que hacer clic
  enfoca el campo. Campo en rojo (error) o ámbar (aviso: `contrast_low`).
- Hecho cuando: un typo en `style` o un bloque que el estilo no acepta se ve junto al campo.

### 3 · Vista previa
- Compilar la pieza activa en el hilo del motor y mostrar el pixmap como textura de egui.
- La caché de memoización de Typst hace el resto: los cambios pequeños recompilan en una fracción.
  Cancelar la compilación en curso cuando llega otro cambio; purgar la caché como hace Grammata.
- Estados visibles: compilando, error (con el mensaje y la línea de Typst), desactualizada.
- Zoom y ajuste a la ventana; la textura se vuelve a rasterizar al cambiar de escala.
- Hecho cuando: editar un título refresca la lámina de forma fluida y es idéntica pixel a pixel al
  PNG que da `instatypst build` (se prueba con una pieza real, como las paridades de `pieza.rs`).

### 4 · Modo debug
Con el documento compilado en memoria, no hace falta ningún formato intermedio:
- **Capa de geometría**: cajas de cada bloque, márgenes y rejilla encima de la lámina, con medidas.
  Dos fuentes, a probar en ese orden: (a) la introspección de Typst (`metadata` con
  `here().position()` por bloque) y (b) el árbol de marcos del `PagedDocument`.
- **Clic en la lámina → código**: el mapa posición↔fuente de `engine.rs` (`line_for_pos`, y
  `jump_from_click` de `typst-ide`). Apunta al código Typst del preset; el paso a «qué bloque del
  contrato» necesita que cada preset etiquete sus bloques (comprobar cuántos ya lo hacen).
- **Inspector**: al pasar el ratón, el elemento (texto, fuente, tamaño, color, caja) y el
  contraste real del texto sobre su fondo.
- Hecho cuando: hacer clic en el epígrafe de una lámina `liso` selecciona el bloque `epigraph` en
  el panel de contenido.

### 5 · Ejes del contrato como controles
Un panel por eje, con lo que ya existe (todo sale de datos, no de listas escritas a mano):
- **Estilo y variante**: del catálogo de firmas (`Catalog::embedded()`, vía `instatypst-piece`).
  Elegir un estilo cambia los bloques disponibles.
- **Formato**: `square`, `portrait`, `story`, `landscape`, `a4`, `a5` o tamaño propio (100–4320).
- **Paleta**: nombre, o fuente (`base`, `from_image`) con roles que la pisan. Con `instatypst-color`:
  selector de color → `from_base`; imagen → roles de la foto o Material; contraste en vivo con los
  mismos mínimos (4,5 y 3) que el CLI. **Presets** con las paletas reales del proyecto (granates,
  bosque, terracota, océano, oscuro/crema/ascua, verde/hueso/hondo/arena). Las de la persona viven
  en `~/.config/instatypst/paletas.toml` (propuesta).
- **Fondo**: sólido, grid, rules, paper, contours, imagen (con duotono opcional más adelante).
- **Tipografía** y **marca** (texto, imagen, función Typst; tamaño).
- Hecho cuando: los cuatro presets prioritarios (01, 04, 11, 17) se arman solo con paneles.

### 6 · Editor de bloques guiado por la firma
- La lista de bloques de la pieza, con los requeridos marcados, los «uno de» agrupados y los
  opcionales en un menú «añadir». Un bloque que el estilo no acepta no se ofrece.
- Campos por tipo (title, quote, epigraph con fuente, attribution, list con orden, contrast
  izquierda/derecha, number, date…) con las reglas del contrato en el formulario: único,
  repetible, `total ≥ value`, día 1–31.
- Alternar entre **bloques** y **texto en Typst** (las dos formas son equivalentes). Pasar de
  bloques a texto es siempre posible; de texto a bloques usa `markup` y muestra el error con su
  línea y columna. El editor de texto reutiliza el resaltado de la maqueta y, si se quiere,
  el autocompletado de `typst-ide` que ya está resuelto en Grammata (`ide.rs`).
- Hecho cuando: todas las piezas de `tests/fixtures/contrato/valid` se editan sin salir de los paneles.

### 7 · Salida y pulido
- Compilar todas las piezas con el motor (barra de progreso, cancelar), abrir la carpeta de salida.
- PDF para A4 y A5 (`typst-pdf`), con elección de páginas como en Grammata.
- Estilo propio (`style = { file, fn }`): elegir archivo y función; avisar de que es código de confianza.
- Deshacer/rehacer sobre el modelo, no sobre el texto.
- Atajos, tema claro/oscuro, y la persistencia de la disposición del dock (los *presets* de
  disposición de la maqueta ya existen).

### 8 · Después (cuando la GUI ya sirva)
- **El CLI usa `instatypst-engine`** en vez de lanzar `typst` (`process.rs`, `typst.rs`): sin la
  dependencia de tener `typst` instalado, una sola compilación con la caché compartida entre piezas
  y una vista previa y un `build` que no pueden diferir.
- Vista SVG (`typst-svg`) si el zoom o la inspección de trazos lo piden.

## Orden y dependencias

`0 → 1 → 2 → 3` es el camino crítico: con eso ya hay un editor con vista previa. El modo debug (4)
depende de la 3. Las fases 5 y 6 son independientes entre sí y se pueden alternar. La 2 puede
empezar antes que la 3 (no necesita imágenes). La 7 y la 8 van al final.

## Riesgos

- **Costo de embeber Typst**: tiempo de compilación (LTO completo en release), tamaño del binario
  y memoria (Grammata anotó ~240 MB con documentos grandes). Aquí las piezas son pequeñas, pero hay
  que medir al llegar a la fase 3 y purgar la caché de Typst tras cada compilación.
- **Versión de Typst fijada**: GUI, CLI y `typst-syntax` deben ir juntos. Subir de versión es un
  cambio coordinado y se prueba con las paridades de render.
- **Licencia de `engine.rs`**: ver la fase 0. Es lo primero que hay que aclarar.
- **Del código Typst al bloque del contrato**: `jump_from_click` apunta a líneas del preset, no a
  bloques. Si los presets no etiquetan sus bloques, el clic en la lámina solo llega al código.
  Se resuelve etiquetando, y se comprueba en la fase 4 con un preset antes de generalizar.
- **Firmas desfasadas**: el catálogo ya viaja dentro del binario del CLI, copia manual de
  `schema/firmas.json` de Instatypst. Al pasar a `instatypst-piece`, GUI y CLI comparten las mismas
  firmas por código. Queda el desfase de siempre entre Instatypst y esa copia; se vigila con un test
  que compare ambos archivos.
- **Dos fuentes de verdad de validación**: la GUI valida con las mismas funciones que `check`, no con
  una copia. Si algún día divergen, manda `check`.

## Fuera de alcance (por ahora)

Roles de lámina, marco y rótulo de serie (v2), recoloreado de fondos con `lutgen`
(`background.recolor`, v2), `colorous` y esquemas categóricos para gráficos, edición colaborativa,
una versión web, hayro y la vista previa por PDF.

## Preguntas que hay que cerrar al empezar

1. **Licencia de `engine.rs`**: ¿eres el único autor de ese código y lo puedes publicar bajo MIT?
   Si no, se reescribe.
2. ¿`instatypst-engine` es un crate de este workspace o uno aparte que usen también Grammata y esta
   GUI? Compartirlo ahorra mantener dos `World`; es más trabajo al principio y ata los ritmos de
   los dos proyectos. Recomiendo empezar dentro del workspace y extraerlo cuando Grammata lo pida.
3. ¿Dónde viven las paletas de la persona: `~/.config/instatypst/paletas.toml` o dentro del
   documento (`[palettes.*]`)? Probablemente las dos, con el documento mandando.
