//! Noticing when asset files change, to reload them while the game runs.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How often a [`FileWatcher`] looks at its files by default: fast enough to
/// feel immediate after saving a file, rare enough to cost nothing.
pub const DEFAULT_CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// Watches files by asking for their modification time from time to time,
/// which works the same on every platform and needs no system service.
///
/// Each file is known by a key of type `K`, e.g. the texture it was loaded into.
#[derive(Debug)]
pub struct FileWatcher<K> {
    files: Vec<WatchedFile<K>>,
    check_interval: Duration,
    since_last_check: Duration,
}

#[derive(Debug)]
struct WatchedFile<K> {
    key: K,
    file: PathBuf,
    /// `None` while the file cannot be read, e.g. while an editor rewrites it.
    modified: Option<SystemTime>,
}

impl<K> FileWatcher<K> {
    /// A watcher without files, that looks at them every `check_interval`.
    #[must_use]
    pub fn new(check_interval: Duration) -> Self {
        Self {
            files: Vec::new(),
            check_interval,
            since_last_check: Duration::ZERO,
        }
    }

    /// Starts watching `file`, known as `key`: from now on, changing it is
    /// reported.
    pub fn watch(&mut self, key: K, file: impl Into<PathBuf>) {
        let file = file.into();
        let modified = modification_time(&file);
        self.files.push(WatchedFile {
            key,
            file,
            modified,
        });
    }

    /// Number of files watched.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Lets `elapsed` time pass; when the check interval has passed, calls
    /// `on_change` with the key of every file modified since it was last seen.
    /// Removed files are not reported, but they are when they come back.
    pub fn poll(&mut self, elapsed: Duration, mut on_change: impl FnMut(&K)) {
        self.since_last_check += elapsed;
        if self.since_last_check < self.check_interval {
            return;
        }
        self.since_last_check = Duration::ZERO;
        for watched in &mut self.files {
            let modified = modification_time(&watched.file);
            if modified.is_some() && modified != watched.modified {
                on_change(&watched.key);
            }
            watched.modified = modified;
        }
    }
}

impl<K> Default for FileWatcher<K> {
    fn default() -> Self {
        Self::new(DEFAULT_CHECK_INTERVAL)
    }
}

fn modification_time(file: &Path) -> Option<SystemTime> {
    std::fs::metadata(file)
        .and_then(|metadata| metadata.modified())
        .ok()
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;
    use crate::asset_folder::test_support::TemporaryFolder;

    const INTERVAL: Duration = Duration::from_millis(100);

    /// Gives `file` a modification time different from any it had.
    fn touch(file: &Path, seconds_after_epoch: u64) {
        File::options()
            .write(true)
            .open(file)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds_after_epoch))
            .unwrap();
    }

    fn changes(watcher: &mut FileWatcher<&'static str>, elapsed: Duration) -> Vec<&'static str> {
        let mut changed = Vec::new();
        watcher.poll(elapsed, |&key| changed.push(key));
        changed
    }

    #[test]
    fn modified_files_are_reported_once() {
        let folder = TemporaryFolder::new("watch_modified");
        folder.write("hero.png", b"old");
        folder.write("enemy.png", b"old");
        let mut watcher = FileWatcher::new(INTERVAL);
        watcher.watch("hero", folder.0.root().join("hero.png"));
        watcher.watch("enemy", folder.0.root().join("enemy.png"));

        touch(&folder.0.root().join("hero.png"), 1);

        assert_eq!(changes(&mut watcher, INTERVAL), ["hero"]);
        assert_eq!(changes(&mut watcher, INTERVAL), Vec::<&str>::new());
        assert_eq!(watcher.file_count(), 2);
    }

    #[test]
    fn files_are_only_checked_once_the_interval_passed() {
        let folder = TemporaryFolder::new("watch_interval");
        folder.write("hero.png", b"old");
        let mut watcher = FileWatcher::new(INTERVAL);
        watcher.watch("hero", folder.0.root().join("hero.png"));
        touch(&folder.0.root().join("hero.png"), 1);

        assert_eq!(changes(&mut watcher, INTERVAL / 2), Vec::<&str>::new());
        assert_eq!(changes(&mut watcher, INTERVAL / 2), ["hero"]);
    }

    #[test]
    fn removed_files_are_reported_when_they_come_back() {
        let folder = TemporaryFolder::new("watch_removed");
        folder.write("hero.png", b"old");
        let file = folder.0.root().join("hero.png");
        let mut watcher = FileWatcher::new(INTERVAL);
        watcher.watch("hero", &file);

        std::fs::remove_file(&file).unwrap();
        assert_eq!(changes(&mut watcher, INTERVAL), Vec::<&str>::new());

        folder.write("hero.png", b"new");
        touch(&file, 1);
        assert_eq!(changes(&mut watcher, INTERVAL), ["hero"]);
    }
}
