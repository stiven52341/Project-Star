# Roadmap

Work advances in phases; every phase leaves the engine compiling with its tests green.

- [x] **Phase 0 — Foundations**: workspace, rules, CI, `star_core`, basic
      `star_math`, `star_ecs` (entities), platform detection, `App`.
- [ ] **Phase 1 — Window and main loop**:
  - [x] OS window and events (winit), continuous loop, resizing.
  - [x] Logging (`log`; binaries choose the implementation).
  - [x] `Time` integrated in the loop (`Game::update` receives the frame time).
  - [ ] Lifecycle on Android/iOS (suspend/resume, mobile entry point).
- [x] **Phase 2 — Input**:
  - [x] 2a Keyboard (physical keys) and mouse (buttons, cursor, wheel), with
        per-frame state; `Game::update` receives the `Input`.
  - [x] 2b Action map (`"jump"` → keys and mouse buttons), the API games use;
        `App::with_actions`. Gamepad bindings come with 2d.
  - [x] 2c Touch screens: fingers with id, position and start position (up
        to 10 at once); `Binding::Touch` (any finger) for actions.
  - [x] 2d Gamepads (`gilrs`): buttons by position, sticks and triggers, up
        to 8 at once; `Binding::GamepadButton` for actions. On Android and
        iOS `gilrs` has no backend yet: gamepads will come with Phase 11.
- [x] **Phase 3 — 2D rendering**:
  - [x] wgpu integration: DX12, Vulkan, Metal and OpenGL; backend selection;
        clearing the screen with a color; recovery from a lost surface.
  - [x] Shader pipeline (WGSL), triangle meshes with per-vertex color,
        orthographic 2D camera, depth buffer (examples `triangle_2d`/`triangle_3d`).
  - [x] Textures (from pixels in memory, smooth or pixelated filtering),
        sprites (tint, rotation, anchor, layers, sprite sheet cells) and
        sprite batching (example `sprites_2d`).
  - [x] Loading PNG images from files (`GameContext::load_texture`).
- [x] **Phase 4 — Full ECS**:
  - [x] 4a `World`: entities, components in sparse sets, queries
        (`World::each`) and resources.
  - [x] 4b Systems and stages (`Schedule`).
  - [x] 4c The engine owns the `World`: `Time` and `Input` as resources,
        systems registered in `App` (balls of `sprites_2d` are entities).
- [x] **Phase 5 — Assets**:
  - [x] 5a Asset folder and portable asset paths.
  - [x] 5b Texture caching: each file is loaded once per filter.
  - [x] 5c JPEG images (zune-jpeg), besides PNG.
  - [x] 5d Hot-reload of textures (polling modification times).
- [x] **Phase 6 — Scenes**:
  - [x] 6a Node tree on the ECS world (names, parents, children).
  - [x] 6b 2D transforms relative to the parent, and their propagation.
  - [x] 6c Sprite nodes, drawn by the engine.
  - [x] 6d `.scene` files (TOML): loading, instancing, capturing and saving.
  - Next, on top of it: the real hierarchy and inspector of the editor
    (Phase 12), and scenes from StarScript.
- [x] **Phase 7 — Audio** (decision 45):
  - [x] 7a Decoding sounds (`SoundFormat`, symphonia): WAV, OGG Vorbis,
        MP3 and FLAC into a `Sound` in memory (`star_core`).
  - [x] 7b Mixer (`star_audio`): voices that play `Sound`s with volume,
        loops and stop, converted to the output's rate and channels; tested
        without an audio device.
  - [x] 7c Audio output (cpal) owned by the engine, and loading sounds from
        the asset folder (`GameContext::load_sound`), with an example
        (`sounds_2d`).
  - [x] 7d Sounds in StarScript (part of 13.5): `Sound.load`, `play`,
        `stop`, `is_playing`, `volume` and `looping`, and `Audio.volume`.
- [x] **Phase 8 — 2D physics** (decisions 46 and 47):
  - [x] 8a Simulation (`star_physics`, rapier2d): `Physics2d` with dynamic,
        static and kinematic bodies; circles, rectangles and capsules;
        friction, bounciness, density; sensors; fixed steps; contacts that
        start and end.
  - [x] 8b Physics in the engine: `Body2D` nodes moved by the engine's
        `Physics2d` (a resource, with `App::with_gravity`), contacts between
        nodes, bodies in `.scene` files (`[node.body]`), and an example
        (`physics_2d`).
  - [x] 8c Physics in StarScript (part of 13.5): `Body` objects in the
        `body` field of nodes (`Body.circle`, `rectangle`, `capsule`), with
        their surface, velocity, `apply_impulse` and the nodes they touch;
        bodies in scenes loaded by scripts.
- [x] **Phase 9 — 3D rendering** (decision 48):
  - [x] 9a Indexed 3D meshes (`MeshData`: positions, normals, texture
        coordinates), primitives (cube, plane, sphere), uploaded with
        `GameContext::create_mesh_3d` and drawn with a `Material` (base
        color) by `Frame::draw_mesh_3d`, with back-face culling.
  - [x] 9b PBR materials (metallic-roughness, base color texture) and
        lights: ambient, directional and point lights (decision 48).
  - [x] 9c Example `scene_3d`: rows of plastic and metal spheres from
        rough to polished, a checkered cube, the sun and an orbiting lamp.
  - [x] 9d 3D nodes: `Transform3D` (with quaternions), `MeshNode` and
        `LightNode`, drawn by the engine and kept in `.scene` files
        (`[node.transform_3d]`, `[node.mesh]`, `[node.light]`); StarScript
        has them since 13.5k.
- [x] **Phase 10 — 3D physics** (rapier3d):
  - [x] 10a `Physics3d`: spheres, boxes and capsules; dynamic, static and
        kinematic bodies; sensors and contacts, in fixed steps as in 2D.
  - [x] 10b `Body3D` nodes the engine simulates, kept in `.scene` files
        (`[node.body_3d]`).
  - [x] 10c Example `physics_3d`: balls and crates dropped into an arena,
        a kinematic bar sweeping the floor and a sensor plate that paints
        green what goes through it. StarScript has 3D bodies since 13.5k.
- [ ] **Phase 11 — Mobile**: Android and iOS packaging.
  - [x] 11a Android: games run as a NativeActivity (`android_main!` in
        the game's library, logs in `adb logcat`), packaged with
        `cargo apk2`; the renderer lets go of its surface in the
        background and draws again on return. `scene_3d` runs on the
        emulator.
  - [x] 11b Android assets: `AssetFolder::packaged` (`PackagedFiles`),
        read on Android from the APK through `star_platform::PackagedAssets`
        and used by `App` by default. `scene_2d` ships its scene and images
        in its APK (`assets = "assets"`) and adds systems with a tap; both
        tried on the emulator.
  - [x] 11c Android in CI (clippy of the engine and the examples for
        `aarch64-linux-android`, and an APK of `scene_2d`), and every
        example as an APK: a library with `android_main!` and a desktop
        `main`. `App::with_development_assets` keeps the packaged assets
        on Android; `physics_2d` and `sounds_2d` also answer taps.
  - [ ] 11d iOS (needs macOS and Xcode).
- [x] **Phase 12 — Visual editor** (egui):
  - [x] Project manager: new project (name, location, 2D/3D, platforms),
        recent projects, open project, native folder picker.
  - [x] Main window: menu, Scene / View / Inspector / Files panels, status bar.
  - [x] Engine rendering inside the view (wgpu shared with egui), grid,
        editor camera (pan/orbit and zoom).
  - [x] Editor in English (default) and Spanish, chosen from a “Language”
        menu and remembered between sessions.
  - [x] Real hierarchy and inspector, log console, in parts:
    - [x] 12a Open a `.scene` from “Assets” in the “Scene” tab: its tree
          of nodes in the “Scene” panel, and the parts of the node
          selected in the inspector (read only).
    - [x] 12b Edit scenes: the node's values in the inspector, adding
          and removing its parts, adding and deleting nodes, and saving
          the file (`SceneOutline::to_text`, `add_node`, `remove_node`).
    - [x] 12c Draw the open scene in the view (sprites, meshes, lights),
          with the selected node marked.
    - [x] 12d Log console in the bottom panel: the editor's log, kept
          while writing to the terminal, filtered by level.
- [ ] **Phase 13 — StarScript and Blueprints** (`star_script`, see `docs/starscript.md`):
  - [x] 13.1 Lexical specification and lexer (tokens, errors with line and column).
  - [x] 13.2 Parser and AST (error recovery, all errors at once).
  - [x] 13.3 Semantic analysis: types, inference, null safety, classes,
        inheritance and safe construction.
  - [x] 13.4 Rust generation and end-to-end test (`.star` → binary), with the
        `star_script_runtime` runtime.
  - [ ] 13.5 Engine API for StarScript, in parts (each piece of the engine is
        born with its StarScript API):
    - [x] 13.5a Lifecycle: engine class `Game` (`start`, `update`, `quit`);
          a script that extends it runs in a window of the engine.
    - [x] 13.5b Input in StarScript: class `Input` (`is_down`, `just_pressed`,
          `just_released`) over the actions of `project.toml`; misspelled
          action names are compile errors.
    - [x] 13.5c Nodes in StarScript: engine classes `Node` and `Sprite`,
          which scripts extend with their own `start` and `update`; the
          game's tree is mirrored into the engine's world.
    - [x] 13.5d Look of the game: `Color` (sprite tint, `Game.background`),
          a `Camera` node that `Game.camera` is seen from, and a fallback
          logger so the engine's warnings always reach the console.
    - [x] 13.5e Scenes in StarScript: `Scene.load` builds the `Node` and
          `Sprite` objects of a `.scene` file (`null` if it cannot).
    - [x] 13.5f Script classes in scenes: a scene node names the StarScript
          class it is built as (`class = "Rock"`), created without arguments.
    - [x] 13.5g Pointer in StarScript: `Input.pointer_x`, `pointer_y` and
          `has_pointer`, the mouse or a finger in world units, seen from
          the game's camera.
    - [x] 13.5h Analog input by actions: stick directions as bindings
          (`"GamepadLeftStickLeft"`…), `Input.strength(action)` (triggers
          and sticks give how far they go) and `Input.axis(negative,
          positive)`.
    - [x] 13.5i Sounds in StarScript: `Sound` and `Audio` (step 7d).
    - [x] 13.5j Physics in StarScript: `Body` (step 8c).
    - [x] 13.5k 3D in StarScript: `Node3D` (extends `Node`: `z`, `pitch`,
          `yaw`, `scale_z`, `look_at`), `Mesh`, `Light` and `Camera3D`;
          `Game.camera_3d` and `Game.ambient`; 3D bodies with `Body.sphere`
          and `Body.box`; `Scene.load` builds 3D nodes.
  - [x] 13.6 Editor integration: compiling and an error panel.
    - [x] 13.6a Programs of several files (`SourceSet`): each file is read
          on its own and their classes are checked as one program; errors
          name their file.
    - [x] 13.6b Checking a project in the editor: every `.star` of
          `assets/`, the input actions of `project.toml` and the script
          classes named by its scenes.
    - [x] 13.6c Error panel: “Compile”, a “Problems” tab and the result
          in the status bar, checked off the UI thread. (13.6b and 13.6c
          went in together: the editor is a binary, so a check without its
          panel would be dead code.)
    - [x] 13.6d “Play”: the checked game is built with Cargo and run in
          its own window, with an “Output” tab for what Cargo and the game
          write, and “Stop”. It needs the editor run from the repository.
  - [x] 13.7 Blueprint model and translation to StarScript (`star_blueprint`):
    - [x] 13.7a Model: classes as node graphs, pins of each node, the rules
          (`validate`) and `.blueprint` files (TOML).
    - [x] 13.7b Translation to StarScript, remembering which code comes from
          which node.
    - [x] 13.7c “Compile” in the editor checks blueprints with the scripts,
          and their errors name the node.
  - [x] 13.8 Visual blueprint editor (drawn with egui, in tabs of the
        central area):
    - [x] 13.8a Open blueprints from “Assets” in a tab and see their
          graphs: nodes, pins and wires, with pan and zoom.
    - [x] 13.8b Edit graphs: move nodes, connect and remove wires, add and
          delete nodes, change their properties, and save.
    - [x] 13.8c Edit the class (variables, graphs and their signature),
          create new blueprints, and mark on their nodes the mistakes
          “Compile” finds.
  - [x] 13.9 StarScript code editor (egui's `TextEdit`, in tabs of the
        central area):
    - [x] 13.9a Open `.star` files from “Assets” in a tab: colors from the
          compiler's lexer, line numbers, the cursor's line and column,
          saving, and “Problems” opening a script at the problem.
    - [x] 13.9b Writing aids: closing brackets and quotes, indentation on
          new lines, `Tab`/`Shift+Tab` over lines, `Ctrl+/` comments.
    - [x] 13.9c Mistakes while writing: checked in the background and
          marked on their lines.
    - [x] 13.9d Suggestions: names in scope and members, shown as faint
          text and accepted with `Tab`, with a list of the others.
    - [x] 13.9e Go to definition (`Ctrl`+click or `F12`); mistakes and
          suggestions checked on one thread that lives with the project,
          without translating to Rust.
    - [x] 13.9f Find and replace in a script (`Ctrl+F`): the matches
          painted, the next and previous one, case, and replacing one
          or all.
    - [x] 13.9g The type of a name under the pointer: variables (even
          inferred), members with their class, and classes with their
          base.
    - [x] 13.9h Suggestions follow calls (`make().`), elements of lists
          (`rocks[0].`) and literals, list the methods of basic types and
          lists (`"a".length()`), and use the types the check inferred.
    - [x] 13.9i The check while writing also reports what only the
          translation to Rust finds.
    - [x] 13.9j The list of suggestions is faint and the arrows move the
          cursor until `Ctrl+I` makes it active. Chains inside
          parentheses (`(a + b).`) or after list literals suggest nothing,
          by decision (see `docs/architecture.md`).
  - [x] 13.10 Concurrency and parallelism (`docs/starscript.md`, section 7;
        decision 54):
    - [x] 13.10a Syntax: the keywords `async`, `await` and `parallel`;
          `async` and `parallel` as modifiers of methods, `await` before
          an expression.
    - [x] 13.10b Coroutines in the analysis: `async` methods and their
          overrides, where `await` goes, starting coroutines, and
          `Time.wait` and `Time.next_frame`.
    - [x] 13.10c Coroutines in Rust: `async` methods as futures, run once
          per frame by the runtime; those of nodes end when their node
          leaves the tree; console programs wait for theirs.
    - [x] 13.10d Parallel functions in the analysis: only `static`, only
          values in and out, and only values and other `parallel`
          functions inside.
    - [x] 13.10e Parallel functions in Rust: threads of the runtime,
          values copied in and out, and runtime errors on those threads
          stopping the game.
    - [x] 13.10f The code editor and an example: suggestions and types
          show `async` and `parallel`; an example game that uses both.
- [x] 13.11 Number types (decision 55): `short` (`i16`), `int` (`i32`),
      `bigint` (`i64`), `float` (`f32`) and `double` (`f64`); literals take
      the type their place needs and must fit in it, and numbers widen by
      themselves where no value is lost.
- [x] **Editor ideas**, done one by one:
  - [x] Undo and redo in the scene editor (`Ctrl+Z`, `Ctrl+Shift+Z` or
        `Ctrl+Y`, and the “Edit” menu).
  - [x] Moving nodes to another parent by dragging them in the tree.
  - [x] Dropping a `.star` (or a `.blueprint`) on a node to give it its
        class.
  - [x] “Show in file explorer” in “Assets” (decision 56: the system's own
        program, no dependency).
  - [x] A slider for the size of the icons of “Assets”, remembered by the
        editor's preferences.
- [ ] **Owner's list (2026-11-23)**, done one by one:
  - [x] Opening a project opens again the scene it had open.
  - [x] Dragging scripts from “Assets” to the view and to the inspector.
  - [x] Dropping a blueprint on a node does not give it its class (bug).
  - [x] Ready-made functions in blueprints (for example, of the transform),
        and variable nodes that read or set fields of another object.
  - [x] A pack of ready-made blueprints (2D movement, 3D movement…).
  - [x] Renaming a `.star` does not rename its class (bug): it now renames
        the class, and its uses, in the whole project (blueprints too).
  - [x] A keyword for variables set from the inspector: `export var`.
    - [x] 7a The language: `export`, on fields of nodes of simple types.
    - [x] 7b Scenes keep the values (`[node.fields]`) and games apply them.
    - [x] 7c The inspector edits them.
    - [x] 7d Blueprint variables can be exported.
  - [x] Decorators (`@`) in StarScript.
    - [x] 8a The syntax and the engine's decorators (`@range`, `@tooltip`,
          `@header`, `@multiline`) for `export` fields.
    - [x] 8b The inspector follows them.
    - [x] 8c Blueprint variables have them.
    - [x] 8d Decorators of the user: classes that extend `Decorator`, with
          `before` and `after`, on methods that return nothing; and
          `Time.now()`.
  - [x] Ready-made nodes: cameras, 2D and 3D lights, collision shapes…
    - [x] 9a Cameras in scenes: `[node.camera]`/`[node.camera_3d]` with
          `current`, “2D camera”/“3D camera” in Create and the inspector,
          drawn in the view; `Camera` renamed `Camera2D`.
    - [x] 9b Physics nodes: Create → Physics offers static, rigid,
          kinematic bodies and areas (static sensors), 2D or 3D by the
          project, and the view draws the shape of every body.
    - [x] 9c Spot lights: `SpotLight` in the renderer (a cone with a
          soft edge), `kind = "spot"` with `angle` and `blend` in scenes
          and StarScript, in Create and the inspector, and their cone drawn
          in the view.
    - [x] 9d 2D lights: sprites lit by a 2D ambient light (white by
          default, so nothing changes until a game darkens it) and 2D point
          and spot lights; `[node.light_2d]`, StarScript's `Light2D` and
          `Game.ambient_2d`, in Create and the inspector, their reach drawn
          in the view.
  - [x] Finding nodes of the scene by name and by tag, in StarScript and
        blueprints: `tags` of nodes (scenes, inspector), `find_by_name` and
        `find_by_tag` of `Node` (descendants) and `Scene` (the whole
        game), and the blueprints' “For each” node to go through lists.
  - [ ] Global and local position, as separate properties.
- [ ] **Future — Consoles** (PlayStation, Xbox).
