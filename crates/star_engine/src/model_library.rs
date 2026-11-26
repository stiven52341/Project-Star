//! The 3D models a game loaded from asset files (decision 83), uploaded
//! once each: their meshes and textures on the GPU, ready to be drawn.

use std::collections::HashMap;
use std::sync::Arc;

use star_assets::{AssetFolder, AssetPath};
use star_core::{EngineResult, Model};
use star_math::Mat4;
use star_render::{Color, Material, Mesh3dId, MeshData, Renderer, TextureFilter, TextureId};

/// A mesh of a model on the GPU, with its material and place in the model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShownPart {
    pub(crate) mesh: Mesh3dId,
    pub(crate) material: Material,
    pub(crate) transform: Mat4,
}

/// Remembers the models already uploaded, and those that failed, so a
/// model is read once and a broken file warns once.
#[derive(Debug, Default)]
pub(crate) struct ModelLibrary {
    loaded: HashMap<AssetPath, Option<Arc<[ShownPart]>>>,
}

impl ModelLibrary {
    /// The parts of the model at `path` of `folder`, read and given to
    /// `upload` the first time; `None` if it cannot be (warned once).
    pub(crate) fn load(
        &mut self,
        folder: &AssetFolder,
        path: &AssetPath,
        upload: impl FnOnce(&Model) -> EngineResult<Vec<ShownPart>>,
    ) -> Option<Arc<[ShownPart]>> {
        if let Some(known) = self.loaded.get(path) {
            return known.clone();
        }
        let parts = folder
            .load_model(path)
            .and_then(|model| upload(&model))
            .map(Arc::from)
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
pub(crate) fn upload_model(renderer: &mut Renderer, model: &Model) -> EngineResult<Vec<ShownPart>> {
    let textures = model
        .images()
        .iter()
        .map(|image| renderer.create_texture(image, TextureFilter::Smooth))
        .collect::<EngineResult<Vec<TextureId>>>()?;
    model
        .parts()
        .iter()
        .map(|part| {
            let [r, g, b, a] = part.material.base_color;
            Ok(ShownPart {
                mesh: renderer.create_mesh_3d(&MeshData::from_model_part(part)?)?,
                material: Material {
                    base_color: Color::from_linear(Color::rgba(r, g, b, a)),
                    base_color_texture: part
                        .material
                        .texture
                        .and_then(|index| textures.get(index).copied()),
                    metallic: part.material.metallic,
                    roughness: part.material.roughness,
                },
                transform: Mat4::from_columns(part.transform),
            })
        })
        .collect()
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
