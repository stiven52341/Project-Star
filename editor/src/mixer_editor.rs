//! The mixer tab (decision 87): the project's groups of sounds (music,
//! effects…) and the volume each starts at, `[audio.groups]` of
//! `project.toml`, to add, change and remove.

use std::path::{Path, PathBuf};

use eframe::egui;

use crate::project::{read_sound_groups, set_sound_groups};
use crate::style::{ERROR_COLOR, PROPERTY_SPACING};
use crate::texts;

/// What names the mixer among the tabs: no file has it.
pub(crate) const MIXER_TAB: &str = "@mixer";
/// The loudest a group can start at: twice as loud as recorded.
const MAX_GROUP_VOLUME: f32 = 2.0;

/// The groups of a project, being edited.
pub(crate) struct MixerTab {
    /// The project's folder, where `project.toml` is.
    directory: PathBuf,
    groups: Vec<(String, f32)>,
    /// The name of the group to add.
    new_group: String,
    unsaved: bool,
    save_error: Option<String>,
}

impl MixerTab {
    /// The mixer of the project in `directory`.
    pub(crate) fn open(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
            groups: read_sound_groups(directory),
            new_group: String::new(),
            unsaved: false,
            save_error: None,
        }
    }

    /// What names it among the tabs.
    #[expect(
        clippy::unused_self,
        reason = "every tab is asked its path the same way"
    )]
    pub(crate) const fn path(&self) -> &str {
        MIXER_TAB
    }

    /// The name of its tab, marked if it has unsaved changes.
    pub(crate) fn title(&self) -> String {
        let name = texts::current().mixer();
        if self.unsaved {
            texts::unsaved_title(name)
        } else {
            name.to_owned()
        }
    }

    /// Whether it changed since it was opened or saved.
    pub(crate) const fn is_unsaved(&self) -> bool {
        self.unsaved
    }

    /// Writes the groups into `project.toml` if they changed. Returns
    /// whether they are saved now; if not, the tab shows why.
    pub(crate) fn save(&mut self) -> bool {
        if !self.unsaved {
            return true;
        }
        match set_sound_groups(&self.directory, &self.groups) {
            Ok(()) => {
                self.unsaved = false;
                self.save_error = None;
                true
            }
            Err(error) => {
                self.save_error = Some(error.to_string());
                false
            }
        }
    }

    /// The groups, each with its volume and a button to remove it, and a
    /// row to add one.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let texts = texts::current();
        ui.weak(texts.mixer_hint());
        if let Some(error) = &self.save_error {
            ui.colored_label(ERROR_COLOR, error);
        }
        ui.separator();
        let mut removed = None;
        egui::Grid::new("mixer_groups")
            .num_columns(3)
            .spacing(PROPERTY_SPACING)
            .show(ui, |ui| {
                for (index, (name, volume)) in self.groups.iter_mut().enumerate() {
                    ui.label(name.as_str());
                    self.unsaved |= ui
                        .add(egui::Slider::new(volume, 0.0..=MAX_GROUP_VOLUME))
                        .changed();
                    if ui.button(texts.remove_group()).clicked() {
                        removed = Some(index);
                    }
                    ui.end_row();
                }
            });
        if let Some(index) = removed {
            self.groups.remove(index);
            self.unsaved = true;
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_group);
            let name = self.new_group.trim();
            let can_add = !name.is_empty() && !self.groups.iter().any(|(group, _)| group == name);
            if ui
                .add_enabled(can_add, egui::Button::new(texts.add_group()))
                .clicked()
            {
                self.groups.push((name.to_owned(), 1.0));
                self.new_group.clear();
                self.unsaved = true;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    #[test]
    fn saved_groups_are_read_again() {
        let directory = TestDirectory::new("mixer_tab");
        std::fs::create_dir_all(directory.path()).unwrap();
        std::fs::write(
            directory.path().join("project.toml"),
            "[project]\nname = \"Demo\"\n",
        )
        .unwrap();
        let mut mixer = MixerTab::open(directory.path());
        assert_eq!(mixer.groups, []);

        mixer.groups.push(("music".to_owned(), 0.5));
        mixer.unsaved = true;

        assert!(mixer.save());
        assert!(!mixer.is_unsaved());
        assert_eq!(MixerTab::open(directory.path()).groups, mixer.groups);
    }
}
