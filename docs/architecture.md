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
         star_blueprint ── blueprints: node graphs, `.blueprint` files
                        (serde + toml) and their translation to StarScript
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
| 24 | StarScript input: actions in **`project.toml`**, read through `star_script_runtime::input` | Actions are project settings (`[input.actions]`), as an editor panel will show them, so scripts stay clean and the compiler can reject misspelled action names (it receives them in `GameSettings`). `Input.is_down("jump")` is a static call with no object to read the engine's input from, so the generated game copies each action's state into a per-thread table of the runtime once per frame (no allocations; the runtime stays without dependencies) and the calls read it by name. Each action's strength, for `Input.strength` and `Input.axis`, and the pointer are copied the same way, the pointer already in world units: it is converted with the camera the previous frame was drawn from, which is what the player saw when they pointed. Rejected: declaring actions in scripts (typos in key names only fail while playing) |
| 25 | Sprites **batched by texture**, in an immediate `Frame` | The game adds sprites every frame (like meshes, decision 9); the renderer groups consecutive sprites that share a texture into one draw call. The order is explicit (layers, then order of addition), so translucent sprites always blend correctly without sorting by depth. Rejected for now: texture arrays or bindless textures (not available on OpenGL ES and older GPUs) and automatic atlases (they belong to the asset pipeline, Phase 5) |
| 26 | PNG decoding with **png** (MIT/Apache-2.0), in `star_assets` | The image-rs project's decoder: maintained, pure Rust (it builds for every platform, mobile included) and light. PNG is lossless with transparency, the usual format for 2D game art. Only `star_assets` knows it. Rejected: the `image` crate (many formats, but much heavier to build; JPEG came in Phase 5, decision 30) |
| 27 | ECS with **sparse sets** and closure queries, without `unsafe` | Adding and removing components is O(1) and never moves other components, which suits a node tree where objects change often; going through one type reads contiguous memory. Per-type `RefCell`s let one query change some types while reading others, checked at run time instead of with `unsafe`. Queries take a closure (`World::each`) instead of returning an iterator, since an iterator that lends `&mut` components cannot be written in safe Rust. Rejected: archetype tables (faster iteration over many types, but adding a component moves the whole entity, and they need `unsafe` to be fast) and external ECS crates (decision 5 spirit: own facade, and the scene system will shape it) |
| 28 | The engine owns the **`World`**; `Time` and `Input` are resources | One source of truth for the game and its systems: a system that moves things reads the same time as `Game::update`, without copying state into the world each frame. The `Game` trait stays as the simple entry point (`update(&mut World)`, `draw(&World, &mut Frame)`), and grows into ECS systems when the game needs them. Rejected: an ECS left to each game (every game would wire the same loop) and passing time and input apart from the world (systems would need another signature) |
| 29 | **Asset paths** relative to an asset folder, with `/` | Games are copied to other systems and packaged for mobile, where absolute paths, `\` and `../` break or escape the folder; one validated form keeps paths portable and checked when they are written. The folder is found next to the executable first, so a shipped game finds its assets wherever it is launched from. Rejected: plain `std::path::Path` arguments (platform-dependent, and relative to the working directory) |
| 30 | JPEG decoding with **zune-jpeg** (MIT/Apache-2.0/Zlib) | Pure Rust, maintained, fast and light (one small dependency, `zune-core`); JPEG keeps photos and large backgrounds small. Hidden in `star_assets` like `png` (decision 26). Rejected: `jpeg-decoder` (no longer maintained) and the `image` crate (much heavier) |
| 31 | **Hot-reload by polling** modification times | Every 0.5 s, one `stat` per loaded file: nothing to install, the same on every platform, and no cost a game notices for hundreds of files. Rejected for now: the `notify` crate (operating system events: immediate, but a dependency with code per platform and its own threads; it can replace the watcher behind the same API if projects grow to thousands of files) |
| 32 | **Nodes are ECS entities** with `Parent` and `Children` components | One world holds everything, so systems query nodes like any entity and the tree costs nothing to objects outside it. Both directions are stored: children keep their order (drawing, editor) and the parent is found in O(1). Their fields are private and only `Hierarchy` changes them, so they never disagree. Rejected: a separate node tree with entities inside (two sources of truth) and only `Parent` (children would need a search of the world) |
| 33 | **Global transforms computed once per frame**, before drawing | One walk of the tree from the roots, with a reused work list (no allocations once warm), instead of recomputing ancestors every time a global transform is read. `Affine2` keeps skew from non-uniform scales exact. Rejected for now: dirty flags (they pay off with large static trees; they can be added behind the same components) and 3D transforms (they came with Phase 9: `Transform3D`, `Quat`) |
| 34 | `.scene` files in **TOML, a flat list of nodes** | Readable, diff-friendly in git and already used for projects (`serde` and `toml` are workspace dependencies). A flat list with parent positions avoids deep nesting, makes parents-before-children easy to check, and lets one scene file be read in one pass. Textures are named by asset path, so scenes do not depend on load order. Rejected: RON (another dependency) and nested tables (unreadable a few levels deep) |
| 35 | StarScript nodes are **script objects mirrored into the world** | Script objects are reference-counted Rust values that the generated code owns, and the world can only be lent to the engine's code, never kept by script objects without `unsafe`. So the script's tree of `Node` objects is the source of truth while the game runs: after the scripts' code ran each frame, the generated game describes the tree (parents first, texts borrowed, no copies) and `ScriptNodes` updates a world node per script node, destroying those that left. Scripts keep the simple field syntax (`x += 1.0`), and console programs do not depend on the engine (the `Node` module only uses the runtime). Images are named by path (`SpriteFile`) and loaded by the engine when it draws them. Rejected: handles to entities with getters and setters through a global world (needs the world outside the call stack that lends it) and copying every node's state into the script each frame |
| 36 | A **fallback logger** in `star_engine`, installed by `App::run_game` | Problems that do not stop a game (a missing texture) are reported through `log`; without a logger, a StarScript game had no way to show them. If the game installed no logger, a minimal one writes warnings and errors to the standard error output (`warning: ...`); a logger the game installed first is kept. Rejected: making the engine depend on `env_logger` (it stays for binaries that want filters and colors) |
| 37 | StarScript `Color` is an **object**, and the camera is a **node** | StarScript has no value types yet, so `Color` follows the language's one rule for classes (shared references): two sprites given the same color change together, which is documented. A `Camera` node follows its parents for free (child of the ship = follows the ship) and `Game.camera` says which one is used; its world position comes from the mirrored node's global transform, its rotation is ignored so the view never tilts, and views are never thinner than 0.001 units so a wrong height cannot hide the frame. Rejected: loose `red`/`green`/… and `camera_x`/`camera_y` fields (verbose, and following a node would need code in every game) |
| 38 | StarScript scenes are read **through a loader the game registers** in the runtime | The generated `Scene` class only uses `star_script_runtime`, so console programs still do not depend on the engine. The generated game registers, before the game object is created, a loader that reads `SceneOutline`s from the app's asset folder and turns them into the runtime's plain `SceneNode` list; `Scene.load` builds `Node`/`Sprite` objects from it. Results are kept per path (failures too, so a missing file warns once). A scene that cannot be loaded gives `null`, not a stopped game, so the error rule of decision 4 holds without a new `fault` case. Rejected: reading files in the generated `Node` module (console programs would need the engine) and `Game.load_scene` (nodes could not load scenes). Sounds follow the same idea: the generated game registers an `AudioBackend` (`star_script_runtime::audio`) that loads files from the asset folder and plays them on the engine's `Audio`, given to it in `start` (before, it is silent), so `Sound.load` works from field values on |
| 39 | Scene nodes name StarScript classes **by name, resolved by generated code** | A `.scene` file is data, so the class is a text (`class = "Rock"`). The compiler writes `new_scene_object_`, a `match` over the program's classes: the node classes created without arguments are built with `new()`, and the others give the reason they cannot (needs arguments, private `init`, not a node). The object is created as its `init` leaves it and the scene's values are applied on top (name always; transform and image only when the file writes them), as an instance overrides its prefab. Any problem rejects the whole scene (`null`, one warning naming the node), so a typo is never half-loaded. The format version stays 1: the field is optional and old files read the same. Rejected for now: checking the project's scenes at compile time (it fits the editor's build, step 13.6) and passing arguments to `init` from the scene |
| 40 | The editor **checks** projects with `star_script` as a library | “Compile” gives every mistake the compiler can find (scripts, input actions, scene classes) in a fraction of a second, without Cargo, so it can run often. Scripts live anywhere in `assets/`, next to the scenes and images they use, and all of them form one program: each file is tokenized and parsed on its own (an unclosed `{` stays in its file) and their classes are checked together, with spans placed in one range of offsets that `SourceSet` maps back to file, line and column. The check runs on a thread so big projects never freeze the editor. Rejected: building with Cargo as part of “Compile” (minutes instead of a fraction of a second; “Play” does it, see 44) and a `scripts/` folder apart from the assets |
| 41 | Blueprints are **node graphs whose execution is a tree**, in their own crate `star_blueprint` | Each execution pin takes at most one wire, so every graph reads as structured StarScript (`if`/`else`) without jumps; paths that would join again are written as functions. Values flow through data wires and pure nodes run where their value is used. The model is plain data with public fields, like the syntax tree, because the visual editor must hold graphs that are still being built; `validate` lists what is wrong, and types are left to the StarScript compiler, whose errors are placed back on nodes. The files are TOML, like scenes, read with serde, so `star_script` keeps having no dependencies. Rejected: free execution graphs with joins and loops of wires (they need gotos or a structuring pass), and a block editor in the style of Scratch (less familiar for engine users) |
| 42 | The blueprint editor is **drawn by the editor** with egui's painter, in tabs of the central area | No node-editor library: the canvas is a few hundred lines on top of `star_blueprint` (pins, rules, files), which already says what a node is, so there is no second model to keep in step, and no dependency that must follow each egui version. Blueprints open as tabs next to the scene view, so several can be open and the inspector shows what is selected in the tab shown. Rejected: `egui-snarl` and similar crates (their own graph model and egui version), and floating windows (they cover the other panels) |
| 43 | The editor's texts are **plain Rust values, one per language**, chosen through a global current language | The editor shows a few hundred short texts, so a `Texts` struct with one field per text, filled once per language, is enough: a missing translation does not compile, there is nothing to parse at start-up and no dependency. Texts with values are methods that pick the language's wording, so plurals and word order stay natural in each one. The language lives in an atomic, because every panel draws texts and passing it everywhere would touch every function for no gain; tests use a language's `Texts` directly and never change it. English is the default, and a file written with an unknown language falls back to it. Rejected for now: Fluent or gettext catalogs (a dependency and a runtime format, worth it with many languages or translators who do not read Rust) and following the system's language (it needs one more dependency to read it on every platform) |
| 44 | “Play” **builds the game with the system's Cargo**, from the engine's crates of the repository | The game a project makes is the same Rust a shipped game will be, built the same way, so what is played is what ships; no interpreter to keep in step with the compiler. The build runs off the UI thread and shares one target folder, so only the first “Play” builds the engine. It needs Rust installed and the editor run from the repository (a compile-time path to `crates/`): enough while the engine is developed. Rejected for now: shipping prebuilt engine libraries with the editor (comes with packaging), and running the game inside the editor's window (a game is its own process, so a crash or `quit()` never takes the editor with it) |
| 45 | Sound decoding with **symphonia** (MPL-2.0), in `star_assets`; playback with **cpal** (Apache-2.0) and an **own mixer** in `star_audio` | symphonia is pure Rust (it builds for every platform, mobile included), maintained, and reads WAV, OGG Vorbis, MP3 and FLAC with one API; only the needed formats are enabled. Sounds are decoded whole into memory (`Sound`), simple and fast to start; long music may stream from the file later. cpal only opens the system's audio output (on Linux it needs the ALSA headers to build, `libasound2-dev`); mixing (voices, volumes, loops, groups) is the engine's own code, so it can be tested without a device and shaped by the engine's needs. Rejected: `rodio` and `kira` (they bring their own mixer and model), and one decoder crate per format (`hound`, `lewton`…: several APIs for the same job) |
| 46 | 2D physics with **rapier2d** (Apache-2.0), hidden in `star_physics` | Pure Rust (every platform, mobile included), maintained, and mature: stable stacks, sensors, sleeping bodies, joints and continuous collision for later. The engine's own `Physics2d`, `BodySettings`, `Shape2d` and `Contact` are all games see, so the solver can be replaced. Time advances in **fixed steps** (1/60 s, at most 8 per frame), so a game behaves the same at any frame rate. Rejected: an own solver (stable stacking and many bodies are hard to get right) and `box2d` bindings (C code to build on every platform). It needs Rust 1.86 |
| 47 | A StarScript body is a **`Body` object in a field of `Node`**, synced both ways | A node of any class (a `Sprite` too) can have one, as with `Body2D` and `[node.body]`. The simulation runs after the scripts, so before their next frame the generated game copies back the position and rotation of each node with a body, its velocity and what it touches (by `ScriptNodes::read_bodies` and `body_state`); a velocity the script changed and the impulses it gave are passed on through `Body2D`, which keeps them until the body exists. Kinds and shapes are words (`"dynamic"`, `"circle"`), since StarScript has no enums yet; a body the physics cannot simulate is left out with one warning, so a typo never stops the game. Contacts are polled lists (`touching`, `just_touched`, `just_separated`), like `Input`'s actions, and hold their nodes weakly. Rejected: a `Body` node class (a sprite could not be a body, and scenes put both on one node), callbacks on `Node` (every node class would grow), and calls into the physics from scripts (the world is only lent to the engine's code) |
| 48 | 3D materials are **PBR metallic-roughness**, lit by a few lights per frame in one pass | The model of glTF, Unity, Godot and Unreal, so imported models will look as their authors made them; the cost over Blinn-Phong is a few shader lines. Lights are a uniform array of 16 read by every 3D draw (forward shading): simple, every backend runs it, and enough for the games of this phase. Rejected: Blinn-Phong (models would need converting later), deferred shading and clustered lights (worth it with hundreds of lights; they can come behind the same `Frame` API), and shadows for now |
| 49 | 3D physics with **rapier3d** (Apache-2.0), beside rapier2d in `star_physics` | The 3D sibling of rapier2d, of the same authors and version: it shares parry, glamx and nalgebra, so the dependency tree barely grows, and `Physics3d` can have the same API as `Physics2d` (`BodySettings` and `Contact` are generic over the shape and the body id; the fixed-step clock and the contact bookkeeping are shared code). Rejected: avian3d (built on Bevy's ECS) and an own solver (as in decision 46) |
| 50 | StarScript's 3D nodes are **`Node3D` extends `Node`**, and bodies stay one `Body` class | A 3D node keeps `x`, `y`, the scales and the whole tree API of `Node` (`add_child`, `children`, `find_child`, `Scene.load`), with `rotation` as its roll; `Node3D` adds `z`, `pitch`, `yaw`, `scale_z` and `look_at`, and `Mesh`, `Light` and `Camera3D` extend it. A `Body` is a 2D body on a 2D node and a 3D one on a `Node3D` (`Body.sphere`, `Body.box`, `depth`, `velocity_z`, `apply_impulse_3d`); a shape of the other dimension is left out with one warning. Rejected: a separate 3D hierarchy (the game and every node would need a second `add_child`, and scenes could not mix them) and a `Body3D` class (the whole API of `Body` twice, and two body fields on 3D nodes) |
| 51 | Android games are a **NativeActivity** library, packaged with **cargo-apk2** | winit's `android-native-activity` feature runs the event loop in the `AndroidApp` the system starts (`android_main`), with no Java code to write or Gradle to run; `cargo-apk2` (MIT/Apache-2.0, the maintained successor of `cargo-apk`) builds the `cdylib`, the manifest from `[package.metadata.android]`, and signs the APK. A game keeps its desktop `main` and adds `star_engine::android_main!(run)` to its library (the only `unsafe` the project allows, approved by its owner: `#[unsafe(no_mangle)]` on that one function, because the system looks the entry up by its plain name; see `CLAUDE.md` 3.1 and `star_platform/src/entry.rs`); `star_platform::android` keeps the app for the event loop and sends the log to logcat (`android_logger`, MIT/Apache-2.0). Assets are read from the APK's `assets` folder (`[package.metadata.android] assets`) through Android's `AssetManager`: `star_platform::PackagedAssets`, given to `star_assets` as `PackagedFiles`, which `App` uses by default on Android (there are no files to watch, so no hot-reload there); `App::with_development_assets` sets a folder on disk only where assets are not packaged, so examples find theirs both ways. Every example is a library (`cdylib` and `rlib`) with `android_main!`, plus a desktop `main`; the CI checks them for Android and builds an APK. When Android takes the window's surface away, `WindowEventHandler::on_suspended` makes the renderer drop it and `on_resumed` makes a new one, keeping textures and meshes. Rejected: GameActivity (needs a Java/Kotlin part built with Gradle), `xbuild` (less maintained) and a Gradle project per game |
| 52 | The editor edits scenes as **`SceneOutline` data**, not as a live world | An outline is the file's own model (images and meshes named, not loaded), so what the inspector changes is exactly what is saved, with no `SceneSources` to trace back and nothing lost on the way (classes, unknown images). It also needs no GPU to edit. Its tree is kept valid by `add_node` (appended after every node) and `remove_node` (whole subtrees, renumbering the rest), and `to_text` checks it again before writing. `Scene::to_text` goes through outlines too, so there is one writer. Rejected: instancing the scene into an ECS `World` and capturing it to save (it loses what worlds do not keep, like classes, and ties editing to rendering) |
| 53 | The code editor is **egui's `TextEdit` with a layouter**, colored by **the compiler's own lexer** (`star_script::scan`) | `scan` is `tokenize` that keeps going after mistakes and notes where the comments are, so half-written code is colored by the same rules that compile it (numbers, escapes, nested comments), with nothing to keep in step. `TextEdit` already gives the cursor, selection, clipboard, undo and input methods. Rejected: a second, hand-written highlighter in the editor, and a separate code-editor crate (one more dependency to follow egui's versions) |
| 54 | StarScript's concurrency is **coroutines on the game's thread** (`async`/`await`), and its parallelism **`parallel static func`s that only see copied values** | Objects, lists and nodes are `Rc` and `RefCell` (decision 19): sharing them between threads would need `Arc` and locks on every field read, slower for every game and with deadlocks that StarScript could not explain. Most game code that “waits” (blink, cooldowns, cutscenes) only needs to continue in a later frame, so coroutines are Rust `async` methods polled once per frame by an executor of `star_script_runtime` (no dependency), on the same thread, touching anything. Heavy work (paths, terrain) goes to `parallel` functions: the compiler allows them only values, copies the arguments into a thread of the runtime (`std::thread`) and the result back, and a coroutine awaits it. `async` is explicit, so a method's signature says whether it waits and overrides are checked. Coroutines of a node end when it leaves the tree. Rejected: threads over shared objects (above), coroutines without `async` inferred from `await` in the body (the signature would depend on the body), `parallel for` loops (for later, if games need them) and a crate such as rayon or tokio (std is enough) |
| 55 | StarScript has **three integer types and two decimal types**: `short` (`i16`), `int` (`i32`, the usual one), `bigint` (`i64`), `float` (`f32`, the usual one) and `double` (`f64`); a number **widens by itself only where no value is lost** | Most integers of a game (lives, scores, indices, layers) fit in 32 bits, which halves their memory in lists and fields and matches the engine (a sprite's layer is an `i32`); `double` is there for the rare case that needs the precision. `int` and `float` stay the types literals get, so most code never names the others. Widening (`short` → `int` → `bigint`, `float` → `double`, `short` → `float`, `int` → `double`) happens on assignment, arguments, `return`, `T?` and between the two sides of an operator, whose result is the wider type; it is lossless, so it can never surprise. Conversions that may lose values (narrowing, `int` → `float`, `bigint` → `double`) need `as`, so they stay visible. Lists never convert: a `[short]` seen as a `[int]` would be the same shared list with two element types. An integer literal takes the number type its place needs and must fit in it, checked when compiling (`-32768` is a valid `short`). Integers wrap around on overflow. List indices and `length()` are `int`; a range's values have the common type of its ends. Rejected: `int` → `float` by itself (as in C# and Java: it silently rounds integers above 16777216), unsigned types (a negative result would wrap to a huge number, a common bug in games) and `byte` (rarely needed in game logic; it can come later) |
| 56 | “Show in file explorer” **starts the system's own program**, in `star_platform::show_in_file_explorer` | Every desktop system already has one: `explorer /select,` on Windows and `open -R` on macOS select the file; on Linux the freedesktop `FileManager1.ShowItems` call (through `dbus-send`) selects it in Nautilus, Dolphin, Nemo and the like, and `xdg-open` on its folder is the fallback for the rest. Android and iOS have no file manager to show, so it is an error there. The editor calls it on a thread of its own, since the program may take a while to answer, and only logs a failure. Rejected: a crate such as `opener` or `open` (a dependency for a few lines, and it is platform code, which belongs in `star_platform` anyway) |
| 57 | Blueprint ready-made functions come from StarScript's own table of the engine's classes | The blueprint editor lists `star_script::semantic::native_classes` instead of a list of its own: one place says what the engine offers, to scripts and blueprints alike, and nothing can fall out of step. Labels are how the code is written (`Input.is_down(action) -> bool`), in every language. Rejected: a hand-written catalog with translated names (a second list to keep, and names that differ from the code users read) |
| 58 | The program's decorators are **rewritten in the syntax tree** before checking (`star_script::decorated`) | StarScript has no functions as values, so a decorator cannot wrap a method as a closure. Rewriting the method into a hidden one called between `before` and `after` of decorator objects kept in hidden fields reuses all the checks and the translation, with mistakes still pointing at the user's code. `Decorator` is a StarScript class added to every program, not a native one, so classes can extend it without a hand-written Rust module. Hidden names carry `@`, which no written name has, and become Rust names of the generator's own (`open_body_`). Rejected: closures in the language (a large change for one feature) and native hooks in the generated Rust (another path to keep in step with the translation) |
| 59 | Cameras are **parts of nodes with a `current` flag**, and the 2D one is `Camera2D` | A scene must be able to bring its own point of view, as in Godot, without code. `[node.camera]` and `[node.camera_3d]` are components like `[node.light]`; a camera marked `current` becomes `Game.camera`/`camera_3d` as it enters the game's tree (just before its `start`), so `Game.camera` keeps saying which one is used and scripts can still change it. The editor marks only the first camera created as `current`. `Camera` was renamed `Camera2D` to pair with `Camera3D` (owner's choice, 2026-11-24). Rejected: choosing the camera each frame by searching the tree (a walk every frame, and `Game.camera` would not tell the truth); a project setting naming the camera (scenes would not carry it). |
| 60 | Nodes are found **by walking the tree when asked**, and blueprints got a **For each** node | `find_by_name`/`find_by_tag` walk a node's descendants (or the game's, for the static ones of `Scene`, through the root the generated game already keeps), parents first. Games have few nodes and search rarely, so no index is kept that every rename, tag change or move would have to update. Both scopes exist (owner's choice, 2026-11-24): `Scene.*` from any script, `node.*` inside a part of the tree. `find_by_tag` gives a list, and blueprints had no loops, so `for_each` became a node: its body is a nested path, like a branch's, which keeps the execution a tree (decision 41). Rejected: a map of tags kept up to date (more state for little gain), and a first-tagged-only search (a list covers it). |

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
- The engine owns the game's `World` and a `Schedule`. `Time`, `Input`
  and the window's `WindowSize` are resources of the world (`EngineWorld`
  adds the shortcuts `world.time()`, `world.input()` and
  `world.window_size()`), so the `Game` trait and the systems see the same
  data. Systems are registered with
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
- A 3D node is placed by a `Transform3D` (position, a `Quat` rotation and
  scale); the same walk stores its `GlobalTransform3D` (a `Mat4`). 2D and 3D
  transforms pass through each other untouched, so a tree can hold both.
  Rotations are unit quaternions, which combine without gimbal lock;
  `Quat::from_euler` and `to_euler` turn them into pitch, yaw and roll
  (roll first, then pitch, then yaw) for people and files.
- A `SpriteNode` shows a texture at its node: the engine adds the sprites of
  every node with a global transform to the frame before `Game::draw`,
  moved, rotated and scaled with the node.
- A `MeshNode` (a `Mesh3dId` and a `Material`) shows a 3D mesh at its 3D
  node, and a `LightNode` lights the meshes from it: `Directional` shines
  along the node's -z axis (forward), `Point` from its position. The engine
  adds them to the frame after the sprites and before `Game::draw`
  (`draw_3d_nodes`); light nodes beyond `MAX_LIGHTS` are left out, and the
  ambient light stays on the `Frame`.
- A **`Scene`** is a tree of nodes that can be instanced many times
  (`scene.instantiate(&mut world, parent)` returns the copy's root; a failed
  copy leaves nothing behind). `GameContext::load_scene("levels/one.scene")`
  reads it from the asset folder and loads its textures, so the game can
  keep it and instance it later from any system, without the renderer.
  `Scene::capture(&world, root)` takes a subtree of a world, and
  `Scene::to_text` writes it, naming each texture by the file it came from.
- A **`SceneOutline`** is a scene file read and checked, with each image
  named by its asset path instead of loaded (`SceneOutline::read(&folder,
  &path)`). `Scene::from_text` is an outline whose images were loaded; code
  that builds its own nodes, such as StarScript games, reads outlines. The
  editor changes them (`node_mut`, `add_node`, `remove_node`, which keep
  parents first) and writes them with `SceneOutline::to_text`, which
  `Scene::to_text` also uses once each texture is named by its file.
- **`.scene` files** are TOML: a format version and a flat list of
  `[[node]]` tables, each with its name, an optional StarScript `class`
  (the script class it is built as; Rust games ignore it, but scenes keep
  it when written again), the position of its `parent` in the
  list (parents come first; the first node is the only root), an optional
  transform (`position`, `rotation`, `scale`), an optional
  `[node.sprite]` (`texture` as an asset path, `filter`, `size`, `region`,
  `anchor`, `color`, `layer`) and an optional `[node.body]` (see
  [Physics](#physics)). 3D nodes have an optional `[node.transform_3d]`
  (`position`, `rotation` as pitch, yaw and roll in radians, `scale`), an
  optional `[node.mesh]` (`shape` `cube` with a `size` of three numbers,
  `plane` with two, or `sphere` with a `radius` and optional `segments` and
  `rings`; and its material: `color`, `metallic`, `roughness`, `texture`,
  `filter`) and an optional `[node.light]` (`kind` `directional`, `point`
  or `spot`, `color` of three numbers, `intensity`, the `range` of point
  lights and spots, and the `angle` in radians and `blend` of spots)
  and an optional `[node.body_3d]` (see Physics). Cameras are an optional
  `[node.camera]` (`view_height`, `current`) on 2D nodes and
  `[node.camera_3d]` (`fov` in radians, `current`) on 3D ones; what they
  leave out takes its default, and the editor draws what each one sees.
  The editor also draws the shape of every body (`[node.body]`,
  `[node.body_3d]`), placed and turned as its node but not scaled, as the
  physics has it; its “Physics” menu creates static, rigid and kinematic
  bodies and areas (static sensors, which detect every kind of body).
  Nodes may have `tags = ["enemy", …]` (the `Tags` component in worlds).
  Unknown fields are errors, so typos are not silently ignored. Scenes keep
  names, classes, transforms, sprites, bodies, meshes and lights; other
  components will join them later.
- Reading a `Scene` takes a `SceneAssets` (it loads textures and builds the
  meshes of shapes; the engine builds each `MeshShape` once), and writing
  one a `SceneSources` (which file or shape each texture and mesh came
  from: those made in code cannot be saved).
- A `SpriteFile` names a sprite's image by its asset path instead of a
  `TextureId`: the engine loads the texture the first time it draws it (and
  again when the path or filter change), so code without the renderer, such
  as systems and StarScript nodes, can give nodes images. An image that
  cannot be loaded logs one warning and is not drawn.
- A `MeshFile` does the same for 3D: a `MeshShape` with its sizes, a PBR
  surface and an optional image path, built and loaded by the engine when
  it draws it. Every shape of a kind is drawn from one mesh of size 1,
  scaled, so sizes that change every frame build nothing new.
- StarScript nodes (`Node`, `Sprite`) live in the script; after the scripts
  ran each frame, the generated game describes its tree to `ScriptNodes`
  (`star_engine::script_nodes`), which keeps one world node per script node
  (decision 35). A script node's `Body` becomes a `Body2D`, and what the
  simulation did is copied back before the scripts' next frame (decision
  47). A `Node3D` gets a `Transform3D` (its pitch, yaw and roll make a
  `Quat`), a `Body3D` for its `Body`, a `MeshFile` if it is a `Mesh` and a
  `LightNode` if it is a `Light`; the game is seen from `Game.camera_3d`
  when it has one (decision 50).

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
- Sounds are decoded by the `SoundFormat` of their extension (any case):
  WAV, OGG Vorbis (`.ogg`, `.oga`), MP3 and FLAC, all through `symphonia`
  (decision 45). The whole file becomes a `Sound` (`star_core`): its
  samples as `f32` from -1 to 1, interleaved by channel, with its channel
  count (1 to 8) and sample rate. Like `Image`, it lives in `star_core` so
  `star_assets` produces it and `star_audio` will play it without depending
  on each other. Damaged packets are skipped; a sound that changes its
  channels or rate midway is an error. Playback comes in the next steps of
  Phase 7.

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
- `Input::pointer` is where the player points, for games that aim or drag
  with mouse and fingers alike: the first listed finger (on the screen, or
  leaving it this frame) wins over the cursor. `Camera::screen_to_world_2d`
  turns such a pixel into a point of the 2D world.
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
  finger on the screen), `Binding::GamepadButton` (that button on any
  gamepad) or `Binding::GamepadStick` (a stick of any gamepad leaning one
  way, held from `STICK_PRESS_THRESHOLD`, half its travel). On-screen
  virtual buttons (touch regions) will come later. An action is held while **any** of its bindings is
  held: it keeps a count of held bindings, so pressing a second binding of a
  held action is not a new press. Actions are asked for by name
  (`input.actions().just_pressed("jump")`); an unknown name answers `false`.
- **Analog actions**: `Input::action_strength` gives how strongly an action
  is held, from 0 to 1: the strongest of its bindings, computed when asked
  from the devices' state (no copy per frame). Keys and buttons give 1 while
  held, triggers their travel (1 for a held trigger without travel), stick
  directions how far the stick leans that way. Holding (`is_down`) stays a
  yes or no, so code written for keys works with sticks unchanged.

## Audio

- `star_audio` plays `Sound`s (decoded by `star_assets`, see
  [Assets](#assets)) through its own **mixer** (decision 45). A `Mixer` is
  made for an `OutputFormat` (channels and sample rate of the output) and
  `Mixer::mix` fills a buffer of interleaved `f32` samples with the next
  samples of every sound playing; the output device only asks for those
  buffers, so the mixer is tested without one.
- `Mixer::play(Arc<Sound>, PlaySettings)` starts a **voice** and gives its
  `VoiceId` (never reused) to stop it or change its volume. `PlaySettings`
  holds the volume (1 as recorded; negative or not-a-number is silent) and
  whether it loops. Voices that end are forgotten. Sounds are shared with
  `Arc`, since the mixer will run on the audio thread.
- Each voice is read at the output's rate with **linear interpolation**
  between the sound's frames. Channels: a mono sound goes to every channel,
  a mono output hears the average of every channel, and otherwise each
  channel goes to its own (extra output channels are silent). Every voice
  is scaled by its volume and the master volume, added, and clipped to
  -1..1.
- At most `MAX_VOICES` (128) sounds play at once, reserved up front so
  starting one never allocates; more is `EngineError::CapacityExceeded`.
- `AudioOutput::open_default` opens the system's default device through
  `cpal` (only `audio_output.rs` knows it), in the device's preferred
  channels, rate and sample format (`f32`, `f64`, `i16`, `i32`, `u16` or
  `u8`; the mixer always works in `f32`). The device's thread calls back for
  samples and locks the mixer (`Arc<Mutex<Mixer>>`) only while it mixes;
  the game locks it only to start, stop or change a sound. A poisoned lock
  is taken back: every change to the mixer is one step, so it stays whole.
  The stream lives in `AudioOutput`, which stays on the thread that opened
  it, because some systems require it.
- Games play sounds through **`Audio`**, a cheap handle (copies share the
  output) that the engine keeps as a world resource (`EngineWorld::audio`).
  `App::run_game` opens the output before the event loop and keeps it until
  the window closes. Without an audio device it logs a warning and the game
  runs silent: `Audio::silent()` accepts every call, hears nothing and never
  reports a sound as playing, so games need no special case.
- `GameContext::load_sound(path)` decodes a whole sound from the asset
  folder into an `Arc<Sound>`, kept by a `SoundLibrary` (in `star_engine`)
  so a file is decoded once; a failed load is tried again. Sounds have no
  hot-reload yet. Example: `sounds_2d`.

## Physics

- `star_physics` simulates 2D bodies with rapier2d (decision 46). A
  `Physics2d` holds them: `add_body(&BodySettings)` gives a `BodyId`, and
  ids of removed bodies are simply ignored.
- **3D**: a `Physics3d` does the same in space with rapier3d (decision 49):
  `add_body(&BodySettings<Shape3d>, position, rotation)` gives a
  `BodyId3d`, positions and velocities are `Vec3`s and rotations `Quat`s,
  gravity defaults to `DEFAULT_GRAVITY_3D` (-y), and its contacts are
  `Contact<BodyId3d>`. A `Shape3d` is a sphere, a box (`Shape3d::cuboid`)
  or a vertical capsule. Kinds, surfaces, sensors, fixed steps and
  contacts behave as in 2D.
- A body has one `Shape2d`: a circle, a rectangle (turning with the body)
  or a vertical capsule, whose constructors reject sizes that are not
  positive and finite. `BodySettings` gives its `BodyKind` — **dynamic**
  (gravity, collisions, impulses), **static** (never moves) or
  **kinematic** (follows only the velocity the game sets, pushes dynamic
  bodies) — its start (position, rotation, velocity), gravity scale, fixed
  rotation, and surface: friction, bounciness and density. Invalid numbers
  are `EngineError::Physics`.
- `advance(elapsed)` runs as many `FIXED_STEP`s (1/60 s) as fit, keeping the
  rest for the next call; at most `MAX_STEPS_PER_ADVANCE` (8), so a long
  frame slows the game down instead of piling up steps.
- `contacts()` are the bodies that started or stopped touching during the
  last advance (`ContactPhase::Started`/`Ended`). A **sensor** overlaps
  instead of colliding, and its contacts say so. Every pair of kinds is
  reported, so a kinematic character notices a static sensor. A removed
  body's contacts end with the next step: `Physics2d` keeps which body each
  collider was until then.
- **Nodes with bodies**: a `Body2D` component (`star_scene`) makes a node
  with a `Transform2D` a body. The engine owns a `Physics2d` (gravity from
  `App::with_gravity`, `DEFAULT_GRAVITY` otherwise) and a `PhysicsNodes`
  as resources of the world (`world.physics()`, `physics_mut()` and
  `physics_nodes()`). Each frame, after `Stage::Update` and before
  `Stage::Physics`, the engine propagates the transforms and runs
  `step_node_physics`: it removes the bodies of nodes that are gone (or
  whose `Body2D` was replaced), adds the bodies of new `Body2D`s where their
  nodes are, moves a body to its node when the game changed the node's
  `Transform2D` (the component remembers the transform it last wrote),
  advances the simulation by the frame's time, writes the dynamic and
  kinematic bodies back into their nodes' `Transform2D` (in the space of
  the closest ancestor with a global transform), and turns the bodies'
  contacts into `NodeContact`s between nodes. Shapes are not scaled with
  their nodes. Cloning a `Body2D` gives a new body, so copied nodes never
  share one.
- Scenes keep bodies: a `[node.body]` table has the `kind` (`dynamic` by
  default, `static`, `kinematic`), the `shape` (`circle` with `radius`,
  `rectangle` with `size`, `capsule` with `height` and `radius`; a size of
  another shape is an error) and, only when they are not the defaults,
  `gravity_scale`, `fixed_rotation`, `friction`, `bounciness`, `density`
  and `sensor`.
- A `Body2D` also takes a velocity (`set_velocity`) and impulses
  (`apply_impulse`), passed to the body before the next step, so they work
  on the frame the component is added; `velocity()` is the velocity after
  the last step (or the one just set). `BodySettings::check` says whether a
  body can be simulated without adding it.
- StarScript nodes get bodies through the `Body` class (decision 47;
  `docs/starscript.md`).
- **3D nodes with bodies**: a `Body3D` makes a node with a `Transform3D`
  a body of the `Physics3d`, exactly as a `Body2D` does in 2D: `Body2D`
  and `Body3D` are the same generic component (`NodeBody`), and
  `PhysicsNodes` and `PhysicsNodes3d` the same bookkeeping (`BodyNodes`).
  The engine owns a `Physics3d` (Earth's gravity along -y; games change it
  through `world.physics_3d_mut()`) and a `PhysicsNodes3d`, and runs
  `step_node_physics_3d` right after `step_node_physics`. Bodies are
  placed with the rotation of their global matrix (`Quat::from_mat4`), and
  written back into the parent's space; under a parent scaled differently
  along each axis and turned, the rotation is only approximate. Scenes
  keep them in a `[node.body_3d]` table, like `[node.body]` with the
  shapes `sphere` (`radius`), `box` (`size` of three numbers) and
  `capsule` (`height` and `radius`). StarScript nodes get them through
  `Body` (decision 50).

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
- **3D meshes** (`MeshData`) share their corners between triangles through
  `u32` indices, and each corner (`MeshVertex`) has a position, a normal
  and texture coordinates. `MeshData::new` checks whole triangles, indices
  that exist and finite numbers; `cube`, `plane` (facing up) and `sphere`
  (segments and rings, 3 to 1024 and 2 to 1024) build primitives with
  outward normals. `GameContext::create_mesh_3d` uploads one and gives a
  `Mesh3dId`; `Frame::draw_mesh_3d(mesh, Material, transform)` draws it.
  Their triangles are counterclockwise seen from the side they show, and
  the other side is not drawn (back-face culling), so closed meshes cost
  half. They are drawn before colored meshes and sprites.
- **Materials are PBR, metallic-roughness** (decision 48): a `Material`
  has a `base_color`, an optional `base_color_texture` (multiplied by it;
  materials without one get a white pixel), `metallic` (0 to 1) and
  `roughness` (0 to 1, at least 0.04 when drawn). The shader (`surface.wgsl`)
  is Cook-Torrance: GGX distribution, Smith-Schlick geometry and Schlick's
  Fresnel, with 4% reflectance for non-metals.
- **Lights** belong to the `Frame`: `set_ambient_light(AmbientLight)` (none
  at first, kept between frames like the camera) and up to `MAX_LIGHTS`
  (16) `add_light`s per frame, either `DirectionalLight`s (a direction, like
  the sun), `PointLight`s (a position and a range, fading with the square
  of the distance and smoothly reaching zero at the range) or `SpotLight`s
  (a point light shining only in a cone: `angle` from its axis to its edge,
  and `blend`, the part of the cone that fades out towards the edge). Each
  light takes four `vec4`s on the GPU. More lights are
  `EngineError::CapacityExceeded` when rendering; lights that cannot shine
  (a zero direction, no range, a spot's cone that is no cone, numbers that
  are not finite) are skipped. A
  light of intensity 1 straight on a white matte surface gives white.
  Sprites have their own lights: `set_ambient_light_2d` (an
  `AmbientLight`, `AmbientLight::FULL` unless the game changes it, so
  sprites show their own colors) and up to `MAX_LIGHTS` `add_light_2d`s:
  `PointLight2d` and `SpotLight2d` (a direction, `angle` and `blend`). The
  sprite shader multiplies each sprite by the ambient light plus every 2D
  light that reaches it, fading as `(1 - (d / range)²)²`, so 2D lights end
  smoothly at their range instead of falling with the square of the
  distance. 2D light nodes (`Light2dNode`, `[node.light_2d]`) are drawn by
  `draw_sprite_nodes`; the editor draws their reach but does not light its
  view with them, since the 2D ambient light is the game's code.
  Normals use the inverse transpose of the model matrix (`Mat4::inverse`,
  `transposed`), so non-uniform scales keep them right. Lighting happens in
  linear space on sRGB targets; on other targets colors stay as given.
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

A single native window with two screens, in **English** (the default) or
**Spanish**. Every text is in `texts/`: one `Texts` value per language
(`english.rs`, `spanish.rs`, with the same fields, so the compiler checks
that no language misses one) and, in `messages.rs` and
`scene_messages.rs`, the texts built from values. A menu “Language” (in the project manager's header and in
“File”) lists the languages by their own names; the choice applies from
the next frame and is kept in `preferences.toml`, next to the recent
projects in the user's configuration folder (`config_file.rs`). Messages
of the StarScript compiler and of the project check stay in English:

1. **Project manager** (`launcher/`): on the left, recent projects (an
   “Open” button; those that no longer exist appear as “Unavailable”, with
   “Remove”) and “Open another project…” (folder picker). On the right, **new
   project**: name, location (with “Browse…”), 2D or 3D type (exclusive)
   and platforms (checkboxes). “Create and open” creates
   `<location>/<name>/` with `project.toml` and `assets/` (it never
   overwrites) and opens the editor.
2. **Editor** (`workspace/`), maximized: menu (“File” → “Save” /
   “Close project” / “Language” / “Quit”; “Edit” → “Undo” / “Redo” of the
   scene; “Project” → “Compile” / “Play” /
   “Stop”; “Help” → “About”) and the same three buttons next to it, **Scene** panel (the
   tree of the open scene's nodes, see below), a central area with
   **tabs**: “Scene”, the **view** rendered by the engine (reference grid;
   drag = pan in 2D / orbit in 3D, wheel = zoom), always there and named
   after the open scene file, and one tab per open **blueprint** (see
   below), **Inspector** (the node selected in the scene, or the project's
   properties), a bottom panel with four tabs, **Assets** (one
   folder of `assets/` at a time as a grid of tiles, with the way back up
   above it and “Refresh”: folders and pages tagged by kind are painted
   by `asset_icons.rs`, images show a copy shrunk to 96 pixels made once
   by `thumbnails.rs`; a click selects, a double click goes into a folder
   or opens a `.blueprint` or a `.scene`. A right click on the background
   opens “Create” (folder, scene with its root, blueprint, StarScript
   script —the same a new blueprint translates to, so it compiles—,
   Rust script, or a “Blueprint from a template”), “Refresh” and “Show in file explorer” (the folder shown);
   on a tile, also “Open”, “Rename” (`F2`), “Delete” (`Delete`, asked
   first), “Copy path” and “Show in file explorer” (that file or folder,
   selected; see decision 56). A slider next to “Refresh” sizes the icons
   from 60 % to 200 %; the editor's preferences remember it, written once
   the slider is let go. The **templates** (`blueprint_editor/templates.rs`,
   files in `editor/templates/blueprints/`) are ready-made blueprints copied
   into the project under the name given, so they are the project's own:
   `Movement2D` and `Movement3D` (the actions `left`, `right`, `up` and
   `down`, through `Input.axis`), `Spinner2D` and `Spinner3D`, `Mover2D` and
   `Mover3D` (straight lines), and `Lifetime` (removes the node after some
   seconds). The menu lists those of the project's kind. A template that
   reads input actions adds those the project lacks to `project.toml`
   (`project/input_actions.rs`), with keys, arrows and the left stick,
   writing lines into its text so its comments and order stay; if its
   actions are written another way, nothing is changed and “Compile” names
   them. Every template is checked by a test that validates it, translates
   it and compiles it. Names are given in a
   row over the grid; `asset_files.rs` checks them and never overwrites.
   What is renamed or deleted closes the tabs open from it. A script or
   blueprint renamed whose class is called as the file was (and whose new
   name can be a class) renames its class in the whole project
   (`class_rename.rs`): in scripts token by token (never in texts or
   comments), in blueprints (class, base, class calls and types) and in
   the classes of scenes' nodes. Files open with unsaved changes are left
   alone and named in the console; open files without changes are read
   again), **Problems** (what the last
   “Compile” found, as `file:line:column` and a message; it opens by itself
   when there are problems), **Output** (what Cargo and the game write
   while playing) and **Console** (the editor's own log), and a status bar (with the result of the last
   “Compile” and whether the game is building or running).

The editor's log (`editor_log.rs`) is its own `log` logger: it writes to
the terminal through `env_logger` (same `RUST_LOG` filter as before) and
keeps the last 2000 messages in a `LogHistory`, shared behind a mutex so
any thread can log. The **Console** tab shows them with their time since
start, level (errors and warnings in color), module and text, filtered
from a chosen level up (info by default), with “Clear”; it scrolls with
new messages while it is at the bottom. Messages logged before a project
is opened (in the project manager) are kept too.

The view uses `OffscreenRenderer` on egui's wgpu device and registers its
texture in `egui_wgpu` (it is registered again when its size changes). The
texture is `Rgba8Unorm` with colors already encoded in sRGB, because egui
samples textures as gamma values.

The **scene editor** (`scene_editor/`) opens one `.scene` file at a time
in the “Scene” tab, read as a `SceneOutline` (the nodes as the file
describes them, images named by path: nothing is loaded to show them).
The “Scene” panel draws its tree from the parent each node names, every
branch open at first. A right click on a row opens “Create” (a child of
that node from a template of the project's kind: an empty node or a
sprite of the first image in 2D; an empty node, a cube, a sphere, a
plane, a directional or a point light in 3D; named after the template,
`Cube`, `Cube2`… among its siblings), “Duplicate” (`Ctrl+D`: the node
and its descendants, under the same parent) and “Delete” (`Delete`: with
its descendants); the root can be neither duplicated nor deleted. A right
click under the rows creates under the root. A row dragged onto another
row moves its node, with its descendants, to be that node's last child
(dropped under the rows, the root's), through `SceneOutline::move_node`,
which puts the nodes moved after the rest so parents still come first;
rows it cannot land on (itself, its descendants, its own parent) are not
outlined. The node keeps its place in the world: its transform is
recomputed for the new parent (the closest one when uneven scales and
rotations would skew it), and it gets a name free among its new
siblings. A script or a blueprint dragged from “Assets” onto a row gives
that node its class (`class_drop.rs`): the file is read once, when the
drag starts, and only its classes that extend another class are offered,
since a node's class extends `Node` (which classes truly do needs the
whole program, so the check of the scene catches the rest). One class is
given at once; several open a small menu where the file was dropped
(`class_choice.rs`, owned by the scene so every place can open it); a
file with none says so next to the pointer, and no row is outlined. The
same file can be dropped on the view, where it goes to the node under
the pointer (the view is outlined only while one is there), or on the
inspector, where it goes to the node selected. Every place that takes
drops asks for them through `drag_drop::released`, which only takes what
is carried if it is of the kind asked for: egui's own
`dnd_release_payload` takes whatever is carried and throws it away when
the kind differs, so a row that asked first for a dragged node lost the
script dropped on it. The tree (`tree_view.rs`) draws one row
per node across the panel, striped, with guides down to the children, an
arrow to close and open them (closed nodes are kept by path, so they stay
closed while others come and go), an icon for what the node shows most
(light, mesh, sprite, body or nothing) and its class on the right. A
search field lists the nodes whose name contains the text. A node chosen
in the view opens its parents and scrolls into sight. A click selects a node, and the inspector
edits its parts in sections (node, transform, sprite, mesh, light, 2D and
3D body): angles in degrees, colors with egui's picker, images chosen
among those of `assets/`, shapes rebuilt through the physics' own
constructors so sizes it refuses are never kept. Each section can be
removed, and “Add component” adds the parts the node lacks (those of the
project's kind). A node with a class also shows the **`export` fields** of
the class and its bases (`exported_fields.rs`), with the classes of the
last check of the scripts (the live check, asked for when the project
opens): each shows the value the scene gives it, in bold, or the class's
own (`FieldInfo::default`, when the class writes a plain value), and
changing it writes it in `[node.fields]`; “Reset” takes it out again.
The field's decorators (`FieldInfo::hints`) shape its row: `@header` puts
a title before it, `@tooltip` a help over its name, `@range` a slider
(whole numbers keep to the whole values inside it) and `@multiline` a box
of several lines.
Values for fields the class does not export are shown in red, to remove.
Changes mark the tab (`•`) until “Save” (`Ctrl+S`),
“Compile” or “Play” writes the file with `SceneOutline::to_text`; opening
another scene, closing the project or quitting ask first, as blueprints
do. A file that cannot be read or saved shows why in the panel.
“Undo” (`Ctrl+Z`) and “Redo” (`Ctrl+Shift+Z` or `Ctrl+Y`) work while
the “Scene” tab is shown and no text is being typed (text fields undo
their own text). The history (`history.rs`) keeps whole copies of the
scene and of the node selected, up to 100 steps: scenes are small, and a
copy cannot miss a kind of change the way recorded operations could. A
change is only marked while it happens; the step closes once the pointer
is released and no field is being typed in, so one drag of a handle or
one number typed is undone at once. Each copy has a revision number, so
undoing back to the saved scene clears the tab's mark.

The view draws the open scene over its grid as the game would: each
node's place is computed from the outline in one pass (parents come
first; a node without a transform passes its parent's on), sprites are
drawn through `SpriteNode::to_sprite`, meshes from the unit shapes of
`MeshFile::unit_shape` scaled, and the scene's lights light them (a
scene with none gets a preview sun and a soft ambient light, so its
meshes can be seen). The view loads each image of `assets/` and builds
each unit mesh once (`ViewportAssets`); one that fails logs a warning
once and is left out. The node selected is marked with orange lines: a
frame around its sprite, a box around its mesh, or a cross where it is.
A click in the view selects the node under the pointer
(`scene_editor/picking.rs`): a ray from the camera through the point
clicked meets sprites over their rectangle, meshes over their box (a ball
for spheres) and nodes that show nothing over their cross. What a node
shows wins over a cross, then the nearest hit, the higher sprite layer
and the later node (drawn on top); a click on nothing clears the
selection.

Over the node selected the view draws the handles of the tool chosen in
its bar (or with `Q`, `W`, `E`, `R`): **View** only moves the camera,
**Move** has arrows along the world's axes and a square that moves the
node on the screen's plane (`z = 0` in 2D), **Rotate** has rings around
the node's own axes (one around Z in 2D) and **Scale** has lines ending
in boxes along its own axes, an orange one between X and Y that scales
both at once, and a square that scales it evenly. The
geometry is `scene_editor/gizmo.rs`, in the view's pixels and free of
egui: a `Lens` places world points on screen and turns pixels into rays,
handles keep a fixed size on screen, and a `Drag` computes the node's
new local transform from the transform it had when the drag began (so
nothing drifts): the point of the axis nearest the pointer's ray, the
angle turned around the node's point on screen (signed by whether the
axis faces the viewer), or the ratio along an axis, never below 1% of
the size it started at. Positions found in the world go back through
the inverse of the parent's place. 3D nodes are handled in 3D in 3D
projects; drags that grab no handle still move the camera.

Images can be dragged from the assets panel (egui's drag and drop, with
an `ImageDrop` payload: the path and the image's shape, read from its
thumbnail; a small copy follows the pointer). Dropped on the view, they
become a new child of the root where the pointer's ray meets the 2D
plane, or the ground (`y = 0`) in 3D, in front of the camera if it never
does; dropped on a row of the tree, a new child of that node at its
place (`scene_editor/dropping.rs`). In 2D the node gets a sprite one
unit tall, as wide as the image's shape; in 3D, a plane of that size
with the image as its texture. It is named after the file, unique among
its siblings, and selected.

The **blueprint editor** (`blueprint_editor/`) draws a graph of the open
blueprint on a canvas of its own, painted with egui's painter (no node
editor library: the model, its pins and its rules are `star_blueprint`'s).
A selector chooses the graph. Nodes are boxes with a colored title by family
(entry, flow, variables, calls, values), inputs down the left side and
outputs down the right one, in the order of `Node::pins`; execution pins
are triangles and data pins circles, and wires are curves between them.
Dragging with the middle button, or scrolling with two fingers on a
touchpad, moves the view; the mouse wheel, `Ctrl` + wheel and pinching zoom
around the pointer (`CanvasView`, between ×0.25 and ×2). Wheels and
touchpads are told apart by egui's unit of the scroll: lines for a wheel,
points for a touchpad. Where each node and pin is
comes from `layout.rs` (graph units, with tests), so drawing and picking
pins with the mouse use the same geometry.

Editing (`canvas.rs` for the mouse, `edit.rs` for the changes, with tests):
the left button selects (`selection.rs`): clicking a node selects it,
dragging the background draws a box that selects the nodes it touches, and
`Shift` or `Ctrl` add to the selection (clicking a selected node then
leaves it out); dragging a selected node moves all the selected ones.
Dragging from a pin to another joins them, whichever is the output; the
right button opens a menu to add a node where it was opened (values,
variables of the blueprint, calls to its own methods or to any other,
ready-made “Functions”, operators, “If”, “Return”), to delete the selected nodes or to remove a
pin's wires; `Delete` deletes the selected nodes. The entry cannot be deleted. The editor keeps only wires it
can draw: from an output to an input of the same kind, on two nodes, and a
pin that takes one wire (every input, and execution outputs) keeps the
newest one. Everything else (unreachable nodes, cycles, inputs without a
wire) is allowed while building and reported by “Compile”.

The **ready-made functions** (`library.rs`) are the fields and methods of
the engine's classes and the methods of numbers (“Math”), as nodes
already filled in: a field gives a node that reads it and one that sets
it, a method a call with its arguments (pure if it gives a value). They
are read from `star_script::semantic::native_classes`, so what the engine
adds appears without a second list (decision 57). Each class is a group,
and “Transform” gathers the members that place a node. The members of a
class are the blueprint's own only if it extends that class (a base
from the project is taken to be a node); those of another object, wired
to `target`, are in a submenu of their own. A field at the top searches
every group. Events (`start`, `update`) and `async` methods are left
out.

While a blueprint is shown and one node is selected, the **Inspector**
gives its fields (`properties.rs`; with several selected it says how many): a value's type and value, a variable's name
(typed or picked from the blueprint) and whether it is a field of another
object (“Another object”, which adds a `target` pin and titles the node
`.x`; renaming a variable of the blueprint leaves those alone), what a call calls, its method, its
arguments (renaming one keeps its wire), whether it returns a value and
whether it is pure, and an operator. A change that takes pins away also
takes their wires.

With no node selected, the Inspector edits the **class** (name, base,
`base_arguments`, variables with their type, starting value, privacy and
“Inspector”, which makes them `export` and shows the rows of their
decorators: header, range with or without a step, tooltip and multiline)
and the **signature** of the graph shown (name, parameters, return type,
`override`, privacy) (`class_properties.rs`). Renaming a variable, a method
or a parameter renames it in the nodes that use it (`edit.rs`), and a
change that takes pins away takes their wires. “Graphs” in the tab adds a
graph (with its entry) or deletes the one shown. “Create › Blueprint” in
the right-click menu of “Assets” asks for a class name and writes
`<Class>.blueprint` in the folder shown (`creation.rs` gives the text,
`workspace/asset_files.rs` writes it, never overwriting): a `Node` that
redefines `start` and `update`, empty, which compiles as it is.

The mistakes of the last “Compile” are shown in their blueprint: each
`Problem` of a blueprint keeps its `Origin` (graph and node, graph, or
class), so nodes with mistakes get a red border and show them under the
pointer and in the inspector, and the others are listed above the canvas.
Clicking the place of such a problem in “Problems” opens the blueprint on
its graph, with the node selected in the middle of the canvas.

The **code editor** (`script_editor/`) opens `.star` files of “Assets”
(double click) in tabs of the central area, beside the blueprints. The text
is egui's `TextEdit` with a layouter that colors it (`highlight.rs`):
keywords, `true`/`false`/`null`, types (capitalized names and the basic
types), names that are called, numbers, strings, comments and characters
the language does not have, from the pieces `star_script::scan` finds. The
colored layout is kept until the text or the theme changes. The number of
each line is painted left of its first row (`code_view.rs`), long lines
wrap, and the line and column of the cursor are shown below the text.
Clicking a problem of a script in “Problems” opens it with the cursor on
the problem's line and column. Scripts are saved as blueprints are.

Writing aids (`editing.rs`, free of egui and tested on their own) take the
keys they handle before the `TextEdit` sees them: `(`, `[`, `{` and `"`
close themselves where nothing follows that they would wrap (a quote not
after a word nor inside a string), surround a selection, and typing the
closing one again steps over it; `Backspace` between an empty pair deletes
both, and in indentation goes back one level. `Enter` keeps the line's
indentation, one level more after an opening bracket (with the closing one
moved to a line of its own when the cursor was between them); `}` on a
line of only indentation goes one level back. `Tab` types one level (four
spaces, as blueprints are translated) or indents the selected lines, and
`Shift+Tab` takes a level out; `Ctrl+/` comments out the selected lines,
or uncomments them if they all are. Once the `TextEdit` has to handle a
key (editing and moving keys, or shortcuts), it handles the keys after it
in that frame too, so typing order is kept; pressing the key of a
character is not such a key, as the character comes after it in an event
of its own (a `Text`, or an input method's `Commit`).

Mistakes are shown while writing (`workspace/live_check.rs`): half a
second after the last change to a script (or after one is opened), the
project's code is checked by a thread that lives while the project is open
(`project_check::CheckWorker`; it takes request after request, skipping to
the newest texts if several waited), reading the open scripts' texts
instead of their files (`OpenTexts`, `check_project_with`), saved or not.
Its goal is `CheckGoal::Writing`: the program is translated to Rust too,
for the mistakes only the translation finds (several entry points, a game
that needs arguments, a script class that extends a class of the engine
that cannot be extended), but the Rust is not kept, and scenes are not
checked, which only “Compile” (`CheckGoal::Compile`, on a thread of its own
each time) needs. Translating takes about as long as checking (under a
millisecond for a few hundred lines). Its problems are only marked in the
code: “Problems” and “Play” still follow “Compile”. Each mistake of the
script shown is marked with a wavy red line under the name, number or
string it starts at, or under its one character (`mistakes.rs`, with
tests), with its message under the pointer; its line number turns red, and
below the text the mistake of the cursor's line is written, or how many the
script has.

Suggestions (`completion.rs`, free of egui and tested; `suggestions.rs`
shows them): the text is half written while they are needed, so it is
never parsed. The word at the cursor (not inside a comment, a string or a
number) is completed from the compiler's lexer and the class table of the
last check that could read the program, which the live check keeps
(`star_script::analyze_program_with` keeps the classes even when their
code has mistakes of meaning; while a line is half written the program
cannot be read at all, so the previous table is used). After a `.` (or
`?.`), the members of what is before it. `scope.rs` reads the chain before
the `.` backwards with the lexer: names, calls (`make(…)`, skipping their
balanced parentheses), elements (`[…]`) and literals (`"a"`, `1.5`,
`true`; not integers, since `1.` starts a decimal number). Its first link
is `self` or `super`, a variable of the function (with the type the live
check found for it, `ProgramOutline`'s locals, so inferred types work; or,
if the text changed since, the type the text writes: `name: [Rock]` or
`var name = Class(…)`), a class (its static members), a constructor
(`Rock()`), or a field or method of the class being written; each next
link is a member's type, a method's result or a list's element. A class
lists its members (private ones only inside their class); a basic type or
a list lists the language's methods (`star_script::semantic::
builtin_methods`, with their parameter types: `push(Rock)`, `pop() ->
Rock?`). A chain whose value cannot be known (`(a + b).`, `[1, 2].`)
suggests nothing. “Go to definition” and the types under the pointer read
chains the same way. Elsewhere, the names written in the text, the members of the class
being written, the classes, the keywords and the basic types. The chosen
name's rest is painted in faint text after the cursor and the others are
listed under it (with their signatures); `Tab` writes it (a method with its
parentheses, the cursor inside if it takes parameters), and `Esc` hides them
until the word changes. Suggestions show by themselves while writing, so
the list is faint and the up and down arrows keep moving the cursor until
`Ctrl+I` (`Cmd+I` on macOS; not `Ctrl+Space`, which input methods such
as IBus use to change the keyboard's language) makes it active: then the arrows choose in it while that word is
written (the list remembers where the word starts, so typing more of it
keeps it active). `Ctrl+I` also shows suggestions hidden with `Esc`.
These keys are taken before the writing aids, so `Tab` indents only when
nothing is suggested. A chain whose value is an expression in
parentheses (`(a + b).`) or a list written as it is (`[1, 2].`) suggests
nothing: knowing it would need the check to keep the type of every
expression by its place, for a way of writing that games rarely use.

“Go to definition” (`navigation.rs`, tested): `Ctrl`+click (`Cmd` on
macOS; the pointer becomes a hand while `Ctrl` is held over the text) or
`F12` on a name. What names mean comes from `scope.rs`, shared with the
suggestions. A member after a `.` goes to its declaration in its class (or
the base that declares it); otherwise a variable or parameter declared
before in the function being written goes to its declaration in the text,
then a member of the class being written, then a class, then any earlier
declaration in the text. Members and classes are found in the class table
of the last check that could read the program, with their places
(`ClassOrigin::Script` now keeps where the class's name is written), which
`ProgramOutline::locate` turns into a file and a position with the texts
that check read: a script opens with the cursor there; a blueprint opens
(its StarScript is never shown). Names of the engine go nowhere.

The type of the name under the pointer (`hover.rs`, tested): resting the
pointer on a name shows a tooltip with its type written as code, unless a
mistake is marked there (its message is shown instead). A variable,
parameter or loop variable shows `turns: int`, even when its type is
inferred: the live check keeps the locals of each script
(`Analysis::declared_locals`, by the byte where each declaration writes its
name, in `ProgramOutline`), and the editor finds the declaration in the
text as “Go to definition” does and looks it up there, only if the name
there is still the same (the text may be newer than the check); without
it, the class the text writes for it (`rock: Rock`). A member shows the
class that declares it: `Rock.size: int`, `Rock.roll(speed: float) ->
bool`; a class, `class Rock extends Node`; `self` and `super`, their class.
Names in comments and strings, numbers and unknown names show nothing.

Find and replace (`find.rs`, the search itself, and `find_bar.rs`, the bar;
both tested): `Ctrl+F` (`Cmd+F` on macOS) opens a bar over the code with
the selected text, if it is one line. Matches are character ranges, found
without overlapping, with or without telling upper and lower case apart
(`Aa`). The match from the cursor is chosen first; `Enter` and
`Shift+Enter` (or `‹` and `›`) go to the next and previous one, round the
text, and bring it into sight. Every match is painted under the text (a
shape slot added before the text edit), the chosen one stronger. “Replace”
(or `Enter` in its field) replaces the chosen match and chooses the next;
“Replace all” replaces every one, from the last so the places of the others
hold. `Esc` or `×` closes the bar and gives the keyboard back to the code
with the chosen match selected.

Blueprints and scripts are saved with `Ctrl+S` or “File” → “Save”, and
all of them before “Compile”, which reads the files. Tabs with unsaved changes
show `•`; closing one, the project or the window asks first (“Save”,
“Don't save”, “Cancel”). Saving writes the file again from the model, so
comments written by hand in it are not kept.

“Compile” runs `project_check/` (no UI, with tests) on a thread of its own
(`CheckJob`), which asks egui to redraw when it finishes. It checks:

- `project.toml`: its `[input.actions]`, which scripts name.
- Every `.star` and `.blueprint` file of `assets/` (any subfolder), as
  **one program** (`star_script::SourceSet`), including its translation to
  Rust. Blueprints are translated to StarScript first (`star_blueprint`);
  the compiler's mistakes in their code are reported on the blueprint's
  node, variable or graph (`Translation::origin`), not on lines of code the
  user never wrote. A blueprint that cannot be translated is reported
  alone, since the rest of the program would only miss its class.
- Every `.scene` file of `assets/`: that it can be read and, if the project
  has scripts, that the classes its nodes name can build them
  (`ClassTable::scene_class`, the same rule the generated code follows).

When it finds no problems, the check keeps the program's Rust
(`ProjectCheck::program`).

“Play” saves the open blueprints and runs “Compile”; if the check finds
problems, the game is not played and “Problems” is shown. Otherwise
`game_run/` (no UI, with tests), on a thread of its own (`GameRun`):

1. writes the program as a Cargo package (`star_script::package`) in
   `target/star_editor_games/<project>/` of the repository, using the
   engine's crates of the same repository;
2. builds it with `cargo build` (the Cargo that started the editor, or the
   one in the `PATH`), with one `CARGO_TARGET_DIR` for every project, so
   the engine is built once;
3. runs the game with the project's folder as its working directory, so it
   finds `assets/` as a shipped game does.

Every line Cargo and the game write reaches “Output” as it comes (at most
5000 are kept). “Stop” kills the build or the game; closing the project or
the editor stops it too. Console programs (`static func main`) are played
the same way, and their output is shown.

Modules: `project/` (validated model, creation and opening, no UI and with
tests), `project_check/` (see above), `file_tree.rs` (snapshot of a folder,
for the Files tab and the checks), `recent_projects.rs` (recent projects list, at most 10, in
`<config>/ProjectStar/recent_projects.toml`), `last_scenes.rs` (the scene
each project had open, opened again with the project: at most 50 projects,
in `<config>/ProjectStar/last_scenes.toml`, read again on every change so
two editors open at once do not undo each other; a scene that no longer
exists is not opened, and one renamed or deleted is forgotten; it lives
outside the project because it is the editor's state, not the game's),
`folder_dialog.rs` (rfd), `drag_drop.rs` (taking drops by kind),
`game_run/` (see above), `texts/` (every visible text, in English and Spanish) and `style.rs`
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
