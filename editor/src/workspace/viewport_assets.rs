//! The textures and meshes the view has made for the open scene, each
//! made once: the images of the project's assets folder, the meshes of
//! unit shapes and the 3D models.

use std::path::Path;
use std::sync::Arc;

use star_engine::prelude::{AssetFolder, AssetPath, Mesh3dId, MeshShape, TextureFilter, TextureId};
use star_engine::star_scene::ShownModel;
use star_render::interop::OffscreenRenderer;
use star_render::upload_model;

use crate::scene_editor::SceneGpu;

/// What was made, or `None` for what could not be: it is not tried again
/// every frame, and its warning is logged once.
pub(crate) struct ViewportAssets {
    folder: AssetFolder,
    textures: Vec<(AssetPath, TextureFilter, Option<TextureId>)>,
    meshes: Vec<(MeshShape, Option<Mesh3dId>)>,
    models: Vec<(AssetPath, Option<Arc<ShownModel>>)>,
}

impl ViewportAssets {
    /// Reads images from the `assets` folder.
    pub(crate) fn new(assets: &Path) -> Self {
        Self {
            folder: AssetFolder::new(assets),
            textures: Vec::new(),
            meshes: Vec::new(),
            models: Vec::new(),
        }
    }

    /// Makes what is missing with `renderer`.
    pub(crate) fn with<'assets>(
        &'assets mut self,
        renderer: &'assets mut OffscreenRenderer,
    ) -> impl SceneGpu + 'assets {
        Loader {
            assets: self,
            renderer,
        }
    }
}

struct Loader<'assets> {
    assets: &'assets mut ViewportAssets,
    renderer: &'assets mut OffscreenRenderer,
}

impl SceneGpu for Loader<'_> {
    fn texture(&mut self, path: &AssetPath, filter: TextureFilter) -> Option<TextureId> {
        let textures = &mut self.assets.textures;
        if let Some((_, _, texture)) = textures
            .iter()
            .find(|(loaded, used, _)| loaded == path && *used == filter)
        {
            return *texture;
        }
        let texture = self
            .assets
            .folder
            .load_image(path)
            .and_then(|image| self.renderer.create_texture(&image, filter))
            .inspect_err(|error| log::warn!("{error}"))
            .ok();
        textures.push((path.clone(), filter, texture));
        texture
    }

    fn mesh(&mut self, unit: MeshShape) -> Option<Mesh3dId> {
        let meshes = &mut self.assets.meshes;
        if let Some((_, mesh)) = meshes.iter().find(|(built, _)| *built == unit) {
            return *mesh;
        }
        let mesh = unit
            .to_mesh_data()
            .and_then(|data| self.renderer.create_mesh_3d(&data))
            .inspect_err(|error| log::warn!("{error}"))
            .ok();
        meshes.push((unit, mesh));
        mesh
    }

    fn model(&mut self, file: &AssetPath) -> Option<Arc<ShownModel>> {
        if let Some((_, shown)) = self.assets.models.iter().find(|(read, _)| read == file) {
            return shown.clone();
        }
        // Both closures need the renderer, one after the other.
        let renderer = std::cell::RefCell::new(&mut *self.renderer);
        let shown = self
            .assets
            .folder
            .load_model(file)
            .and_then(|model| {
                upload_model(
                    &model,
                    |image| {
                        renderer
                            .borrow_mut()
                            .create_texture(image, TextureFilter::Smooth)
                    },
                    |mesh| renderer.borrow_mut().create_mesh_3d(mesh),
                )
                .map(|parts| Arc::new(ShownModel::new(&model, parts)))
            })
            .inspect_err(|error| log::warn!("{error}"))
            .ok();
        self.assets.models.push((file.clone(), shown.clone()));
        shown
    }
}
