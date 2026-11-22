//! Top menu of the editor window.

use eframe::egui;

use crate::texts;

/// Menu entries that need the editor to react.
pub(crate) enum MenuAction {
    CloseProject,
    Compile,
    ShowAbout,
}

/// The menu; «Compilar» is disabled while `compiling`.
pub(crate) fn show_menu_bar(ui: &mut egui::Ui, compiling: bool) -> Option<MenuAction> {
    let mut action = None;
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button(texts::MENU_FILE, |ui| {
            if ui.button(texts::CLOSE_PROJECT).clicked() {
                action = Some(MenuAction::CloseProject);
            }
            ui.separator();
            if ui.button(texts::QUIT).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button(texts::MENU_PROJECT, |ui| {
            if compile_button(ui, compiling) {
                action = Some(MenuAction::Compile);
            }
        });
        ui.menu_button(texts::MENU_HELP, |ui| {
            if ui.button(texts::ABOUT).clicked() {
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

/// «Compilar», as a menu entry or in the bar itself. Returns whether it was
/// clicked.
fn compile_button(ui: &mut egui::Ui, compiling: bool) -> bool {
    ui.add_enabled(!compiling, egui::Button::new(texts::COMPILE))
        .on_hover_text(texts::COMPILE_HINT)
        .clicked()
}
