//! The textures and meshes the view has made for the open scene, each
//! made once: the images of the project's assets folder and the meshes of
//! unit shapes.

use std::path::Path;

use star_engine::prelude::{AssetFolder, AssetPath, Mesh3dId, MeshShape, TextureFilter, TextureId};
use star_render::interop::OffscreenRenderer;

use crate::scene_editor::SceneGpu;

/// What was made, or `None` for what could not be: it is not tried again
/// every frame, and its warning is logged once.
pub(crate) struct ViewportAssets {
    folder: AssetFolder,
    textures: Vec<(AssetPath, TextureFilter, Option<TextureId>)>,
    meshes: Vec<(MeshShape, Option<Mesh3dId>)>,
}

impl ViewportAssets {
    /// Reads images from the `assets` folder.
    pub(crate) fn new(assets: &Path) -> Self {
        Self {
            folder: AssetFolder::new(assets),
            textures: Vec::new(),
            meshes: Vec::new(),
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
}
