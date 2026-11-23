//! Texts about the editor's log, in each language.

use log::Level;

use super::Texts;

impl Texts {
    pub(crate) const fn level_name(&self, level: Level) -> &'static str {
        match level {
            Level::Error => self.pick("Error", "Error"),
            Level::Warn => self.pick("Warning", "Aviso"),
            Level::Info => self.pick("Info", "Info"),
            Level::Debug => self.pick("Debug", "Depuración"),
            Level::Trace => self.pick("Trace", "Traza"),
        }
    }

    /// The messages of `level` and those more severe, for the console's
    /// filter.
    pub(crate) const fn levels_from(&self, level: Level) -> &'static str {
        match level {
            Level::Error => self.pick("Errors", "Errores"),
            Level::Warn => self.pick("Warnings and errors", "Avisos y errores"),
            Level::Info => self.pick("Info and above", "Info y más graves"),
            Level::Debug | Level::Trace => self.pick("Everything", "Todo"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texts::Language;

    #[test]
    fn levels_are_named_in_the_language_shown() {
        assert_eq!(Language::Spanish.texts().level_name(Level::Warn), "Aviso");
        assert_eq!(
            Language::English.texts().levels_from(Level::Error),
            "Errors"
        );
    }
}
