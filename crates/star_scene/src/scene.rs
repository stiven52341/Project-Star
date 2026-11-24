//! Scenes: trees of nodes saved in files and instanced into worlds.

use std::collections::BTreeMap;

use star_assets::AssetPath;
use star_core::{EngineError, EngineResult};
use star_ecs::{Entity, World};
use star_physics::{BodySettings, Shape3d};
use star_render::{Material, Mesh3dId, MeshShape, TextureFilter, TextureId};

use crate::node_3d_data::MeshOutline;
use crate::scene_outline::{NodeOutline, SceneOutline, SpriteOutline};
use crate::{
    Body2D, Body3D, Camera2dNode, Camera3dNode, Children, FieldValue, Hierarchy, Light2dNode,
    LightNode, MeshNode, Name, SpriteNode, Tags, Transform2D, Transform3D,
};

/// A tree of nodes that can be added to a world as many times as needed:
/// a level, an enemy, a bullet. Read from the text of a `.scene` file, or
/// captured from a world to be saved in one.
///
/// It keeps, for each node, its name, its [`Tags`], its [`Transform2D`], its
/// [`SpriteNode`], its [`Body2D`], its [`Transform3D`], its [`MeshNode`],
/// its [`LightNode`], its [`Light2dNode`], its [`Body3D`] and its cameras ([`Camera2dNode`],
/// [`Camera3dNode`]); other components are not part of
/// scenes yet.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Parents always come before their children; the first node is the root.
    nodes: Vec<SceneNode>,
}

#[derive(Debug, Clone, PartialEq)]
struct SceneNode {
    name: String,
    /// Kept so a scene read and written again does not lose it; worlds
    /// have no use for it.
    class: Option<String>,
    tags: Vec<String>,
    parent: Option<usize>,
    transform: Option<Transform2D>,
    sprite: Option<SpriteNode>,
    body: Option<BodySettings>,
    transform_3d: Option<Transform3D>,
    mesh: Option<MeshNode>,
    light: Option<LightNode>,
    body_3d: Option<BodySettings<Shape3d>>,
    camera: Option<Camera2dNode>,
    camera_3d: Option<Camera3dNode>,
    light_2d: Option<Light2dNode>,
    /// Kept, as `class` is, for the scene to be written again whole.
    fields: BTreeMap<String, FieldValue>,
}

/// Loads what the nodes of a scene show, so a [`Scene`] can be read.
pub trait SceneAssets {
    /// The texture of the image at `path` of the asset folder, drawn with
    /// `filter`.
    ///
    /// # Errors
    ///
    /// Whatever keeps the texture from being loaded.
    fn texture(&mut self, path: &AssetPath, filter: TextureFilter) -> EngineResult<TextureId>;

    /// The 3D mesh of `shape`.
    ///
    /// # Errors
    ///
    /// Whatever keeps the mesh from being built.
    fn mesh(&mut self, shape: MeshShape) -> EngineResult<Mesh3dId>;
}

/// Tells where what the nodes of a [`Scene`] show came from, so it can be
/// saved.
pub trait SceneSources {
    /// The image and filter `texture` was loaded from, if it came from a
    /// file.
    fn texture_source(&self, texture: TextureId) -> Option<(AssetPath, TextureFilter)>;

    /// The shape `mesh` was built from, if it came from one.
    fn mesh_source(&self, mesh: Mesh3dId) -> Option<MeshShape>;
}

impl Scene {
    /// Reads a scene from the text of a `.scene` file. `assets` gives the
    /// textures and meshes its nodes show.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if the text is not a valid scene (bad
    /// TOML, another format version, a missing root or a node before its
    /// parent, an invalid texture path or shape), naming the node when it is
    /// one's, and the error of `assets` if something cannot be loaded.
    pub fn from_text(text: &str, assets: &mut impl SceneAssets) -> EngineResult<Self> {
        let nodes = SceneOutline::from_text(text)?
            .into_nodes()
            .into_iter()
            .map(|node| load_node(node, assets))
            .collect::<EngineResult<_>>()?;
        Ok(Self { nodes })
    }

    /// Writes the scene as the text of a `.scene` file. `sources` tells
    /// which image each texture was loaded from and which shape each mesh
    /// was built from.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`], naming the node, if a node shows a
    /// texture or a mesh that `sources` does not know (one made in code).
    pub fn to_text(&self, sources: &impl SceneSources) -> EngineResult<String> {
        let nodes = self
            .nodes
            .iter()
            .map(|node| outline_node(node, sources))
            .collect::<EngineResult<_>>()?;
        SceneOutline::from_nodes(nodes).to_text()
    }

    /// The scene made of `root` and its whole subtree in `world`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if `root` is not alive.
    pub fn capture(world: &World, root: Entity) -> EngineResult<Self> {
        if !world.is_alive(root) {
            return Err(EngineError::Scene(format!(
                "cannot capture {root:?}: it is not alive"
            )));
        }
        let mut nodes = Vec::new();
        let mut pending = vec![(root, None)];
        while let Some((entity, parent)) = pending.pop() {
            let index = nodes.len();
            nodes.push(capture_node(world, entity, parent));
            if let Some(children) = world.get::<Children>(entity) {
                // Reversed, so they come out of the stack in order.
                pending.extend(
                    children
                        .as_slice()
                        .iter()
                        .rev()
                        .map(|&child| (child, Some(index))),
                );
            }
        }
        Ok(Self { nodes })
    }

    /// Adds a copy of the scene's nodes to `world`, under `parent` (or as a
    /// new root), and returns the copy of the root node.
    ///
    /// # Errors
    ///
    /// Returns an error if `parent` is not alive or there is no room for
    /// more entities; then nothing of the scene stays in the world.
    pub fn instantiate(&self, world: &mut World, parent: Option<Entity>) -> EngineResult<Entity> {
        let mut entities = Vec::with_capacity(self.nodes.len());
        let added = self.add_nodes(world, parent, &mut entities);
        if added.is_err()
            && let Some(&root) = entities.first()
        {
            world.despawn_node(root);
        }
        added?;
        entities
            .first()
            .copied()
            .ok_or_else(|| EngineError::Scene("the scene has no nodes".to_owned()))
    }

    fn add_nodes(
        &self,
        world: &mut World,
        parent: Option<Entity>,
        entities: &mut Vec<Entity>,
    ) -> EngineResult<()> {
        for node in &self.nodes {
            let node_parent = match node.parent {
                Some(index) => entities.get(index).copied(),
                None => parent,
            };
            let entity = world.spawn_node(&node.name, node_parent)?;
            entities.push(entity);
            if !node.tags.is_empty() {
                world.insert(entity, Tags(node.tags.clone()))?;
            }
            if let Some(transform) = node.transform {
                world.insert(entity, transform)?;
            }
            if let Some(sprite) = node.sprite {
                world.insert(entity, sprite)?;
            }
            if let Some(body) = node.body {
                world.insert(entity, Body2D::new(body))?;
            }
            if let Some(transform) = node.transform_3d {
                world.insert(entity, transform)?;
            }
            if let Some(mesh) = node.mesh {
                world.insert(entity, mesh)?;
            }
            if let Some(light) = node.light {
                world.insert(entity, light)?;
            }
            if let Some(body) = node.body_3d {
                world.insert(entity, Body3D::new(body))?;
            }
            if let Some(camera) = node.camera {
                world.insert(entity, camera)?;
            }
            if let Some(camera) = node.camera_3d {
                world.insert(entity, camera)?;
            }
            if let Some(light) = node.light_2d {
                world.insert(entity, light)?;
            }
        }
        Ok(())
    }

    /// Number of nodes, root included.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// The node of `outline`, with its images and mesh loaded.
fn load_node(outline: NodeOutline, assets: &mut impl SceneAssets) -> EngineResult<SceneNode> {
    let named =
        |error: EngineError| EngineError::Scene(format!("node \"{}\": {error}", outline.name));
    let sprite = outline
        .sprite
        .as_ref()
        .map(|sprite| {
            let texture = assets.texture(&sprite.texture, sprite.filter)?;
            Ok(SpriteNode {
                texture,
                region: sprite.region,
                size: sprite.size,
                anchor: sprite.anchor,
                color: sprite.color,
                layer: sprite.layer,
            })
        })
        .transpose()
        .map_err(named)?;
    let mesh = outline
        .mesh
        .as_ref()
        .map(|mesh| load_mesh(mesh, assets))
        .transpose()
        .map_err(named)?;
    Ok(SceneNode {
        name: outline.name,
        class: outline.class,
        tags: outline.tags,
        parent: outline.parent,
        transform: outline.transform,
        sprite,
        body: outline.body,
        transform_3d: outline.transform_3d,
        mesh,
        light: outline.light,
        body_3d: outline.body_3d,
        camera: outline.camera,
        camera_3d: outline.camera_3d,
        light_2d: outline.light_2d,
        fields: outline.fields,
    })
}

fn load_mesh(mesh: &MeshOutline, assets: &mut impl SceneAssets) -> EngineResult<MeshNode> {
    let texture = mesh
        .texture
        .as_ref()
        .map(|(path, filter)| assets.texture(path, *filter))
        .transpose()?;
    let material = Material {
        base_color: mesh.color,
        base_color_texture: texture,
        metallic: mesh.metallic,
        roughness: mesh.roughness,
    };
    Ok(MeshNode::new(assets.mesh(mesh.shape)?, material))
}

/// The outline of `node`, naming by file what it shows.
fn outline_node(node: &SceneNode, sources: &impl SceneSources) -> EngineResult<NodeOutline> {
    let named = |error: EngineError| EngineError::Scene(format!("node \"{}\": {error}", node.name));
    let sprite = node
        .sprite
        .map(|sprite| outline_sprite(sprite, sources))
        .transpose()
        .map_err(named)?;
    let mesh = node
        .mesh
        .map(|mesh| outline_mesh(mesh, sources))
        .transpose()
        .map_err(named)?;
    Ok(NodeOutline {
        name: node.name.clone(),
        class: node.class.clone(),
        tags: node.tags.clone(),
        parent: node.parent,
        transform: node.transform,
        sprite,
        body: node.body,
        transform_3d: node.transform_3d,
        mesh,
        light: node.light,
        body_3d: node.body_3d,
        camera: node.camera,
        camera_3d: node.camera_3d,
        light_2d: node.light_2d,
        fields: node.fields.clone(),
    })
}

fn outline_sprite(sprite: SpriteNode, sources: &impl SceneSources) -> EngineResult<SpriteOutline> {
    let (texture, filter) = sources
        .texture_source(sprite.texture)
        .ok_or_else(|| not_from_a_file("texture"))?;
    Ok(SpriteOutline {
        texture,
        filter,
        size: sprite.size,
        region: sprite.region,
        anchor: sprite.anchor,
        color: sprite.color,
        layer: sprite.layer,
    })
}

fn outline_mesh(mesh: MeshNode, sources: &impl SceneSources) -> EngineResult<MeshOutline> {
    let shape = sources
        .mesh_source(mesh.mesh)
        .ok_or_else(|| not_from_a_file("mesh"))?;
    let material = mesh.material;
    let texture = material
        .base_color_texture
        .map(|texture| {
            sources
                .texture_source(texture)
                .ok_or_else(|| not_from_a_file("texture"))
        })
        .transpose()?;
    Ok(MeshOutline {
        shape,
        color: material.base_color,
        metallic: material.metallic,
        roughness: material.roughness,
        texture,
    })
}

fn not_from_a_file(what: &str) -> EngineError {
    EngineError::Scene(format!(
        "its {what} was not loaded from a file, so it cannot be saved"
    ))
}

fn capture_node(world: &World, entity: Entity, parent: Option<usize>) -> SceneNode {
    SceneNode {
        name: world
            .get::<Name>(entity)
            .map(|name| name.0.clone())
            .unwrap_or_default(),
        class: None,
        tags: world
            .get::<Tags>(entity)
            .map(|tags| tags.0.clone())
            .unwrap_or_default(),
        parent,
        transform: world.get::<Transform2D>(entity).map(|transform| *transform),
        sprite: world.get::<SpriteNode>(entity).map(|sprite| *sprite),
        body: world.get::<Body2D>(entity).map(|body| *body.settings()),
        transform_3d: world.get::<Transform3D>(entity).map(|transform| *transform),
        mesh: world.get::<MeshNode>(entity).map(|mesh| *mesh),
        light: world.get::<LightNode>(entity).map(|light| *light),
        body_3d: world.get::<Body3D>(entity).map(|body| *body.settings()),
        camera: world.get::<Camera2dNode>(entity).map(|camera| *camera),
        camera_3d: world.get::<Camera3dNode>(entity).map(|camera| *camera),
        light_2d: world.get::<Light2dNode>(entity).map(|light| *light),
        fields: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use star_math::Vec2;
    use star_render::Color;

    use super::*;

    const LEVEL: &str = r#"
        format = 1

        [[node]]
        name = "Level"

        [[node]]
        name = "Hero"
        parent = 0
        position = [2.0, 0.5]
        rotation = 0.25

        [node.sprite]
        texture = "sprites/hero.png"
        filter = "pixelated"
        size = [1.0, 2.0]
        color = [1.0, 0.5, 0.5, 1.0]
        layer = 2

        [[node]]
        name = "Sword"
        parent = 1
        scale = [2.0, 2.0]

        [[node]]
        name = "Enemy"
        parent = 0
        position = [5.0, 0.0]

        [node.body]
        shape = "rectangle"
        size = [1.0, 1.0]
        bounciness = 0.5
    "#;

    /// Gives textures and meshes in the order they are asked for, and
    /// remembers them.
    #[derive(Default)]
    struct Textures {
        textures: RefCell<Vec<(AssetPath, TextureFilter)>>,
        meshes: RefCell<Vec<MeshShape>>,
    }

    impl SceneAssets for &Textures {
        fn texture(&mut self, path: &AssetPath, filter: TextureFilter) -> EngineResult<TextureId> {
            let mut loaded = self.textures.borrow_mut();
            loaded.push((path.clone(), filter));
            Ok(TextureId::from_index(loaded.len() - 1))
        }

        fn mesh(&mut self, shape: MeshShape) -> EngineResult<Mesh3dId> {
            let mut built = self.meshes.borrow_mut();
            built.push(shape);
            Ok(Mesh3dId::from_index(built.len() - 1))
        }
    }

    impl SceneSources for Textures {
        fn texture_source(&self, id: TextureId) -> Option<(AssetPath, TextureFilter)> {
            (0..)
                .zip(self.textures.borrow().iter())
                .find_map(|(index, source)| {
                    (TextureId::from_index(index) == id).then(|| source.clone())
                })
        }

        fn mesh_source(&self, id: Mesh3dId) -> Option<MeshShape> {
            (0..)
                .zip(self.meshes.borrow().iter())
                .find_map(|(index, shape)| (Mesh3dId::from_index(index) == id).then_some(*shape))
        }
    }

    fn read(text: &str, textures: &Textures) -> EngineResult<Scene> {
        Scene::from_text(text, &mut &*textures)
    }

    fn level(textures: &Textures) -> Scene {
        read(LEVEL, textures).unwrap()
    }

    fn names(world: &World, nodes: &[Entity]) -> Vec<String> {
        nodes
            .iter()
            .map(|&node| world.get::<Name>(node).unwrap().0.clone())
            .collect()
    }

    #[test]
    fn instances_rebuild_the_tree_with_its_components() {
        let textures = Textures::default();
        let scene = level(&textures);
        let mut world = World::new();

        let root = scene.instantiate(&mut world, None).unwrap();

        assert_eq!(scene.node_count(), 4);
        assert_eq!(names(&world, &world.children(root)), ["Hero", "Enemy"]);
        let hero = world.find_child(root, "Hero").unwrap();
        let transform = *world.get::<Transform2D>(hero).unwrap();
        assert_eq!(transform.position, Vec2::new(2.0, 0.5));
        assert_eq!(transform.scale, Vec2::ONE);
        let sprite = *world.get::<SpriteNode>(hero).unwrap();
        assert_eq!(sprite.size, Vec2::new(1.0, 2.0));
        assert_eq!(sprite.color, Color::rgba(1.0, 0.5, 0.5, 1.0));
        assert_eq!(sprite.layer, 2);
        assert_eq!(
            textures.texture_source(sprite.texture),
            Some((
                AssetPath::new("sprites/hero.png").unwrap(),
                TextureFilter::Pixelated
            ))
        );
        let sword = world.find_child(hero, "Sword").unwrap();
        assert_eq!(
            world.get::<Transform2D>(sword).unwrap().scale,
            Vec2::new(2.0, 2.0)
        );
        assert!(!world.has::<Body2D>(hero));
        let enemy = world.find_child(root, "Enemy").unwrap();
        let body = world.get::<Body2D>(enemy).unwrap();
        assert!((body.settings().bounciness - 0.5).abs() < f32::EPSILON);
        assert_eq!(body.id(), None);
    }

    #[test]
    fn a_scene_can_be_instanced_many_times_under_a_parent() {
        let scene = level(&Textures::default());
        let mut world = World::new();
        let game = world.spawn_node("Game", None).unwrap();

        let first = scene.instantiate(&mut world, Some(game)).unwrap();
        let second = scene.instantiate(&mut world, Some(game)).unwrap();

        assert_ne!(first, second);
        assert_eq!(world.children(game), [first, second]);
        assert_eq!(world.entity_count(), 9);
    }

    #[test]
    fn failed_instances_leave_nothing_behind() {
        let scene = level(&Textures::default());
        let mut world = World::new();
        let gone = world.spawn_node("Gone", None).unwrap();
        world.despawn_node(gone);

        assert!(scene.instantiate(&mut world, Some(gone)).is_err());
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn captured_scenes_are_saved_and_read_back_the_same() {
        let textures = Textures::default();
        let scene = level(&textures);
        let mut world = World::new();
        let root = scene.instantiate(&mut world, None).unwrap();

        let captured = Scene::capture(&world, root).unwrap();
        let text = captured.to_text(&textures).unwrap();
        let read_back = read(&text, &textures).unwrap();

        assert_eq!(captured, scene);
        assert_eq!(read_back.node_count(), scene.node_count());
        assert!(text.contains("texture = \"sprites/hero.png\""), "{text}");
        assert!(!text.contains("region"), "{text}");
        assert!(text.contains("[node.body]"), "{text}");
    }

    #[test]
    fn script_classes_survive_reading_and_writing() {
        let text = "format = 1\n[[node]]\nname = \"Rock\"\nclass = \"Rock\"\n";
        let scene = read(text, &Textures::default()).unwrap();

        let written = scene.to_text(&Textures::default()).unwrap();

        assert!(written.contains("class = \"Rock\""), "{written}");
    }

    #[test]
    fn textures_made_in_code_cannot_be_saved() {
        let scene = level(&Textures::default());

        let error = scene.to_text(&Textures::default()).unwrap_err();

        assert!(error.to_string().contains("\"Hero\""), "{error}");
    }

    #[test]
    fn broken_trees_are_scene_errors() {
        for text in [
            "format = 1",
            "format = 1\n[[node]]\nname = \"A\"\nparent = 0",
            "format = 1\n[[node]]\nname = \"A\"\n[[node]]\nname = \"B\"",
            "format = 1\n[[node]]\nname = \"A\"\n[[node]]\nname = \"B\"\nparent = 1",
        ] {
            let error = read(text, &Textures::default()).unwrap_err();

            assert!(matches!(error, EngineError::Scene(_)), "{text}: {error}");
        }
    }

    #[test]
    fn invalid_texture_paths_name_their_node() {
        let text = "format = 1\n[[node]]\nname = \"Hero\"\n[node.sprite]\n\
                    texture = \"../hero.png\"\nsize = [1.0, 1.0]";

        let error = read(text, &Textures::default()).unwrap_err();

        assert!(error.to_string().contains("\"Hero\""), "{error}");
    }

    const ROOM: &str = r#"
        format = 1

        [[node]]
        name = "Room"

        [[node]]
        name = "Crate"
        parent = 0

        [node.transform_3d]
        position = [1.0, 0.5, -2.0]
        rotation = [0.0, 0.5, 0.0]

        [node.mesh]
        shape = "cube"
        size = [1.0, 1.0, 1.0]
        texture = "crate.png"
        roughness = 0.8

        [node.body_3d]
        kind = "static"
        shape = "box"
        size = [1.0, 1.0, 1.0]

        [[node]]
        name = "Lamp"
        parent = 0

        [node.transform_3d]
        position = [0.0, 3.0, 0.0]

        [node.light]
        kind = "point"
        range = 8.0
    "#;

    #[test]
    fn scenes_keep_3d_nodes_with_their_meshes_lights_and_bodies() {
        let textures = Textures::default();
        let scene = read(ROOM, &textures).unwrap();
        let mut world = World::new();

        let root = scene.instantiate(&mut world, None).unwrap();

        let crate_node = world.find_child(root, "Crate").unwrap();
        let transform = *world.get::<Transform3D>(crate_node).unwrap();
        assert_eq!(transform.position, star_math::Vec3::new(1.0, 0.5, -2.0));
        let mesh = *world.get::<MeshNode>(crate_node).unwrap();
        assert_eq!(
            textures.mesh_source(mesh.mesh),
            Some(MeshShape::Cube {
                size: star_math::Vec3::ONE
            })
        );
        assert_eq!(mesh.material.roughness, 0.8);
        assert!(mesh.material.base_color_texture.is_some());
        let body = world.get::<Body3D>(crate_node).unwrap();
        assert_eq!(body.settings().kind, star_physics::BodyKind::Static);
        assert_eq!(body.id(), None);
        drop(body);
        let lamp = world.find_child(root, "Lamp").unwrap();
        assert!(matches!(
            world.get::<LightNode>(lamp).map(|light| *light),
            Some(LightNode::Point { range: 8.0, .. })
        ));

        let text = Scene::capture(&world, root)
            .unwrap()
            .to_text(&textures)
            .unwrap();
        assert!(
            text.contains("[node.mesh]")
                && text.contains("[node.light]")
                && text.contains("[node.body_3d]"),
            "{text}"
        );
        assert_eq!(read(&text, &textures).unwrap().node_count(), 3);
    }

    #[test]
    fn invalid_meshes_name_their_node() {
        let text = "format = 1\n[[node]]\nname = \"Ball\"\n[node.mesh]\nshape = \"sphere\"";

        let error = read(text, &Textures::default()).unwrap_err();

        assert!(error.to_string().contains("\"Ball\""), "{error}");
    }

    #[test]
    fn capturing_a_dead_node_is_an_error() {
        let mut world = World::new();
        let gone = world.spawn_node("Gone", None).unwrap();
        world.despawn_node(gone);

        assert!(Scene::capture(&world, gone).is_err());
    }
}
