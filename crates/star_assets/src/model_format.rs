//! The 3D model file formats the engine reads.

use star_core::{EngineError, EngineResult};

use crate::AssetPath;

/// A kind of model file, recognized by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModelFormat {
    /// glTF 2.0: `.glb` (one file) or `.gltf` (with the files it names
    /// beside it).
    Gltf,
}

impl ModelFormat {
    /// The format of the model at `path`, from its extension (in any case).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if the extension is not that of a
    /// supported model format.
    pub fn of(path: &AssetPath) -> EngineResult<Self> {
        let extension = path.extension().unwrap_or_default();
        if extension.eq_ignore_ascii_case("glb") || extension.eq_ignore_ascii_case("gltf") {
            Ok(Self::Gltf)
        } else {
            Err(EngineError::Asset(format!(
                "\"{path}\" is not a supported model: use .glb or .gltf"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gltf_files_are_models_in_any_case() {
        for path in ["hero.glb", "chars/HERO.GLTF"] {
            assert_eq!(
                ModelFormat::of(&AssetPath::new(path).unwrap()).unwrap(),
                ModelFormat::Gltf
            );
        }
        assert!(ModelFormat::of(&AssetPath::new("hero.obj").unwrap()).is_err());
    }
}
