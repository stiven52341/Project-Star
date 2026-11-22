//! Every text the editor shows, in one place so it can be translated later.

use std::path::Path;

use star_blueprint::{
    BRANCH_FALSE, BRANCH_TRUE, Blueprint, CONDITION, CallTarget, EXECUTION_INPUT, EXECUTION_OUTPUT,
    LEFT, Literal, NodeKind, Operator, RESULT, RIGHT, TARGET, VALUE,
};

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
pub(crate) const SCENE_TAB: &str = "Escena";
pub(crate) const CLOSE_TAB: &str = "Cerrar";
pub(crate) const OPEN_BLUEPRINT_HINT: &str = "Doble clic para abrir";

// Saving.
pub(crate) const SAVE: &str = "Guardar";
pub(crate) const UNSAVED_CHANGES: &str = "Cambios sin guardar";
pub(crate) const UNSAVED_BLUEPRINT_QUESTION: &str =
    "Este blueprint tiene cambios sin guardar. ¿Guardarlos antes de cerrarlo?";
pub(crate) const UNSAVED_BLUEPRINTS_QUESTION: &str =
    "Hay blueprints con cambios sin guardar. ¿Guardarlos antes de cerrar?";
pub(crate) const DISCARD_CHANGES: &str = "No guardar";
pub(crate) const CANCEL: &str = "Cancelar";

// Blueprint editor.
pub(crate) const SELECT_A_NODE: &str = "Selecciona un nodo para ver sus propiedades.";
pub(crate) const ADD_NODE: &str = "Añadir nodo";
pub(crate) const MENU_VALUE: &str = "Valor";
pub(crate) const MENU_GET_VARIABLE: &str = "Leer variable";
pub(crate) const MENU_SET_VARIABLE: &str = "Asignar variable";
pub(crate) const MENU_CALL: &str = "Llamar";
pub(crate) const MENU_OPERATOR: &str = "Operador";
pub(crate) const NODE_BRANCH: &str = "Si";
pub(crate) const NODE_RETURN: &str = "Devolver";
pub(crate) const OTHER_NAME: &str = "Otra…";
pub(crate) const CALL_OWN_METHOD: &str = "Método de este objeto…";
pub(crate) const CALL_METHOD_OF_VALUE: &str = "Método de otro objeto…";
pub(crate) const CALL_CLASS_FUNCTION: &str = "Función de una clase…";
pub(crate) const DELETE_NODE: &str = "Borrar nodo";
pub(crate) const ENTRY_CANNOT_BE_DELETED: &str = "Todo gráfico necesita su nodo de inicio.";
pub(crate) const DISCONNECT_PIN: &str = "Quitar cables";
pub(crate) const BOOL_KIND: &str = "Booleano";
pub(crate) const INT_KIND: &str = "Entero";
pub(crate) const FLOAT_KIND: &str = "Decimal";
pub(crate) const STRING_KIND: &str = "Texto";
pub(crate) const LITERAL_KIND: &str = "Tipo";
pub(crate) const LITERAL_VALUE: &str = "Valor";
pub(crate) const VARIABLE: &str = "Variable";
pub(crate) const CALL_TARGET: &str = "Llama a";
pub(crate) const CLASS: &str = "Clase";
pub(crate) const METHOD: &str = "Método";
pub(crate) const ARGUMENTS: &str = "Argumentos";
pub(crate) const ADD_ARGUMENT: &str = "Añadir argumento";
pub(crate) const REMOVE: &str = "Quitar";
pub(crate) const RETURNS_VALUE: &str = "Devuelve un valor";
pub(crate) const PURE_CALL: &str = "Pura";
pub(crate) const PURE_CALL_HINT: &str =
    "Solo calcula un valor: no tiene pines de ejecución y se ejecuta donde se usa su resultado.";
pub(crate) const ENTRY_HINT: &str = "Da los parámetros del método.";
pub(crate) const NO_GRAPHS: &str = "El blueprint no tiene gráficos.";
pub(crate) const CANVAS_CONTROLS: &str =
    "Arrastra el fondo para desplazar · Rueda para acercar o alejar";
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

pub(crate) fn blueprint_unreadable(reason: &str) -> String {
    format!("No se pudo abrir el blueprint: {reason}")
}

pub(crate) fn blueprint_not_saved(reason: &str) -> String {
    format!("No se pudo guardar el blueprint: {reason}")
}

/// The name of a tab with unsaved changes.
pub(crate) fn unsaved_title(name: &str) -> String {
    format!("{name} ●")
}

/// `Player : Sprite`, or only the class if it extends nothing.
pub(crate) fn class_heading(blueprint: &Blueprint) -> String {
    match &blueprint.extends {
        Some(base) => format!("{} : {base}", blueprint.class),
        None => blueprint.class.clone(),
    }
}

/// What a node of a blueprint shows in its title.
pub(crate) fn node_title(kind: &NodeKind) -> String {
    match kind {
        NodeKind::Entry => "Inicio".to_owned(),
        NodeKind::Literal { value } => literal_text(value),
        NodeKind::GetVariable { variable } => variable.clone(),
        NodeKind::SetVariable { variable } => format!("Asignar {variable}"),
        NodeKind::Call(call) => match &call.target {
            CallTarget::OwnObject => format!("{}()", call.method),
            CallTarget::Value => format!(".{}()", call.method),
            CallTarget::Class(class) => format!("{class}.{}()", call.method),
        },
        NodeKind::Operator { operator } => operator.symbol().to_owned(),
        NodeKind::Branch => "Si".to_owned(),
        NodeKind::Return => "Devolver".to_owned(),
    }
}

/// What kind of node it is, for the inspector.
pub(crate) const fn node_kind_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Entry => "Inicio",
        NodeKind::Literal { .. } => MENU_VALUE,
        NodeKind::GetVariable { .. } => MENU_GET_VARIABLE,
        NodeKind::SetVariable { .. } => MENU_SET_VARIABLE,
        NodeKind::Call(_) => "Llamada",
        NodeKind::Operator { .. } => MENU_OPERATOR,
        NodeKind::Branch => NODE_BRANCH,
        NodeKind::Return => NODE_RETURN,
    }
}

pub(crate) fn node_number(id: u32) -> String {
    format!("nodo {id}")
}

pub(crate) const fn call_target_name(target: &CallTarget) -> &'static str {
    match target {
        CallTarget::OwnObject => "Este objeto",
        CallTarget::Value => "Otro objeto (pin «objeto»)",
        CallTarget::Class(_) => "Una clase",
    }
}

/// An operator's name in the editor, with its symbol.
pub(crate) const fn operator_name(operator: Operator) -> &'static str {
    match operator {
        Operator::Add => "Sumar (+)",
        Operator::Subtract => "Restar (-)",
        Operator::Multiply => "Multiplicar (*)",
        Operator::Divide => "Dividir (/)",
        Operator::Remainder => "Resto (%)",
        Operator::Negate => "Cambiar signo (-)",
        Operator::Equal => "Igual (==)",
        Operator::NotEqual => "Distinto (!=)",
        Operator::Less => "Menor (<)",
        Operator::LessOrEqual => "Menor o igual (<=)",
        Operator::Greater => "Mayor (>)",
        Operator::GreaterOrEqual => "Mayor o igual (>=)",
        Operator::And => "Y (&&)",
        Operator::Or => "O (||)",
        Operator::Not => "No (!)",
    }
}

/// A constant as StarScript would write it: `2.0`, `"jump"`.
fn literal_text(value: &Literal) -> String {
    match value {
        Literal::Bool(value) => value.to_string(),
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) => format!("{value:?}"),
        Literal::String(text) => format!("{text:?}"),
    }
}

/// The name a pin of a node of `kind` shows next to it. The pins every node
/// of a kind has get a Spanish name, or none if its place tells what it is
/// (the execution in and out, the sides of an operator); parameters and
/// arguments show their own names.
pub(crate) fn pin_label<'name>(kind: &NodeKind, name: &'name str) -> &'name str {
    let named_by_user = match kind {
        NodeKind::Entry => name != EXECUTION_OUTPUT,
        NodeKind::Call(_) => !matches!(name, EXECUTION_INPUT | EXECUTION_OUTPUT | TARGET | RESULT),
        _ => false,
    };
    if named_by_user {
        return name;
    }
    match name {
        EXECUTION_INPUT | EXECUTION_OUTPUT | LEFT | RIGHT => "",
        BRANCH_TRUE => "sí",
        BRANCH_FALSE => "no",
        CONDITION => "condición",
        VALUE => "valor",
        RESULT => "resultado",
        TARGET => "objeto",
        other => other,
    }
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

#[cfg(test)]
mod tests {
    use star_blueprint::Call;

    use super::*;

    fn call(target: CallTarget, arguments: &[&str]) -> NodeKind {
        NodeKind::Call(Call {
            target,
            method: "is_down".to_owned(),
            arguments: arguments.iter().map(|name| (*name).to_owned()).collect(),
            returns_value: true,
            pure: true,
        })
    }

    #[test]
    fn nodes_are_titled_by_what_they_do() {
        let literal = NodeKind::Literal {
            value: Literal::String("jump".to_owned()),
        };
        let multiply = NodeKind::Operator {
            operator: Operator::Multiply,
        };
        let set = NodeKind::SetVariable {
            variable: "speed".to_owned(),
        };

        assert_eq!(node_title(&literal), "\"jump\"");
        assert_eq!(
            node_title(&NodeKind::Literal {
                value: Literal::Float(2.0)
            }),
            "2.0"
        );
        assert_eq!(node_title(&multiply), "*");
        assert_eq!(node_title(&set), "Asignar speed");
        assert_eq!(
            node_title(&call(CallTarget::Class("Input".to_owned()), &[])),
            "Input.is_down()"
        );
        assert_eq!(node_title(&call(CallTarget::Value, &[])), ".is_down()");
    }

    #[test]
    fn pins_of_every_node_of_a_kind_get_spanish_names() {
        assert_eq!(pin_label(&NodeKind::Branch, BRANCH_TRUE), "sí");
        assert_eq!(pin_label(&NodeKind::Branch, EXECUTION_INPUT), "");
        assert_eq!(pin_label(&call(CallTarget::Value, &[]), TARGET), "objeto");
    }

    #[test]
    fn arguments_and_parameters_keep_their_names() {
        let call = call(CallTarget::OwnObject, &[VALUE]);

        assert_eq!(pin_label(&call, VALUE), VALUE);
        assert_eq!(pin_label(&NodeKind::Entry, "delta"), "delta");
        assert_eq!(pin_label(&NodeKind::Entry, EXECUTION_OUTPUT), "");
    }
}
