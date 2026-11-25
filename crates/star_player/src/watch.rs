//! Noticing that the scripts of a project changed while it plays, so the
//! player reloads them (decision 76). It looks at the files from time to
//! time, as `star_assets`' `FileWatcher` does: that works the same on
//! every platform and needs no system service. Unlike that watcher, it
//! also notices new and removed files, which change the program too.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use star_engine::star_assets::DEFAULT_CHECK_INTERVAL;

use crate::project::program_files;

/// What the player knows of a file of the program: its path in the assets
/// folder, when it last changed and its length (two saves within the
/// clock's precision still differ in it, most of the time).
type Seen = (String, Option<SystemTime>, Option<u64>);

/// The scripts and blueprints of an assets folder, as last seen.
#[derive(Debug)]
pub(crate) struct Scripts {
    folder: PathBuf,
    seen: Vec<Seen>,
    since_last_look: Duration,
}

impl Scripts {
    /// Starts watching the program files of `folder`, as they are now.
    pub(crate) fn new(folder: PathBuf) -> Self {
        let seen = look(&folder).unwrap_or_default();
        Self {
            folder,
            seen,
            since_last_look: Duration::ZERO,
        }
    }

    /// Lets `seconds` pass; whether a file of the program changed, was
    /// added or was removed since it last looked, which it does every
    /// [`DEFAULT_CHECK_INTERVAL`]. A folder it cannot read now is looked
    /// at again later.
    pub(crate) fn changed(&mut self, seconds: f32) -> bool {
        self.since_last_look += Duration::from_secs_f32(seconds.max(0.0));
        if self.since_last_look < DEFAULT_CHECK_INTERVAL {
            return false;
        }
        self.since_last_look = Duration::ZERO;
        match look(&self.folder) {
            Some(seen) if seen != self.seen => {
                self.seen = seen;
                true
            }
            _ => false,
        }
    }
}

/// The program files of `folder`, sorted by path; `None` if it cannot be
/// read.
fn look(folder: &std::path::Path) -> Option<Vec<Seen>> {
    let mut paths = Vec::new();
    program_files(folder, "", &mut paths).ok()?;
    Some(
        paths
            .into_iter()
            .map(|path| {
                let metadata = fs::metadata(folder.join(&path)).ok();
                let modified = metadata.as_ref().and_then(|file| file.modified().ok());
                let length = metadata.map(|file| file.len());
                (path, modified, length)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A whole interval, in seconds.
    const LATER: f32 = 0.5;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join("star_player_watch").join(name);
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("enemies")).expect("folder");
        fs::write(folder.join("main.star"), "class Main extends Game {}\n").expect("file");
        folder
    }

    #[test]
    fn changed_new_and_removed_files_are_noticed_after_an_interval() {
        let folder = folder("changes");
        let mut scripts = Scripts::new(folder.clone());
        assert!(!scripts.changed(LATER), "nothing changed yet");

        fs::write(folder.join("main.star"), "class Main extends Game {\n}\n").expect("file");
        assert!(!scripts.changed(0.1), "before the interval");
        assert!(scripts.changed(LATER), "a changed file");
        assert!(!scripts.changed(LATER), "seen already");

        fs::write(folder.join("enemies/drone.blueprint"), "").expect("file");
        assert!(scripts.changed(LATER), "a new blueprint");

        fs::remove_file(folder.join("enemies/drone.blueprint")).expect("removed");
        assert!(scripts.changed(LATER), "a removed blueprint");

        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn other_files_are_not_the_program() {
        let folder = folder("others");
        let mut scripts = Scripts::new(folder.clone());

        fs::write(folder.join("level.scene"), "").expect("file");
        fs::write(folder.join(".main.star.swp"), "").expect("file");

        assert!(!scripts.changed(LATER));
        let _ = fs::remove_dir_all(&folder);
    }
}
