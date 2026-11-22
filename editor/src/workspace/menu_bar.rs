//! Top menu of the editor window.

use eframe::egui;

use crate::language_menu::show_language_menu;
use crate::texts::{self, Language};

/// Saves the blueprint shown.
pub(crate) const SAVE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);

/// Menu entries that need the editor to react.
pub(crate) enum MenuAction {
    Save,
    CloseProject,
    Compile,
    ShowAbout,
    ChangeLanguage(Language),
}

/// The menu; “Save” is enabled if there is something to `save`, and
/// “Compile” is disabled while `compiling`.
pub(crate) fn show_menu_bar(
    ui: &mut egui::Ui,
    can_save: bool,
    compiling: bool,
) -> Option<MenuAction> {
    let mut action = None;
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button(texts::current().menu_file, |ui| {
            let save = egui::Button::new(texts::current().save)
                .shortcut_text(ui.ctx().format_shortcut(&SAVE_SHORTCUT));
            if ui.add_enabled(can_save, save).clicked() {
                action = Some(MenuAction::Save);
            }
            if ui.button(texts::current().close_project).clicked() {
                action = Some(MenuAction::CloseProject);
            }
            ui.separator();
            if let Some(language) = show_language_menu(ui) {
                action = Some(MenuAction::ChangeLanguage(language));
            }
            ui.separator();
            if ui.button(texts::current().quit).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button(texts::current().menu_project, |ui| {
            if compile_button(ui, compiling) {
                action = Some(MenuAction::Compile);
            }
        });
        ui.menu_button(texts::current().menu_help, |ui| {
            if ui.button(texts::current().about).clicked() {
                action = Some(MenuAction::ShowAbout);
            }
        });
        ui.separator();
        if compile_button(ui, compiling) {
            action = Some(MenuAction::Compile);
        }
    });
    action
}

/// “Compile”, as a menu entry or in the bar itself. Returns whether it was
/// clicked.
fn compile_button(ui: &mut egui::Ui, compiling: bool) -> bool {
    ui.add_enabled(!compiling, egui::Button::new(texts::current().compile))
        .on_hover_text(texts::current().compile_hint)
        .clicked()
}
