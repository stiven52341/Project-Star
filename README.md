# ProjectStar

Motor de videojuegos **2D y 3D** multiplataforma escrito en **Rust**, inspirado en
la experiencia de Unity y Godot.

**Plataformas:** Windows · macOS · Linux · Android · iOS (consolas en el futuro).

## Requisitos

- [Rust](https://rustup.rs) estable (≥ 1.85 para el motor, ≥ 1.95 para el editor). El archivo `rust-toolchain.toml`
  selecciona la versión automáticamente.

## Comandos

```bash
cargo build --workspace                                   # compilar todo
cargo test --workspace                                    # tests
cargo clippy --workspace --all-targets -- -D warnings     # lints
cargo fmt --all                                           # formato
cargo run -p hello_star                                   # ejemplo mínimo (abre una ventana)
cargo run -p hello_star -- vulkan                         # forzar backend: vulkan|dx12|metal|opengl
cargo run -p triangle_2d                                  # prueba de render: triángulo 2D
cargo run -p triangle_3d                                  # prueba de render: triángulo girando en 3D
cargo run -p star_editor                                  # editor: crear un nuevo proyecto
cargo run -p star_script --example tokenize -- f.star     # StarScript: ver los tokens de un archivo
cargo run -p star_script --example parse -- f.star        # StarScript: ver el árbol sintáctico
cargo run -p star_script --example check -- f.star        # StarScript: comprobar tipos y errores
```

## Estructura

```
crates/      Módulos del motor (star_core, star_math, star_ecs, ...)
editor/      Editor visual (en desarrollo)
examples/    Ejemplos de uso del motor
assets/      Recursos compartidos de ejemplos
docs/        Arquitectura, hoja de ruta y especificación de StarScript
```

## Licencia

ProjectStar es software libre bajo la [GNU GPL v3 o posterior](LICENSE), con una
[excepción para juegos](LICENSE-GAME-EXCEPTION.md):

- **Tus juegos son tuyos.** Un juego hecho con ProjectStar, incluido el código
  generado desde StarScript o blueprints, puede publicarse con la licencia que
  quieras, también cerrada y de pago.
- **El motor sigue siendo libre.** Si modificas el motor y distribuyes esa
  versión (o un juego que la incluya), debes publicar el código de tus cambios
  al motor bajo la GPL.
- Un motor, editor o herramienta derivados de ProjectStar no cuentan como
  juego: son GPL.

Normas del proyecto: [`CLAUDE.md`](CLAUDE.md) · Arquitectura:
[`docs/architecture.md`](docs/architecture.md) · Hoja de ruta:
[`docs/roadmap.md`](docs/roadmap.md)
