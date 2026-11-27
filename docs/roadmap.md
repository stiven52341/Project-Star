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
- [x] **Owner's list (2026-11-23)**, done one by one:
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
          `main` (first called `current`), “2D camera”/“3D camera” in Create and the inspector,
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
  - [x] Global and local position: `x`, `y`… stay local, and nodes get
        `global_x`/`global_y`/`global_rotation` (`global_z` in 3D), their
        setters, `translate`, `translate_global` and `rotate` (and their
        3D versions), also under “Transform” in blueprints.
- [x] **Owner's requests (2026-11-24)**:
  - [x] One main camera per scene: the cameras' flag is `main`, a scene
        with two is an error, and the editor keeps only one marked.
  - [x] Playing without scripts: the project's main scene (`main_scene`
        in `project.toml`, “Set as main scene” in the assets), seen from
        its main camera, which it must have (decision 62).
  - [x] A small preview of what a camera sees, in 2D and 3D: the camera
        selected, or else the main one, in a corner of the view.
- [ ] **Faster “Play” (planned 2026-11-24)**. Measured on the test
      project (4 cores): a change to the game's code alone rebuilds in
      1.6 s, but a full build takes 10 min 37 s, almost all of it
      third-party crates (naga, wgpu-core, rapier, parry, x11rb, winit) at
      `opt-level = 3`; the engine's own crates take about 3 min of it.
      Full builds happen because each game package resolves its own
      `Cargo.lock` (10 versions differed from the repository's and drift
      when Cargo re-resolves), after toolchain updates, and the engine's
      crates are rebuilt at `opt-level = 3` after every engine change.
  - [x] a. Write the repository's `Cargo.lock` into the game package, so
        every project shares the same versions and builds; write
        `Cargo.toml` and the code only when they changed. Measured: after
        one project caught up with the repository's versions, a second
        project built in 11.8 s instead of a full build.
  - [x] b. Prepare the engine in the background when a project opens (a
        build of a small package with the same dependencies), so the first
        “Play” finds it built; show that it is being prepared.
  - [x] c. Measure lighter profiles: the engine's crates at
        `opt-level = 1` and dependencies without full debug information
        (`debug = "line-tables-only"`), checking the frame rate of the
        examples. Measured with a game of 1000 physics bodies and 4000
        spinning sprites (CPU-bound at 60 FPS): line tables only cut the
        full build from 568 s to 485 s and its folder from 2.8 GB to
        1.8 GB, with the same frame rate, so games and the repository use
        it. The engine at `opt-level = 1` only saved 12 s more and dropped
        to 58 FPS, so it stays at 3; a change to the engine's code rebuilds
        in about 4 s either way (incremental builds), not minutes.
  - [x] d. Try sharing the third-party builds with the editor's `target`
        (same lockfile and profile); keep it only if it really reuses them.
        Tried and not kept: a game built in the editor's `target` reused
        182 crates but rebuilt 61, among them the heaviest (wgpu, naga,
        winit, rapier, parry, symphonia), in 316 s instead of 485 s. The
        repository's workspace unifies features with the editor (egui
        turns on more of wgpu and winit) and its engine crates are members
        at `opt-level = 1`, so their builds differ from a game's. Steps a
        and b already make that cost a one-time one, and sharing the
        folder would make every “Play” wait for the developer's own Cargo
        commands (and the other way round).
  - [x] e. Projects without scripts: a ready-made player built with the
        editor that opens the main scene, so they play without Cargo
        (decision 63): the warm-up builds the scene player, and “Play”
        only lets Cargo check it (0.26 s) before running it.
  - [x] f. Running StarScript without Rust while editing (an
        interpreter), for instant “Play” and hot reload: decided by the
        owner (decision 64) and planned as Phase 14. Before it:
    - [x] f1. Move the engine classes and the game loop out of the text
          templates of `codegen/native.rs` into real Rust, built once with
          the engine instead of inside every game. They name the classes
          generated for each program (`crate::classes`, 175 times), so
          first what does not need them, and then a design for the rest
          that the interpreter can share.
      - [x] f1a. The new crate `star_script_engine` (decision 65) holds
            what did not need them, about 250 lines: reading scenes, the
            audio backend, sharing input, the scene player's arguments.
      - [x] f1b. The engine classes (`Game`, `Node`, `Sprite`,
            `Camera2D`, `Scene`, `Body`, `Sound`, `Color`, the 3D ones),
            mirroring nodes and the game loop: their state and methods as
            types of `star_script_engine` that generated classes and the
            interpreter both use, generic over a `Host` (agreed with
            the owner, decision 65).
        - [x] f1b1. `Host`, the fields of `Node`, `Node3D` and
              `Color`, and `Node`'s methods (tree, tags, searches,
              global and local places) in `star_script_runtime::nodes`
              and `color`, with tests on a host of plain `Rc`s.
        - [x] f1b2. `Sprite`, `Camera2D`, `Light2D` and the 3D nodes:
              their fields in `star_script_runtime::node_classes`, the
              methods of `Node3D` in `nodes`, and all of them generated
              from one description each (`codegen/native_nodes.rs`).
        - [x] f1b3. `Body`: its fields, impulses and contacts in
              `star_script_runtime::bodies`; the generated game only
              passes them to the engine and back.
        - [x] f1b4. `Game`, mirroring nodes and the game loop: the
              fields of `Game` and running a frame of the tree in
              `star_script_runtime::game`; the engine's side (mirroring,
              syncing bodies, cameras, drawing, `EngineGame`) generic in
              `star_script_engine::game`. A generated game only creates
              its game and calls `star_script_engine::game::run`.
        - [x] f1b5. `Scene` and `Sound`: building a scene's nodes in
              `star_script_runtime::scene_tree` (`Host::create` makes the
              class a node names, `Host::set_exported` its `export`
              values) and `Sound`'s fields in `audio::SoundFields`.
    - [x] f2. Measure “Play” with scripts on a realistic, big project,
          before and after f1. Measured on a generated game of 101
          classes (3348 lines of StarScript: enemies, bullets, pickups
          and spawners with bodies, tags, lists and input), with the
          engine already built:

          | Rebuild after… | Before f1 | After f1 |
          |---|---|---|
          | Lines of generated Rust | 16 667 | 15 665 |
          | Nothing changed (only linking) | 2.2 s | 2.1 s |
          | A change inside a method | 2.6 s | 2.4 s |
          | A new field in one class | 26 s | 25 s |
          | The game's code from scratch | 37 s | 37 s |

          With one family of classes (2081 lines of Rust after f1) a new
          field costs 7.6 s before and 7.3 s after. f1 saves little time
          by itself: in a big game the user's code outweighs the
          templates, and the shared code is generic, so it is still
          instantiated inside every game. What costs is the game's own
          code at `opt-level = 1`: a change to a class's fields rebuilds
          most of it with optimizations. With the game's code at
          `opt-level = 0` (the engine and the dependencies still at 3) a
          new field took 4 s and a build from scratch 12 s, but a stress
          game (4000 sprites and 1000 balls with bodies, without vsync)
          dropped from 45 to 26 FPS (46 at `opt-level = 3`). Its sprites
          running no code of their own dropped the same (46 to 28): the
          cost is the shared loop (running the tree, mirroring nodes,
          syncing bodies), generic over the `Host` and so built with the
          game's optimization level, not the user's code. So games stay
          at `opt-level = 1` for now. The interpreter (Phase 14) removes
          this cost from “Play” altogether.
- [x] **Phase 14 — StarScript interpreter** (decision 64, rule 3.7 of
      `CLAUDE.md`): “Play” in the editor interprets StarScript, and
      building the project compiles it. Done part by part; each part comes
      with conformance tests that run the same programs compiled and
      interpreted and expect the same output.
  - [x] 14.1 The crate `star_script_interpreter` and the conformance
        harness. Its console part only needs `star_script`; games (14.6)
        also need the engine classes of f1, and its place among the layers
        is settled then, in `CLAUDE.md`. First: values (`short`,
        `int`, `bigint`, `float`, `double`, `bool`, `string`, lists,
        objects, `null`), console programs (`static func main`,
        `Debug.print`), expressions, statements and control flow.
        Design approved by the owner (2026-11-24, decision 66):
        - No external dependencies: `star_script` (lexer, parser,
          analysis) and `star_script_runtime` (the shared rules, such as
          integer division or the text of numbers).
        - Two stages. *Lowering* turns the syntax tree and its `Analysis`
          into the interpreter's own resolved tree: locals are numbered
          slots, every operation knows its type (`int + int` wrapping
          around), conversions such as `short` to `int` are explicit and
          calls point at their function, so running needs no table
          lookups or type decisions. *Running* walks that tree with
          values: `Null`, `Short`, `Int`, `BigInt`, `Float`, `Double`,
          `Bool`, `String` (`Rc<str>`), `List` (shared, as compiled) and,
          from 14.2, objects.
        - Runtime errors without panics: the interpreter returns a
          `Result`. The checks of `star_script_runtime` (division by
          zero, index out of range) become functions that return a
          `Result`, which generated code still turns into a `fault`, so
          both give the same messages.
        - `Debug.print` writes to a `Write` given to the interpreter: the
          console in the player, a buffer in tests.
        - Conformance: the interpreter runs the programs of
          `star_script/tests/programs` (compiled by its end-to-end test)
          and expects the same `.out` and `.fault`, from a list of the
          programs it covers that every step grows. What it cannot run
          yet is reported when the program is lowered, not halfway
          through running it.
    - [x] 14.1a The crate, values and console programs with static
          functions: `main` and calls between static functions,
          literals, variables, arithmetic of the five number types, `as`
          between numbers, comparisons and logic, `if`, `while`, `for`
          over a range, `break`, `continue`, `return`, `Debug.print` and
          `to_string`; with a new conformance program. Done:
          `functions.star` (calls and recursion, wrapping of the five
          number types, `as`, short-circuit logic, loops) and
          `division_fault` run interpreted with the same output; the
          checks of the runtime got versions that return a
          `RuntimeError` (`try_int_divide`, `try_int_remainder`).
          Methods of objects are not lowered (nothing can create an
          object yet), and fields, constants, lists, `null` and the other
          methods are refused where the program uses them.
    - [x] 14.1b Lists, `null`, `??`, constants and the methods that
          `basics.star` uses: `basics`, `division_fault` and
          `index_fault` give the same output interpreted and compiled
          (the other methods stay in 14.3). Done, with every method of
          numbers, strings and lists (they were small, so 14.3 keeps
          only decorators and `Time.now`): lists are shared as compiled,
          a nullable value is the value or `null` (nothing wraps it),
          `?.` on basic types and lists, constants are functions that
          compute their value each time, as compiled. The runtime's
          index check got `try_index`, so both stop with the same
          message.
  - [x] 14.2 Classes: fields, methods, constructors and `init`,
        inheritance and `override`, static members, null safety, number
        widening and `as`, equality, and the runtime errors of
        `star_script_runtime::fault` with the same messages.
        Design approved by the owner (2026-11-24, decision 67):
        - Objects are a new value, `Object(Rc<Object>)`: the class it
          was created as, and the fields of every class of its hierarchy
          in one row, the root's first. Lowering turns each field
          (`class`, index) into its position in that row, so a subclass
          finds its base's fields where the base does. Reading a field
          copies its value and writing replaces it, without keeping
          borrows open, as the compiled `Field` does. Equality is
          identity (`Rc::ptr_eq`); objects that refer to each other are
          not released, as in the compiled game.
        - Methods of objects are functions whose first local is `self`.
          Each class gets a table of its overridable methods, built at
          lowering: a subclass starts from its base's table and puts its
          overrides in their places. A call through an object takes the
          function from its class's table, so each object runs its own
          version; `private` methods and `super.method()` call their
          function directly, as compiled.
        - Construction keeps the two phases of the compiled code: each
          class's constructor first runs its field values and the start
          of `init` up to `super.init(…)` (or to the last assignment of
          a pending field) into locals of its own, without `self`; then
          it calls its base's constructor with its arguments; the root
          creates the object with every field; and then each `init`
          runs the rest of its body with `self`, from the root down.
          The interpreter does it with recursion: each constructor's
          locals stay alive while its base builds the object.
        - `is` asks whether the object's class is the class or a
          subclass (an ancestor table per class, built at lowering);
          `as` to a subclass gives the object or `null`; narrowing after
          `is` or `!= null` changes nothing at run time, since values
          carry their class. `?.` on objects gives `null` without
          running the arguments.
        - Classes that extend engine classes (`Node`, `Game`, …) are
          refused until 14.6; the object leaves room for the engine's
          part (the `Host` fields of decision 65), so nodes will be the
          same objects with their `NodeFields` added. Decorators (14.3)
          are fields and methods the compiler writes, so they will only
          need `Time.now`.
    - [x] 14.2a Objects of classes without a base: fields with and
          without values, `init`, methods of objects and `self`,
          `private` methods, fields of other objects, `?.` on objects,
          equality by identity; with a new conformance program. Done:
          `classes.star` runs both ways with the same output. Programs
          with inheritance are refused when lowered, for now.
    - [x] 14.2b Inheritance: fields of the base, two-phase construction
          with `super.init(…)`, `override` through the tables,
          `super.method()`, `is`, `as` and narrowing: `objects.star`
          gives the same output interpreted and compiled. Done; the
          tables are built bases first, wherever classes are written.
          Classes that extend engine classes are refused until 14.6.
          `decorators.star` only lacks `Time.now` (14.3).
  - [x] 14.3 The rest of the console language: decorators (the user's
        `Decorator` classes) and `Time.now` (the methods of strings,
        lists and numbers, and constants, came in 14.1b). Done:
        decorators are fields and methods the compiler writes, so they
        ran once classes did; `Time.now` reads the game's clock in
        `star_script_runtime::tasks`, the one the compiled game reads,
        so both give the same seconds. `decorators.star` gives the same
        output interpreted and compiled.
  - [x] 14.4 Coroutines: `async`/`await`, `Time.wait` and
        `Time.next_frame`, run once per frame, and ending with their node.
        Design approved by the owner (2026-11-24, decision 68):
        - An interpreted coroutine is a Rust future, so it can stop at an
          `await` and go on in another frame. The body of an `async`
          method is run by a second, `async` version of the runner; it
          becomes a `star_script_runtime::Task` like a compiled `async`
          method, and goes through the same functions: `tasks::start`
          for a call written alone, `.await` for `await`, and
          `tasks::wait`, `tasks::next_frame` and the game's clock for
          `Time`. So interpreted and compiled coroutines share one
          scheduler, and they start, wait and go on in the same order.
          The coroutines of a node will use `Task::owned`, as compiled,
          when nodes come (14.6).
        - Only what can wait is run as a future: lowering marks the
          statements and expressions that contain an `await`, and the
          `async` runner hands everything else to the usual runner. A
          future boxed per step only exists along the way to an
          `await`; methods that are not `async` (`update` and the rest)
          run as now, without allocations.
        - A coroutine outlives the call that started it, so the program
          and its output are shared (`Rc`) instead of borrowed:
          `Program::lower` gives a program that can be cloned at no
          cost, and `Debug.print` writes to an output that the caller
          gives once and can read back (a buffer in tests, the console
          in the player).
        - Runtime errors in a coroutine: the compiled game stops with a
          fault from wherever it happens. The interpreter keeps the first
          error of a coroutine; the console loop (and later the game
          loop) checks it after each turn of coroutines and stops the
          program with it, as it does for errors of `main`. The runtime
          gets `tasks::clear`, so the waiting coroutines of a stopped
          program do not run later.
        - Console programs: `main` (which can be `async`) runs, and then
          the coroutines still waiting go on with the clock moved by
          real time, as `tasks::run_until_finished` does, until none is
          left or one fails.
        - Rejected: rewriting the runner as an explicit stack machine
          whose frames can be kept and resumed (a large rewrite of what
          works, and a scheduler of its own beside the runtime's, whose
          order would have to be kept identical by hand).
    - [x] 14.4a A shared program and output (`Rc`), with no change in
          what programs do.
    - [x] 14.4b `async` methods, `await`, coroutines started by a call
          alone, `Time.wait`, `Time.next_frame`, `async main` and errors
          in coroutines: `coroutines.star` gives the same output
          interpreted and compiled. Done. A failing coroutine stops the
          program at once, as compiled: an error before its first wait
          stops the code that started it, and after one no coroutine
          goes on. The conformance test now also covers `objects.star`
          and `decorators.star`, which had been left out of its list;
          `decorators.star` showed that a decorated method and its
          hidden code shared their parameters' places, so the analysis
          gave both one local (the compiler names locals by their text
          and did not notice): the calling method's parameters now get
          places of their own.
  - [x] 14.5 Parallel functions: `parallel static func` on threads, with
        values copied in and out.
        Design approved by the owner (decision 69, 2026-11-24):
        - Interpreted `parallel` functions run on the same threads as
          compiled ones: an awaited call goes through
          `star_script_runtime::parallel::run`, whose pool, copies and
          timing (the coroutine goes on in the first turn after the work
          ends) are already there. The work it receives is a plain
          function of the interpreter, and what it needs travels in its
          arguments: the lowered program, the function and copies of the
          values.
        - The lowered program is shared between threads (`Arc`, so it
          must be `Send + Sync`): texts become `Arc<str>` (one atomic
          count per copy, no allocation) and the tree's constants a type
          of their own without lists or objects. Each thread runs the
          program with its own values: lists and objects stay `Rc`, on
          one thread, and a `parallel` function only sees copies, which
          the analysis already guarantees.
        - Copies: a value that can cross threads (numbers, `bool`,
          `string`, `null`, and lists of them, deeply) is copied into a
          `Send` form when the work starts and back into values when its
          result arrives, as the compiled `Plain` does; a list changed
          by the function is its own copy.
        - Inside a `parallel` function other `parallel` functions and
          constants are called directly, on the same thread. A runtime
          error there comes back with the result, and the coroutine that
          awaits it stops with it (the path of 14.4), as compiled.
        - Rejected: running them on the game's thread (same output, but
          heavy work would freeze “Play” and behave unlike the built
          game), lowering the program again on each thread (slow, and a
          cache of programs per thread), and lists shared between threads
          (`Arc<Mutex>`: a cost on every list access of the game).
    - [x] 14.5a A program that can be shared between threads (`Arc`,
          `Arc<str>` texts, constants of their own), with no change in
          what programs do. The tree's constants are `Literal`s, which
          give their value when run (a text shares the literal's), and a
          test checks that the lowered program is `Send + Sync`.
    - [x] 14.5b `parallel` functions: awaited calls on the runtime's
          threads with copies, direct calls between them, and their
          errors: `parallel.star` and `parallel_fault.star` give the same
          output interpreted and compiled. Lowering turns an awaited call
          into a coroutine of its own kind (`Coroutine::Parallel`), and
          inside a `parallel` function the same call stays a plain call.
          The thread runs the shared program with a `Machine` of its own,
          whose output goes nowhere, since such a function cannot print.
  - [x] 14.6 Games: `Game` (`start`, `update`, `quit`) in a window of the
        engine, `Input` and its actions, `Color`, `Camera2D`, using the
        engine classes that f1 made shared.
        Design approved by the owner (decision 70, 2026-11-24):
        - The interpreter is one more `Host` of `star_script_runtime`,
          as the generated Rust is: the game loop, the input, the frame
          of the nodes and the window stay in `star_script_runtime` and
          `star_script_engine`, written once, and an interpreted game
          runs through `star_script_engine::game::run::<Interpreted>`.
          The interpreter keeps depending only on `star_script` and
          `star_script_runtime` (a tool, with no engine): the code that
          joins it with the engine is the player of 14.10, and here a
          test. `CLAUDE.md` says so, as 14.1 promised.
        - Engine objects: an object keeps its script fields as now, plus
          the runtime's fields of its engine class (`ColorFields`,
          `NodeFields` with `Camera2dFields`, `GameFields`), which the
          `Host` must hand out by reference. So the object becomes
          generic over that part (`Object<E>`), and a value says which
          kind it holds: `Object`, `Color`, `Node`, `Game`. Code that
          reads script fields works on any of them; the engine's code
          gets its fields with no check that could fail. The classes of
          later steps (`Sprite`, `Body`, `Node3D`…) are types with no
          values in the `Host` for now, so they need no made-up answers.
        - Engine classes in the program's tables: `Game`, `Node`,
          `Camera2D` and `Color` get their place, so `is`, `as` and
          classes that extend them work; `start` and `update` take
          places in the table of overridable methods, doing nothing
          unless a script overrides them. Their fields become tree nodes
          of their own (read and set an engine field), and the methods
          of `Input` call the runtime's functions, as compiled.
        - The `Host`'s functions take no interpreter (`start(node)`), so
          a game is run with the program kept per thread, as the runtime
          keeps its coroutines and input. A runtime error is kept as a
          coroutine's is: the rest of the frame is skipped, the game
          closes after it, and the error is returned (the player of
          14.10 prints it, as a compiled game's fault does).
        - The program is a game when a class extends `Game` (one entry
          point, as compiled: `main` or that class). The names of the
          input actions become owned by the runtime (declared once per
          game), since the interpreter reads them when it starts, not
          when it is built.
        - Tests: console programs with `Input` (`pointer.star`) and a new
          `colors.star` (`Color`, `Camera2D`, classes that extend them)
          in both suites; unit tests that turn frames without a window
          (`star_script_runtime::game::run_frame`); and an ignored test,
          like `engine_game.rs`, that plays the same game interpreted in
          a window, with `star_script_engine` as a dev-dependency of the
          interpreter.
        - Rejected: the interpreter depending on the engine (every
          console test would build wgpu, and the layers would change), a
          game loop of its own (a second copy of what
          `star_script_engine` does, whose order would have to match by
          hand), and one object type with an optional engine part (the
          `Host` must give fields by reference, and a missing part would
          need a panic or a made-up value).
    - [x] 14.6a Engine objects: `Color`, `Camera2D` (with the fields of
          `Node`), classes that extend them, `is`/`as` with engine
          classes, and `Input`: `pointer.star` and `colors.star` conform.
          The interpreter's `Host` (`host.rs`) gives the runtime its
          nodes, cameras and colors; the game and the 3D classes are
          still types without values, and nodes run no code of their own
          until 14.6b. Fields of engine classes and script fields share
          one tree node (`Field` with a `FieldRef`), so coroutines needed
          no new code for them. `Color` cannot be extended (as compiled),
          so `colors.star` keeps colors in an object of its own instead;
          `Node`'s methods, `tags` and `body` stay for 14.7 and 14.8.
    - [x] 14.6b Games: the interpreter as a `Host`, a class that extends
          `Game` as the entry point, owned action names, errors that close
          the game, frames without a window in unit tests and the ignored
          window test. `Program::new_game` creates the game and keeps it
          per thread (`play.rs`) until `end_game`, which gives the error
          that closed it. Each class knows the `start` and `update` a
          script wrote, so the engine's calls need no lookup by name;
          `super.start()` and `super.update(delta)` of the engine do
          nothing. `Game.add_child` came too, since nodes in the game's
          tree is what makes a `main` camera the game's (the methods of
          `Node` stay for 14.7). The window test has no test harness (a
          window needs the main thread) and plays only with
          `-- --ignored`; it passed on this machine.
  - [x] 14.7 Nodes and scenes: `Node`, `Sprite`, `Scene.load`, script
        classes named by scenes, `export var` values, tags and
        `find_by_name`/`find_by_tag`, global and local positions.
        Design approved by the owner (decision 71, 2026-11-24):
        - The methods of `Node` call the runtime's functions of nodes
          (`star_script_runtime::nodes`) with the interpreter's `Host`, as
          the compiled ones do: the tree, `find_child`, `find_by_name` and
          `find_by_tag`, `has_tag`, and the 2D global placement. Lists of
          nodes they give (`children()`, `find_by_tag`) are new lists, so
          they become new lists of the interpreter.
        - `tags` is a list that scripts change and the runtime reads
          (`has_tag`, scenes), so it must be one list for both. The
          runtime stops fixing its type: the `Host` says what a list of
          tags is (`type Tags`, how to make one and whether it has a tag).
          The compiled `Host` keeps `List<String>`; the interpreter gives
          its own lists, so `node.tags.push(…)` is seen by `has_tag`.
          Same output, a small change in the generated `Host`.
        - `Sprite` and `Light2D` join (the 2D node classes), with their
          runtime fields beside those of `Node`, like `Camera2D`'s;
          `Light2D` moves here from 14.8 so `finding.star` can conform.
        - A coroutine of a node's method goes on only while the node is
          in the game's tree, as compiled; `Scene.find_by_name` and
          `Scene.find_by_tag` search the tree of the game being played
          (nothing in a console program).
        - Scenes: `Scene.load` is the runtime's; the interpreter's `Host`
          creates the engine's classes, and the program's classes by the
          name a scene gives, with the reasons of
          `check_scene_class` (shared with the compiler) when one cannot
          be used. Each class keeps its `export` fields by name, and
          their values are converted with the runtime's `scene_fields`,
          so the errors are the same. A program with no `Game` class and
          a main scene plays that scene, as compiled.
        - Tests: `nodes.star` and `finding.star` in the conformance test
          (`placement.star` needs `Node3D`, 14.8); unit tests that load
          scenes from a loader in memory, without files; and the window
          test plays the node game of `engine_game.rs` with its scene
          and expects the same output.
        - Rejected: a second kind of list in the interpreter for `tags`
          (every list operation would handle both), and copying `tags`
          when read (a `push` would be lost, unlike compiled).
    - [x] 14.7a Nodes: the methods of `Node`, `tags` (the `Host`'s
          list, compiled too), `Sprite`, `Light2D`, the life of node
          coroutines and `Scene.find_by_*`: `nodes.star` and
          `finding.star` conform. Every method of the engine that a
          script overrides runs the script's version through an object
          (the engine's `start` and `update` too), and methods of the
          engine take `?.`. `Scene.load` already calls the runtime, which
          finds no scene without a game; reading them is 14.7b.
    - [x] 14.7b Scenes: `Scene.load` with the program's classes, `export`
          fields, main-scene programs, unit tests with scenes in memory
          and the node game in the window test. Lowering keeps, for every
          class a scene can name, its constructor or the analysis's
          reason, and its `export` fields; the runtime's `Host::create`
          and `set_exported` reach the program that loads the scene
          through the thread, as games reach theirs. A runtime error in
          the `init` of a scene's class stops the code that loaded it.
          The node game and its scenes moved to
          `star_script/tests/games/`, so the compiled `engine_game` test
          and the interpreter's new `window_scene` test (a window opens
          once per process) play the same files; both passed on this
          machine with the same output.
  - [x] 14.8 Physics, sounds and 3D (`Body`, `Sound`, `Audio`,
        `Node3D`, `Mesh`, `Light`, `Camera3D`).
        Design approved by the owner (decision 72, 2026-11-24):
        - Bodies and sounds become objects of the interpreter with the
          runtime's fields, as colors are: a value of kind `Body` holds
          `BodyFields` (the `Host`'s body becomes that object), and one of
          kind `Sound` holds `SoundFields`. `Body.circle`, `rectangle`,
          `capsule`, `sphere` and `box`, `apply_impulse(_3d)`, the
          contact lists (new lists of the interpreter, as `children()`),
          `Sound.load`/`play`/`stop`/`is_playing` and `Audio.volume`/
          `set_volume` call the runtime, so they need no engine: without
          a game nothing simulates bodies and no sound loads, as
          compiled. A node's `body` field is the runtime's.
        - The game loop of `star_script_engine` already moves bodies,
          reads contacts and plays sounds through the `Host`, so an
          interpreted game gets physics and audio with no code of its own.
        - 3D: a node's parts gain the fields of `Node3D` (kept apart, like
          a camera's, so the `Host` can hand out a `Node3D`), and its
          class parts `Mesh`, `Light` and `Camera3D`. Their methods
          (`look_at`, `global_z`, `set_global_position_3d`,
          `translate_3d`, `translate_global_3d`) join those of `Node` as
          runtime calls that scripts may override; `Game.camera_3d`
          joins `camera`; scenes create 3D nodes too. Classes may extend
          all of them.
        - With this, every program of the conformance test runs
          interpreted, so its list of covered programs goes: every
          program in `programs/` must behave the same both ways, and a
          new feature cannot be added to one side only (rule 3.7).
        - Window tests: the window opens once per process, so the
          `window` test becomes one binary that plays each game in a
          child process of itself (`window -- --ignored` starts them all,
          one after another). The games of `engine_game.rs` (counting,
          node, physics, physics 3D, coroutine and the scene-only game)
          move to `star_script/tests/games/` with their scenes, and each
          one is played interpreted expecting the output the compiled
          test expects; `window_scene` folds into it.
        - Rejected: a body or sound held directly in a value without an
          object (it would need a second path for `is`, `==` and fields,
          which objects already have), and simulating bodies outside the
          engine (the compiled console program does not either).
    - [x] 14.8a Bodies and sounds: `Body`, `Sound`, `Audio` and a node's
          `body`; `bodies.star` and `sounds.star` conform.
          Values of kind `Body` and `Sound` are objects with the runtime's
          `BodyFields` and `SoundFields` (the interpreter's `Host::Body` is
          that object, so bodies the runtime makes for scenes are ones
          too); `Body`'s static constructors, impulses and contact lists
          (new lists of the interpreter), `Sound.load`/`play`/`stop`/
          `is_playing` and `Audio.volume`/`set_volume` call the runtime.
          Static methods of the engine are now recognized by being
          static, not by their class. Unit tests cover contacts set by the
          physics, impulses that add up until the physics takes them, and
          sounds that do not load without audio.
    - [x] 14.8b 3D: `Node3D`, `Mesh`, `Light`, `Camera3D`, their methods,
          `Game.camera_3d` and 3D nodes in scenes; `nodes_3d.star` and
          `placement.star` conform, and the conformance test covers every
          program.
          A 3D node's parts hold `Node3dFields` in an `Rc` (the `Host`'s
          `Node3D`), and `Camera3D`'s fields are kept apart as
          `Camera2D`'s (the `Host`'s camera reference is generic over
          them). The methods of `Node3D` are runtime calls that scripts
          may override, scenes create every 3D node, and classes may
          extend them all. The conformance test now runs every program of
          `programs/`, and a new unit test checks that lowering knows
          every method and field of every engine class, so nothing of the
          engine is left to the compiler alone. One gap found on the way,
          older than this step: reading a constant through an object
          (`Main().SIZE`) compiles (the object is evaluated for its
          effects) but the interpreter refuses it; it needs an expression
          that evaluates one value and gives another, in both runners.
          Closed after 14.9: `Expression::Then` runs the object for its
          effects and gives the constant, in both runners, with the
          conformance program `programs/constants_through_objects.star`.
    - [x] 14.8c Games in windows: the window test plays every game of
          `engine_game.rs` interpreted, each in its own process.
          The games of `engine_game.rs`, their scenes and their expected
          output (`.out`) are files of `star_script/tests/games/`, read by
          the compiled test and by the interpreter's `window` test, which
          plays each one in a child process of itself (`window --play
          <game>`) with the project's actions and an asset folder of its
          own: counting (with `Sound` and `Audio`), a misspelled binding,
          nodes and scenes, 2D and 3D physics, coroutines and a missing
          main scene. `window_scene` folded into it. Both passed on this
          machine with the same output.
  - [x] 14.9 Blueprints: they become StarScript, so they need no
        interpreter of their own; conformance tests with the blueprint
        templates.
        Design approved by the owner (decision 73, 2026-11-24):
        - A blueprint runs as the StarScript `star_blueprint::translate`
          writes for it, compiled or interpreted, so neither side needs
          code for blueprints: what must be checked is that their
          translations behave the same both ways.
        - Programs from blueprints: for each template of the editor,
          `star_script/tests/programs/blueprint_<template>.star` holds the
          template's translation followed by a `Main` that puts it under a
          parent, runs its `update` for a few frames with a fixed `delta`
          and prints its fields, with its `.out`. The compiled end-to-end
          test and the interpreter's conformance test already run every
          program there, so they compare them with no change.
        - Those programs must not go stale: a test of the editor (where
          the templates live) checks that each one starts with the
          current translation of its template and that every template has
          one; when a template or the translator changes, it fails with
          the new translation in its message.
        - One more blueprint, kept with `star_blueprint`'s tests, uses
          every kind of node (literals, variables, calls of the engine and
          of its own methods, operators, branches, “For each” and
          returns), and `programs/blueprint_every_node.star` is its
          translation with a `Main`, checked the same way by a test of
          `star_blueprint`: the templates only use a few kinds of node.
        - The compiled end-to-end test also runs the crate's example
          script with a `Main` of its own, which the interpreter's
          conformance test does not: that `Main` and its output move to
          files that both tests read.
        - Rejected: translating blueprints inside `star_script`'s tests (a
          cycle of dependencies between `star_script` and
          `star_blueprint`), and a runner of its own for blueprints (the
          runners of programs already compare both ways).
    - [x] 14.9a The templates: one program per template, and the editor's
          test that keeps them up to date.
          `programs/blueprint_<template>.star` (lowercase names) for the
          seven templates, each running its `update` for a few frames of
          0.25 s. The movement templates read input actions, which a
          program must declare to compile, so a program may come with
          `NAME.actions` (one action per line, no bindings), read by both
          the compiled and the interpreted runners; without a window no
          action is held, so their axes give 0. The editor's test
          `every_template_has_its_conformance_program` checks each
          program's start and its `.actions` against the template.
    - [x] 14.9b Every kind of node, and the example script in the
          interpreter's conformance.
          `star_blueprint/tests/every_node.blueprint` (`Tally`: a “For
          each” summing a list, a branch on `%` and `==`, calls of
          `Debug.print`, of its own `parity` and of the engine's
          `translate`, and returns) is
          `programs/blueprint_every_node.star` with a `Main`; the test
          `star_blueprint/tests/conformance.rs` keeps it current. The
          example script's `Main` and output are
          `star_script/tests/example/player_main.star` and `.out`, read
          by the end-to-end test and by the interpreter's new
          `the_example_script_behaves_as_compiled`.
  - [x] 14.10 The interpreted player: the scene player of step e grows
        into a player that loads the project's scripts when it starts;
        “Play” uses it for every project, and any project plays in about
        a second.
        Design approved by the owner (decision 74, 2026-11-24):
        - A new crate, `star_player` (Tools), joins the interpreter with
          the engine: `star_player::play(directory)` reads the project in
          `directory` and plays it interpreted through
          `star_script_engine::game::run::<Interpreted>`, as the window
          test of 14.8c does. It depends only on crates of the workspace
          (`star_engine`, `star_script`, `star_script_interpreter`,
          `star_script_engine`, `star_blueprint`) and on `serde` and
          `toml`, which the workspace already uses: no new external
          dependency.
        - Reading a project, written once: the settings of `project.toml`
          (input actions, main scene, title), now read by the editor's
          `project_check/input_actions.rs`, move to `star_player::project`,
          and the editor uses them from there. The player gathers the
          `.star` files of `assets/` and the translations of its
          `.blueprint` files into one `SourceSet`, as the editor's check
          does (that one keeps its own code, which places mistakes on
          blueprint nodes for the Problems panel).
        - What it does when it starts: a project that does not check or
          lower prints the compiler's reports (file, line and column) to
          the error output and ends with failure; the editor already
          refuses to play a project with problems, so this only guards
          files changed outside it. Then it builds the action map from
          the bindings (a wrong binding stops it, as compiled), sets the
          assets folder and the window's title, and plays. `Debug.print`
          writes to the standard output and a runtime error is printed
          as a compiled game prints it, `StarScript runtime error: …`,
          ending with failure, so the editor's console shows both the
          same way as today.
        - How the editor runs it: the player is built outside the
          editor's workspace, as the scene player is (decision 63: the
          workspace's features differ from games'): the editor writes a
          package of one `main` that calls `star_player::main()`, in its
          folder of games, which the warm-up builds. “Play” saves (as
          now), checks the project, lets Cargo confirm the player is up
          to date (about 0.3 s) and runs it in the project's folder with
          no arguments. That is the same path for every project, with or
          without scripts, so any project plays in about a second.
        - What goes away: “Play” no longer builds a game per project, so
          the compiled scene player of decision 63 has no use and is
          removed (`star_script::player_program`, `generate_player`,
          `star_script_engine::player`, and its part of
          `engine_game.rs`), as is the editor's package per project.
          The check keeps translating to Rust, for the mistakes only the
          compiler finds and for “Build” (14.11), which builds the game
          with Cargo again. A project without scripts or main scene (a
          game written in Rust) still has nothing to play.
        - Tests: unit tests of `star_player::project` (settings, scripts
          and blueprints read from a folder, reports of a broken
          project); a window test of `star_player`, ignored like the
          others, that plays small project folders in child processes
          (a game with a blueprint, a scene-only project, a project with
          a mistake and one with a wrong binding); the editor's tests of
          “Play” and the warm-up run the player instead of the scene
          player.
        - Rejected: the player as a binary of the editor's workspace
          (the features of decision 63), the editor sending the program
          to the player instead of the player reading the project (14.12
          needs the player to read the files again anyway, and “Play”
          already saves first), and keeping compiled builds in “Play”
          until 14.11 (two ways to play, when the interpreter covers the
          whole language).
    - [x] 14.10a The crate `star_player`: reading a project (the settings
          moved from the editor) and playing it, with its tests.
          `star_player::project::Project::read` reads `project.toml`
          (`read_game_settings`, moved from the editor's
          `project_check/input_actions.rs`; the editor also takes
          `MANIFEST_FILE_NAME` and `ASSETS_DIRECTORY` from there) and the
          `.star` and `.blueprint` files of `assets/` (sorted, hidden ones
          left out), and `Project::program` checks and lowers them; its
          `Problems` name each file, line and column, or a blueprint's
          graph and node. `star_player::play` plays a game in a window
          (or runs a console program), and `star_player::main` plays the
          current folder. The interpreter gained `Program::title`, the
          window's title as compiled (the `Game` class's name, or the
          project's for a main scene). Tests: 8 unit tests of reading
          projects (with the 4 of the settings, moved), and
          `tests/window.rs` (ignored), which plays 5 project folders in
          child processes: a game with the blueprint of every node, a
          scene-only project with a missing scene, a script mistake, a
          misspelled binding and a runtime error while the game is
          created.
    - [x] 14.10b The editor plays every project with it: the warm-up
          builds it, “Play” runs it, and the compiled scene player is
          removed.
          `game_run/player.rs` writes the package `_star_player` (its
          `main` calls `star_player::main`, with the repository's
          lockfile and the engine's profile) in the editor's folder of
          games; the warm-up builds it, and “Play” lets Cargo confirm it
          is up to date (0.26 s measured) and runs it in the project's
          folder. `PlayRequest` is only the folder now (`PlayedGame`, the
          package per project and `package_name` went away), and the
          check's `ProjectCheck` lost `scene_only`: its `program` only
          tells whether there is a game (`ProblemsPanel::playable`,
          refusing with the same messages as before). Removed:
          `star_script::player_program`, `codegen::generate_player`,
          `EntryPoint::Player`, `star_script_engine::player` and the
          scene player's part of `engine_game.rs` (whose package folder
          now drops programs of earlier runs). The texts of “Play” speak
          of preparing the player instead of building the game. Tests:
          the editor's tests play a console project and a project whose
          main scene is missing through the player, and the warm-up's
          test builds it.
  - [x] 14.11 Building with the compiler from the editor: a “Build”
        command (and later the export of games) that compiles the project
        with Cargo, and a way to play that compiled build to check it.
        Design approved by the owner (decision 75, 2026-11-24):
        - One command, “Play compiled”, next to “Play” in the bar (and
          its menu): it saves, checks the project as “Play” does, writes
          the game's generated Rust as a package of its own, builds it
          with Cargo and runs it in the project's folder, with Cargo's
          output and the game's in the Output panel and “Stop” working as
          now. It is how a project is checked the way it will ship, since
          “Play” runs it interpreted. A command that only builds, without
          running, waits for the export of games: until then nothing
          would use what it builds.
        - Where and how it builds: as “Play” built before 14.10b, in the
          editor's folder of games, one package per project named from
          the project (`My Game` → `my_game`), with the repository's
          lockfile, in the build profile of development (dependencies
          optimized). Its target folder is the player's, so the engine,
          the runtime and `star_script_engine` that the warm-up built for
          the player are reused: only the game's own code is compiled
          (seconds; minutes only after the engine changes). Release
          builds, with their own long build of the engine, belong to the
          export.
        - What it builds: the `GeneratedProgram` the check already keeps
          (`ProjectCheck::program`). A project without a game or main
          scene is refused with the messages “Play” gives.
        - In code: `PlayRequest` gains how it is played,
          `Interpreted` (the player) or `Compiled { name, program }`;
          `game_run` writes, builds and runs either one through the same
          steps, and the Output panel says “Preparing the player…” or
          “Building the game…” as fits. The package writing removed in
          14.10b (`write_package`, `package_name`) comes back for it.
        - Tests: the editor's test that a console project is built and
          run compiled, the package name and lockfile tests (restored),
          and a new one that plays a small project both ways and expects
          the same output, the editor's own view of rule 3.7.
        - Rejected: a release build (it rebuilds the whole engine with
          LTO, minutes each time the engine changes: the export's job),
          a “Build” that does not run what it builds (nothing to do with
          the result until the export), and building inside the
          project's folder (a target folder per project, each building
          the engine again).
        Done: “Play compiled” sits after “Play” in the bar and in the
        Project menu. `game_run::PlayMode` is `Interpreted` or
        `Compiled { name, program }`, `game_run/compiled.rs` writes the
        game's package (`write_game`, with `package_name` and
        `write_package` back from before 14.10b), and the workspace's
        `PlayWay` says how to play once the check ends;
        `ProblemsPanel::played_program` gives the program or why there
        is nothing to play. The Output panel says “Preparing the player…”
        or “Building the game…”. Tests: the package name and lockfile
        tests (restored), a console project built and run compiled, and
        `projects_play_the_same_interpreted_and_compiled` (classes, a
        loop, integer and float division and a coroutine that goes on
        after `main`, the same output both ways).
  - [x] 14.12 Hot reload: changed scripts reload while the game is
        played, keeping the state of the objects that are still valid.
        Design approved by the owner (decision 76, 2026-11-24):
        - What sets it off: saving. While it plays, the player looks
          every half second (as `star_assets`' `FileWatcher` does, by
          modification times: no new dependency) at the `.star` and
          `.blueprint` files of `assets/`, new and removed ones included.
          When they changed, it reads, checks and lowers the program
          again, between two frames. Changes to `project.toml` (actions,
          main scene) and to scenes are not reloaded: they need a new
          “Play”.
        - A program with mistakes is not loaded: the player prints them,
          as at the start (`file:line:column: message`), saying the game
          goes on with the code it had, and the next save tries again.
          They reach the editor's Output panel as the game's error
          output does now.
        - Which objects stay: every object the program created (the
          game, its nodes, every object of a script class) keeps
          existing, with its engine part (position, body, sound…) and
          its place in the tree. Its class is found again by name in the
          new program, and its fields by name: a field that is still
          there with the same type keeps its value; a new field, or one
          whose type changed, gets its initial value as when an object
          is created (field initializers cannot see the object, so they
          can run for it); a removed field is dropped. Methods, new
          classes and constants simply are the new code's from then on;
          `start` does not run again.
        - When it cannot: if an object that exists belongs to a class
          that was removed, or that no longer extends the same engine
          class (its engine part could not change), nothing is reloaded,
          and the player says which class and that a new “Play” is
          needed.
        - Coroutines started before the reload end: they run the old
          code, which knows the old places of the fields. The message of
          the reload says how many ended, so a game whose `start`
          started one knows to play again. (Keeping them would need
          every class and field to keep its place across reloads, with
          old and new code mixed: a later step, if it is missed.)
        - In code: the interpreter remembers the objects of the game it
          plays (weak references, added when an object is created, so a
          dropped object is not kept alive), and an object's class can
          change (`Cell<ClassRef>`). `star_script_interpreter::reload`
          gives the game playing on this thread a new `Program`: it
          checks every object first, then moves classes and fields and
          swaps the code, and says how many objects it kept and how many
          coroutines ended, or why it could not. `star_script_engine`'s
          game loop gains a version of `run` with a step between frames,
          where the player looks at the files and reloads; compiled games
          keep `run`. Hot reload is part of playing in the editor, not of
          the language, so the compiled side has nothing to match (rule
          3.7 is about the language).
        - Tests: the interpreter's (headless, as its game tests are): a
          changed method takes effect while fields keep their values; a
          new field gets its initial value; a field whose type changed is
          reset; a removed field is dropped; a removed class of a living
          object refuses the reload and keeps the old code; coroutines
          end and are counted. The player's: noticing changed, new and
          removed files; a program with mistakes is reported and not
          loaded. A window test (ignored) whose game prints its frames,
          while the test rewrites a script and expects the new output.
        - Rejected: restarting the game on every change (the state is
          what hot reload is for); keeping coroutines on the old code
          (field places differ); a file-watching dependency (polling
          already works on every platform); the editor sending the
          program through a pipe (the player already reads the files,
          and saving is the signal); restarting the game when a class
          of a living object goes away (it would lose the state without
          being asked; a new “Play” does that).
    - [x] 14.12a The interpreter: objects remembered, `reload`, with its
          tests. Every object of a class of the program is remembered
          when its root constructor makes it (weak references in the
          `Machine`, shared by its copies; dropped ones are forgotten
          whenever the list doubles). Each `Class` of the lowered code
          has its name and the fields it declares (`FieldLayout`: name,
          type as StarScript writes it, and a function that gives the
          value written in the class or the type's default, `None` if
          only `init` gives one); an object's class is a `Cell`, and
          `Object::reshape` gives it a new class and row.
          `star_script_interpreter::reload(&Program)` plans every living
          object first (class by name, same engine class, each field of
          the new row kept by name and type or started), then runs the
          values of the new fields, reshapes the objects, ends the
          coroutines (counted) and swaps the program of the game playing.
          Added to the design: a new field that only `init` can give a
          value refuses the reload too (`ReloadError::FieldNeedsInit`),
          since its objects already ran `init`. Tests (`tests/reload.rs`):
          changed methods with kept fields; kept, started, reset and
          dropped fields of a game, a node and a plain object; a removed
          class and a changed engine class keep the old code; a new
          field only `init` gives a value; coroutines end and are
          counted; dropped objects are not remembered; only a game
          playing is reloaded.
    - [x] 14.12b The player: watching the files, reloading between
          frames through `star_script_engine`, with its tests and the
          window test. `star_script_engine::game::run_with_step` calls a
          step before each frame's code with the seconds since the last
          one (`run` is it with an empty step, for compiled games).
          `star_player`'s `watch::Scripts` looks at the program files of
          `assets/` (the same listing `Project::read` uses) every
          `DEFAULT_CHECK_INTERVAL` of `star_assets`, by path,
          modification time and length, so new and removed files count.
          When they changed, the player reads the project again and
          reloads its program, saying on the error output (the standard
          output is the game's) how many objects it kept and coroutines
          ended, or the mistakes or the reason it was not reloaded, with
          “the game goes on with the code it had”. Tests: the watcher
          (changed, new and removed files; other files are not the
          program), reloading a saved project into a game playing, a
          program with mistakes and one its objects cannot follow; the
          window test plays a game that prints with its old script,
          saves the script again while it plays and expects the new
          output with the frames kept.
- [x] **Owner's requests (2026-11-25)**:
  - [x] Blueprint wires can be selected with a click (drawn wider, in
        the selection's color) and removed with `Delete` or the right
        button's “Remove wire”.
  - [x] `Time.delta()`: the seconds of the frame going on, the `delta`
        that `update` gets, for code away from `update`; compiled and
        interpreted, so also in the code editor and the blueprints'
        “Functions”.
  - [x] Referring to the node of a script: nothing new, since a script's
        class is the node, so `self` (written or implicit) already is it.
  - [x] A script with mistakes dragged onto a node said that none of its
        classes extends another; it now says it has mistakes, and the
        first one.
- [x] **Phase 15 — Animation editor, 2D and 3D** (owner's request,
      2026-11-25; done part by part, each designed with the owner first).
      Ideas taken from Godot's `AnimationPlayer`, `AnimatedSprite2D` and
      `AnimationTree`, and Unity's Animator, without copying them:
  - [x] 15a Frame animations for 2D: named clips of sprite sheet cells
        (frames per second, loop), played by a node from StarScript
        (`play`, `stop`, the clip that ends), saved in the project. Agreed
        with the owner: the clips live in their own `.frames` file of the
        assets (reused by any scene and node), the existing `Sprite` plays
        them (no new class), and a script learns that a clip ended through
        an event it overrides, `animation_finished(name)`, like `update`.
    - [x] 15a.1 Sprite sheet cells everywhere: a sprite's image is cut into
          `columns` x `rows` cells and shows one, its `frame` (counted from
          the top-left, row by row); in the engine's sprites
          (`SpriteSheet`), the scene files, the inspector, and StarScript's
          `Sprite` (compiled and interpreted). In scenes the cells cut the
          sprite's `region`; any value shows a cell (frames go round the
          sheet), so scripts can step `frame` without checks.
    - [x] 15a.2 The `.frames` file (image, cells, clips with their frames,
          speed and loop) and playing it (decision 78): `Sprite`'s field
          `frames` and `play(name)`, `stop()`, `is_playing()` and
          `animation()` (methods, as `Sound`'s), the frame moved on by the
          runtime after each `update`, and the clip a scene's sprite starts
          with (`[node.sprite]` `frames` and `animation`).
    - [x] 15a.3 The `animation_finished(name)` event, also as a blueprint
          graph: a sprite's method the runtime calls when a clip that does
          not loop ends (compiled and interpreted, with a game of each
          `engine_game` and `window` play), and an entry of the blueprint's
          “Graphs” menu for each event of its engine base it does not
          redefine yet (so `start` and `update` can come back too).
    - [x] 15a.4 The editor's frames panel: cutting the sheet, picking each
          clip's cells by clicking them, speed, loop and a preview; the
          inspector chooses a sprite's `.frames` and first clip. A tab of
          its own for `.frames` files, made from an image's menu (“New
          animations”); choosing a sprite's file copies its image, cells
          and first frame into the sprite, so the scene shows it.
  - [x] 15b Keyframe tracks: any property of a node and its children
        (position, rotation, scale, color, visibility, exported fields) over
        time, with interpolation (step, linear, eased) and calls to methods
        at given times; one animation file read by the scene's node. Agreed
        with the owner: a new node class, `Animator` (a child that animates
        its parent and the nodes under it, by path), plays them; they live
        in their own `.anim` file of the assets (several named animations,
        each with its length, loop, property tracks and call tracks); calls
        take no arguments; and `Node` gains `visible` first, so it can be
        animated.
    - [x] 15b.1 `visible` everywhere: a hidden node and the nodes under it
          are not drawn nor light the scene, but still move, collide and run
          their code; in the engine (`Hidden`, kept down the tree by the
          transform propagation as `HiddenInTree`), scene files (`visible =
          false`), the inspector and the scene view (hidden nodes are not
          drawn nor picked), and StarScript's `Node` (compiled and
          interpreted).
    - [x] 15b.2 The `.anim` file: animations with a length and a loop,
          property tracks (a node's path, a property, an interpolation:
          step, linear, ease in, out or in-out; keys of time and value:
          numbers, `bool`, `int`, text and colors) and call tracks (a
          node's path, a time and a method without arguments), validated
          when read (`star_scene::TrackAnimations`, decision 79).
    - [x] 15b.3 The `Animator` node: its file, `play(name)`, `stop()`,
          `is_playing()`, `animation()` and the event
          `animation_finished(name)`, the starting animation of a scene's
          animator, and the property tracks applied after each `update`
          (the engine nodes' fields and the classes' `export` fields;
          compiled and interpreted, with a game of each `engine_game` and
          `window` play: `animator_game`, from a scene).
    - [x] 15b.4 Call tracks: the methods called as the playhead passes
          their times (also when a frame jumps over several, and when an
          animation loops), with a game of each `engine_game` and `window`
          play (`animator_game` calls the knight's `swing` and `strike`).
    - [x] 15b.5 The editor: `Animator` among the nodes to add, its file and
          starting animation in the inspector, `.anim` among the assets
          (created from Create > Keyframe animations with an example in
          comments; editing them is 15c's timeline).
  - [x] 15c The editor's timeline (designed with the owner, 2026-11-26:
        a panel under the scene for the animator selected, a preview that
        leaves the scene as it was, keys from a diamond in the inspector,
        and everything below in this step):
    - [x] 15c.1 A track's value at a time in `star_scene`
          (`PropertyTrack::value_at`, as the runtime plays it) and a
          node's properties by StarScript's names in the editor
          (`animated_properties`).
    - [x] 15c.2 The “Animation” tab: animations chosen, created, renamed,
          duplicated and deleted, their length and loop, play, pause and
          stop, the ruler and playhead, lanes of keys and calls, the
          preview on the scene view, and the `.anim` files in the scene's
          undo history and saved with it.
    - [x] 15c.3 Key buttons in the inspector, for the engine's properties
          and the `export` fields.
    - [x] 15c.4 Keys and calls selected, dragged, deleted, copied and
          pasted; a double click adds a key; the time and value of the one
          selected typed.
    - [x] 15c.5 Each track's interpolation, chosen with its curve drawn,
          and call tracks added from a menu of the methods an animation
          can call.
  - [x] 15d Tweens from code (designed with the owner, 2026-11-26,
        decision 80): `tween`, `tween_by`, `tween_color` and `tween_with`
        with `TweenOptions` (a delay, a way back, repeats), `async`
        methods of `Node`, and `stop_tweens()`; a new tween of a property
        takes the place of the old one. Compiled and interpreted, with the
        conformance programs `tweens` and `tween_fault` and the game
        `tween_game` of `engine_game` and `window`.
  - [x] 15e Transitions between animations: a state machine (idle → run →
        jump) with conditions, cross-fades and blends by a value (walk/run
        by speed), editable as a graph like the blueprints. Agreed with the
        owner (2026-11-26): a `.states` file of the assets, named by an
        `Animator`'s new field `states`, decides which animations of its
        `.anim` file play; without it the animator plays as before. States
        play one animation or blend several by a number; transitions go
        from a state (or from any) to another, with conditions on values
        the scripts set and a cross-fade; scripts use `set_value`,
        `set_flag`, `trigger` and `state()`, and `play` jumps to a state.
    - [x] 15e.1 The `.states` file (`star_scene::AnimationStates`): values
          (numbers, flags, triggers), the starting state, states (an
          animation, or a blend of animations at points of a number) with
          their place on the graph, and transitions (from, to, conditions,
          fade, waiting for the end), validated when read (decision 81).
    - [x] 15e.2 Cross-fades in the runtime: two animations' tracks applied
          together by weights (decimals and colors blended, the rest
          jumping halfway; a property only one changes keeps its value),
          `star_script_runtime::poses`.
    - [x] 15e.3 The machine in the runtime (`star_script_runtime::machines`)
          and StarScript's API (`states`, `set_value`, `set_flag`,
          `trigger`, `state()`; `play` jumps to a state), compiled and
          interpreted, read from scenes (`[node.animator]` `states`), with
          the conformance program `animator_states` and the game
          `states_game` of `engine_game` and `window`.
    - [x] 15e.4 Blends by a value (walk/run by speed), synced by phase
          (done with 15e.3: a blend state mixes the two animations around
          its value, their rounds lasting the lengths by their weights).
    - [x] 15e.5 The editor: a graph tab for `.states` files, states as
          boxes and transitions as arrows, edited in the inspector;
          Create > State machine, and the animator's “States” row.
  - [x] 15f Skeletons (agreed with the owner, 2026-11-26: cut-out 2D
        first, and the `gltf` crate to import 3D models):
    - [x] 15f.1 2D bones: bone nodes that sprites hang from (cut-out
          characters), animated by the `.anim` tracks; sprites that bend
          with weighted polygons may come later. Decision 82: `Bone2D`
          with a `length` (compiled and interpreted, conformance program
          `bones`), `[node.bone]` in scenes, and the editor's “2D bone”
          (drawn, picked, and new bones under a bone start at its end).
    - [x] 15f.2 Importing glTF models (meshes and materials), wrapped by
          `star_assets`. Agreed with the owner (2026-11-26): a new node
          class `Model` (extends `Node3D`) with a `file` field shows a
          `.glb`/`.gltf` whole, every mesh and material of it.
      - [x] 15f.2a Models in memory (`star_core::Model`) and reading glTF
            files (`.glb`, `.gltf` with its files beside it) in
            `star_assets`, through the `gltf` crate.
      - [x] 15f.2b Showing models: the engine's `ModelNode`, drawn with the
            3D meshes, and `[node.model]` in scenes.
      - [x] 15f.2c StarScript's `Model`, compiled and interpreted
            (conformance program `models`), read from scenes and drawn by
            the engine through the mirror of the script's nodes.
      - [x] 15f.2d The editor: `.glb`/`.gltf` among the assets, dropped on
            the scene as a `Model` (3D projects), its inspector (the file)
            and Add component > Model, drawn in the view and the camera
            preview; models are picked as plain 3D nodes, by their
            position.
    - [x] 15f.3 3D skinned meshes: a model's skeleton moving its mesh on
          the GPU. Agreed with the owner (2026-11-26): the joints stay
          inside the model (they are not nodes of the scene), and the
          model's animations are played by an `Animator` whose
          `animations` names the `.glb`.
      - [x] 15f.3a The skeleton (joints, rest pose, inverse bind
            matrices), each vertex's joints and weights, and the
            animations (channels of translation, rotation and scale) read
            from glTF into `star_core::Model`.
      - [x] 15f.3b Skinning on the GPU: a skinned variant of the 3D
            pipeline, with each draw's joint matrices.
      - [x] 15f.3c The engine poses a model: its joints from the rest pose
            or from animations at times and weights, and draws its skinned
            parts with them.
    - [x] 15f.4 A model's animations played by an `Animator` (and its
          states): an animator whose `animations` is a `.glb` plays its
          clips (they loop) on its parent `Model`, with fades and blends of
          `.states` machines, compiled and interpreted; the editor offers
          models among an animator's files. Done: each animator records
          the clips it shows (`tracks::shown_clips`), the mirror gives
          them to its parent `Model` as a `ModelPose`, and the game
          `model_game` (a skinned `wave.glb`) plays the same compiled and
          interpreted.
- [ ] **Phase 16 — Ready-made nodes and tools** (owner's request,
      2026-11-25; mostly after Godot's nodes and templates). Each is an
      engine node or a class the editor offers, usable from StarScript and
      blueprints, and editable like any other:
  - [ ] 16a Character controllers: a body that moves and slides along
        walls and floors (Godot's `CharacterBody2D`/`3D` and
        `move_and_slide`), knowing if it is on the floor, a wall or the
        ceiling, with slopes and steps.
  - [ ] 16b Ready players: `Player2D` (top-down and platformer, with
        jump, gravity and the project's actions) and `Player3D` (first and
        third person, with a mouse/stick camera), each with physics or
        without (moving the transform), as nodes or as script and
        blueprint templates.
  - [ ] 16c Cameras that help: following a node with smoothing, limits and
        a dead zone, screen shake; a 3D orbit camera.
  - [ ] 16d Detection and queries: areas that report what enters and
        leaves (sensors as nodes), ray casts and shape casts as nodes,
        visible-on-screen notifiers.
  - [ ] 16e Gameplay helpers: timers, paths with followers, spawners,
        health/damage, a pool of reusable objects.
  - [ ] 16f Worlds: tile maps for 2D (a tile set, painting in the editor,
        collisions per tile), parallax backgrounds, 3D grid maps.
  - [ ] 16g Navigation: walkable areas and agents that find paths around
        obstacles, 2D and 3D.
  - [ ] 16h Interface nodes for games: labels, buttons, bars, panels and
        containers, anchored to the screen.
- [ ] **Phase 17 — Ready-made optimization** (owner's request,
      2026-11-25), above all for 3D, from the techniques most used in the
      industry. Each is measured on an example before and after, and kept
      only if it pays off (as the faster “Play” steps were):
  - [ ] 17a Frustum culling: what is out of the camera's view is not drawn
        (bounding boxes and spheres per mesh, 2D and 3D).
  - [ ] 17b Occlusion culling: what is hidden behind walls is not drawn
        (occluders placed or baked, or a depth pre-pass), for 3D indoors.
  - [ ] 17c Level of detail (LOD): simpler meshes far from the camera,
        generated or given, with visibility ranges per node, and impostors
        (billboards) for the farthest.
  - [ ] 17d Batching and instancing: many copies of one mesh in one draw
        (GPU instancing, Godot's `MultiMesh`), static meshes merged into
        one, and fewer state changes by sorting draws by material.
  - [ ] 17e Texture atlases: small images packed into one texture by the
        editor, so sprites and materials share draws.
  - [ ] 17f Textures: mipmaps, GPU compressed formats per platform (BC on
        desktops, ASTC/ETC2 on mobiles) and size limits on export.
  - [ ] 17g Baking: lightmaps and ambient occlusion baked into textures for
        static scenes, reflection probes, and baking materials into simple
        textures.
  - [ ] 17h Shadows and lights within budget: shadow cascades and
        distances, lights culled per object or clustered (decision 48 left
        room for them behind `Frame`).
  - [ ] 17i Profiling in the editor: frame time, draws, triangles and
        memory, so the right technique can be chosen.
- [ ] **Owner's requests (2026-11-26)**, done one by one, each with a
      plan agreed with the owner first:
  - To fix:
    - [x] 3D collision shapes follow their node: a `Body3D` on a simple
          mesh that was changed (a cube scaled, for example) gets a
          collision shape of the mesh's real size, not of the unscaled
          one. Complex models (a soldier, for example) can still take a
          simple shape (a capsule), and also an automatic one that fits
          every polygon of the model (a triangle mesh): costly, but the
          engine is a tool and must spare the developer that work.
          Plan agreed with the owner (decision 84):
      - [x] a. Shapes follow the global scale of their node, in 2D and
            3D (as in Unity): boxes and rectangles along each axis,
            spheres and circles by the largest one, capsules with their
            height by y and their radius by the largest of the others; a
            scale that changes while playing rebuilds the shape, and the
            editor draws it scaled.
      - [x] b. A `mesh` shape for 3D bodies, made from the node's `Mesh`
            or `Model` (a skinned model in its rest pose): its exact
            triangles for static and kinematic bodies, its convex hull for
            dynamic ones (triangles have no volume to fall and roll with).
      - [x] c. A “Fit” button in the body's inspector section that sizes
            its box, sphere or capsule to the bounds of the node's mesh or
            model. Agreed after: shapes get an `offset` from their node
            (as the center of Unity's colliders), so a model whose origin
            is at its feet gets a shape around it, not half underground:
        - [x] c1. `offset` of 2D and 3D shapes: physics (stretched with
              the scale), `.scene` files, the inspector and the drawing,
              and StarScript (`offset_x`, `offset_y`, `offset_z` of
              `Body`, compiled and interpreted).
        - [x] c2. The “Fit” button: sizes and offset from the bounds.
  - To do:
    - [x] Frames per second shown while a game runs. Plan agreed with the
          owner (decision 85): a small counter in a corner of the game's
          window (frames per second and milliseconds per frame), drawn
          with a tiny digit font of the engine's own, no dependencies;
          `show_fps` in `project.toml` (`"editor"`, the default: when
          played from the editor; `"always"`; `"never"`), chosen in the
          editor's Project menu; F3 shows or hides it while playing, and
          StarScript's `Debug.show_fps(shown)` too.
      - [x] a. `star_render`: overlay sprites, in window pixels, drawn
            over everything in a pass of their own.
      - [x] b. `star_engine`: the counter, its font and the overlay, F3,
            `App::with_fps_shown`.
      - [x] c. StarScript `Debug.show_fps(shown: bool)`, compiled and
            interpreted.
      - [x] d. `show_fps` in `project.toml`, honored by Play, Play
            compiled and exported games, and the Project menu.
    - [x] Basic 2D bodies: shape nodes for 2D (a box, for example) that
          can be scaled, given collisions and used as a floor, and hidden
          while their collisions and everything else stay (as in Unity).
          Plan agreed with the owner (decision 86): no new node, but
          images the engine brings, `@square` and `@circle`, for a
          `Sprite` with its color; a body on a node whose sprite shows one
          always takes that shape and the sprite's size; `visible = false`
          already hides a node and keeps its collisions.
      - [x] a. `star_assets`: the built-in images, read by `load_image`
            wherever an image is read (games, the editor).
      - [x] b. Bodies copy them: scenes, StarScript nodes (compiled and
            interpreted, a window game) and the physics of nodes.
      - [x] c. The editor: the images offered to sprites, “Create > 2D >
            Square / Circle”, the body's shape shown as the sprite's.
    - [x] An audio editor, to manage a project's sounds (after Unity's).
          Plan agreed with the owner (2026-11-27, decision 87), in parts:
      - [x] a. A sound's tab: its waveform, length, channels and rate, and
            “Play” / “Stop” to hear it in the editor; the assets' menu
            plays a sound too.
      - [x] b. A `SoundPlayer` node (as Unity's audio source): its sound,
            volume, looping and “play on start”, in the inspector, scenes
            and StarScript (`play`, `stop`, compiled and interpreted).
        - [x] b1. `[node.sound_player]` in scenes, played by the engine.
        - [x] b2. StarScript's `SoundPlayer`, compiled and interpreted.
        - [x] b3. The editor: its inspector section, “Create”, and a sound
              dropped on the scene.
      - [x] c. Groups and a mixer: groups of sounds (music, effects…)
            with their volume, in a mixer tab; each sound and player
            belongs to one; StarScript `Audio.set_group_volume`.
        - [x] c1. Groups in the mixer, players, StarScript (`group`,
              `Audio.group_volume`, `set_group_volume`, compiled and
              interpreted) and `[audio.groups]` of `project.toml`.
        - [x] c2. The editor: the mixer tab and a player's group.
      - [x] d. 2D and 3D sound: players sound quieter with their distance
            to the camera (between a near and a far distance), and from
            the left or the right.
    - [ ] A sprite editor (after Godot's). Plan agreed with the owner
          (2026-11-27, decision 88): four parts, done one by one, in this
          order:
      - [ ] A. Regions: the part of an image a sprite shows, in pixels.
        - [x] A1. The engine: `star_render::PixelRegion`, made a
              `TextureRegion` once the texture's size is known
              (`Renderer::texture_size`); scenes keep `region` as
              `[x, y, width, height]` in pixels; `SpriteFile` has one; the
              inspector edits it in pixels (a width or height of 0 is the
              whole image).
        - [ ] A2. StarScript: `region_x`, `region_y`, `region_width` and
              `region_height` of `Sprite`, in pixels (a width of 0 is the
              whole image), compiled and interpreted; scenes loaded by
              scripts keep their sprites' regions.
        - [ ] A3. The editor: “Edit region…” in the sprite's inspector
              opens a window with the image (zoom, pan), where a rectangle
              is dragged, snapped to pixels or to the sheet's cells.
      - [ ] D. Import settings of each image (filter, pixels per unit,
            default anchor), kept in a file beside it.
      - [ ] B. Slices: an image cut into named rectangles of any size, by
            hand, by a grid with margin and spacing, or found by
            transparency; regions and frames may name them.
      - [ ] C. Painting pixels: pencil, eraser, fill, picker and color, with
            undo, saved to the same PNG.
- [ ] **Future — Consoles** (PlayStation, Xbox).
