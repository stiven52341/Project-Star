//! The sounds a game loaded from asset files.

use std::collections::HashMap;
use std::sync::Arc;

use star_assets::{AssetFolder, AssetPath};
use star_core::{EngineResult, Sound};

/// Remembers which sound files are already decoded, so loading one twice
/// gives the same sound in memory instead of reading it again.
#[derive(Debug, Default)]
pub(crate) struct SoundLibrary {
    loaded: HashMap<AssetPath, Arc<Sound>>,
}

impl SoundLibrary {
    /// The sound of the file at `path` in `folder`, decoded the first time.
    /// A file that cannot be loaded is not remembered: it is tried again.
    pub(crate) fn load(
        &mut self,
        folder: &AssetFolder,
        path: AssetPath,
    ) -> EngineResult<Arc<Sound>> {
        if let Some(sound) = self.loaded.get(&path) {
            return Ok(Arc::clone(sound));
        }
        let sound = Arc::new(folder.load_sound(&path)?);
        self.loaded.insert(path, Arc::clone(&sound));
        Ok(sound)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    /// A WAV file of one silent mono frame at 8 kHz.
    fn tiny_wav() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&38_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1_u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&8000_u32.to_le_bytes());
        bytes.extend_from_slice(&16_000_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes
    }

    fn temporary_folder(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("star_engine_sounds_{name}_{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn sounds_are_decoded_once() {
        let root = temporary_folder("once");
        fs::write(root.join("beep.wav"), tiny_wav()).unwrap();
        let folder = AssetFolder::new(&root);
        let mut library = SoundLibrary::default();
        let path = AssetPath::new("beep.wav").unwrap();

        let first = library.load(&folder, path.clone()).unwrap();
        fs::remove_file(root.join("beep.wav")).unwrap();
        let second = library.load(&folder, path).unwrap();

        assert!(Arc::ptr_eq(&first, &second));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_loads_are_tried_again() {
        let root = temporary_folder("again");
        let folder = AssetFolder::new(&root);
        let mut library = SoundLibrary::default();
        let path = AssetPath::new("late.wav").unwrap();

        assert!(library.load(&folder, path.clone()).is_err());

        fs::write(root.join("late.wav"), tiny_wav()).unwrap();

        assert!(library.load(&folder, path).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
}
