//! Every text the editor shows, in one place so it can be translated later.

use std::path::Path;

use crate::project::{
    CreateProjectError, OpenProjectError, ProjectDimension, ProjectNameError, ProjectSettingsError,
};

// Launcher.
pub(crate) const PROJECT_MANAGER: &str = "Gestor de proyectos";
pub(crate) const RECENT_PROJECTS: &str = "Proyectos recientes";
pub(crate) const NO_RECENT_PROJECTS: &str = "Aún no hay proyectos recientes.";
pub(crate) const OPEN: &str = "Abrir";
pub(crate) const REMOVE_FROM_LIST: &str = "Quitar";
pub(crate) const REMOVE_FROM_LIST_HINT: &str = "Quitar de la lista (no borra archivos)";
pub(crate) const PROJECT_UNAVAILABLE: &str =
    "No disponible: la carpeta no existe o no es un proyecto";
pub(crate) const OPEN_OTHER_PROJECT: &str = "Abrir otro proyecto…";
pub(crate) const CHOOSE_PROJECT_FOLDER: &str = "Elige la carpeta del proyecto";

// New project form.
pub(crate) const NEW_PROJECT: &str = "Nuevo proyecto";
pub(crate) const PROJECT_NAME: &str = "Nombre";
pub(crate) const PROJECT_NAME_HINT: &str = "Mi juego";
pub(crate) const LOCATION: &str = "Ubicación";
pub(crate) const BROWSE: &str = "Examinar…";
pub(crate) const CHOOSE_LOCATION: &str = "Elige dónde crear el proyecto";
pub(crate) const DIMENSION: &str = "Tipo";
pub(crate) const DIMENSION_2D_DESCRIPTION: &str = "Sprites y cámara ortográfica";
pub(crate) const DIMENSION_3D_DESCRIPTION: &str = "Mallas y cámara en perspectiva";
pub(crate) const PLATFORMS: &str = "Plataformas";
pub(crate) const DESKTOP: &str = "Escritorio";
pub(crate) const MOBILE: &str = "Móvil";
pub(crate) const CREATE_PROJECT: &str = "Crear y abrir";
pub(crate) const WILL_BE_CREATED_IN: &str = "Se creará en:";
pub(crate) const LOCATION_REQUIRED: &str = "Indica una ubicación para el proyecto.";

// Editor window.
pub(crate) const MENU_FILE: &str = "Archivo";
pub(crate) const CLOSE_PROJECT: &str = "Cerrar proyecto";
pub(crate) const QUIT: &str = "Salir";
pub(crate) const MENU_HELP: &str = "Ayuda";
pub(crate) const ABOUT: &str = "Acerca de";
pub(crate) const ABOUT_DESCRIPTION: &str = "Motor de videojuegos 2D y 3D multiplataforma.";
pub(crate) const LICENSE_NOTICE: &str =
    "Software libre: puedes redistribuirlo y modificarlo bajo la GNU GPL v3 o posterior.";
pub(crate) const GAME_EXCEPTION_NOTICE: &str =
    "Los juegos que crees con él pueden usar la licencia que quieras.";
pub(crate) const NO_WARRANTY: &str = "Se ofrece SIN NINGUNA GARANTÍA.";
pub(crate) const SOURCE_CODE: &str = "Código fuente";
pub(crate) const HIERARCHY: &str = "Escena";
pub(crate) const EMPTY_SCENE: &str = "La escena está vacía.";
pub(crate) const INSPECTOR: &str = "Inspector";
pub(crate) const PROJECT: &str = "Proyecto";
pub(crate) const ENGINE: &str = "Motor";
pub(crate) const FOLDER: &str = "Carpeta";
pub(crate) const ASSETS: &str = "Archivos";
pub(crate) const REFRESH: &str = "Actualizar";
pub(crate) const EMPTY_ASSETS: &str =
    "La carpeta assets está vacía. Copia aquí imágenes, sonidos y modelos.";
pub(crate) const MENU_PROJECT: &str = "Proyecto";
pub(crate) const COMPILE: &str = "Compilar";
pub(crate) const COMPILE_HINT: &str =
    "Comprueba los scripts, las acciones de project.toml y las clases de las escenas";
pub(crate) const COMPILING: &str = "Compilando…";
pub(crate) const PROBLEMS: &str = "Problemas";
pub(crate) const NOT_COMPILED: &str =
    "Pulsa «Compilar» para comprobar los scripts .star de la carpeta assets.";
pub(crate) const CHECK_FAILED: &str = "No se pudo compilar";
pub(crate) const CHECK_LOST: &str = "La compilación se detuvo sin dar un resultado.";
pub(crate) const NO_GPU_FOR_VIEWPORT: &str = "la interfaz no se está dibujando con wgpu";

pub(crate) fn viewport_title(dimension_label: &str) -> String {
    format!("Vista {dimension_label}")
}

pub(crate) fn viewport_controls(dimension: ProjectDimension) -> &'static str {
    match dimension {
        ProjectDimension::TwoD => "Arrastra para desplazar · Rueda para acercar o alejar",
        ProjectDimension::ThreeD => "Arrastra para orbitar · Rueda para acercar o alejar",
    }
}

pub(crate) fn viewport_error(error: &str) -> String {
    format!("No se pudo renderizar la escena: {error}")
}

pub(crate) fn problems_title(count: usize) -> String {
    format!("{PROBLEMS} ({count})")
}

pub(crate) fn problems_found(count: usize) -> String {
    if count == 1 {
        "1 problema".to_owned()
    } else {
        format!("{count} problemas")
    }
}

pub(crate) fn check_passed(scripts: usize) -> String {
    match scripts {
        0 => "Sin problemas (el proyecto no tiene scripts)".to_owned(),
        1 => "Sin problemas en 1 script".to_owned(),
        _ => format!("Sin problemas en {scripts} scripts"),
    }
}

pub(crate) fn check_not_started(error: &std::io::Error) -> String {
    format!("No se pudo empezar a compilar: {error}")
}

pub(crate) fn assets_unreadable(error: &std::io::Error) -> String {
    format!("No se pudo leer la carpeta assets: {error}")
}

pub(crate) fn settings_error(error: ProjectSettingsError) -> String {
    match error {
        ProjectSettingsError::InvalidName(name_error) => name_error_text(name_error),
        ProjectSettingsError::NoPlatformSelected => {
            "Selecciona al menos una plataforma.".to_owned()
        }
    }
}

pub(crate) fn creation_error(error: &CreateProjectError) -> String {
    match error {
        CreateProjectError::AlreadyExists(path) => {
            format!(
                "Ya existe {}. Elige otro nombre o ubicación.",
                path.display()
            )
        }
        CreateProjectError::Io(io_error) => format!("No se pudo crear el proyecto: {io_error}"),
    }
}

pub(crate) fn open_error(error: &OpenProjectError) -> String {
    match error {
        OpenProjectError::NotAProject(path) => format!(
            "{} no es un proyecto de ProjectStar (no contiene project.toml).",
            path.display()
        ),
        OpenProjectError::Io(io_error) => format!("No se pudo leer el proyecto: {io_error}"),
        OpenProjectError::InvalidManifest(manifest_error) => {
            format!("El archivo project.toml no es válido: {manifest_error}")
        }
    }
}

pub(crate) fn folder_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn name_error_text(error: ProjectNameError) -> String {
    match error {
        ProjectNameError::Empty => "Escribe un nombre para el proyecto.".to_owned(),
        ProjectNameError::TooLong => format!(
            "El nombre no puede superar {} caracteres.",
            crate::project::MAX_NAME_LENGTH
        ),
        ProjectNameError::InvalidStart => {
            "El nombre debe empezar por una letra o un número.".to_owned()
        }
        ProjectNameError::InvalidCharacter(character) => format!(
            "El nombre no puede contener «{character}». Usa letras, números, espacios, - o _."
        ),
        ProjectNameError::Reserved => "Ese nombre está reservado por el sistema.".to_owned(),
    }
}
