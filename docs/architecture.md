# ProjectStar architecture

## Principles

1. **Modularity**: each subsystem is an independent crate with a single
   responsibility. It can be built, tested and replaced on its own.
2. **Dependencies go downwards**: a crate only depends on lower layers.
3. **Platform-agnostic**: OS-specific code lives in `star_platform`.
4. **Own APIs**: external libraries are wrapped so they do not leak into the public API.
5. **First-class 2D and 3D**: neither is a special case of the other in the
   user API, although they share infrastructure.

## Layers

```
Layer 5  editor/, examples/, games
            │
Layer 4  star_engine ─────────────── facade: App, prelude, re-exports
            │
Layer 3  star_scene ──────────────── node tree + components
            │
Layer 2  star_render  star_audio  star_physics  star_input  star_assets
            │
Layer 1  star_ecs  star_platform
            │
Layer 0  star_core  star_math

Tools:   star_script ── StarScript → Rust compiler (no dependencies; the
                        editor and the command line will use it)
         star_script_runtime ── support for the generated Rust (no
                        dependencies; games use it)
```

## Game model (goal)

- **Scenes** as a tree of **nodes** (Godot style), that can be saved and instanced.
- Each node is backed by an ECS **entity**, with **components** that provide
  behavior (Unity style).
- **Systems** go through components every frame in a defined order of stages
  (input → logic → physics → render).

## Recorded decisions

| # | Decision | Reason |
|---|----------|--------|
| 1 | Rust edition 2024, stable | Memory safety without GC; cross-platform ecosystem |
| 2 | Cargo workspace with `star_*` crates | Incremental builds and clear boundaries between modules |
| 3 | `unsafe` denied by default | Only allowed in FFI/platform code, with a justification |
| 4 | Errors through `EngineResult<T>` | No panics in the engine; the game decides how to react |
| 5 | Own math (`star_math`) | Stable facade; the implementation may use SIMD or an external library |
| 6 | Rendering on **wgpu** (not an own implementation) | One API for DX12, Vulkan, Metal and OpenGL; it translates shaders (naga). Writing 4 own backends would take years |
| 7 | Windows with **winit** | The Rust standard for windows/events on desktop and mobile; it is what wgpu uses |
| 8 | wgpu and winit do not appear in the public API | Only `star_render` knows wgpu and only `star_platform` knows winit. This allows adding console backends (not supported by wgpu) without breaking games. Exception: decision 15 |
| 9 | Immediate-mode rendering through `Frame` | The game describes each frame (`Frame::draw(mesh, transform)`); the renderer turns it into GPU work. Simple for now; the scene will feed the `Frame` later |
| 10 | Column-major matrices, right-handed, depth `0..1` | It is what WGSL and modern APIs expect; +Y up, the camera looks towards -Z |
| 11 | Editor interface with **egui** (through `eframe`) | Immediate-mode UI widely used in Rust tools; it uses our same wgpu and winit versions (no two copies to build). Only `editor/` uses it; the engine does not depend on it. It requires Rust 1.95, so only the editor declares that `rust-version` |
| 12 | Editor files in TOML with `serde` + `toml` | `project.toml` and the recent projects list are read and written with serde: no manual escaping, and clear errors if a file is corrupt |
| 13 | Native dialogs with **rfd** | The system's own folder picker (XDG portal on Linux, native on Windows/macOS), only in the editor |
| 14 | User paths in `star_platform::user_directories` | The home and configuration folders depend on the OS; that code (`cfg!`) lives in the platform layer |
| 15 | `star_render::interop` (feature `interop`) for the editor | The editor's view renders with the engine into a texture of the **same wgpu device** as egui: no CPU copies. It exposes wgpu types, which is why it is behind a feature that only the editor enables. Rejected alternative: copying every frame to the CPU (slow) |
| 16 | Own language **StarScript** compiled to Rust (`star_script`) | It hides lifetimes and borrowing behind an object-oriented, statically typed language, without losing Rust's performance. Flow: blueprints → StarScript → Rust → machine code. Hand-written lexer and parser, without generators (`logos`, `pest`): full control of error messages and zero dependencies. See `docs/starscript.md` |
| 17 | **Null-safe**, statically typed StarScript | `T` is never `null`; only `T?` can be, with `?.`, `??` and narrowing after a check (only on what cannot change, so it is sound). Two-phase construction as in Swift: `self` does not exist until the object is complete. So a game cannot fail because of a `null` or a half-built object, and the generated Rust uses `Option<T>` without `unwrap` |
| 18 | **GPL-3.0-or-later + game exception** license | The engine and the editor are free software and their improvements must be shared, but games may have any license (like GCC's exception). Without the exception, every game linked with the engine would have to be GPL. See `LICENSE-GAME-EXCEPTION.md` |
| 19 | StarScript objects with **reference counting** (`Rc`) and an own runtime | Automatic release and accesses that never fail, without a global store or checks on every field (rejected alternative: generational handles, slower and with accesses that can fail). Inheritance through traits that extend each other; two-phase construction through a function that creates the object. Errors that only appear while playing (index out of range, division by zero) stop the game with a clear message, from a single point: `star_script_runtime::fault` (exception to decision 4). See `docs/starscript.md`, section 4 |
| 20 | StarScript games **extend the engine class `Game`** | The engine calls `start` and `update(delta)` through a generated adapter that implements the `Game` trait of `star_engine`; `quit()` sets an internal field that the loop checks (`Game::is_finished`). Only games depend on `star_engine`: console programs (`static func main()`) still do not depend on wgpu. The StarScript API for input and nodes will be made together with phases 2 and 6, so it is not invented twice |
| 21 | Raw input events defined in `star_platform`, input state in `star_input` | `star_platform` (layer 1) is the only crate that knows winit, so it translates OS events into its own vocabulary (`InputEvent`, `Key`, `MouseButton`); `star_input` (layer 2) builds the per-frame state from them, without knowing winit. Keys are identified by their **physical position** (US QWERTY names), so WASD works on any layout. State lives in fixed-size arrays: updating it never allocates in the game loop |
| 22 | Actions **identified by name** (`"jump"`) | It is what StarScript and the editor will use (names in project settings and scripts), and the lookup does not allocate. Action names are unique and non-empty, checked when the `ActionMap` is built (`EngineError::Input`). Rejected for now: typed action ids, which would add a second way to do the same thing |
| 23 | Gamepads with **gilrs** (MIT/Apache-2.0) | Maintained and pure Rust, with backends for Windows (WGI/XInput), macOS (IOKit) and Linux (evdev), SDL-style mappings and dead zones. Only `star_platform` knows it. On Linux it needs the libudev headers to build (`libudev-dev`). Rejected: writing our own backends per OS (a lot of platform code for little gain) |
| 24 | StarScript input: actions in **`project.toml`**, read through `star_script_runtime::input` | Actions are project settings (`[input.actions]`), as an editor panel will show them, so scripts stay clean and the compiler can reject misspelled action names (it receives them in `GameSettings`). `Input.is_down("jump")` is a static call with no object to read the engine's input from, so the generated game copies each action's state into a per-thread table of the runtime once per frame (no allocations; the runtime stays without dependencies) and the calls read it by name. Rejected: declaring actions in scripts (typos in key names only fail while playing) |
| 25 | Sprites **batched by texture**, in an immediate `Frame` | The game adds sprites every frame (like meshes, decision 9); the renderer groups consecutive sprites that share a texture into one draw call. The order is explicit (layers, then order of addition), so translucent sprites always blend correctly without sorting by depth. Rejected for now: texture arrays or bindless textures (not available on OpenGL ES and older GPUs) and automatic atlases (they belong to the asset pipeline, Phase 5) |
| 26 | PNG decoding with **png** (MIT/Apache-2.0), in `star_assets` | The image-rs project's decoder: maintained, pure Rust (it builds for every platform, mobile included) and light. PNG is lossless with transparency, the usual format for 2D game art. Only `star_assets` knows it. Rejected: the `image` crate (many formats, but much heavier to build; more formats can come in Phase 5) |

## Input

```
OS ──winit──▶ star_platform (InputEvent) ──▶ star_input::Input ──▶ Game::update
```

- Events arrive between frames and are applied to `Input` as they come.
  After `Game::update`, `Input::end_frame` forgets what was *just* pressed,
  released or scrolled; held keys stay held.
- A press and a release within the same frame are both reported
  (`just_pressed` and `just_released`), so quick taps are never lost.
- Key repeats and synthetic events are ignored. When the window loses focus
  every key and button is released, so none stays "stuck".
- The cursor is in physical pixels from the top-left corner (`None` outside
  the window); the wheel is in lines (touchpad pixels are converted).
- **Touch**: `Touches` keeps up to `MAX_TOUCHES` (10) fingers in a fixed
  array, each with its id, position, start position, `is_down`,
  `just_started` and `just_ended`. A finger that leaves the screen stays
  listed until the frame ends, so quick taps are not lost; a touch cancelled
  by the system counts as ended. More fingers than the capacity are ignored.
- **Gamepads**: they are not delivered through the window, so
  `star_platform` polls `gilrs` once per frame (before the redraw) and
  reports the gamepads already connected at start. `Gamepads` keeps up to
  `MAX_GAMEPADS` (8). Buttons are named by position (`South` is «A» on
  Xbox and «✕» on PlayStation); sticks go from -1 to 1 (positive Y up, dead
  zone already applied) and triggers from 0 to 1, besides being buttons.
  If gamepads cannot be opened, the game still runs (a warning is logged).
- Raw keyboard and mouse are the base layer: games normally use **actions**.
  An `ActionMap` (given to `App::with_actions`) binds each action name to
  one or more `Binding`s: a key, a mouse button or `Binding::Touch` (any
  finger on the screen) or `Binding::GamepadButton` (that button on any
  gamepad). On-screen
  virtual buttons (touch regions) will come later. An action is held while **any** of its bindings is
  held: it keeps a count of held bindings, so pressing a second binding of a
  held action is not a new press. Actions are asked for by name
  (`input.actions().just_pressed("jump")`); an unknown name answers `false`.

## Rendering

```
game → star_engine::App ──(GraphicsBackend: Auto | Vulkan | DirectX12 | Metal | OpenGl)
            │
            ├── star_platform: Window + event loop (winit)
            └── star_render:   Renderer (wgpu) → DX12 / Vulkan / Metal / OpenGL
                                     └── (future) console backend
```

- `Auto` lets wgpu choose the best GPU; an explicit backend fails with a clear
  error if it does not exist on the platform (e.g. DirectX 12 on Linux).
- If the chosen adapter is a software (CPU) renderer, a warning is logged.
- A frame's cycle: `Game::update(&Time, &Input)` → `Game::draw(&mut Frame)` →
  `Renderer::render(&Frame)`. Meshes are created in `Game::start` with
  `GameContext::create_mesh` and identified by a `MeshId`.
- `SceneRenderer` (internal) draws a `Frame` into any target; `Renderer`
  (window) and `interop::OffscreenRenderer` (texture, for the editor) use it.
- Triangle meshes (`create_mesh`) and line meshes (`create_lines`).
- Each draw has its transform (projection·view·model) in a slot of a single
  uniform buffer with a dynamic offset: it works on every backend, OpenGL
  included, and uploads all the matrices of the frame in a single copy.
- `Camera::new_2d` (orthographic, its width follows the aspect ratio so
  nothing is stretched) and `Camera::new_3d` (60° perspective).
- `Color` is expressed in sRGB space (like color pickers) and converted to
  linear when the render target is sRGB.
- Textures are created from an `Image` (RGBA8 pixels in sRGB, validated
  when built) with `GameContext::create_texture(&image, TextureFilter)` and
  identified by a `TextureId`. `Smooth` filtering blends pixels, `Pixelated`
  keeps them sharp (pixel art). Texture coordinates are clamped, so cells of a
  sprite sheet never bleed into their neighbors.
- `GameContext::load_texture(path, filter)` reads a PNG file through
  `star_assets::load_png` (any PNG kind becomes RGBA8). `Image` lives in
  `star_core`, so `star_assets` produces it and `star_render` consumes it
  without depending on each other (they share layer 2); `star_engine`
  connects them. Paths are relative to the working directory until Phase 5
  brings asset folders. sRGB targets get sRGB
  textures; other targets (the editor's) get the values unchanged, as with colors.
- Sprites (`Frame::draw_sprite(Sprite)`) are textured rectangles on the
  `z = 0` plane: position, size (negative mirrors), rotation around an
  anchor, tint, a `TextureRegion` (`TextureRegion::cell` for sprite sheets)
  and a layer. They are drawn after meshes, by layer and then in the order
  they were added; meshes in front hide them, but they do not hide each other
  through depth.
- **Sprite batching**: every frame, the sprites are sorted (unstable sort by
  layer and position: no allocation), their 4 corners written to one shared
  vertex buffer, and consecutive sprites with the same texture drawn by a
  single indexed call. Buffers are reused and only grow (doubling) when a
  frame has more sprites than any before.
- GPU tests (`star_render/src/gpu_tests.rs`, ignored by default) draw without
  a window and read the pixels back: `cargo test -p star_render -- --ignored`.

## Editor

A single native window with two screens. Its texts are in Spanish for now
(all of them in `texts.rs`, ready to be translated):

1. **Project manager** (`launcher/`): on the left, recent projects (an
   «Abrir» button; those that no longer exist appear as «No disponible», with
   «Quitar») and «Abrir otro proyecto…» (folder picker). On the right, **new
   project**: name, location (with «Examinar…»), 2D or 3D type (exclusive)
   and platforms (checkboxes). «Crear y abrir» creates
   `<location>/<name>/` with `project.toml` and `assets/` (it never
   overwrites) and opens the editor.
2. **Editor** (`workspace/`), maximized: menu («Archivo» → «Cerrar
   proyecto» / «Salir»; «Ayuda» → «Acerca de»), **Scene** panel (hierarchy, empty until `star_scene`
   exists), central **view** rendered by the engine (reference grid; drag =
   pan in 2D / orbit in 3D, wheel = zoom), **Inspector** (project
   properties), **Files** (tree of `assets/`, with «Actualizar») and a
   status bar.

The view uses `OffscreenRenderer` on egui's wgpu device and registers its
texture in `egui_wgpu` (it is registered again when its size changes). The
texture is `Rgba8Unorm` with colors already encoded in sRGB, because egui
samples textures as gamma values.

Modules: `project/` (validated model, creation and opening, no UI and with
tests), `recent_projects.rs` (recent projects list, at most 10, in
`<config>/ProjectStar/recent_projects.toml`), `folder_dialog.rs` (rfd),
`texts.rs` (every visible text, to translate them later) and `style.rs`
(shared visual constants).

Example of `project.toml`:

```toml
# ProjectStar project file.
[project]
name = "My Space Game"
engine_version = "0.1.0"
dimension = "3d"
platforms = ["windows", "linux", "android"]
```
