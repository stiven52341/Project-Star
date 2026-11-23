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

    /// The keys of the list of suggestions, `active` or not yet; `choose`
    /// makes it active.
    pub(crate) fn suggestion_keys(&self, active: bool, choose: &str) -> String {
        match (self.language, active) {
            (Language::English, true) => "Tab: write · Up/Down: choose · Esc: hide".to_owned(),
            (Language::Spanish, true) => {
                "Tab: escribir · Arriba/Abajo: elegir · Esc: ocultar".to_owned()
            }
            (Language::English, false) => format!("Tab: write · {choose}: choose · Esc: hide"),
            (Language::Spanish, false) => {
                format!("Tab: escribir · {choose}: elegir · Esc: ocultar")
            }
        }
    }

    /// The hint of the field of the text to find.
    pub(crate) const fn find_hint(&self) -> &'static str {
        self.pick("Find", "Buscar")
    }

    /// The hint of the field of what replaces the text found.
    pub(crate) const fn replace_hint(&self) -> &'static str {
        self.pick("Replace with", "Reemplazar por")
    }

    /// What the button that tells upper and lower case apart does.
    pub(crate) const fn match_case(&self) -> &'static str {
        self.pick("Match case", "Distinguir mayúsculas y minúsculas")
    }

    pub(crate) const fn previous_match(&self) -> &'static str {
        self.pick("Previous match (Shift+Enter)", "Anterior (Mayús+Intro)")
    }

    pub(crate) const fn next_match(&self) -> &'static str {
        self.pick("Next match (Enter)", "Siguiente (Intro)")
    }

    pub(crate) const fn close_find(&self) -> &'static str {
        self.pick("Close (Esc)", "Cerrar (Esc)")
    }

    /// The button that replaces the match chosen.
    pub(crate) const fn replace(&self) -> &'static str {
        self.pick("Replace", "Reemplazar")
    }

    /// The button that replaces every match.
    pub(crate) const fn replace_all(&self) -> &'static str {
        self.pick("Replace all", "Reemplazar todo")
    }

    /// What the find bar says when the text is nowhere.
    pub(crate) const fn no_matches(&self) -> &'static str {
        self.pick("No results", "Sin resultados")
    }

    /// Which match is chosen, of how many.
    pub(crate) fn match_count(&self, chosen: usize, count: usize) -> String {
        match self.language {
            Language::English => format!("{chosen} of {count}"),
            Language::Spanish => format!("{chosen} de {count}"),
        }
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
