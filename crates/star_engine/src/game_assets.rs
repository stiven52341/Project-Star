//! The assets of a running game: where they are read from and what is loaded.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use star_assets::{AssetFolder, AssetPath};
use star_core::{EngineResult, Sound};
use star_ecs::{Entity, World};
use star_math::Mat4;
use star_physics::ShapeKind3d;
use star_render::{Camera, Frame, Mesh3dId, MeshShape, Renderer, TextureFilter, TextureId};
use star_scene::{
    Body3D, CollisionMesh, GlobalTransform2D, GlobalTransform3D, MeshNode, ModelDrawing, ModelNode,
    ModelPose, Name, Scene, SceneAssets, SpriteNode, is_shown,
};

use crate::EngineWorld as _;
use crate::mesh_file::MeshFile;
use crate::model_library::{ModelLibrary, upload_to};
use crate::sound_library::SoundLibrary;
use crate::sound_players::{SoundPlayers, listener_of};
use crate::sprite_file::SpriteFile;
use crate::texture_library::TextureLibrary;

/// The asset folder of a game, the textures and sounds loaded from it, and
/// the meshes built from shapes.
pub(crate) struct GameAssets {
    folder: AssetFolder,
    textures: TextureLibrary,
    sounds: SoundLibrary,
    /// Every shape's mesh, built once; few shapes are used, so a list.
    meshes: Vec<(MeshShape, Mesh3dId)>,
    models: ModelLibrary,
    /// Where posed models are worked out, kept between frames.
    model_drawing: ModelDrawing,
    /// The nodes warned that their mesh body has nothing to take its
    /// triangles from.
    unshaped: HashSet<Entity>,
    sound_players: SoundPlayers,
}

impl GameAssets {
    /// The assets of `folder`, watched for changes if `hot_reload`.
    pub(crate) fn new(folder: AssetFolder, hot_reload: bool) -> Self {
        Self {
            folder,
            textures: TextureLibrary::new(hot_reload),
            sounds: SoundLibrary::default(),
            meshes: Vec::new(),
            models: ModelLibrary::default(),
            model_drawing: ModelDrawing::default(),
            unshaped: HashSet::new(),
            sound_players: SoundPlayers::default(),
        }
    }

    pub(crate) fn folder(&self) -> &AssetFolder {
        &self.folder
    }

    /// The texture of the image at `path`, loaded and uploaded the first time.
    pub(crate) fn load_texture(
        &mut self,
        renderer: &mut Renderer,
        path: AssetPath,
        filter: TextureFilter,
    ) -> EngineResult<TextureId> {
        let folder = &self.folder;
        self.textures.load(folder, path, filter, |path| {
            renderer.create_texture(&folder.load_image(path)?, filter)
        })
    }

    /// The mesh of `shape`, built and uploaded the first time.
    pub(crate) fn mesh(
        &mut self,
        renderer: &mut Renderer,
        shape: MeshShape,
    ) -> EngineResult<Mesh3dId> {
        if let Some((_, mesh)) = self.meshes.iter().find(|(built, _)| *built == shape) {
            return Ok(*mesh);
        }
        let mesh = renderer.create_mesh_3d(&shape.to_mesh_data()?)?;
        self.meshes.push((shape, mesh));
        Ok(mesh)
    }

    /// Starts the sound players of `world` that play on start, and stops
    /// those that are gone (see [`SoundPlayers::update`]).
    ///
    /// # Errors
    ///
    /// [`EngineError::Ecs`](star_core::EngineError::Ecs) if a running query
    /// holds the players.
    pub(crate) fn play_sound_players(
        &mut self,
        world: &World,
        camera: &Camera,
    ) -> EngineResult<()> {
        let audio = world.audio();
        let Self {
            folder,
            sounds,
            sound_players,
            ..
        } = self;
        sound_players.update(world, &audio, |player| {
            sounds.load(folder, player.sound.clone())
        })?;
        sound_players.place(world, &audio, &listener_of(camera));
        Ok(())
    }

    /// The sound at `path`, decoded the first time.
    pub(crate) fn load_sound(&mut self, path: AssetPath) -> EngineResult<Arc<Sound>> {
        self.sounds.load(&self.folder, path)
    }

    /// Reads the scene file at `path`, loading the textures and meshes its
    /// nodes show.
    pub(crate) fn load_scene(
        &mut self,
        renderer: &mut Renderer,
        path: &str,
    ) -> EngineResult<Scene> {
        let path = AssetPath::new(path)?;
        let text = self.folder.read_text(&path)?;
        let mut loader = SceneLoader {
            assets: self,
            renderer,
        };
        Scene::from_text(&text, &mut loader)
            .map_err(|error| star_core::EngineError::Scene(format!("\"{path}\": {error}")))
    }

    /// Adds to `frame` the sprite of every node with a [`SpriteFile`] and a
    /// global transform that is not hidden, loading the images it needs. An image that cannot
    /// be loaded logs a warning once, and its sprite is not drawn.
    pub(crate) fn draw_sprite_files(
        &mut self,
        world: &World,
        renderer: &mut Renderer,
        frame: &mut Frame,
    ) -> EngineResult<()> {
        world.each::<(&GlobalTransform2D, &mut SpriteFile)>(|entity, (global, file)| {
            if !is_shown(world, entity) {
                return;
            }
            let texture = file.texture(
                |path, filter| self.load_texture(renderer, path.clone(), filter),
                |error| log::warn!("{error}"),
            );
            if let Some(texture) = texture {
                let sprite = SpriteNode::new(texture, file.size)
                    .with_layer(file.layer)
                    .with_color(file.color)
                    .with_sheet(file.sheet);
                frame.draw_sprite(sprite.to_sprite(*global));
            }
        })
    }

    /// Adds to `frame` the mesh of every node with a [`MeshFile`] and a
    /// global 3D transform that is not hidden, building the meshes and loading the images it
    /// needs. A mesh that cannot be built logs a warning once, and is not
    /// drawn; an image that cannot be loaded logs one, and the mesh is
    /// drawn without it.
    pub(crate) fn draw_mesh_files(
        &mut self,
        world: &World,
        renderer: &mut Renderer,
        frame: &mut Frame,
    ) -> EngineResult<()> {
        world.each::<(&GlobalTransform3D, &mut MeshFile)>(|entity, (global, file)| {
            if !is_shown(world, entity) {
                return;
            }
            let built = file
                .unit_shape()
                .and_then(|(unit, scale)| Ok((self.mesh(renderer, unit)?, scale)));
            let (mesh, scale) = match built {
                Ok(built) => built,
                Err(error) => return file.reject(&error, |error| log::warn!("{error}")),
            };
            let material = file.material(
                |path, filter| self.load_texture(renderer, path.clone(), filter),
                |error| log::warn!("{error}"),
            );
            frame.draw_mesh_3d(mesh, material, global.matrix() * Mat4::from_scale(scale));
        })
    }

    /// Adds to `frame` every part of the model of each node with a
    /// [`ModelNode`] and a global 3D transform that is not hidden, reading
    /// and uploading each model the first time. A model that cannot be read
    /// logs a warning once, and is not drawn.
    pub(crate) fn draw_models(
        &mut self,
        world: &World,
        renderer: &mut Renderer,
        frame: &mut Frame,
    ) -> EngineResult<()> {
        world.each::<(&GlobalTransform3D, &ModelNode)>(|entity, (global, node)| {
            if !is_shown(world, entity) {
                return;
            }
            let Some(model) = self
                .models
                .load(&self.folder, &node.file, |model| upload_to(renderer, model))
            else {
                return;
            };
            let pose = world.get::<ModelPose>(entity);
            let clips = pose.as_ref().map_or(&[][..], |pose| pose.clips.as_slice());
            if let Err(error) = model.draw(global.matrix(), clips, &mut self.model_drawing, frame) {
                log::warn!("the model \"{}\" is not drawn whole: {error}", node.file);
            }
        })
    }

    /// Gives a [`CollisionMesh`] to every node whose 3D body has a mesh
    /// shape and that has none yet: the triangles of its [`ModelNode`]'s
    /// model (read and uploaded the first time), or of the shape of its
    /// [`MeshFile`] or [`MeshNode`]. A node that shows neither keeps waiting, warned once.
    ///
    /// # Errors
    ///
    /// [`EngineError::Ecs`](star_core::EngineError::Ecs) if a running query
    /// holds the components it uses.
    pub(crate) fn give_collision_meshes(
        &mut self,
        world: &mut World,
        renderer: &mut Renderer,
    ) -> EngineResult<()> {
        let mut waiting = Vec::new();
        world.each::<&Body3D>(|node, body| {
            if body.settings().shape.kind() == ShapeKind3d::Mesh
                && world.get::<CollisionMesh>(node).is_none()
            {
                waiting.push(node);
            }
        })?;
        for node in waiting {
            if let Some(mesh) = self.collision_mesh(world, node, renderer) {
                world.insert(node, mesh)?;
            } else if self.unshaped.insert(node) {
                log::warn!(
                    "the mesh body of \"{}\" waits for a mesh or a model to take its triangles from",
                    world
                        .get::<Name>(node)
                        .map_or_else(String::new, |name| name.0.clone())
                );
            }
        }
        Ok(())
    }

    /// The triangles of what `node` shows, if they are known.
    fn collision_mesh(
        &mut self,
        world: &World,
        node: Entity,
        renderer: &mut Renderer,
    ) -> Option<CollisionMesh> {
        if let Some(file) = world.get::<ModelNode>(node).map(|model| model.file.clone()) {
            let model = self
                .models
                .load(&self.folder, &file, |model| upload_to(renderer, model))?;
            return model.collision.clone();
        }
        let shape = if let Some(file) = world.get::<MeshFile>(node) {
            file.shape
        } else {
            let mesh = world.get::<MeshNode>(node)?.mesh;
            self.meshes.iter().find(|(_, built)| *built == mesh)?.0
        };
        shape
            .to_mesh_data()
            .and_then(|data| CollisionMesh::from_mesh(&data))
            .inspect_err(|error| log::warn!("{error}"))
            .ok()
    }

    /// Uploads again the textures whose files changed. A file that cannot be
    /// loaded, e.g. one an image editor is still writing, only logs a warning:
    /// the texture keeps its last image until the file changes again.
    pub(crate) fn reload_changed_textures(&mut self, renderer: &mut Renderer, elapsed: Duration) {
        let folder = &self.folder;
        self.textures.reload_changed(elapsed, |texture| {
            let reloaded = folder
                .load_image(&texture.path)
                .and_then(|image| renderer.replace_texture(texture.id, &image, texture.filter));
            match reloaded {
                Ok(()) => log::info!("Reloaded \"{}\"", texture.path),
                Err(error) => log::warn!("Cannot reload \"{}\": {error}", texture.path),
            }
        });
    }
}

/// Loads what the nodes of a scene show into the game's assets.
struct SceneLoader<'a> {
    assets: &'a mut GameAssets,
    renderer: &'a mut Renderer,
}

impl SceneAssets for SceneLoader<'_> {
    fn texture(&mut self, path: &AssetPath, filter: TextureFilter) -> EngineResult<TextureId> {
        self.assets
            .load_texture(self.renderer, path.clone(), filter)
    }

    fn mesh(&mut self, shape: MeshShape) -> EngineResult<Mesh3dId> {
        self.assets.mesh(self.renderer, shape)
    }
}
