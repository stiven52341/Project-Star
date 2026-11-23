//! The code editor: a StarScript script of the assets folder, open in a tab
//! of the central area, its text colored and its lines numbered.

mod code_view;
mod completion;
mod editing;
mod find;
mod find_bar;
mod highlight;
mod hover;
mod mistakes;
mod navigation;
mod scope;
mod suggestions;

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_script::Position;

use self::code_view::CodeView;
pub(crate) use self::code_view::Edited;
pub(crate) use self::completion::CandidateKind;
use self::mistakes::Mistake;
use self::scope::Known;
use crate::project::ASSETS_DIRECTORY;
use crate::project_check::{Problem, ProgramOutline};
use crate::style::ERROR_COLOR;
use crate::texts;

/// A script open in the editor.
pub(crate) struct ScriptTab {
    /// Where it is inside the assets folder: `enemies/rock.star`.
    path: String,
    /// Its file on disk.
    file: PathBuf,
    /// Its text, or why its file could not be read.
    text: Result<String, String>,
    view: CodeView,
    /// Whether it changed since it was opened or saved.
    unsaved: bool,
    /// Why the last save failed, until one works.
    save_error: Option<String>,
}

impl ScriptTab {
    /// Opens the script at `path` inside the `assets` folder.
    pub(crate) fn open(assets: &Path, path: String) -> Self {
        let file = assets.join(&path);
        let text = fs::read_to_string(&file).map_err(|error| error.to_string());
        Self {
            path,
            file,
            text,
            view: CodeView::default(),
            unsaved: false,
            save_error: None,
        }
    }

    /// Where it is inside the assets folder.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The name of its file, for its tab, marked if it has unsaved changes.
    pub(crate) fn title(&self) -> String {
        let name = self.path.rsplit('/').next().unwrap_or(&self.path);
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

    /// Writes it to its file if it has unsaved changes. Returns whether it
    /// is saved now; if not, the tab shows why.
    pub(crate) fn save(&mut self) -> bool {
        let Ok(text) = &self.text else {
            return true;
        };
        if !self.unsaved {
            return true;
        }
        match fs::write(&self.file, text) {
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

    /// Puts the cursor at `position` (line and column, from 1), in sight.
    pub(crate) const fn reveal(&mut self, position: Position) {
        self.view.jump_to(position);
    }

    /// Its text, if its file could be read.
    pub(crate) fn text(&self) -> Option<&str> {
        self.text.as_deref().ok()
    }

    /// Shows the text to edit, with the mistakes among `problems` that are
    /// in it marked, and suggestions and the types of names from the
    /// `outline` of the program, if known. Returns whether the user changed
    /// it, and the place of the program they asked to go to.
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        problems: &[Problem],
        outline: Option<&ProgramOutline>,
    ) -> Edited {
        let texts = texts::current();
        let text = match &mut self.text {
            Ok(text) => text,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts.script_unreadable(error));
                return Edited::default();
            }
        };
        if let Some(error) = &self.save_error {
            ui.colored_label(ERROR_COLOR, texts.script_not_saved(error));
        }
        let file = format!("{ASSETS_DIRECTORY}/{}", self.path);
        let mistakes: Vec<Mistake<'_>> = problems
            .iter()
            .filter(|problem| problem.file == file)
            .filter_map(|problem| {
                problem.position.map(|position| Mistake {
                    position,
                    message: &problem.message,
                })
            })
            .collect();
        let id = ui.make_persistent_id(("script", &self.path));
        let knows = Known {
            classes: outline.map(|outline| &outline.classes),
            locals: outline.and_then(|outline| outline.locals_of(&self.path)),
        };
        let edited = self.view.show(ui, text, id, &mistakes, knows);
        self.unsaved |= edited.changed;
        edited
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    const ROCK: &str = "class Rock {\n}\n";

    fn assets_with_rock(test_name: &str) -> TestDirectory {
        let assets = TestDirectory::new(test_name);
        fs::create_dir_all(assets.path()).unwrap();
        fs::write(assets.path().join("rock.star"), ROCK).unwrap();
        assets
    }

    #[test]
    fn scripts_open_from_the_assets_folder() {
        let assets = assets_with_rock("script_tab_open");

        let tab = ScriptTab::open(assets.path(), "rock.star".to_owned());

        assert_eq!(tab.text.as_deref(), Ok(ROCK));
        assert_eq!(tab.title(), "rock.star");
        assert!(!tab.is_unsaved());
    }

    #[test]
    fn missing_files_say_why() {
        let assets = TestDirectory::new("script_tab_missing");

        let tab = ScriptTab::open(assets.path(), "gone.star".to_owned());

        assert!(tab.text.is_err());
    }

    #[test]
    fn saved_changes_are_in_the_file() {
        let assets = assets_with_rock("script_tab_save");
        let mut tab = ScriptTab::open(assets.path(), "rock.star".to_owned());
        tab.text = Ok("class Stone {\n}\n".to_owned());
        tab.unsaved = true;

        assert!(tab.save());

        let text = fs::read_to_string(assets.path().join("rock.star")).unwrap();
        assert_eq!(text, "class Stone {\n}\n");
        assert!(!tab.is_unsaved());
    }

    #[test]
    fn failed_saves_keep_the_changes_and_say_why() {
        let assets = assets_with_rock("script_tab_save_fails");
        let mut tab = ScriptTab::open(assets.path(), "rock.star".to_owned());
        tab.file = assets.path().join("missing").join("rock.star");
        tab.unsaved = true;

        assert!(!tab.save());

        assert!(tab.is_unsaved());
        assert!(tab.save_error.is_some());
    }
}
