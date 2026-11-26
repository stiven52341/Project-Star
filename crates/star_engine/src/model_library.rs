//! The 3D models a game loaded from asset files (decision 83), uploaded
//! once each: their meshes and textures on the GPU, ready to be drawn.

use std::collections::HashMap;
use std::sync::Arc;

use star_assets::{AssetFolder, AssetPath};
use star_core::{EngineResult, Model};
use star_render::{Renderer, TextureFilter, upload_model};
use star_scene::ShownModel;

/// Remembers the models already uploaded, and those that failed, so a
/// model is read once and a broken file warns once.
#[derive(Debug, Default)]
pub(crate) struct ModelLibrary {
    loaded: HashMap<AssetPath, Option<Arc<ShownModel>>>,
}

impl ModelLibrary {
    /// The model at `path` of `folder`, read and given to `upload` the
    /// first time; `None` if it cannot be (warned once).
    pub(crate) fn load(
        &mut self,
        folder: &AssetFolder,
        path: &AssetPath,
        upload: impl FnOnce(&Model) -> EngineResult<ShownModel>,
    ) -> Option<Arc<ShownModel>> {
        if let Some(known) = self.loaded.get(path) {
            return known.clone();
        }
        let parts = folder
            .load_model(path)
            .and_then(|model| upload(&model))
            .map(Arc::new)
            .inspect_err(|error| log::warn!("Cannot show the model \"{path}\": {error}"))
            .ok();
        self.loaded.insert(path.clone(), parts.clone());
        parts
    }
}

/// Uploads the meshes and images of `model` to `renderer`.
///
/// # Errors
///
/// Why a mesh or an image could not be uploaded.
pub(crate) fn upload_to(renderer: &mut Renderer, model: &Model) -> EngineResult<ShownModel> {
    // Both closures need the renderer, one after the other.
    let renderer = std::cell::RefCell::new(renderer);
    let parts = upload_model(
        model,
        |image| {
            renderer
                .borrow_mut()
                .create_texture(image, TextureFilter::Smooth)
        },
        |mesh| renderer.borrow_mut().create_mesh_3d(mesh),
    )?;
    Ok(ShownModel::new(model, parts))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;

    use star_core::EngineError;

    use super::*;

    /// A temporary asset folder, deleted when dropped.
    struct Folder(PathBuf);

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn models_are_read_once_and_broken_ones_warn_once() {
        let root = std::env::temp_dir().join("star_engine_model_library");
        fs::create_dir_all(&root).unwrap();
        let folder = Folder(root);
        fs::write(folder.0.join("broken.glb"), b"not a model").unwrap();
        let assets = AssetFolder::new(&folder.0);
        let path = AssetPath::new("broken.glb").unwrap();
        let mut library = ModelLibrary::default();
        let uploads = Cell::new(0);

        for _ in 0..2 {
            let parts = library.load(&assets, &path, |_| {
                uploads.set(uploads.get() + 1);
                Err(EngineError::Rendering("never".to_owned()))
            });
            assert!(parts.is_none());
        }

        assert_eq!(uploads.get(), 0, "the file never decodes");
        assert_eq!(library.loaded.len(), 1, "the failure is remembered");
    }
}
