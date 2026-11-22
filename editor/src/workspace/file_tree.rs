//! Snapshot of a folder's contents, for the assets browser.

use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Folders deeper than this are listed empty; also protects against symlink loops.
const MAX_SCAN_DEPTH: usize = 16;

/// A file or a folder with its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileNode {
    File {
        name: String,
    },
    Folder {
        name: String,
        path: PathBuf,
        children: Vec<FileNode>,
    },
}

impl FileNode {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::File { name } | Self::Folder { name, .. } => name,
        }
    }

    const fn is_folder(&self) -> bool {
        matches!(self, Self::Folder { .. })
    }
}

/// Lists `directory` recursively: folders first, then files, alphabetically.
/// Hidden entries (starting with `.`) are skipped.
pub(crate) fn scan_directory(directory: &Path) -> io::Result<Vec<FileNode>> {
    scan(directory, 0)
}

fn scan(directory: &Path, depth: usize) -> io::Result<Vec<FileNode>> {
    let mut nodes = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let node = if entry.file_type()?.is_dir() {
            let path = entry.path();
            let children = if depth < MAX_SCAN_DEPTH {
                scan(&path, depth + 1)?
            } else {
                Vec::new()
            };
            FileNode::Folder {
                name,
                path,
                children,
            }
        } else {
            FileNode::File { name }
        };
        nodes.push(node);
    }
    nodes.sort_by(compare_nodes);
    Ok(nodes)
}

fn compare_nodes(first: &FileNode, second: &FileNode) -> Ordering {
    second.is_folder().cmp(&first.is_folder()).then_with(|| {
        first
            .name()
            .to_lowercase()
            .cmp(&second.name().to_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    #[test]
    fn lists_folders_first_then_files_alphabetically_skipping_hidden() {
        let root = TestDirectory::new("file_tree_order");
        fs::create_dir_all(root.path().join("sprites")).expect("folder");
        fs::write(root.path().join("b.png"), "").expect("file");
        fs::write(root.path().join("A.wav"), "").expect("file");
        fs::write(root.path().join(".hidden"), "").expect("file");
        fs::write(root.path().join("sprites").join("hero.png"), "").expect("file");

        let nodes = scan_directory(root.path()).expect("scanned");

        let names: Vec<&str> = nodes.iter().map(FileNode::name).collect();
        assert_eq!(names, ["sprites", "A.wav", "b.png"]);
        assert!(matches!(
            &nodes[0],
            FileNode::Folder { children, .. } if children.len() == 1
        ));
    }

    #[test]
    fn missing_folders_are_an_error() {
        let root = TestDirectory::new("file_tree_missing");

        assert!(scan_directory(root.path()).is_err());
    }
}
