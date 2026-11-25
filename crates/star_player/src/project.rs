//! A project as the player reads it from its folder: the settings of
//! `project.toml`, and its program, made of the StarScript files of its
//! assets folder and the translations of its blueprints.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use star_blueprint::{Blueprint, Translation, translate};
use star_script::{GameSettings, SourceSet, Span, check_program_with};
use star_script_interpreter::Program;

pub use crate::settings::read_game_settings;

/// The file with a project's settings, in its folder.
pub const MANIFEST_FILE_NAME: &str = "project.toml";
/// The folder of a project's assets (scripts, blueprints, scenes, images
/// and sounds), in its folder.
pub const ASSETS_DIRECTORY: &str = "assets";
/// The ending of the names of StarScript files.
const SCRIPT_EXTENSION: &str = ".star";
/// The ending of the names of blueprints.
const BLUEPRINT_EXTENSION: &str = ".blueprint";

/// A project read from its folder, ready to be checked and played.
#[derive(Debug)]
pub struct Project {
    directory: PathBuf,
    settings: GameSettings,
    sources: SourceSet,
    /// The translation of each blueprint, by the name of its file, to name
    /// the part of the blueprint a mistake of its code comes from.
    translations: HashMap<String, Translation>,
}

/// What is wrong with a project, one problem per line, each naming its
/// file (and the line and column, or the part of a blueprint, where it is),
/// as the editor's Problems panel names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problems(Vec<String>);

impl Problems {
    /// Each problem, as `file:line:column: message` or `file: message`.
    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.0
    }
}

impl fmt::Display for Problems {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.join("\n"))
    }
}

impl std::error::Error for Problems {}

impl Project {
    /// Reads the project in `directory`: its settings, its StarScript files
    /// and its blueprints, translated to StarScript.
    ///
    /// # Errors
    ///
    /// Every file that cannot be read, a `project.toml` that is not valid,
    /// and every blueprint that cannot become StarScript.
    pub fn read(directory: &Path) -> Result<Self, Problems> {
        let mut problems = Vec::new();
        let settings = fs::read_to_string(directory.join(MANIFEST_FILE_NAME))
            .map_err(|error| error.to_string())
            .and_then(|text| read_game_settings(&text))
            .unwrap_or_else(|message| {
                problems.push(format!("{MANIFEST_FILE_NAME}: {message}"));
                GameSettings::default()
            });
        let mut project = Self {
            directory: directory.to_owned(),
            settings,
            sources: SourceSet::new(),
            translations: HashMap::new(),
        };
        let mut paths = Vec::new();
        if let Err(error) = program_files(&project.assets_folder(), "", &mut paths) {
            problems.push(format!("{ASSETS_DIRECTORY}: {error}"));
        }
        for path in paths {
            if let Err(problem) = project.add_file(&path) {
                problems.extend(problem);
            }
        }
        if problems.is_empty() {
            Ok(project)
        } else {
            Err(Problems(problems))
        }
    }

    /// The settings of its `project.toml`.
    #[must_use]
    pub const fn settings(&self) -> &GameSettings {
        &self.settings
    }

    /// Its assets folder, where its game loads scenes, images and sounds.
    #[must_use]
    pub fn assets_folder(&self) -> PathBuf {
        self.directory.join(ASSETS_DIRECTORY)
    }

    /// Checks its program and prepares it to be interpreted.
    ///
    /// # Errors
    ///
    /// Every mistake the compiler finds in its code, or what the
    /// interpreter cannot run; that the project has nothing to play if it
    /// has no scripts, no blueprints and no main scene.
    pub fn program(&self) -> Result<Program, Problems> {
        if self.sources.is_empty() && self.settings.main_scene.is_none() {
            return Err(Problems(vec![format!(
                "{MANIFEST_FILE_NAME}: the project has nothing to play: no scripts, no \
                 blueprints and no main scene"
            )]));
        }
        let checked = check_program_with(&self.sources, &self.settings).map_err(|errors| {
            Problems(
                errors
                    .iter()
                    .map(|error| self.problem(error.span(), &error.to_string()))
                    .collect(),
            )
        })?;
        Program::lower(&checked)
            .map_err(|error| Problems(vec![self.problem(error.span, &error.to_string())]))
    }

    /// Adds the StarScript file or blueprint at `path` inside the assets
    /// folder to the program, named from the project's folder.
    fn add_file(&mut self, path: &str) -> Result<(), Vec<String>> {
        let name = format!("{ASSETS_DIRECTORY}/{path}");
        let text = fs::read_to_string(self.assets_folder().join(path))
            .map_err(|error| vec![format!("{name}: {error}")])?;
        if !path.ends_with(BLUEPRINT_EXTENSION) {
            self.sources.add(name, text);
            return Ok(());
        }
        let blueprint =
            Blueprint::from_text(&text).map_err(|error| vec![format!("{name}: {error}")])?;
        let translation = translate(&blueprint).map_err(|errors| {
            errors
                .iter()
                .map(|error| format!("{name}: {error}"))
                .collect::<Vec<_>>()
        })?;
        self.sources.add(name.clone(), translation.source());
        self.translations.insert(name, translation);
        Ok(())
    }

    /// `message` about the code at `span`, with its file, line and column;
    /// in a blueprint, with the part of it the code was translated from.
    fn problem(&self, span: Span, message: &str) -> String {
        let Some(location) = self.sources.locate(span) else {
            return format!("{ASSETS_DIRECTORY}: {message}");
        };
        let file = location.file;
        if let Some(translation) = self.translations.get(file) {
            return match translation.origin(location.offset) {
                Some(origin) => format!("{file}: {origin}: {message}"),
                None => format!("{file}: {message}"),
            };
        }
        let position = location.position;
        format!("{file}:{}:{}: {message}", position.line, position.column)
    }
}

/// Adds to `paths` the StarScript files and blueprints in `folder` and its
/// folders, as paths inside the assets folder (`prefix` is `folder`'s),
/// with `/` between folders on every platform, sorted by name. Hidden files
/// and folders are left out, as the editor leaves them out.
pub(crate) fn program_files(
    folder: &Path,
    prefix: &str,
    paths: &mut Vec<String>,
) -> std::io::Result<()> {
    let mut entries = fs::read_dir(folder)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = format!("{prefix}{name}");
        let kind = entry.file_type()?;
        if kind.is_dir() {
            program_files(&entry.path(), &format!("{path}/"), paths)?;
        } else if kind.is_file()
            && (name.ends_with(SCRIPT_EXTENSION) || name.ends_with(BLUEPRINT_EXTENSION))
        {
            paths.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
