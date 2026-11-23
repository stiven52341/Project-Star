//! What the menus of the assets panel start: naming a new file or folder,
//! or a new name for one, in a row over the grid; and asking before
//! deleting one.

use std::path::Path;

use eframe::egui;

use super::asset_files::{AssetFileError, NewAsset, create, delete, rename};
use crate::project::ProjectDimension;
use crate::style::ERROR_COLOR;
use crate::texts;

/// A file or folder of the panel a menu acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Target {
    /// Where it is inside the assets folder.
    pub(super) path: String,
    pub(super) name: String,
    pub(super) is_folder: bool,
}

/// What the user chose in a menu that needs more from them.
pub(super) enum Choice {
    Create(NewAsset),
    Rename(Target),
    Delete(Target),
}

/// What changed on disk.
pub(super) enum Outcome {
    Created(String),
    Renamed { from: String, to: String },
    Deleted(String),
}

/// The row that names a file and the question before deleting, while one
/// is open.
pub(super) struct AssetActions {
    dimension: ProjectDimension,
    naming: Option<Naming>,
    deleting: Option<Deleting>,
}

/// The row that names a new file or folder, or gives one a new name.
struct Naming {
    purpose: Purpose,
    name: String,
    /// Why the last try failed.
    error: Option<String>,
    /// Whether the field still has to take the keyboard.
    focus: bool,
}

enum Purpose {
    Create(NewAsset),
    Rename(Target),
}

struct Deleting {
    target: Target,
    error: Option<String>,
}

impl AssetActions {
    /// Creates scenes for a project of `dimension`.
    pub(super) const fn new(dimension: ProjectDimension) -> Self {
        Self {
            dimension,
            naming: None,
            deleting: None,
        }
    }

    /// Opens the row or the question that `choice` needs.
    pub(super) fn choose(&mut self, choice: Choice) {
        let (purpose, name) = match choice {
            Choice::Create(kind) => (Purpose::Create(kind), kind.default_name().to_owned()),
            Choice::Rename(target) => {
                let name = stem(&target).to_owned();
                (Purpose::Rename(target), name)
            }
            Choice::Delete(target) => {
                self.deleting = Some(Deleting {
                    target,
                    error: None,
                });
                return;
            }
        };
        self.naming = Some(Naming {
            purpose,
            name,
            error: None,
            focus: true,
        });
    }

    /// Shows the row and the question that are open; files are created in
    /// `folder` of the `assets` folder. Returns what changed on disk.
    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        assets: &Path,
        folder: &str,
    ) -> Option<Outcome> {
        let named = self.show_naming(ui, assets, folder);
        let deleted = self.show_delete_question(ui.ctx(), assets);
        named.or(deleted)
    }

    fn show_naming(&mut self, ui: &mut egui::Ui, assets: &Path, folder: &str) -> Option<Outcome> {
        let naming = self.naming.as_mut()?;
        let texts = texts::current();
        let (label, confirm) = match &naming.purpose {
            Purpose::Create(kind) => (texts.name_of_new(*kind), texts.create),
            Purpose::Rename(_) => (texts.rename_to, texts.rename),
        };
        let mut submit = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            ui.label(label);
            let field = ui.text_edit_singleline(&mut naming.name);
            if naming.focus {
                field.request_focus();
                naming.focus = false;
            }
            let entered =
                field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            submit = ui.button(confirm).clicked() || entered;
            cancel = ui.button(texts.cancel).clicked()
                || ui.input(|input| input.key_pressed(egui::Key::Escape));
        });
        if let Some(error) = &naming.error {
            ui.colored_label(ERROR_COLOR, error);
        }
        if cancel {
            self.naming = None;
            return None;
        }
        if !submit {
            return None;
        }
        let done = match &naming.purpose {
            Purpose::Create(kind) => {
                create(assets, folder, *kind, &naming.name, self.dimension).map(Outcome::Created)
            }
            Purpose::Rename(target) => rename(assets, &target.path, &naming.name, target.is_folder)
                .map(|to| Outcome::Renamed {
                    from: target.path.clone(),
                    to,
                }),
        };
        match done {
            Ok(outcome) => {
                self.naming = None;
                Some(outcome)
            }
            Err(error) => {
                naming.error = Some(texts.asset_file_error(&error));
                None
            }
        }
    }

    fn show_delete_question(&mut self, context: &egui::Context, assets: &Path) -> Option<Outcome> {
        let deleting = self.deleting.as_mut()?;
        let texts = texts::current();
        let mut answer = None;
        let modal = egui::Modal::new(egui::Id::unique("delete_asset")).show(context, |ui| {
            ui.label(texts.delete_question(&deleting.target.name, deleting.target.is_folder));
            if let Some(error) = &deleting.error {
                ui.colored_label(ERROR_COLOR, error);
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(texts.delete).clicked() {
                    answer = Some(true);
                }
                if ui.button(texts.cancel).clicked() {
                    answer = Some(false);
                }
            });
        });
        if modal.should_close() && answer.is_none() {
            answer = Some(false);
        }
        if !answer? {
            self.deleting = None;
            return None;
        }
        match delete(assets, &deleting.target.path, deleting.target.is_folder) {
            Ok(()) => {
                let path = deleting.target.path.clone();
                self.deleting = None;
                Some(Outcome::Deleted(path))
            }
            Err(error) => {
                deleting.error = Some(texts.asset_file_error(&AssetFileError::Io(error)));
                None
            }
        }
    }
}

/// The name of `target` without a file's extension: what renaming edits.
fn stem(target: &Target) -> &str {
    if target.is_folder {
        return &target.name;
    }
    target
        .name
        .rsplit_once('.')
        .map_or(target.name.as_str(), |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(name: &str, is_folder: bool) -> Target {
        Target {
            path: format!("levels/{name}"),
            name: name.to_owned(),
            is_folder,
        }
    }

    #[test]
    fn renaming_edits_names_without_their_extension() {
        assert_eq!(stem(&target("Intro.scene", false)), "Intro");
        assert_eq!(stem(&target("v1.2", true)), "v1.2");
    }

    #[test]
    fn choices_open_the_row_or_the_question_they_need() {
        let mut actions = AssetActions::new(ProjectDimension::TwoD);

        actions.choose(Choice::Create(NewAsset::Scene));
        actions.choose(Choice::Delete(target("Intro.scene", false)));

        assert_eq!(
            actions.naming.map(|naming| naming.name),
            Some(NewAsset::Scene.default_name().to_owned())
        );
        assert!(actions.deleting.is_some());
    }
}
