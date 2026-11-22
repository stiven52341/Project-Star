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
        ui.strong(texts::current().unsaved_changes);
        ui.label(question);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(texts::current().save).clicked() {
                choice = Some(UnsavedChoice::Save);
            }
            if ui.button(texts::current().discard_changes).clicked() {
                choice = Some(UnsavedChoice::Discard);
            }
            if ui.button(texts::current().cancel).clicked() {
                choice = Some(UnsavedChoice::Cancel);
            }
        });
    });
    if modal.should_close() && choice.is_none() {
        choice = Some(UnsavedChoice::Cancel);
    }
    choice
}
