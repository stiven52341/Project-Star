# ProjectStar — Project rules

> This file contains the **inviolable rules** of the project. Every contribution
> (human or Claude's) must follow them. If a rule needs to change, it is changed
> here first, explicitly and in agreement with the project owner.

## 1. Vision

ProjectStar is a **cross-platform game engine** for **2D and 3D** games,
inspired by the user experience of **Unity** and **Godot** (scenes,
nodes/entities, components, visual editor, friendly workflow).

- Inspiration ≠ copying: code, assets or documentation from Unity, Godot or
  other engines are **never** copied. Only design ideas are taken.
- Development is **incremental**: the engine is built part by part, little by
  little. Every step must leave the project compiling with its tests green.

## 2. Target platforms

| Platform | Status |
|----------|--------|
| Windows  | Main target |
| macOS    | Main target |
| Linux    | Main target |
| Android  | Main target |
| iOS      | Main target |
| PlayStation / Xbox | Future (their SDKs are under NDA; they will live in isolated modules) |

Rules:
- All platform-dependent code lives **only** in `star_platform` (or in
  explicit backend crates). The rest of the engine is platform-agnostic.
- Use `#[cfg(...)]` only in the platform layers, never scattered across the engine.
- No public API may assume keyboard/mouse: touch and gamepads exist too.

## 3. Inviolable rules

### 3.1 Language
- The engine is written in **Rust** (edition 2024, `stable` toolchain).
- **`unsafe` is forbidden** by default (`unsafe_code = "deny"`). It is only
  allowed in FFI/platform layers, with a justified crate-level
  `#![allow(unsafe_code)]` and a `// SAFETY:` comment explaining each block.
  - **Only agreed exception** (approved by the owner for this one place):
    the `android_main!` macro in `star_platform/src/entry.rs`. Android does
    not call `main`: its NativeActivity glue loads the game's library and
    looks up a C function named exactly `android_main`. Rust only exports a
    function under its plain name with `#[no_mangle]`, which edition 2024
    marks `#[unsafe(no_mangle)]` (two symbols with the same name would be
    undefined behavior). The symbol must live in the game's own crate, so
    the macro expands there, with an allow on that one function and its
    `// SAFETY:` comment. `unsafe` stays forbidden everywhere else, and
    this macro must not grow any other unsafe code.

### 3.2 Clean Code
- Clear, descriptive names; no cryptic abbreviations.
- Small functions that do **one thing**. If one needs a comment to explain
  *what* it does, it should probably be split.
- **SOLID**, **DRY**, **KISS** and **YAGNI** principles.
- No magic numbers: use named constants.
- No dead or commented-out code. No `TODO` without context.
- Comments explain *why*, not *what*.
- Cohesive files and modules: one concept per module.

### 3.3 Rust good practices
- Code must pass **without warnings**:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
- **No `unwrap()` / `expect()` / `panic!()`** in engine code. Errors are
  propagated with `Result` (`EngineResult<T>`). They are only allowed in tests.
  - **Only agreed exception**: `star_script_runtime::fault`, the single point
    that stops a game on a StarScript error that can only happen while
    playing (index out of range, integer division by zero).
- Every public API has `///` documentation (`missing_docs` is enabled).
  CamelCase proper names (e.g. `ProjectStar`, `PlayStation`) are registered
  in `doc-valid-idents` in `clippy.toml`, without disabling the lint.
- Prefer composition over inheritance; small, focused traits.
- Types with invariants: private fields + constructors/validation.
- `#[must_use]` on pure functions and builders.
- Avoid memory allocations in the game loop (hot path) whenever possible.

### 3.4 Tests
- All new logic includes unit tests (`#[cfg(test)] mod tests`).
- Fixed bugs get a regression test.

### 3.5 Dependencies
- Every external dependency must be justified (maintained, cross-platform and
  with a **GPL-3.0-compatible** license: MIT, Apache-2.0, BSD, Zlib, ISC,
  BSL-1.0, MPL-2.0, LGPL or GPL-3.0). Never GPL-2.0-only, AGPL, proprietary or
  "non-commercial" licenses. If a dependency offers several licenses, one
  compatible license is enough.
- Versions are declared **only once**, in `[workspace.dependencies]` of the
  root `Cargo.toml`.
- External dependencies are wrapped behind the engine's own APIs, so they can
  be replaced without breaking users.
  - **Only agreed exception**: `star_render::interop` (feature `interop`,
    disabled by default) exposes `wgpu` types so the **editor** can share the
    GPU with its interface. No game or engine crate may enable it.

### 3.6 License
- ProjectStar is **GPL-3.0-or-later** (`LICENSE`), with the **game exception**
  in `LICENSE-GAME-EXCEPTION.md`: games made with the engine (including code
  generated from StarScript and blueprints) may use any license, even a closed
  one. A modified engine is still GPL.
- Every contribution is published under those same terms.
- No code is copied from projects with incompatible licenses (see 1: nor from
  Unity or Godot, whatever their license).
- Code generated by the engine's tools must not include GPL license notices:
  it belongs to the game's author.

### 3.7 StarScript has two implementations
- StarScript runs in two ways (decision 64, agreed with the owner on
  2026-11-24):
  - **Interpreted** by `star_script_interpreter` when the game is played
    in the editor (“Play”), so it starts at once and can reload scripts.
  - **Compiled** to Rust by `star_script` when the project is built
    (exported games, and “Build” in the editor), so shipped games are
    native code.
- Both share the lexer, the parser and the semantic analysis: only the
  execution differs. `docs/starscript.md` defines what a program does;
  neither implementation is the reference for the other.
- **Every StarScript feature is implemented for both, in the same
  change**: syntax, meaning, engine classes and methods, and runtime
  errors. It comes with a conformance test that runs the same program
  compiled and interpreted and expects the same output.
- The interpreter covers the whole language, and “Play” runs every
  project with the interpreted player, `star_player` (decision 74).
  Features must not depend on something only the generated Rust can do.

## 4. Architecture

Cargo workspace with modular crates (prefix `star_`). Dependencies only go
**downwards** through the layers; never upwards, never circular.

```
Layer 5  editor/, examples/, games          (users of the engine)
Layer 4  star_engine                         (public facade, App, prelude)
Layer 3  star_scene                          (scenes, node hierarchy)
Layer 2  star_render, star_audio, star_physics, star_input, star_assets
Layer 1  star_ecs, star_platform
Layer 0  star_core, star_math                (no dependencies on other layers)
Tools    star_script                         (StarScript compiler; no dependencies)
         star_script_runtime                 (support for the generated Rust; no dependencies)
         star_script_engine                  (connects star_script_runtime with star_engine)
         star_script_interpreter             (runs StarScript without building it, for “Play”;
                                              only star_script and star_script_runtime: the
                                              player joins it with star_script_engine)
         star_blueprint                      (blueprints: node graphs translated to StarScript)
         star_player                         (plays a project interpreted, for “Play”: reads its
                                              folder and joins the interpreter with star_script_engine)
```

| Crate | Responsibility |
|-------|----------------|
| `star_core` | Errors, time, version, images and sounds in memory, basic utilities |
| `star_math` | Vectors, matrices, quaternions, transforms |
| `star_ecs` | Entities, components, systems |
| `star_platform` | OS windows and events (winit), gamepads (gilrs), platform detection, user folders, showing files in the system's file manager |
| `star_input` | Keyboard, mouse, touch, gamepads (abstract actions) |
| `star_assets` | Loading (PNG through png, JPEG through zune-jpeg; WAV, OGG, MP3 and FLAC through symphonia), caching and hot-reload of resources |
| `star_render` | 2D and 3D rendering on wgpu (DX12, Vulkan, Metal, OpenGL) |
| `star_audio` | Sound and music playback: an own mixer of voices (volume, loops) over the audio output |
| `star_physics` | 2D and 3D physics: bodies, shapes, sensors and contacts over rapier2d and rapier3d (fixed steps) |
| `star_scene` | Scenes and Godot/Unity-style node tree |
| `star_engine` | Public entry point that integrates everything |
| `star_script` | StarScript compiler (`.star` → Rust). Specification in `docs/starscript.md` |
| `star_script_runtime` | Support for the generated Rust (and the interpreter): shared lists, fields, equality, input actions, coroutines, parallel functions, runtime errors, the node tree and the fields of the engine node classes (generic over a `Host`) |
| `star_script_engine` | What every StarScript game does with the engine, as real Rust built once: the game loop, mirroring the game's nodes in the engine's world and syncing their bodies, reading scenes, playing sounds, sharing input with the runtime. Generic over the runtime's `Host`; generated games use it, and interpreted games through the interpreter's `Host` |
| `star_script_interpreter` | Interpreter of StarScript (decisions 64, 66, 68, 69, 70, 71, 72 and 73): lowers a checked program to its own tree and runs it, with the same output and runtime errors as the compiled program. It runs console programs (static functions, numbers, texts, lists, `null`, constants, classes, inheritance, decorators, coroutines and `parallel` functions: the whole console language) and games: it is a `Host` of `star_script_runtime`, so `star_script_engine` plays a class that extends `Game` in a window, with every engine class: `Input`, `Color`, `Node` (its methods, tags and searches), `Camera2D`, `Sprite`, `Light2D`, the 3D nodes (`Node3D`, `Mesh`, `Light`, `Camera3D`), scenes (`Scene.load` with the program's classes and `export` fields, and main-scene programs), physics bodies (`Body`, a node's `body` and its contacts) and sounds (`Sound`, `Audio`); every program of the conformance test runs the same both ways. blueprints run as their StarScript translation, with a conformance program for each template and one with every kind of node (decision 73); `star_player` plays projects with it (decision 74), and Phase 14 of `docs/roadmap.md` brings hot reload. It depends only on `star_script` and `star_script_runtime` (the engine is a dev-dependency of its window tests) |
| `star_blueprint` | Blueprints: classes as node graphs, their `.blueprint` files (TOML) and their translation to StarScript |
| `star_player` | The interpreted player (decision 74): reads a project from its folder (`project.toml`, the scripts and blueprints of `assets/`), reports its mistakes with file, line and column, and plays it with `star_script_interpreter` through `star_script_engine`, as a compiled game plays (title, input actions, output, runtime errors). The editor's “Play” runs it for every project, built once outside the editor's workspace |
| `editor` | Visual editor (egui, in English or Spanish): project manager, main window, scene editor (node tree, inspector, scene drawn in the view), log console, blueprint editor (node graphs on a canvas), code editor (StarScript scripts, colored), “Compile” (checks a project's StarScript and blueprints with `star_script` and `star_blueprint`) and “Play” (runs `star_player` in the project's folder, which plays it interpreted) |

More detail in `docs/architecture.md`. Roadmap in `docs/roadmap.md`.

## 5. Conventions

- **Language**: identifiers, code comments, `///` documentation, `README.md`
  and project documents (`CLAUDE.md`, `docs/`) in **English** (the standard
  of the Rust ecosystem, and the public face of the repository). The
  conversation with the owner is in **Spanish**.
- Names: `snake_case` for functions/modules, `PascalCase` for types,
  `SCREAMING_SNAKE_CASE` for constants.
- Commits follow **Conventional Commits** (`feat:`, `fix:`, `refactor:`,
  `docs:`, `test:`, `chore:`), in English, short and in the imperative mood.

## 6. Workflow with Claude

1. Before implementing anything, review this file and `docs/`.
2. Work in small, verifiable steps; explain the design before big changes.
3. After every change, `cargo fmt`, `cargo clippy` and `cargo test` must pass.
4. Update `docs/` and this file when the architecture or a decision changes.
5. Do not add dependencies, crates or features beyond what was asked without asking first.
