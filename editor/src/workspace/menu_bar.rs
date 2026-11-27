//! Top menu of the editor window.

use eframe::egui;
use star_script::ShowFps;

use crate::language_menu::show_language_menu;
use crate::texts::{self, Language};

/// Saves the blueprint shown.
pub(crate) const SAVE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);

/// Undoes the last change to the scene.
pub(crate) const UNDO_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z);

/// Makes again the last change undone in the scene.
pub(crate) const REDO_SHORTCUT: egui::KeyboardShortcut = egui::KeyboardShortcut::new(
    egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
    egui::Key::Z,
);

/// The other key for redo, usual on Windows and Linux.
pub(crate) const REDO_OTHER_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Y);

/// Menu entries that need the editor to react.
pub(crate) enum MenuAction {
    Save,
    Undo,
    Redo,
    CloseProject,
    Compile,
    Play,
    PlayCompiled,
    Stop,
    ShowAbout,
    ChangeLanguage(Language),
    /// Changes when the game shows its frames per second.
    SetShowFps(ShowFps),
    /// Opens the tab of the groups of sounds.
    OpenMixer,
}

/// Whether the scene shown has changes to undo and to redo.
#[derive(Clone, Copy, Default)]
pub(crate) struct UndoSteps {
    pub(crate) can_undo: bool,
    pub(crate) can_redo: bool,
}

/// What the menu's entries can do now.
#[derive(Clone, Copy)]
pub(crate) struct MenuState {
    /// There is something to save.
    pub(crate) can_save: bool,
    /// What “Undo” and “Redo” can do.
    pub(crate) steps: UndoSteps,
    /// A check is running: “Compile” and “Play” wait for it.
    pub(crate) compiling: bool,
    /// The game is being built or played: “Stop” stops it.
    pub(crate) playing: bool,
    /// When the game shows its frames per second.
    pub(crate) show_fps: ShowFps,
}

/// The menu, with its entries enabled as `state` allows.
pub(crate) fn show_menu_bar(ui: &mut egui::Ui, state: MenuState) -> Option<MenuAction> {
    let MenuState {
        can_save,
        compiling,
        playing,
        steps,
        show_fps,
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
        ui.menu_button(texts::current().menu_edit, |ui| {
            if let Some(chosen) = edit_buttons(ui, steps) {
                action = Some(chosen);
            }
        });
        ui.menu_button(texts::current().menu_project, |ui| {
            if let Some(chosen) = project_buttons(ui, compiling, playing) {
                action = Some(chosen);
            }
            ui.separator();
            if let Some(chosen) = show_fps_menu(ui, show_fps) {
                action = Some(MenuAction::SetShowFps(chosen));
            }
            if ui.button(texts::current().open_mixer()).clicked() {
                action = Some(MenuAction::OpenMixer);
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

/// The choices of when the game shows its frames per second, `current`
/// marked. Returns the one clicked, if it is another.
fn show_fps_menu(ui: &mut egui::Ui, current: ShowFps) -> Option<ShowFps> {
    let texts = texts::current();
    let mut chosen = None;
    ui.menu_button(texts.show_fps(), |ui| {
        for choice in ShowFps::ALL {
            if ui
                .radio(choice == current, texts.show_fps_choice(choice))
                .clicked()
                && choice != current
            {
                chosen = Some(choice);
            }
        }
    });
    chosen
}

/// “Undo” and “Redo”, enabled as `steps` allows. Returns the one clicked.
fn edit_buttons(ui: &mut egui::Ui, steps: UndoSteps) -> Option<MenuAction> {
    let texts = texts::current();
    let undo =
        egui::Button::new(texts.undo).shortcut_text(ui.ctx().format_shortcut(&UNDO_SHORTCUT));
    let redo =
        egui::Button::new(texts.redo).shortcut_text(ui.ctx().format_shortcut(&REDO_SHORTCUT));
    if ui.add_enabled(steps.can_undo, undo).clicked() {
        return Some(MenuAction::Undo);
    }
    if ui.add_enabled(steps.can_redo, redo).clicked() {
        return Some(MenuAction::Redo);
    }
    None
}

/// “Compile”, “Play”, “Play compiled” and “Stop”, as menu entries or in
/// the bar itself.
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
        .add_enabled(
            !compiling && !playing,
            egui::Button::new(texts.play_compiled),
        )
        .on_hover_text(texts.play_compiled_hint)
        .clicked()
    {
        action = Some(MenuAction::PlayCompiled);
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
