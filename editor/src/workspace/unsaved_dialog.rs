//! The question asked before closing blueprints with unsaved changes.

use eframe::egui;

use crate::texts;

/// What the user answered.
pub(crate) enum UnsavedChoice {
    /// Save, then close.
    Save,
    /// Close without saving.
    Discard,
    /// Do not close.
    Cancel,
}

/// A modal window with `question`; returns the answer once given.
pub(crate) fn show_unsaved_dialog(
    context: &egui::Context,
    question: &str,
) -> Option<UnsavedChoice> {
    let mut choice = None;
    let modal = egui::Modal::new(egui::Id::unique("unsaved_changes")).show(context, |ui| {
        ui.strong(texts::UNSAVED_CHANGES);
        ui.label(question);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(texts::SAVE).clicked() {
                choice = Some(UnsavedChoice::Save);
            }
            if ui.button(texts::DISCARD_CHANGES).clicked() {
                choice = Some(UnsavedChoice::Discard);
            }
            if ui.button(texts::CANCEL).clicked() {
                choice = Some(UnsavedChoice::Cancel);
            }
        });
    });
    if modal.should_close() && choice.is_none() {
        choice = Some(UnsavedChoice::Cancel);
    }
    choice
}
