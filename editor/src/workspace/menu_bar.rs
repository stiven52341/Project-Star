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
    Play,
    Stop,
    ShowAbout,
    ChangeLanguage(Language),
}

/// What the menu's entries can do now.
#[derive(Clone, Copy)]
pub(crate) struct MenuState {
    /// There is something to save.
    pub(crate) can_save: bool,
    /// A check is running: “Compile” and “Play” wait for it.
    pub(crate) compiling: bool,
    /// The game is being built or played: “Stop” stops it.
    pub(crate) playing: bool,
}

/// The menu, with its entries enabled as `state` allows.
pub(crate) fn show_menu_bar(ui: &mut egui::Ui, state: MenuState) -> Option<MenuAction> {
    let MenuState {
        can_save,
        compiling,
        playing,
    } = state;
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
            if let Some(chosen) = project_buttons(ui, compiling, playing) {
                action = Some(chosen);
            }
        });
        ui.menu_button(texts::current().menu_help, |ui| {
            if ui.button(texts::current().about).clicked() {
                action = Some(MenuAction::ShowAbout);
            }
        });
        ui.separator();
        if let Some(chosen) = project_buttons(ui, compiling, playing) {
            action = Some(chosen);
        }
    });
    action
}

/// “Compile”, “Play” and “Stop”, as menu entries or in the bar itself.
/// Returns the one clicked.
fn project_buttons(ui: &mut egui::Ui, compiling: bool, playing: bool) -> Option<MenuAction> {
    let texts = texts::current();
    let mut action = None;
    if ui
        .add_enabled(!compiling, egui::Button::new(texts.compile))
        .on_hover_text(texts.compile_hint)
        .clicked()
    {
        action = Some(MenuAction::Compile);
    }
    if ui
        .add_enabled(!compiling && !playing, egui::Button::new(texts.play))
        .on_hover_text(texts.play_hint)
        .clicked()
    {
        action = Some(MenuAction::Play);
    }
    if ui
        .add_enabled(playing, egui::Button::new(texts.stop))
        .on_hover_text(texts.stop_hint)
        .clicked()
    {
        action = Some(MenuAction::Stop);
    }
    action
}
