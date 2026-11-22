//! Scenes: trees of nodes saved in files and instanced into worlds.

use star_assets::AssetPath;
use star_core::{EngineError, EngineResult};
use star_ecs::{Entity, World};
use star_math::Vec2;
use star_render::{Color, Sprite, TextureFilter, TextureId, TextureRegion};

use crate::scene_file::{FORMAT_VERSION, FilterData, NodeData, SceneFile, SpriteData};
use crate::scene_outline::{NodeOutline, SceneOutline};
use crate::{Children, Hierarchy, Name, SpriteNode, Transform2D};

/// A tree of nodes that can be added to a world as many times as needed:
/// a level, an enemy, a bullet. Read from the text of a `.scene` file, or
/// captured from a world to be saved in one.
///
/// It keeps, for each node, its name, its [`Transform2D`] and its
/// [`SpriteNode`]; other components are not part of scenes yet.
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
    parent: Option<usize>,
    transform: Option<Transform2D>,
    sprite: Option<SpriteNode>,
}

/// Gives the texture of an image of the asset folder, drawn with a filter.
pub trait TextureLoader: FnMut(&AssetPath, TextureFilter) -> EngineResult<TextureId> {}

impl<F: FnMut(&AssetPath, TextureFilter) -> EngineResult<TextureId>> TextureLoader for F {}

impl Scene {
    /// Reads a scene from the text of a `.scene` file. `load_texture` gives
    /// the texture of every image the sprites show.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if the text is not a valid scene (bad
    /// TOML, another format version, a missing root or a node before its
    /// parent, an invalid texture path) and the error of `load_texture` if a
    /// texture cannot be loaded.
    pub fn from_text(text: &str, mut load_texture: impl TextureLoader) -> EngineResult<Self> {
        let nodes = SceneOutline::from_text(text)?
            .into_nodes()
            .into_iter()
            .map(|node| load_node(node, &mut load_texture))
            .collect::<EngineResult<_>>()?;
        Ok(Self { nodes })
    }

    /// Writes the scene as the text of a `.scene` file. `texture_source`
    /// tells which image and filter each texture was loaded from.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if a sprite shows a texture that
    /// `texture_source` does not know (one made in code, not from a file).
    pub fn to_text(
        &self,
        texture_source: impl Fn(TextureId) -> Option<(AssetPath, TextureFilter)>,
    ) -> EngineResult<String> {
        let nodes = self
            .nodes
            .iter()
            .map(|node| write_node(node, &texture_source))
            .collect::<EngineResult<_>>()?;
        SceneFile {
            format: FORMAT_VERSION,
            nodes,
        }
        .to_text()
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
            if let Some(transform) = node.transform {
                world.insert(entity, transform)?;
            }
            if let Some(sprite) = node.sprite {
                world.insert(entity, sprite)?;
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

/// The node of `outline`, with its image loaded.
fn load_node(
    outline: NodeOutline,
    load_texture: &mut impl TextureLoader,
) -> EngineResult<SceneNode> {
    let sprite = outline
        .sprite
        .map(|sprite| {
            let texture = load_texture(&sprite.texture, sprite.filter)?;
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
        .map_err(|error: EngineError| {
            EngineError::Scene(format!("node \"{}\": {error}", outline.name))
        })?;
    Ok(SceneNode {
        name: outline.name,
        class: outline.class,
        parent: outline.parent,
        transform: outline.transform,
        sprite,
    })
}

fn write_node(
    node: &SceneNode,
    texture_source: &impl Fn(TextureId) -> Option<(AssetPath, TextureFilter)>,
) -> EngineResult<NodeData> {
    let sprite = node
        .sprite
        .map(|sprite| write_sprite(sprite, texture_source))
        .transpose()
        .map_err(|error| EngineError::Scene(format!("node \"{}\": {error}", node.name)))?;
    Ok(NodeData {
        name: node.name.clone(),
        class: node.class.clone(),
        parent: node.parent,
        position: node.transform.map(|transform| array(transform.position)),
        rotation: node.transform.map(|transform| transform.rotation),
        scale: node.transform.map(|transform| array(transform.scale)),
        sprite,
    })
}

fn write_sprite(
    sprite: SpriteNode,
    texture_source: &impl Fn(TextureId) -> Option<(AssetPath, TextureFilter)>,
) -> EngineResult<SpriteData> {
    let (path, filter) = texture_source(sprite.texture).ok_or_else(|| {
        EngineError::Scene(
            "its texture was not loaded from a file, so it cannot be saved".to_owned(),
        )
    })?;
    let region = sprite.region;
    let color = sprite.color;
    Ok(SpriteData {
        texture: path.as_str().to_owned(),
        filter: filter.into(),
        size: array(sprite.size),
        region: (region != TextureRegion::FULL).then_some([
            region.min.x,
            region.min.y,
            region.max.x,
            region.max.y,
        ]),
        anchor: (sprite.anchor != Sprite::CENTER).then(|| array(sprite.anchor)),
        color: (color != Color::WHITE).then_some([color.r, color.g, color.b, color.a]),
        layer: sprite.layer,
    })
}

fn capture_node(world: &World, entity: Entity, parent: Option<usize>) -> SceneNode {
    SceneNode {
        name: world
            .get::<Name>(entity)
            .map(|name| name.0.clone())
            .unwrap_or_default(),
        class: None,
        parent,
        transform: world.get::<Transform2D>(entity).map(|transform| *transform),
        sprite: world.get::<SpriteNode>(entity).map(|sprite| *sprite),
    }
}

const fn array(vector: Vec2) -> [f32; 2] {
    [vector.x, vector.y]
}

impl From<FilterData> for TextureFilter {
    fn from(filter: FilterData) -> Self {
        match filter {
            FilterData::Smooth => Self::Smooth,
            FilterData::Pixelated => Self::Pixelated,
        }
    }
}

impl From<TextureFilter> for FilterData {
    fn from(filter: TextureFilter) -> Self {
        match filter {
            TextureFilter::Smooth => Self::Smooth,
            TextureFilter::Pixelated => Self::Pixelated,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

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
    "#;

    /// Gives textures in the order they are asked for, and remembers them.
    #[derive(Default)]
    struct Textures(RefCell<Vec<(AssetPath, TextureFilter)>>);

    impl Textures {
        fn load(&self, path: &AssetPath, filter: TextureFilter) -> TextureId {
            let mut loaded = self.0.borrow_mut();
            loaded.push((path.clone(), filter));
            TextureId::from_index(loaded.len() - 1)
        }

        fn source(&self, id: TextureId) -> Option<(AssetPath, TextureFilter)> {
            (0..)
                .zip(self.0.borrow().iter())
                .find_map(|(index, source)| {
                    (TextureId::from_index(index) == id).then(|| source.clone())
                })
        }
    }

    fn level(textures: &Textures) -> Scene {
        Scene::from_text(LEVEL, |path: &AssetPath, filter| {
            Ok(textures.load(path, filter))
        })
        .unwrap()
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
            textures.source(sprite.texture),
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
        let enemy = world.find_child(root, "Enemy").unwrap();
        assert!(!world.has::<Transform2D>(enemy));
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
        let text = captured.to_text(|id| textures.source(id)).unwrap();
        let read_back = Scene::from_text(&text, |path: &AssetPath, filter| {
            Ok(textures.load(path, filter))
        })
        .unwrap();

        assert_eq!(captured, scene);
        assert_eq!(read_back.node_count(), scene.node_count());
        assert!(text.contains("texture = \"sprites/hero.png\""), "{text}");
        assert!(!text.contains("region"), "{text}");
    }

    #[test]
    fn script_classes_survive_reading_and_writing() {
        let text = "format = 1\n[[node]]\nname = \"Rock\"\nclass = \"Rock\"\n";
        let scene =
            Scene::from_text(text, |_: &AssetPath, _| Ok(TextureId::from_index(0))).unwrap();

        let written = scene.to_text(|_| None).unwrap();

        assert!(written.contains("class = \"Rock\""), "{written}");
    }

    #[test]
    fn textures_made_in_code_cannot_be_saved() {
        let scene = level(&Textures::default());

        let error = scene.to_text(|_| None).unwrap_err();

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
            let error = Scene::from_text(text, |_: &AssetPath, _| Ok(TextureId::from_index(0)))
                .unwrap_err();

            assert!(matches!(error, EngineError::Scene(_)), "{text}: {error}");
        }
    }

    #[test]
    fn invalid_texture_paths_name_their_node() {
        let text = "format = 1\n[[node]]\nname = \"Hero\"\n[node.sprite]\n\
                    texture = \"../hero.png\"\nsize = [1.0, 1.0]";

        let error =
            Scene::from_text(text, |_: &AssetPath, _| Ok(TextureId::from_index(0))).unwrap_err();

        assert!(error.to_string().contains("\"Hero\""), "{error}");
    }

    #[test]
    fn capturing_a_dead_node_is_an_error() {
        let mut world = World::new();
        let gone = world.spawn_node("Gone", None).unwrap();
        world.despawn_node(gone);

        assert!(Scene::capture(&world, gone).is_err());
    }
}
