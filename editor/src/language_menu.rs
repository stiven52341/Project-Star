//! The menu to choose the language of the editor, shared by every screen.

use eframe::egui;

use crate::texts::{self, Language};

/// The menu of languages, each in its own name. Returns the one chosen, if
/// it is not the language already shown.
pub(crate) fn show_language_menu(ui: &mut egui::Ui) -> Option<Language> {
    let current = texts::current_language();
    let mut chosen = None;
    ui.menu_button(texts::current().language_menu, |ui| {
        for language in Language::ALL {
            if ui
                .radio(language == current, language.native_name())
                .clicked()
            {
                ui.close();
                chosen = (language != current).then_some(language);
            }
        }
    });
    chosen
}
