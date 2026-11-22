# Hoja de ruta

Se avanza por fases; cada fase deja el motor compilando y con tests en verde.

- [x] **Fase 0 — Fundaciones**: workspace, normas, CI, `star_core`, `star_math`
      básico, `star_ecs` (entidades), detección de plataforma, `App`.
- [ ] **Fase 1 — Ventana y bucle principal**:
  - [x] Ventana y eventos del SO (winit), bucle continuo, redimensionado.
  - [x] Logging (`log`; los binarios eligen la implementación).
  - [x] `Time` integrado en el bucle (`Game::update` recibe el tiempo del frame).
  - [ ] Ciclo de vida en Android/iOS (suspender/reanudar, punto de entrada móvil).
- [ ] **Fase 2 — Input**: teclado, ratón, táctil, mandos y mapa de acciones.
- [ ] **Fase 3 — Render 2D**:
  - [x] Integración de wgpu: DX12, Vulkan, Metal y OpenGL; selección de backend;
        limpieza de pantalla con color; recuperación de superficie perdida.
  - [x] Pipeline de shaders (WGSL), mallas de triángulos con color por vértice,
        cámara 2D ortográfica, buffer de profundidad (ejemplos `triangle_2d`/`triangle_3d`).
  - [ ] Texturas y sprites, lotes (batching) de sprites.
- [ ] **Fase 4 — ECS completo**: componentes, consultas, sistemas y etapas.
- [ ] **Fase 5 — Assets**: carga de texturas/sonidos, handles, hot-reload.
- [ ] **Fase 6 — Escenas**: árbol de nodos, serialización.
- [ ] **Fase 7 — Audio**.
- [ ] **Fase 8 — Física 2D**.
- [ ] **Fase 9 — Render 3D**: materiales, luces, mallas indexadas. (Ya existen
      `Mat4` y cámara en perspectiva.)
- [ ] **Fase 10 — Física 3D**.
- [ ] **Fase 11 — Móviles**: empaquetado Android e iOS.
- [ ] **Fase 12 — Editor visual** (egui):
  - [x] Gestor de proyectos: nuevo proyecto (nombre, ubicación, 2D/3D,
        plataformas), recientes, abrir proyecto, selector de carpeta nativo.
  - [x] Ventana principal: menú, paneles Escena / Vista / Inspector / Archivos,
        barra de estado.
  - [x] Render del motor dentro de la vista (wgpu compartido con egui), cuadrícula,
        cámara de edición (desplazar/orbitar y zoom).
  - [ ] Jerarquía e inspector reales (tras `star_scene`), consola de logs.
- [ ] **Fase 13 — StarScript y Blueprints** (`star_script`, ver `docs/starscript.md`):
  - [x] 13.1 Especificación del léxico y lexer (tokens, errores con línea y columna).
  - [x] 13.2 Parser y AST (recuperación de errores, todos los errores a la vez).
  - [x] 13.3 Análisis semántico: tipos, inferencia, null safety, clases, herencia
        y construcción segura.
  - [x] 13.4 Generación de Rust y prueba de punta a punta (`.star` → binario),
        con el runtime `star_script_runtime`.
  - [ ] 13.5 API del motor para StarScript, por partes (cada pieza del motor
        nace con su API de StarScript):
    - [x] 13.5a Ciclo de vida: clase del motor `Game` (`start`, `update`, `quit`);
          un script que la extiende corre en una ventana del motor.
    - [ ] 13.5b Input en StarScript, junto con la Fase 2.
    - [ ] 13.5c Nodos en StarScript, junto con la Fase 6.
  - [ ] 13.6 Integración en el editor: compilar y panel de errores.
  - [ ] 13.7 Modelo de blueprints y traducción a StarScript.
  - [ ] 13.8 Editor visual de blueprints.
- [ ] **Futuro — Consolas** (PlayStation, Xbox).
