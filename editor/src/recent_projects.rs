//! Projects opened recently, remembered between editor sessions.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use star_engine::prelude::ENGINE_NAME;
use star_engine::star_platform::user_directories::config_directory;

/// How many projects the list remembers.
const MAX_RECENT_PROJECTS: usize = 10;
const RECENT_PROJECTS_FILE_NAME: &str = "recent_projects.toml";

/// Project folders, most recently opened first, without duplicates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RecentProjects {
    projects: Vec<PathBuf>,
}

impl RecentProjects {
    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.projects
    }

    /// Moves `directory` to the top of the list, forgetting the oldest
    /// entries beyond the limit.
    pub(crate) fn add(&mut self, directory: PathBuf) {
        self.remove(&directory);
        self.projects.insert(0, directory);
        self.projects.truncate(MAX_RECENT_PROJECTS);
    }

    pub(crate) fn remove(&mut self, directory: &Path) {
        self.projects.retain(|path| path != directory);
    }

    fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }
}

/// The recent projects list, kept in sync with its file.
///
/// Losing this list is harmless, so read and write failures are logged
/// instead of interrupting the user.
pub(crate) struct RecentProjectsStore {
    file: Option<PathBuf>,
    projects: RecentProjects,
}

impl RecentProjectsStore {
    /// Loads the list saved in the user's configuration folder.
    pub(crate) fn load() -> Self {
        let file = config_directory()
            .map(|directory| directory.join(ENGINE_NAME).join(RECENT_PROJECTS_FILE_NAME));
        Self::load_from(file)
    }

    fn load_from(file: Option<PathBuf>) -> Self {
        let projects = file.as_deref().map(read_list).unwrap_or_default();
        Self { file, projects }
    }

    pub(crate) fn paths(&self) -> &[PathBuf] {
        self.projects.paths()
    }

    /// Records that the project in `directory` was just opened.
    pub(crate) fn add(&mut self, directory: &Path) {
        let directory = fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf());
        self.projects.add(directory);
        self.save();
    }

    pub(crate) fn remove(&mut self, directory: &Path) {
        self.projects.remove(directory);
        self.save();
    }

    fn save(&self) {
        let Some(file) = &self.file else {
            return;
        };
        if let Err(error) = write_list(&self.projects, file) {
            log::warn!("Cannot save the recent projects list: {error}");
        }
    }
}

fn read_list(file: &Path) -> RecentProjects {
    match fs::read_to_string(file) {
        Ok(text) => RecentProjects::from_toml(&text).unwrap_or_else(|error| {
            log::warn!("Ignoring corrupt {}: {}", file.display(), error.message());
            RecentProjects::default()
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => RecentProjects::default(),
        Err(error) => {
            log::warn!("Cannot read {}: {error}", file.display());
            RecentProjects::default()
        }
    }
}

fn write_list(projects: &RecentProjects, file: &Path) -> io::Result<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file, projects.to_toml().map_err(io::Error::other)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn newest_project_goes_first() {
        let mut recent = RecentProjects::default();

        recent.add("a".into());
        recent.add("b".into());

        assert_eq!(recent.paths(), paths(&["b", "a"]));
    }

    #[test]
    fn reopening_a_project_moves_it_to_the_top_without_duplicates() {
        let mut recent = RecentProjects::default();
        recent.add("a".into());
        recent.add("b".into());

        recent.add("a".into());

        assert_eq!(recent.paths(), paths(&["a", "b"]));
    }

    #[test]
    fn only_the_latest_projects_are_kept() {
        let mut recent = RecentProjects::default();

        for index in 0..=MAX_RECENT_PROJECTS {
            recent.add(PathBuf::from(index.to_string()));
        }

        assert_eq!(recent.paths().len(), MAX_RECENT_PROJECTS);
        assert_eq!(
            recent.paths()[0],
            PathBuf::from(MAX_RECENT_PROJECTS.to_string())
        );
    }

    #[test]
    fn remove_forgets_a_project() {
        let mut recent = RecentProjects::default();
        recent.add("a".into());

        recent.remove(Path::new("a"));

        assert_eq!(recent.paths(), [] as [PathBuf; 0]);
    }

    #[test]
    fn store_persists_between_sessions() {
        let folder = TestDirectory::new("recent_persists");
        let file = Some(folder.path().join(RECENT_PROJECTS_FILE_NAME));
        let project = folder.path().join("missing project");

        RecentProjectsStore::load_from(file.clone()).add(&project);
        let reloaded = RecentProjectsStore::load_from(file);

        assert_eq!(reloaded.paths(), [project]);
    }

    #[test]
    fn corrupt_file_gives_an_empty_list() {
        let folder = TestDirectory::new("recent_corrupt");
        let file = folder.path().join(RECENT_PROJECTS_FILE_NAME);
        fs::create_dir_all(folder.path()).expect("folder");
        fs::write(&file, "projects = 42").expect("written");

        let store = RecentProjectsStore::load_from(Some(file));

        assert_eq!(store.paths(), [] as [PathBuf; 0]);
    }
}
