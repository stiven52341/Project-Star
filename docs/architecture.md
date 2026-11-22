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
  behavior (Unity style). See [Scenes](#scenes).
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
| 26 | PNG decoding with **png** (MIT/Apache-2.0), in `star_assets` | The image-rs project's decoder: maintained, pure Rust (it builds for every platform, mobile included) and light. PNG is lossless with transparency, the usual format for 2D game art. Only `star_assets` knows it. Rejected: the `image` crate (many formats, but much heavier to build; JPEG came in Phase 5, decision 30) |
| 27 | ECS with **sparse sets** and closure queries, without `unsafe` | Adding and removing components is O(1) and never moves other components, which suits a node tree where objects change often; going through one type reads contiguous memory. Per-type `RefCell`s let one query change some types while reading others, checked at run time instead of with `unsafe`. Queries take a closure (`World::each`) instead of returning an iterator, since an iterator that lends `&mut` components cannot be written in safe Rust. Rejected: archetype tables (faster iteration over many types, but adding a component moves the whole entity, and they need `unsafe` to be fast) and external ECS crates (decision 5 spirit: own facade, and the scene system will shape it) |
| 28 | The engine owns the **`World`**; `Time` and `Input` are resources | One source of truth for the game and its systems: a system that moves things reads the same time as `Game::update`, without copying state into the world each frame. The `Game` trait stays as the simple entry point (`update(&mut World)`, `draw(&World, &mut Frame)`), and grows into ECS systems when the game needs them. Rejected: an ECS left to each game (every game would wire the same loop) and passing time and input apart from the world (systems would need another signature) |
| 29 | **Asset paths** relative to an asset folder, with `/` | Games are copied to other systems and packaged for mobile, where absolute paths, `\` and `../` break or escape the folder; one validated form keeps paths portable and checked when they are written. The folder is found next to the executable first, so a shipped game finds its assets wherever it is launched from. Rejected: plain `std::path::Path` arguments (platform-dependent, and relative to the working directory) |
| 30 | JPEG decoding with **zune-jpeg** (MIT/Apache-2.0/Zlib) | Pure Rust, maintained, fast and light (one small dependency, `zune-core`); JPEG keeps photos and large backgrounds small. Hidden in `star_assets` like `png` (decision 26). Rejected: `jpeg-decoder` (no longer maintained) and the `image` crate (much heavier) |
| 31 | **Hot-reload by polling** modification times | Every 0.5 s, one `stat` per loaded file: nothing to install, the same on every platform, and no cost a game notices for hundreds of files. Rejected for now: the `notify` crate (operating system events: immediate, but a dependency with code per platform and its own threads; it can replace the watcher behind the same API if projects grow to thousands of files) |
| 32 | **Nodes are ECS entities** with `Parent` and `Children` components | One world holds everything, so systems query nodes like any entity and the tree costs nothing to objects outside it. Both directions are stored: children keep their order (drawing, editor) and the parent is found in O(1). Their fields are private and only `Hierarchy` changes them, so they never disagree. Rejected: a separate node tree with entities inside (two sources of truth) and only `Parent` (children would need a search of the world) |
| 33 | **Global transforms computed once per frame**, before drawing | One walk of the tree from the roots, with a reused work list (no allocations once warm), instead of recomputing ancestors every time a global transform is read. `Affine2` keeps skew from non-uniform scales exact. Rejected for now: dirty flags (they pay off with large static trees; they can be added behind the same components) and 3D transforms (with Phase 9, which brings quaternions) |
| 34 | `.scene` files in **TOML, a flat list of nodes** | Readable, diff-friendly in git and already used for projects (`serde` and `toml` are workspace dependencies). A flat list with parent positions avoids deep nesting, makes parents-before-children easy to check, and lets one scene file be read in one pass. Textures are named by asset path, so scenes do not depend on load order. Rejected: RON (another dependency) and nested tables (unreadable a few levels deep) |
| 35 | StarScript nodes are **script objects mirrored into the world** | Script objects are reference-counted Rust values that the generated code owns, and the world can only be lent to the engine's code, never kept by script objects without `unsafe`. So the script's tree of `Node` objects is the source of truth while the game runs: after the scripts' code ran each frame, the generated game describes the tree (parents first, texts borrowed, no copies) and `ScriptNodes` updates a world node per script node, destroying those that left. Scripts keep the simple field syntax (`x += 1.0`), and console programs do not depend on the engine (the `Node` module only uses the runtime). Images are named by path (`SpriteFile`) and loaded by the engine when it draws them. Rejected: handles to entities with getters and setters through a global world (needs the world outside the call stack that lends it) and copying every node's state into the script each frame |
| 36 | A **fallback logger** in `star_engine`, installed by `App::run_game` | Problems that do not stop a game (a missing texture) are reported through `log`; without a logger, a StarScript game had no way to show them. If the game installed no logger, a minimal one writes warnings and errors to the standard error output (`warning: ...`); a logger the game installed first is kept. Rejected: making the engine depend on `env_logger` (it stays for binaries that want filters and colors) |

## ECS

- `World` (`star_ecs`) holds entities, components and resources. Any
  `'static` type can be a component (at most one of each type per entity)
  or a resource (one value for the whole world: the time, the input, a score).
- Entities are handles with a generation (`Entity`): a destroyed entity's
  index is reused, but old handles stop being alive.
- Components of each type live in a **sparse set**: a dense array of values
  (contiguous to go through), the entity of each value, and a sparse array
  from entity index to position. Adding, removing and finding are O(1).
- Each component type (and each resource) sits in its own `RefCell`, so
  queries borrow several types at once without `unsafe`:
  `world.each::<(&mut Position, &Velocity)>(|entity, (position, velocity)| …)`.
  A query goes through the entities of its rarest component and looks up the
  others; it allocates nothing. Asking for a type twice with a `&mut`, or
  changing a type that a running query uses, is an `EngineError::Ecs`.
- Structural changes (spawn, despawn, add or remove components) need
  `&mut World`; reading and changing existing components only `&World`, so a
  query can read other entities (`world.get`) while it runs.
- A `Schedule` runs systems (`FnMut(&mut World) -> EngineResult<()>`:
  functions, or closures that keep their own state) once per frame, stage by
  stage — `First`, `Update`, `Physics`, `Last` — and, within a stage, in the
  order they were added. The first failing system stops the frame with its
  error. Systems get `&mut World`, so they can spawn and despawn directly.
- The engine owns the game's `World` and a `Schedule`. `Time` and `Input`
  are resources of the world (`EngineWorld` adds the shortcuts
  `world.time()` and `world.input()`), so the `Game` trait and the systems
  see the same data. Systems are registered with
  `App::with_system(Stage::Update, system)`; the game's own `update` runs
  right after `Stage::First`. `GameContext::world()` gives the world in
  `Game::start`, to spawn the first entities.

## Scenes

- A **node** is an entity of the world with a `Name` and its place in the
  tree: `Parent` and `Children` components (`star_scene`), changed only
  through the `Hierarchy` methods of `World` — `spawn_node(name, parent)`,
  `set_parent` (which rejects loops), `parent`, `children`, `find_child` and
  `despawn_node` (the whole subtree). Children keep the order they were
  added in. Any other component works on nodes as on any entity.
- A 2D node is placed by a `Transform2D` (position, rotation, scale)
  relative to its parent. Every frame, after the last stage and before
  drawing, the engine runs `TransformPropagation`, which walks the tree from
  the roots and stores each node's `GlobalTransform2D` (an `Affine2` of
  `star_math`); a node without `Transform2D` passes its parent's on. Systems
  that read global transforms see those of the end of the previous frame.
- A `SpriteNode` shows a texture at its node: the engine adds the sprites of
  every node with a global transform to the frame before `Game::draw`,
  moved, rotated and scaled with the node.
- A **`Scene`** is a tree of nodes that can be instanced many times
  (`scene.instantiate(&mut world, parent)` returns the copy's root; a failed
  copy leaves nothing behind). `GameContext::load_scene("levels/one.scene")`
  reads it from the asset folder and loads its textures, so the game can
  keep it and instance it later from any system, without the renderer.
  `Scene::capture(&world, root)` takes a subtree of a world, and
  `Scene::to_text` writes it, naming each texture by the file it came from.
- **`.scene` files** are TOML: a format version and a flat list of
  `[[node]]` tables, each with its name, the position of its `parent` in the
  list (parents come first; the first node is the only root), an optional
  transform (`position`, `rotation`, `scale`) and an optional
  `[node.sprite]` (`texture` as an asset path, `filter`, `size`, `region`,
  `anchor`, `color`, `layer`). Unknown fields are errors, so typos are not
  silently ignored. Scenes keep names, transforms and sprites; other
  components (and the StarScript classes of nodes) will join them later.
- A `SpriteFile` names a sprite's image by its asset path instead of a
  `TextureId`: the engine loads the texture the first time it draws it (and
  again when the path or filter change), so code without the renderer, such
  as systems and StarScript nodes, can give nodes images. An image that
  cannot be loaded logs one warning and is not drawn.
- StarScript nodes (`Node`, `Sprite`) live in the script; after the scripts
  ran each frame, the generated game describes its tree to `ScriptNodes`
  (`star_engine::script_nodes`), which keeps one world node per script node
  (decision 35).

## Assets

- A game's assets live in its **asset folder** (`AssetFolder`, in
  `star_assets`): `assets/` next to the executable when it exists (a shipped
  game), otherwise `assets/` in the working directory (development), or the
  folder given to `App::with_asset_folder`. Editor projects already create it.
- Assets are named by an **`AssetPath`**, e.g. `"sprites/hero.png"`:
  relative, always with `/`, and without `.`, `..`, `\` or `:`. The same
  path works on every platform and can never point outside the folder.
- Images are decoded by the `ImageFormat` of their extension (any case):
  PNG through `png`, JPEG (`.jpg`, `.jpeg`) through `zune-jpeg`. Both
  always give RGBA8; errors name the asset path and the file.
- **Caching**: `GameContext::load_texture` keeps a `TextureLibrary` (in
  `star_engine`) from `(path, filter)` to `TextureId`, so loading a file
  again returns the same texture without decoding or uploading it. A failed
  load is not remembered: it is tried again.
- **Hot-reload** (`App::with_hot_reload`, on by default in debug builds):
  a `FileWatcher` (in `star_assets`) looks at the modification time of every
  loaded file every 0.5 s of game time; a changed file is decoded again and
  replaces its texture's image, so the sprites that use it change on the next
  frame. A file that cannot be read or decoded yet (an image editor still
  writing it) only logs a warning and keeps the last image.
- Sounds are loaded in Phase 7, together with their playback.

## Input

```
OS ──winit──▶ star_platform (InputEvent) ──▶ star_input::Input (world resource) ──▶ game and systems
```

- Events arrive between frames and are applied to `Input` as they come.
  After the game and every system ran, `Input::end_frame` forgets what was *just* pressed,
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
- A frame's cycle: textures whose files changed are reloaded (hot-reload) →
  systems of `Stage::First` → `Game::update(&mut World)` → systems of
  `Update`, `Physics` and `Last` → global transforms of the nodes → sprites
  of the sprite nodes → `Game::draw(&World, &mut Frame)` →
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
- `GameContext::load_texture(path, filter)` loads an image from the asset
  folder (see [Assets](#assets)). `Image` lives in `star_core`, so
  `star_assets` produces it and `star_render` consumes it without depending
  on each other (they share layer 2); `star_engine` connects them. sRGB
  targets get sRGB textures; other targets (the editor's) get the values
  unchanged, as with colors.
- `Renderer::replace_texture(id, &image, filter)` gives a texture a new image
  under the same `TextureId` (hot-reload uses it).
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
