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
- [ ] **Phase 7 — Audio**: playback, and loading sounds from the asset folder.
- [ ] **Phase 8 — 2D physics**.
- [ ] **Phase 9 — 3D rendering**: materials, lights, indexed meshes. (`Mat4`
      and a perspective camera already exist.)
- [ ] **Phase 10 — 3D physics**.
- [ ] **Phase 11 — Mobile**: Android and iOS packaging.
- [ ] **Phase 12 — Visual editor** (egui):
  - [x] Project manager: new project (name, location, 2D/3D, platforms),
        recent projects, open project, native folder picker.
  - [x] Main window: menu, Scene / View / Inspector / Files panels, status bar.
  - [x] Engine rendering inside the view (wgpu shared with egui), grid,
        editor camera (pan/orbit and zoom).
  - [ ] Real hierarchy and inspector (after `star_scene`), log console.
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
  - [x] 13.6 Editor integration: compiling and an error panel.
    - [x] 13.6a Programs of several files (`SourceSet`): each file is read
          on its own and their classes are checked as one program; errors
          name their file.
    - [x] 13.6b Checking a project in the editor: every `.star` of
          `assets/`, the input actions of `project.toml` and the script
          classes named by its scenes.
    - [x] 13.6c Error panel: «Compilar», a «Problemas» tab and the result
          in the status bar, checked off the UI thread. (13.6b and 13.6c
          went in together: the editor is a binary, so a check without its
          panel would be dead code.)
    - Next, on top of it: building the generated Rust with Cargo and
      running the game from the editor.
  - [ ] 13.7 Blueprint model and translation to StarScript (`star_blueprint`):
    - [x] 13.7a Model: classes as node graphs, pins of each node, the rules
          (`validate`) and `.blueprint` files (TOML).
    - [x] 13.7b Translation to StarScript, remembering which code comes from
          which node.
    - [ ] 13.7c «Compilar» in the editor checks blueprints with the scripts,
          and their errors name the node.
  - [ ] 13.8 Visual blueprint editor.
- [ ] **Future — Consoles** (PlayStation, Xbox).
