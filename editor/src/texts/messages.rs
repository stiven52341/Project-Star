//! Texts built from values (names, numbers, errors), in each language.

use star_blueprint::{
    BRANCH_FALSE, BRANCH_TRUE, CONDITION, CallTarget, EXECUTION_INPUT, EXECUTION_OUTPUT, LEFT,
    Literal, NodeKind, Operator, RESULT, RIGHT, TARGET, VALUE,
};

use super::{Language, Texts};
use crate::blueprint_editor::NewBlueprintError;
use crate::project::{
    CreateProjectError, MAX_NAME_LENGTH, OpenProjectError, ProjectDimension, ProjectNameError,
    ProjectSettingsError,
};

impl Texts {
    /// The text in this language, of a pair written in every language.
    const fn pick(&self, english: &'static str, spanish: &'static str) -> &'static str {
        match self.language {
            Language::English => english,
            Language::Spanish => spanish,
        }
    }

    pub(crate) fn viewport_title(&self, dimension_label: &str) -> String {
        match self.language {
            Language::English => format!("{dimension_label} view"),
            Language::Spanish => format!("Vista {dimension_label}"),
        }
    }

    pub(crate) const fn viewport_controls(&self, dimension: ProjectDimension) -> &'static str {
        match dimension {
            ProjectDimension::TwoD => self.pick(
                "Drag to pan · Wheel to zoom in or out",
                "Arrastra para desplazar · Rueda para acercar o alejar",
            ),
            ProjectDimension::ThreeD => self.pick(
                "Drag to orbit · Wheel to zoom in or out",
                "Arrastra para orbitar · Rueda para acercar o alejar",
            ),
        }
    }

    pub(crate) fn viewport_error(&self, error: &str) -> String {
        match self.language {
            Language::English => format!("Could not render the scene: {error}"),
            Language::Spanish => format!("No se pudo renderizar la escena: {error}"),
        }
    }

    pub(crate) fn blueprint_unreadable(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not open the blueprint: {reason}"),
            Language::Spanish => format!("No se pudo abrir el blueprint: {reason}"),
        }
    }

    pub(crate) fn blueprint_not_saved(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not save the blueprint: {reason}"),
            Language::Spanish => format!("No se pudo guardar el blueprint: {reason}"),
        }
    }

    pub(crate) fn new_blueprint_error(&self, error: &NewBlueprintError) -> String {
        match (error, self.language) {
            (NewBlueprintError::InvalidName, _) => self
                .pick(
                    "The name must start with a letter or _ and have only letters, digits and _.",
                    "El nombre debe empezar por una letra o _ y tener solo letras, números y _.",
                )
                .to_owned(),
            (NewBlueprintError::AlreadyExists(path), Language::English) => {
                format!("{path} already exists.")
            }
            (NewBlueprintError::AlreadyExists(path), Language::Spanish) => {
                format!("Ya existe {path}.")
            }
            (NewBlueprintError::Io(error), Language::English) => {
                format!("Could not create the blueprint: {error}")
            }
            (NewBlueprintError::Io(error), Language::Spanish) => {
                format!("No se pudo crear el blueprint: {error}")
            }
        }
    }

    /// What a node of a blueprint shows in its title.
    pub(crate) fn node_title(&self, kind: &NodeKind) -> String {
        match kind {
            NodeKind::Entry => self.pick("Start", "Inicio").to_owned(),
            NodeKind::Literal { value } => literal_text(value),
            NodeKind::GetVariable { variable } => variable.clone(),
            NodeKind::SetVariable { variable } => match self.language {
                Language::English => format!("Set {variable}"),
                Language::Spanish => format!("Asignar {variable}"),
            },
            NodeKind::Call(call) => match &call.target {
                CallTarget::OwnObject => format!("{}()", call.method),
                CallTarget::Value => format!(".{}()", call.method),
                CallTarget::Class(class) => format!("{class}.{}()", call.method),
            },
            NodeKind::Operator { operator } => operator.symbol().to_owned(),
            NodeKind::Branch => self.node_branch.to_owned(),
            NodeKind::Return => self.node_return.to_owned(),
        }
    }

    /// What kind of node it is, for the inspector.
    pub(crate) const fn node_kind_name(&self, kind: &NodeKind) -> &'static str {
        match kind {
            NodeKind::Entry => self.pick("Start", "Inicio"),
            NodeKind::Literal { .. } => self.menu_value,
            NodeKind::GetVariable { .. } => self.menu_get_variable,
            NodeKind::SetVariable { .. } => self.menu_set_variable,
            NodeKind::Call(_) => self.pick("Call", "Llamada"),
            NodeKind::Operator { .. } => self.menu_operator,
            NodeKind::Branch => self.node_branch,
            NodeKind::Return => self.node_return,
        }
    }

    pub(crate) fn node_number(&self, id: u32) -> String {
        match self.language {
            Language::English => format!("node {id}"),
            Language::Spanish => format!("nodo {id}"),
        }
    }

    pub(crate) const fn call_target_name(&self, target: &CallTarget) -> &'static str {
        match target {
            CallTarget::OwnObject => self.pick("This object", "Este objeto"),
            CallTarget::Value => self.pick(
                "Another object (“object” pin)",
                "Otro objeto (pin «objeto»)",
            ),
            CallTarget::Class(_) => self.pick("A class", "Una clase"),
        }
    }

    /// An operator's name in the editor, with its symbol.
    pub(crate) const fn operator_name(&self, operator: Operator) -> &'static str {
        match operator {
            Operator::Add => self.pick("Add (+)", "Sumar (+)"),
            Operator::Subtract => self.pick("Subtract (-)", "Restar (-)"),
            Operator::Multiply => self.pick("Multiply (*)", "Multiplicar (*)"),
            Operator::Divide => self.pick("Divide (/)", "Dividir (/)"),
            Operator::Remainder => self.pick("Remainder (%)", "Resto (%)"),
            Operator::Negate => self.pick("Negate (-)", "Cambiar signo (-)"),
            Operator::Equal => self.pick("Equal (==)", "Igual (==)"),
            Operator::NotEqual => self.pick("Not equal (!=)", "Distinto (!=)"),
            Operator::Less => self.pick("Less (<)", "Menor (<)"),
            Operator::LessOrEqual => self.pick("Less or equal (<=)", "Menor o igual (<=)"),
            Operator::Greater => self.pick("Greater (>)", "Mayor (>)"),
            Operator::GreaterOrEqual => self.pick("Greater or equal (>=)", "Mayor o igual (>=)"),
            Operator::And => self.pick("And (&&)", "Y (&&)"),
            Operator::Or => self.pick("Or (||)", "O (||)"),
            Operator::Not => self.pick("Not (!)", "No (!)"),
        }
    }

    /// The name a pin of a node of `kind` shows next to it. The pins every
    /// node of a kind has get a name in this language, or none if its place
    /// tells what it is (the execution in and out, the sides of an
    /// operator); parameters and arguments show their own names.
    pub(crate) fn pin_label<'name>(&self, kind: &NodeKind, name: &'name str) -> &'name str {
        let named_by_user = match kind {
            NodeKind::Entry => name != EXECUTION_OUTPUT,
            NodeKind::Call(_) => {
                !matches!(name, EXECUTION_INPUT | EXECUTION_OUTPUT | TARGET | RESULT)
            }
            _ => false,
        };
        if named_by_user {
            return name;
        }
        match name {
            EXECUTION_INPUT | EXECUTION_OUTPUT | LEFT | RIGHT => "",
            BRANCH_TRUE => self.pick("true", "sí"),
            BRANCH_FALSE => self.pick("false", "no"),
            CONDITION => self.pick("condition", "condición"),
            VALUE => self.pick("value", "valor"),
            RESULT => self.pick("result", "resultado"),
            TARGET => self.pick("object", "objeto"),
            other => other,
        }
    }

    pub(crate) fn problems_title(&self, count: usize) -> String {
        format!("{} ({count})", self.problems)
    }

    pub(crate) fn problems_found(&self, count: usize) -> String {
        match (count, self.language) {
            (1, Language::English) => "1 problem".to_owned(),
            (1, Language::Spanish) => "1 problema".to_owned(),
            (_, Language::English) => format!("{count} problems"),
            (_, Language::Spanish) => format!("{count} problemas"),
        }
    }

    pub(crate) fn check_passed(&self, scripts: usize) -> String {
        match (scripts, self.language) {
            (0, _) => self
                .pick(
                    "No problems (the project has no scripts)",
                    "Sin problemas (el proyecto no tiene scripts)",
                )
                .to_owned(),
            (1, _) => self
                .pick("No problems in 1 script", "Sin problemas en 1 script")
                .to_owned(),
            (_, Language::English) => format!("No problems in {scripts} scripts"),
            (_, Language::Spanish) => format!("Sin problemas en {scripts} scripts"),
        }
    }

    pub(crate) fn check_not_started(&self, error: &std::io::Error) -> String {
        match self.language {
            Language::English => format!("Could not start compiling: {error}"),
            Language::Spanish => format!("No se pudo empezar a compilar: {error}"),
        }
    }

    pub(crate) fn assets_unreadable(&self, error: &std::io::Error) -> String {
        match self.language {
            Language::English => format!("Could not read the assets folder: {error}"),
            Language::Spanish => format!("No se pudo leer la carpeta assets: {error}"),
        }
    }

    pub(crate) fn settings_error(&self, error: ProjectSettingsError) -> String {
        match error {
            ProjectSettingsError::InvalidName(name_error) => self.name_error(name_error),
            ProjectSettingsError::NoPlatformSelected => self
                .pick(
                    "Select at least one platform.",
                    "Selecciona al menos una plataforma.",
                )
                .to_owned(),
        }
    }

    pub(crate) fn creation_error(&self, error: &CreateProjectError) -> String {
        match (error, self.language) {
            (CreateProjectError::AlreadyExists(path), Language::English) => format!(
                "{} already exists. Choose another name or location.",
                path.display()
            ),
            (CreateProjectError::AlreadyExists(path), Language::Spanish) => format!(
                "Ya existe {}. Elige otro nombre o ubicación.",
                path.display()
            ),
            (CreateProjectError::Io(error), Language::English) => {
                format!("Could not create the project: {error}")
            }
            (CreateProjectError::Io(error), Language::Spanish) => {
                format!("No se pudo crear el proyecto: {error}")
            }
        }
    }

    pub(crate) fn open_error(&self, error: &OpenProjectError) -> String {
        match (error, self.language) {
            (OpenProjectError::NotAProject(path), Language::English) => format!(
                "{} is not a ProjectStar project (it has no project.toml).",
                path.display()
            ),
            (OpenProjectError::NotAProject(path), Language::Spanish) => format!(
                "{} no es un proyecto de ProjectStar (no contiene project.toml).",
                path.display()
            ),
            (OpenProjectError::Io(error), Language::English) => {
                format!("Could not read the project: {error}")
            }
            (OpenProjectError::Io(error), Language::Spanish) => {
                format!("No se pudo leer el proyecto: {error}")
            }
            (OpenProjectError::InvalidManifest(error), Language::English) => {
                format!("The project.toml file is not valid: {error}")
            }
            (OpenProjectError::InvalidManifest(error), Language::Spanish) => {
                format!("El archivo project.toml no es válido: {error}")
            }
        }
    }

    fn name_error(&self, error: ProjectNameError) -> String {
        match (error, self.language) {
            (ProjectNameError::Empty, _) => self
                .pick(
                    "Write a name for the project.",
                    "Escribe un nombre para el proyecto.",
                )
                .to_owned(),
            (ProjectNameError::TooLong, Language::English) => {
                format!("The name cannot be longer than {MAX_NAME_LENGTH} characters.")
            }
            (ProjectNameError::TooLong, Language::Spanish) => {
                format!("El nombre no puede superar {MAX_NAME_LENGTH} caracteres.")
            }
            (ProjectNameError::InvalidStart, _) => self
                .pick(
                    "The name must start with a letter or a digit.",
                    "El nombre debe empezar por una letra o un número.",
                )
                .to_owned(),
            (ProjectNameError::InvalidCharacter(character), Language::English) => format!(
                "The name cannot contain “{character}”. Use letters, digits, spaces, - or _."
            ),
            (ProjectNameError::InvalidCharacter(character), Language::Spanish) => format!(
                "El nombre no puede contener «{character}». Usa letras, números, espacios, - o _."
            ),
            (ProjectNameError::Reserved, _) => self
                .pick(
                    "That name is reserved by the system.",
                    "Ese nombre está reservado por el sistema.",
                )
                .to_owned(),
        }
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

#[cfg(test)]
mod tests {
    use star_blueprint::Call;

    use super::*;

    const ENGLISH: &Texts = Language::English.texts();
    const SPANISH: &Texts = Language::Spanish.texts();

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

        assert_eq!(ENGLISH.node_title(&literal), "\"jump\"");
        assert_eq!(
            ENGLISH.node_title(&NodeKind::Literal {
                value: Literal::Float(2.0)
            }),
            "2.0"
        );
        assert_eq!(ENGLISH.node_title(&multiply), "*");
        assert_eq!(ENGLISH.node_title(&set), "Set speed");
        assert_eq!(SPANISH.node_title(&set), "Asignar speed");
        assert_eq!(
            ENGLISH.node_title(&call(CallTarget::Class("Input".to_owned()), &[])),
            "Input.is_down()"
        );
        assert_eq!(
            ENGLISH.node_title(&call(CallTarget::Value, &[])),
            ".is_down()"
        );
    }

    #[test]
    fn pins_of_every_node_of_a_kind_are_named_in_the_language_shown() {
        assert_eq!(ENGLISH.pin_label(&NodeKind::Branch, BRANCH_TRUE), "true");
        assert_eq!(SPANISH.pin_label(&NodeKind::Branch, BRANCH_TRUE), "sí");
        assert_eq!(ENGLISH.pin_label(&NodeKind::Branch, EXECUTION_INPUT), "");
        assert_eq!(
            SPANISH.pin_label(&call(CallTarget::Value, &[]), TARGET),
            "objeto"
        );
    }

    #[test]
    fn arguments_and_parameters_keep_their_names() {
        let call = call(CallTarget::OwnObject, &[VALUE]);

        assert_eq!(SPANISH.pin_label(&call, VALUE), VALUE);
        assert_eq!(SPANISH.pin_label(&NodeKind::Entry, "delta"), "delta");
        assert_eq!(SPANISH.pin_label(&NodeKind::Entry, EXECUTION_OUTPUT), "");
    }

    #[test]
    fn counts_agree_with_their_noun() {
        assert_eq!(ENGLISH.problems_found(1), "1 problem");
        assert_eq!(ENGLISH.problems_found(3), "3 problems");
        assert_eq!(SPANISH.problems_found(3), "3 problemas");
        assert_eq!(ENGLISH.check_passed(2), "No problems in 2 scripts");
    }

    #[test]
    fn every_operator_has_a_name_in_every_language() {
        for texts in [ENGLISH, SPANISH] {
            for operator in Operator::ALL {
                assert!(texts.operator_name(operator).contains(operator.symbol()));
            }
        }
    }
}
