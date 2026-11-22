//! Top menu of the editor window.

use eframe::egui;

use crate::texts;

/// Menu entries that need the editor to react.
pub(crate) enum MenuAction {
    CloseProject,
    ShowAbout,
}

pub(crate) fn show_menu_bar(ui: &mut egui::Ui) -> Option<MenuAction> {
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
        ui.menu_button(texts::MENU_HELP, |ui| {
            if ui.button(texts::ABOUT).clicked() {
                action = Some(MenuAction::ShowAbout);
            }
        });
    });
    action
}
