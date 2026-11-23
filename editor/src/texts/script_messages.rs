//! Texts of the code editor, in each language.

use super::{Language, Texts};

impl Texts {
    pub(crate) fn script_unreadable(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not open the script: {reason}"),
            Language::Spanish => format!("No se pudo abrir el script: {reason}"),
        }
    }

    pub(crate) fn script_not_saved(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not save the script: {reason}"),
            Language::Spanish => format!("No se pudo guardar el script: {reason}"),
        }
    }

    /// Where the cursor of the code editor is.
    pub(crate) fn cursor_position(&self, line: usize, column: usize) -> String {
        match self.language {
            Language::English => format!("Line {line}, column {column}"),
            Language::Spanish => format!("Línea {line}, columna {column}"),
        }
    }

    /// What the inspector shows while a script is shown.
    pub(crate) const fn script_inspector_hint(&self) -> &'static str {
        self.pick(
            "Scripts are edited as text in the central area.",
            "Los scripts se editan como texto en el área central.",
        )
    }
}
