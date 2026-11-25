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
- [ ] **Owner's requests (2026-11-24)**:
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
  - [ ] f. Running StarScript without Rust while editing (an
        interpreter), for instant “Play” and hot reload: decided by the
        owner (decision 64) and planned as Phase 14. Before it:
    - [ ] f1. Move the engine classes and the game loop out of the text
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
- [ ] **Phase 14 — StarScript interpreter** (decision 64, rule 3.7 of
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
  - [ ] 14.9 Blueprints: they become StarScript, so they need no
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
    - [ ] 14.9b Every kind of node, and the example script in the
          interpreter's conformance.
  - [ ] 14.10 The interpreted player: the scene player of step e grows
        into a player that loads the project's scripts when it starts;
        “Play” uses it for every project, and any project plays in about
        a second.
  - [ ] 14.11 Building with the compiler from the editor: a “Build”
        command (and later the export of games) that compiles the project
        with Cargo, and a way to play that compiled build to check it.
  - [ ] 14.12 Hot reload: changed scripts reload while the game is
        played, keeping the state of the objects that are still valid.
- [ ] **Future — Consoles** (PlayStation, Xbox).
