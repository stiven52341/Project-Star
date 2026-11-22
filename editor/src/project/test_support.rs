//! Helpers shared by the tests that touch the file system.

use std::fs;
use std::path::{Path, PathBuf};

/// Folder unique to one test, empty at start and removed when dropped.
pub(crate) struct TestDirectory(PathBuf);

impl TestDirectory {
    pub(crate) fn new(test_name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("star_editor_{test_name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
