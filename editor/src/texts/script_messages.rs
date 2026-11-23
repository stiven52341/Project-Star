//! Texts of the code editor, in each language.

use super::{Language, Texts};
use crate::script_editor::CandidateKind;

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

    /// How many mistakes the script shown has.
    pub(crate) fn mistakes_in_script(&self, count: usize) -> String {
        match (self.language, count) {
            (Language::English, 1) => "1 mistake in this script".to_owned(),
            (Language::English, _) => format!("{count} mistakes in this script"),
            (Language::Spanish, 1) => "1 error en este script".to_owned(),
            (Language::Spanish, _) => format!("{count} errores en este script"),
        }
    }

    /// The keys of the list of suggestions.
    pub(crate) const fn suggestion_keys(&self) -> &'static str {
        self.pick(
            "Tab: write · Up/Down: choose · Esc: hide",
            "Tab: escribir · Arriba/Abajo: elegir · Esc: ocultar",
        )
    }

    /// What a suggested name is, when it has no signature to show.
    pub(crate) const fn candidate_kind(&self, kind: CandidateKind) -> &'static str {
        match kind {
            CandidateKind::Value => self.pick("value", "valor"),
            CandidateKind::Method => self.pick("method", "método"),
            CandidateKind::Name => self.pick("name", "nombre"),
            CandidateKind::Class => self.pick("class", "clase"),
            CandidateKind::Keyword => self.pick("keyword", "palabra clave"),
            CandidateKind::BasicType => self.pick("type", "tipo"),
        }
    }
}
