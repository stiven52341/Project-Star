# ProjectStar

A cross-platform **2D and 3D** game engine written in **Rust**, inspired by the
workflow of Unity and Godot.

**Platforms:** Windows · macOS · Linux · Android · iOS (consoles in the future).

> **Status:** early development. The engine, the editor and the StarScript
> language are being built step by step; see the [roadmap](docs/roadmap.md).

## Highlights

- **Rendering on wgpu**: DirectX 12, Vulkan, Metal and OpenGL from a single API.
- **Visual editor** (egui): project manager and main window, with the engine
  rendering live inside the viewport.
- **StarScript**: an object-oriented, statically typed and **null-safe**
  language that compiles to Rust, so games get Rust's performance without its
  complexity. Visual blueprints will compile to StarScript, and plain Rust is
  always available too: `blueprints → StarScript → Rust → machine code`.

## Requirements

- Stable [Rust](https://rustup.rs) (≥ 1.86 for the engine, ≥ 1.95 for the
  editor). The `rust-toolchain.toml` file selects the version automatically.
- On Linux, the libudev headers (gamepad support) and the ALSA headers
  (audio), e.g. on Debian/Ubuntu:
  `sudo apt install libudev-dev libasound2-dev pkg-config`.

## Commands

```bash
cargo build --workspace                                   # build everything
cargo test --workspace                                    # run the tests
cargo clippy --workspace --all-targets -- -D warnings     # lints
cargo fmt --all                                           # format the code
cargo run -p hello_star                                   # minimal example (opens a window)
cargo run -p hello_star -- vulkan                         # force a backend: vulkan|dx12|metal|opengl
cargo run -p triangle_2d                                  # render test: 2D triangle
cargo run -p triangle_3d                                  # render test: spinning 3D triangle
cargo run -p scene_3d                                     # 3D test: PBR spheres, a textured cube, sun and lamp
cargo run -p sprites_2d                                   # render test: batched sprites (arrows/WASD move)
cargo run -p scene_2d                                     # scene test: a .scene file instanced (Space adds more)
cargo run -p sounds_2d                                    # audio test: Space beeps, M loops a melody, up/down volume
cargo run -p physics_2d                                   # 2D physics: Space drops balls, C crates; a moving paddle and a sensor
cargo run -p physics_3d                                   # 3D physics: Space drops balls, C crates; a sweeping bar and a sensor
cargo run -p star_editor                                  # editor: create or open a project
cargo run -p star_script --example tokenize -- f.star     # StarScript: show the tokens of a file
cargo run -p star_script --example parse -- f.star        # StarScript: show the syntax tree
cargo run -p star_script --example check -- f.star        # StarScript: check types and errors
cargo run -p star_script --example run -- f.star          # StarScript: compile to Rust, build and run (games open a window)
cargo run -p star_script --example run -- --rust f.star   # StarScript: show the generated Rust
cargo run -p star_script --example run -- crates/star_script/examples/scripts/input/jumper.star  # StarScript: input actions from project.toml
cargo run -p star_script --example run -- crates/star_script/examples/scripts/nodes/space.star   # StarScript: nodes, sprites, colors, a camera and a scene with script classes (arrows/WASD move)
```

## Project layout

```
crates/      Engine modules (star_core, star_math, star_render, star_script, ...)
editor/      Visual editor
examples/    Examples of how to use the engine
assets/      Assets shared by the examples
docs/        Architecture, roadmap and StarScript specification
```

## Documentation

- [`CLAUDE.md`](CLAUDE.md): project rules (code quality, architecture, conventions).
- [`docs/architecture.md`](docs/architecture.md): architecture and recorded decisions.
- [`docs/roadmap.md`](docs/roadmap.md): roadmap.
- [`docs/starscript.md`](docs/starscript.md): StarScript language specification.

Code, comments, API documentation and project documents are all in English.

## License

ProjectStar is free software under the [GNU GPL v3 or later](LICENSE), with a
[game exception](LICENSE-GAME-EXCEPTION.md):

- **Your games are yours.** A game made with ProjectStar, including code
  generated from StarScript or blueprints, can be released under any license
  you want, including closed-source and commercial ones.
- **The engine stays free.** If you modify the engine and distribute that
  version (or a game that includes it), you must publish the source code of
  your engine changes under the GPL.
- An engine, editor or tool derived from ProjectStar is not a game: it is GPL.
