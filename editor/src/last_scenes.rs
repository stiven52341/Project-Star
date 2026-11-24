//! The scene each project had open, remembered between editor sessions so
//! opening a project shows it again.
//!
//! It is kept in the user's configuration folder rather than in the
//! project: it is the editor's state, not part of the game.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config_file::{config_file, read_or_default, write};

/// How many projects the list remembers a scene for.
const MAX_REMEMBERED_PROJECTS: usize = 50;
const LAST_SCENES_FILE_NAME: &str = "last_scenes.toml";

/// The scene a project had open: its folder and the scene's path inside
/// its assets folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LastScene {
    project: PathBuf,
    scene: String,
}

/// The last scene of each project, most recently opened first, one per
/// project.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct LastScenes {
    scenes: Vec<LastScene>,
}

impl LastScenes {
    /// The scene `project` had open, if one is remembered.
    fn of(&self, project: &Path) -> Option<&str> {
        self.scenes
            .iter()
            .find(|last| last.project == project)
            .map(|last| last.scene.as_str())
    }

    /// Remembers `scene` as the one `project` has open, forgetting the
    /// oldest projects beyond the limit.
    fn set(&mut self, project: PathBuf, scene: String) {
        self.forget(&project);
        self.scenes.insert(0, LastScene { project, scene });
        self.scenes.truncate(MAX_REMEMBERED_PROJECTS);
    }

    fn forget(&mut self, project: &Path) {
        self.scenes.retain(|last| last.project != project);
    }
}

/// The last scene of one project, kept in sync with the file shared by
/// every project.
///
/// Losing it is harmless, so write failures are only logged.
pub(crate) struct LastSceneStore {
    file: Option<PathBuf>,
    project: PathBuf,
}

impl LastSceneStore {
    /// The store of the project in `directory`, in the user's
    /// configuration folder.
    pub(crate) fn new(directory: &Path) -> Self {
        Self::with_file(config_file(LAST_SCENES_FILE_NAME), directory)
    }

    fn with_file(file: Option<PathBuf>, directory: &Path) -> Self {
        let project = fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf());
        Self { file, project }
    }

    /// The scene the project had open when it was last used, if it still
    /// exists inside `assets`.
    pub(crate) fn scene(&self, assets: &Path) -> Option<String> {
        let scenes = self.read();
        let scene = scenes.of(&self.project)?;
        assets.join(scene).is_file().then(|| scene.to_owned())
    }

    /// Remembers that the project has `scene` open now.
    pub(crate) fn remember(&self, scene: &str) {
        let mut scenes = self.read();
        if scenes.of(&self.project) == Some(scene) {
            return;
        }
        scenes.set(self.project.clone(), scene.to_owned());
        self.write(&scenes);
    }

    /// Forgets the scene of the project: none is open.
    pub(crate) fn forget(&self) {
        let mut scenes = self.read();
        if scenes.of(&self.project).is_none() {
            return;
        }
        scenes.forget(&self.project);
        self.write(&scenes);
    }

    /// Read again each time, so several editors open at once do not undo
    /// each other's projects.
    fn read(&self) -> LastScenes {
        self.file
            .as_deref()
            .map(read_or_default)
            .unwrap_or_default()
    }

    fn write(&self, scenes: &LastScenes) {
        let Some(file) = &self.file else {
            return;
        };
        if let Err(error) = write(scenes, file) {
            log::warn!("Cannot save the last scene of the project: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    #[test]
    fn each_project_keeps_its_own_scene() {
        let mut scenes = LastScenes::default();

        scenes.set("a".into(), "main.scene".to_owned());
        scenes.set("b".into(), "levels/one.scene".to_owned());
        scenes.set("a".into(), "menu.scene".to_owned());

        assert_eq!(scenes.of(Path::new("a")), Some("menu.scene"));
        assert_eq!(scenes.of(Path::new("b")), Some("levels/one.scene"));
        assert_eq!(scenes.scenes.len(), 2);
    }

    #[test]
    fn only_the_latest_projects_are_kept() {
        let mut scenes = LastScenes::default();

        for index in 0..=MAX_REMEMBERED_PROJECTS {
            scenes.set(PathBuf::from(index.to_string()), "main.scene".to_owned());
        }

        assert_eq!(scenes.scenes.len(), MAX_REMEMBERED_PROJECTS);
        assert_eq!(scenes.of(Path::new("0")), None);
    }

    #[test]
    fn the_scene_is_remembered_between_sessions() {
        let folder = TestDirectory::new("last_scene_persists");
        let assets = folder.path().join("assets");
        fs::create_dir_all(assets.join("levels")).unwrap();
        fs::write(assets.join("levels/one.scene"), "").unwrap();
        let file = Some(folder.path().join(LAST_SCENES_FILE_NAME));

        LastSceneStore::with_file(file.clone(), folder.path()).remember("levels/one.scene");
        let reopened = LastSceneStore::with_file(file, folder.path());

        assert_eq!(reopened.scene(&assets).as_deref(), Some("levels/one.scene"));
    }

    #[test]
    fn scenes_gone_or_forgotten_are_not_opened() {
        let folder = TestDirectory::new("last_scene_gone");
        let assets = folder.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("main.scene"), "").unwrap();
        let store =
            LastSceneStore::with_file(Some(folder.path().join(LAST_SCENES_FILE_NAME)), &assets);

        store.remember("deleted.scene");
        assert_eq!(store.scene(&assets), None);

        store.remember("main.scene");
        store.forget();
        assert_eq!(store.scene(&assets), None);
    }
}
