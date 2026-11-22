# ProjectStar — Normas del proyecto

> Este archivo contiene las **normas inviolables** del proyecto. Toda contribución
> (humana o de Claude) debe cumplirlas. Si una norma necesita cambiar, se cambia
> aquí primero, de forma explícita y acordada con el dueño del proyecto.

## 1. Visión

ProjectStar es un **motor de videojuegos multiplataforma** para juegos **2D y 3D**,
inspirado en la experiencia de uso de **Unity** y **Godot** (escenas, nodos/entidades,
componentes, editor visual, flujo de trabajo amigable).

- Inspiración ≠ copia: **nunca** se copia código, assets ni documentación de Unity,
  Godot u otros motores. Solo se toman ideas de diseño.
- El desarrollo es **incremental**: se construye parte por parte, poco a poco.
  Cada paso debe dejar el proyecto compilando y con los tests en verde.

## 2. Plataformas objetivo

| Plataforma | Estado |
|------------|--------|
| Windows    | Objetivo principal |
| macOS      | Objetivo principal |
| Linux      | Objetivo principal |
| Android    | Objetivo principal |
| iOS        | Objetivo principal |
| PlayStation / Xbox | Futuro (requieren SDK bajo NDA; irán en módulos aislados) |

Reglas:
- Todo código dependiente de plataforma vive **solo** en `star_platform`
  (o en crates de backend explícitos). El resto del motor es agnóstico a la plataforma.
- Usar `#[cfg(...)]` únicamente en las capas de plataforma, nunca disperso por el motor.
- Ninguna API pública puede asumir teclado/ratón: también existen táctil y mandos.

## 3. Normas inviolables

### 3.1 Lenguaje
- El motor se escribe en **Rust** (edición 2024, toolchain `stable`).
- **`unsafe` está prohibido** por defecto (`unsafe_code = "deny"`). Solo se permite en
  capas FFI/plataforma, con `#![allow(unsafe_code)]` justificado a nivel de crate y un
  comentario `// SAFETY:` explicando cada bloque.

### 3.2 Clean Code
- Nombres claros y descriptivos; nada de abreviaturas crípticas.
- Funciones pequeñas que hacen **una sola cosa**. Si necesita un comentario para
  explicar *qué* hace, probablemente debe dividirse.
- Principios **SOLID**, **DRY**, **KISS** y **YAGNI**.
- Sin números mágicos: usar constantes con nombre.
- Sin código muerto ni código comentado. Sin `TODO` sin contexto.
- Los comentarios explican el *porqué*, no el *qué*.
- Archivos y módulos cohesionados: un concepto por módulo.

### 3.3 Buenas prácticas de Rust
- El código debe pasar **sin advertencias**:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
- **Nada de `unwrap()` / `expect()` / `panic!()`** en código del motor. Los errores se
  propagan con `Result` (`EngineResult<T>`). Solo se permiten en tests.
  - **Única excepción acordada**: `star_script_runtime::fault`, el punto único
    que detiene un juego ante un error de StarScript que solo puede ocurrir al
    jugar (índice fuera de rango, división entera entre cero).
- Toda API pública lleva documentación `///` (`missing_docs` activado).
  Los nombres propios en CamelCase (p. ej. `ProjectStar`, `PlayStation`) se
  registran en `doc-valid-idents` de `clippy.toml`, sin desactivar el lint.
- Preferir composición sobre herencia; traits pequeños y enfocados.
- Tipos con invariantes: campos privados + constructores/validación.
- `#[must_use]` en funciones puras y builders.
- Evitar asignaciones de memoria en el bucle de juego (hot path) siempre que sea posible.

### 3.4 Tests
- Toda lógica nueva incluye tests unitarios (`#[cfg(test)] mod tests`).
- Los bugs corregidos llevan un test de regresión.

### 3.5 Dependencias
- Cada dependencia externa debe estar justificada (mantenida, soporte
  multiplataforma y licencia **compatible con GPL-3.0**: MIT, Apache-2.0, BSD,
  Zlib, ISC, BSL-1.0, MPL-2.0, LGPL o GPL-3.0). Nunca GPL-2.0-only, AGPL ni
  licencias propietarias o «no comerciales». Si una dependencia ofrece varias
  licencias, basta con que una sea compatible.
- Las versiones se declaran **una sola vez** en `[workspace.dependencies]` del
  `Cargo.toml` raíz.
- Las dependencias externas se encapsulan detrás de APIs propias del motor, para
  poder reemplazarlas sin romper a los usuarios.
  - **Única excepción acordada**: `star_render::interop` (feature `interop`,
    desactivada por defecto) expone tipos de `wgpu` para que el **editor** comparta
    la GPU con su interfaz. Ningún juego ni crate del motor puede activarla.

### 3.6 Licencia
- ProjectStar es **GPL-3.0-or-later** (`LICENSE`), con la **excepción para
  juegos** de `LICENSE-GAME-EXCEPTION.md`: los juegos hechos con el motor
  (incluido el código generado desde StarScript y blueprints) pueden usar
  cualquier licencia, incluso cerrada. El motor modificado sigue siendo GPL.
- Toda contribución se publica bajo esas mismas condiciones.
- No se copia código de proyectos con licencias incompatibles (ver 1: tampoco
  de Unity ni Godot, sea cual sea su licencia).
- El código generado por las herramientas del motor no debe incluir avisos de
  licencia GPL: pertenece al autor del juego.

## 4. Arquitectura

Workspace de Cargo con crates modulares (prefijo `star_`). Las dependencias van
**solo hacia abajo** en las capas; nunca hacia arriba ni circulares.

```
Capa 5  editor/, examples/, juegos          (usuarios del motor)
Capa 4  star_engine                          (fachada pública, App, prelude)
Capa 3  star_scene                           (escenas, jerarquía de nodos)
Capa 2  star_render, star_audio, star_physics, star_input, star_assets
Capa 1  star_ecs, star_platform
Capa 0  star_core, star_math                 (sin dependencias de otras capas)
Herram. star_script                          (compilador de StarScript; sin dependencias)
        star_script_runtime                  (soporte del Rust generado; sin dependencias)
```

| Crate | Responsabilidad |
|-------|-----------------|
| `star_core` | Errores, tiempo, versión, utilidades base |
| `star_math` | Vectores, matrices, cuaterniones, transformaciones |
| `star_ecs` | Entidades, componentes, sistemas |
| `star_platform` | Ventanas y eventos del SO (winit), detección de plataforma, carpetas del usuario |
| `star_input` | Teclado, ratón, táctil, mandos (acciones abstractas) |
| `star_assets` | Carga, caché y hot-reload de recursos |
| `star_render` | Renderizado 2D y 3D sobre wgpu (DX12, Vulkan, Metal, OpenGL) |
| `star_audio` | Reproducción de sonido y música |
| `star_physics` | Física 2D y 3D |
| `star_scene` | Escenas y árbol de nodos estilo Godot/Unity |
| `star_engine` | Punto de entrada público que integra todo |
| `star_script` | Compilador de StarScript (`.star` → Rust). Especificación en `docs/starscript.md` |
| `star_script_runtime` | Soporte del Rust generado: listas compartidas, campos, igualdad, errores en ejecución |
| `editor` | Editor visual (egui): gestor de proyectos y ventana principal |

Más detalle en `docs/architecture.md`. Hoja de ruta en `docs/roadmap.md`.

## 5. Convenciones

- **Idioma**: identificadores, comentarios de código y documentación `///` en
  **inglés** (estándar del ecosistema Rust). El `README.md` también en **inglés**,
  porque es la portada pública del repositorio. Documentos del proyecto
  (`CLAUDE.md`, `docs/`) y conversación en **español**.
- Nombres: `snake_case` funciones/módulos, `PascalCase` tipos, `SCREAMING_SNAKE_CASE`
  constantes.
- Commits con **Conventional Commits** (`feat:`, `fix:`, `refactor:`, `docs:`,
  `test:`, `chore:`), en inglés, cortos y en imperativo.

## 6. Flujo de trabajo con Claude

1. Antes de implementar algo, revisar este archivo y `docs/`.
2. Trabajar en pasos pequeños y verificables; explicar el diseño antes de cambios grandes.
3. Tras cada cambio: `cargo fmt`, `cargo clippy` y `cargo test` deben pasar.
4. Actualizar `docs/` y este archivo cuando cambie la arquitectura o una decisión.
5. No añadir dependencias, crates ni funcionalidades fuera de lo pedido sin consultarlo.
