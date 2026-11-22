# Arquitectura de ProjectStar

## Principios

1. **Modularidad**: cada subsistema es un crate independiente con una única
   responsabilidad. Se puede compilar, testear y reemplazar por separado.
2. **Dependencias hacia abajo**: un crate solo depende de capas inferiores.
3. **Agnóstico a la plataforma**: el código específico de SO vive en `star_platform`.
4. **APIs propias**: las librerías externas se envuelven para no filtrarse en la API pública.
5. **2D y 3D de primera clase**: ninguna de las dos es un caso especial de la otra
   a nivel de API de usuario, aunque compartan infraestructura.

## Capas

```
Capa 5  editor/, examples/, juegos
           │
Capa 4  star_engine ─────────────── fachada: App, prelude, re-exports
           │
Capa 3  star_scene ──────────────── árbol de nodos + componentes
           │
Capa 2  star_render  star_audio  star_physics  star_input  star_assets
           │
Capa 1  star_ecs  star_platform
           │
Capa 0  star_core  star_math

Herramientas: star_script ── compilador StarScript → Rust (sin dependencias;
                             lo usarán el editor y la línea de comandos)
```

## Modelo de juego (objetivo)

- **Escenas** como árbol de **nodos** (al estilo Godot), guardables e instanciables.
- Cada nodo está respaldado por una **entidad** del ECS, con **componentes**
  que aportan comportamiento (al estilo Unity).
- **Sistemas** recorren componentes cada frame en un orden de etapas definido
  (input → lógica → física → render).

## Decisiones registradas

| # | Decisión | Motivo |
|---|----------|--------|
| 1 | Rust edición 2024, stable | Seguridad de memoria sin GC; ecosistema multiplataforma |
| 2 | Workspace de Cargo con crates `star_*` | Compilación incremental y límites claros entre módulos |
| 3 | `unsafe` denegado por defecto | Solo permitido en FFI/plataforma con justificación |
| 4 | Errores vía `EngineResult<T>` | Sin pánicos en el motor; el juego decide cómo reaccionar |
| 5 | Matemáticas propias (`star_math`) | Fachada estable; la implementación puede usar SIMD o una librería externa |
| 6 | Render sobre **wgpu** (no implementación propia) | Una API para DX12, Vulkan, Metal y OpenGL; traduce shaders (naga). Escribir 4 backends propios costaría años |
| 7 | Ventanas con **winit** | Estándar de Rust para ventanas/eventos en escritorio y móvil; es el que usa wgpu |
| 8 | wgpu y winit no aparecen en la API pública | Solo `star_render` conoce wgpu y solo `star_platform` conoce winit. Permite añadir backends de consola (no soportados por wgpu) sin romper juegos. Excepción: decisión 15 |
| 9 | Render en modo inmediato vía `Frame` | El juego describe cada frame (`Frame::draw(malla, transform)`); el renderer lo convierte en trabajo de GPU. Simple ahora; la escena alimentará el `Frame` más adelante |
| 10 | Matrices column-major, mano derecha, profundidad `0..1` | Es lo que esperan WGSL y las APIs modernas; +Y arriba, la cámara mira hacia -Z |
| 11 | Interfaz del editor con **egui** (vía `eframe`) | UI inmediata muy usada en herramientas de Rust; usa nuestras mismas versiones de wgpu y winit (sin compilar dos copias). Solo la usa `editor/`; el motor no depende de ella. Exige Rust 1.95, así que solo el editor declara ese `rust-version` |
| 12 | Archivos del editor en TOML con `serde` + `toml` | `project.toml` y la lista de recientes se leen y escriben con serde: sin escapes manuales y con errores claros si un archivo está corrupto |
| 13 | Diálogos nativos con **rfd** | Selector de carpetas del propio sistema (portal XDG en Linux, nativo en Windows/macOS), solo en el editor |
| 14 | Rutas del usuario en `star_platform::user_directories` | Carpeta personal y de configuración dependen del SO; ese código (`cfg!`) vive en la capa de plataforma |
| 15 | `star_render::interop` (feature `interop`) para el editor | La vista del editor renderiza con el motor en una textura del **mismo dispositivo wgpu** que egui: sin copias por CPU. Expone tipos wgpu, por eso está tras una feature que solo activa el editor. Alternativa descartada: copiar cada frame a la CPU (lento) |
| 16 | Lenguaje propio **StarScript** compilado a Rust (`star_script`) | Oculta lifetimes y préstamos tras un lenguaje orientado a objetos con tipos estáticos, sin perder el rendimiento de Rust. Flujo: blueprints → StarScript → Rust → máquina. Lexer y parser escritos a mano, sin generadores (`logos`, `pest`): control total de los mensajes de error y cero dependencias. Ver `docs/starscript.md` |
| 17 | StarScript **null-safe** y con tipos estáticos | `T` nunca es `null`; solo `T?`, con `?.`, `??` y estrechamiento tras comprobar (solo sobre lo inmutable, para que sea sólido). Construcción en dos fases como Swift: `self` no existe hasta que el objeto está completo. Así un juego no puede fallar por un `null` ni por un objeto a medio construir, y el Rust generado usa `Option<T>` sin `unwrap` |
| 18 | Licencia **GPL-3.0-or-later + excepción para juegos** | El motor y el editor son software libre y sus mejoras deben compartirse, pero los juegos pueden tener cualquier licencia (como la excepción de GCC). Sin la excepción, todo juego enlazado con el motor tendría que ser GPL. Ver `LICENSE-GAME-EXCEPTION.md` |

## Render

```
juego → star_engine::App ──(GraphicsBackend: Auto | Vulkan | DirectX12 | Metal | OpenGl)
            │
            ├── star_platform: Window + bucle de eventos (winit)
            └── star_render:   Renderer (wgpu) → DX12 / Vulkan / Metal / OpenGL
                                     └── (futuro) backend de consola
```

- `Auto` deja que wgpu elija la mejor GPU; un backend explícito falla con un error
  claro si no existe en la plataforma (p. ej. DirectX 12 en Linux).
- Si el adaptador elegido es un renderizador por software (CPU), se emite un aviso.
- Ciclo de un frame: `Game::update(&Time)` → `Game::draw(&mut Frame)` →
  `Renderer::render(&Frame)`. Las mallas se crean en `Game::start` con
  `GameContext::create_mesh` y se identifican con `MeshId`.
- `SceneRenderer` (interno) dibuja un `Frame` en cualquier destino; lo usan
  `Renderer` (ventana) y `interop::OffscreenRenderer` (textura, para el editor).
- Mallas de triángulos (`create_mesh`) y de líneas (`create_lines`).
- Cada draw tiene su transform (proyección·vista·modelo) en una ranura de un
  único uniform buffer con offset dinámico: funciona en todos los backends,
  OpenGL incluido, y sube todas las matrices del frame en una sola copia.
- `Camera::new_2d` (ortográfica, ancho según el aspect ratio para no deformar)
  y `Camera::new_3d` (perspectiva de 60°).
- `Color` se expresa en espacio sRGB (como los selectores de color) y se convierte
  a lineal cuando el render target es sRGB.

## Editor

Una sola ventana nativa con dos pantallas:

1. **Gestor de proyectos** (`launcher/`): a la izquierda, proyectos recientes
   (botón «Abrir»; los que ya no existen aparecen como «No disponible» con
   «Quitar») y «Abrir otro proyecto…» (selector de carpeta). A la derecha,
   **Nuevo proyecto**: nombre, ubicación (con «Examinar…»), tipo 2D o 3D
   (exclusivo) y plataformas (checks). «Crear y abrir» crea
   `<ubicación>/<nombre>/` con `project.toml` y `assets/` (nunca sobrescribe)
   y abre el editor.
2. **Editor** (`workspace/`), maximizado: menú (Archivo → Cerrar proyecto /
   Salir; Ayuda → Acerca de), panel **Escena** (jerarquía, vacía hasta que
   exista `star_scene`), **vista** central renderizada por el motor
   (cuadrícula de referencia; arrastrar = desplazar en 2D / orbitar en 3D,
   rueda = zoom), **Inspector** (propiedades del proyecto),
   **Archivos** (árbol de `assets/`, con «Actualizar») y barra de estado.

La vista usa `OffscreenRenderer` sobre el dispositivo wgpu de egui y registra
su textura en `egui_wgpu` (se re-registra al cambiar de tamaño). La textura es
`Rgba8Unorm` con colores ya codificados en sRGB, porque egui muestrea las
texturas como valores gamma.

Módulos: `project/` (modelo validado, creación y apertura, sin UI y con tests),
`recent_projects.rs` (lista de recientes, máx. 10, en
`<config>/ProjectStar/recent_projects.toml`), `folder_dialog.rs` (rfd),
`texts.rs` (todos los textos visibles, para traducirlos más adelante) y
`style.rs` (constantes visuales compartidas).

Ejemplo de `project.toml`:

```toml
# ProjectStar project file.
[project]
name = "Mi Juego Espacial"
engine_version = "0.1.0"
dimension = "3d"
platforms = ["windows", "linux", "android"]
```
