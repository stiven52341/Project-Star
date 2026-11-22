//! Native "choose folder" dialog.

use std::path::{Path, PathBuf};

/// Asks the user to choose a folder, starting at `start` if it exists.
/// Returns `None` if the dialog is cancelled.
///
/// Blocks until the dialog closes, like any modal dialog.
pub(crate) fn pick_folder(title: &str, start: &Path) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    if start.is_dir() {
        dialog = dialog.set_directory(start);
    }
    dialog.pick_folder()
}
