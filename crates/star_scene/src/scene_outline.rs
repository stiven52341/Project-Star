//! What a `.scene` file describes, before any image is loaded.

use star_assets::{AssetFolder, AssetPath};
use star_core::{EngineError, EngineResult};
use star_math::Vec2;
use star_render::{Color, Sprite, TextureFilter, TextureRegion};

use star_physics::{BodySettings, Shape3d};

use crate::body_data::{Body3dData, BodyData};
use crate::node_3d_data::{LightData, MeshOutline, MeshTable, Transform3dData};
use crate::scene_file::{FORMAT_VERSION, NodeData, SceneFile, SpriteData};
use crate::{LightNode, Transform2D, Transform3D};

/// The nodes of a `.scene` file, checked to form one tree, with their images
/// named by asset path instead of loaded. A [`Scene`](crate::Scene) is built
/// from it by loading those images; code that builds its own nodes, such as
/// StarScript games, reads it directly, and the editor changes it and
/// writes it back.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneOutline {
    nodes: Vec<NodeOutline>,
}

/// One node of a [`SceneOutline`].
#[derive(Debug, Clone, PartialEq)]
pub struct NodeOutline {
    /// Its name.
    pub name: String,
    /// The StarScript class it is built as, if the file names one. Games
    /// written in Rust build every node the same way and ignore it.
    pub class: Option<String>,
    /// Position of its parent in [`SceneOutline::nodes`]; only the root,
    /// the first node, has none.
    pub parent: Option<usize>,
    /// Where it is relative to its parent, if the file places it.
    pub transform: Option<Transform2D>,
    /// The image it shows, if any.
    pub sprite: Option<SpriteOutline>,
    /// Its body in the 2D physics, if any.
    pub body: Option<BodySettings>,
    /// Where it is in 3D relative to its parent, if the file places it so.
    pub transform_3d: Option<Transform3D>,
    /// The 3D mesh it shows, if any.
    pub mesh: Option<MeshOutline>,
    /// The light it shines, if any.
    pub light: Option<LightNode>,
    /// Its body in the 3D physics, if any.
    pub body_3d: Option<BodySettings<Shape3d>>,
}

/// The image a node of a [`SceneOutline`] shows.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteOutline {
    /// The image, in the asset folder.
    pub texture: AssetPath,
    /// How the image is sampled.
    pub filter: TextureFilter,
    /// Width and height in world units, before the node's scale.
    pub size: Vec2,
    /// Part of the image shown.
    pub region: TextureRegion,
    /// Point of the sprite placed at the node, from `(0, 0)` (bottom-left
    /// corner) to `(1, 1)` (top-right corner).
    pub anchor: Vec2,
    /// Multiplied by the image's colors.
    pub color: Color,
    /// Sprites in higher layers are drawn on top of lower ones.
    pub layer: i32,
}

impl SceneOutline {
    /// Reads the text of a `.scene` file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if the text is not a valid scene: bad
    /// TOML, another format version, a missing root, a node before its
    /// parent or an invalid texture path.
    pub fn from_text(text: &str) -> EngineResult<Self> {
        let file = SceneFile::parse(text)?;
        validate_tree(&file.nodes)?;
        let nodes = file
            .nodes
            .into_iter()
            .map(read_node)
            .collect::<EngineResult<_>>()?;
        Ok(Self { nodes })
    }

    /// Reads the `.scene` file at `path` in `folder`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`], naming the file, if it cannot be read
    /// or is not a valid scene (see [`SceneOutline::from_text`]).
    pub fn read(folder: &AssetFolder, path: &AssetPath) -> EngineResult<Self> {
        folder
            .read_text(path)
            .and_then(|text| Self::from_text(&text))
            .map_err(|error| EngineError::Scene(format!("\"{path}\": {error}")))
    }

    /// The nodes, parents before their children; the first one is the root.
    #[must_use]
    pub fn nodes(&self) -> &[NodeOutline] {
        &self.nodes
    }

    /// Takes the nodes out of the outline, in the same order as
    /// [`SceneOutline::nodes`].
    #[must_use]
    pub fn into_nodes(self) -> Vec<NodeOutline> {
        self.nodes
    }

    /// The outline of `nodes`, which [`SceneOutline::to_text`] checks.
    pub(crate) const fn from_nodes(nodes: Vec<NodeOutline>) -> Self {
        Self { nodes }
    }

    /// The node at `index`, to change it. Its [`NodeOutline::parent`] is
    /// best left alone: a parent that does not come before it breaks the
    /// tree, which [`SceneOutline::to_text`] then refuses to write.
    pub fn node_mut(&mut self, index: usize) -> Option<&mut NodeOutline> {
        self.nodes.get_mut(index)
    }

    /// Adds `node` as the last child of the node at `parent`. Returns its
    /// position.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if there is no node at `parent`.
    pub fn add_node(&mut self, parent: usize, mut node: NodeOutline) -> EngineResult<usize> {
        if parent >= self.nodes.len() {
            return Err(EngineError::Scene(format!(
                "cannot add \"{}\": the scene has no node {parent}",
                node.name
            )));
        }
        node.parent = Some(parent);
        // After every node, so its parent comes before it.
        self.nodes.push(node);
        Ok(self.nodes.len() - 1)
    }

    /// Removes the node at `index` with all its descendants; the nodes
    /// after them move back to fill their places.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if `index` is the root, which every
    /// scene needs, or there is no node there.
    pub fn remove_node(&mut self, index: usize) -> EngineResult<()> {
        if index == 0 || index >= self.nodes.len() {
            return Err(EngineError::Scene(format!(
                "cannot remove node {index}: it is the root or it does not exist"
            )));
        }
        let removed = self.subtree(index);
        let new_positions = new_positions(&removed);
        let mut position = 0;
        self.nodes.retain(|_| {
            position += 1;
            !removed[position - 1]
        });
        for node in &mut self.nodes {
            node.parent = node
                .parent
                .and_then(|parent| new_positions.get(parent).copied().flatten());
        }
        Ok(())
    }

    /// Moves the node at `index`, with all its descendants, to be the last
    /// child of the node at `parent`. The nodes keep their order otherwise,
    /// with the ones moved after the rest, so parents still come first.
    /// Returns the node's new position.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if `index` is the root, if there is
    /// no node at `index` or `parent`, or if `parent` is the node itself or
    /// one of its descendants, which would leave them out of the tree.
    pub fn move_node(&mut self, index: usize, parent: usize) -> EngineResult<usize> {
        let count = self.nodes.len();
        if index == 0 || index >= count || parent >= count {
            return Err(EngineError::Scene(format!(
                "cannot move node {index} under node {parent}: the root cannot move, \
                 and both must exist"
            )));
        }
        let moved = self.subtree(index);
        if moved[parent] {
            return Err(EngineError::Scene(format!(
                "cannot move node {index} under node {parent}, which is inside it"
            )));
        }
        let order: Vec<usize> = (0..count)
            .filter(|&position| !moved[position])
            .chain((0..count).filter(|&position| moved[position]))
            .collect();
        let mut new_positions = vec![0; count];
        for (new, &old) in order.iter().enumerate() {
            new_positions[old] = new;
        }
        let mut old_nodes: Vec<Option<NodeOutline>> = std::mem::take(&mut self.nodes)
            .into_iter()
            .map(Some)
            .collect();
        self.nodes = order
            .iter()
            .filter_map(|&old| old_nodes[old].take())
            .collect();
        for node in &mut self.nodes {
            node.parent = node.parent.map(|parent| new_positions[parent]);
        }
        let new_index = new_positions[index];
        self.nodes[new_index].parent = Some(new_positions[parent]);
        Ok(new_index)
    }

    /// Marks the node at `index` and its descendants. Parents come first,
    /// so one pass finds them all.
    fn subtree(&self, index: usize) -> Vec<bool> {
        let mut inside = vec![false; self.nodes.len()];
        for position in index..self.nodes.len() {
            inside[position] = position == index
                || self.nodes[position]
                    .parent
                    .is_some_and(|parent| inside[parent]);
        }
        inside
    }

    /// Writes the outline as the text of a `.scene` file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if its nodes no longer form one tree
    /// (see [`SceneOutline::node_mut`]).
    pub fn to_text(&self) -> EngineResult<String> {
        let nodes: Vec<NodeData> = self.nodes.iter().map(write_node).collect();
        validate_tree(&nodes)?;
        SceneFile {
            format: FORMAT_VERSION,
            nodes,
        }
        .to_text()
    }
}

/// Where each node ends up once those marked in `removed` are taken out.
fn new_positions(removed: &[bool]) -> Vec<Option<usize>> {
    let mut kept = 0;
    removed
        .iter()
        .map(|&removed| {
            (!removed).then(|| {
                kept += 1;
                kept - 1
            })
        })
        .collect()
}

/// Checks that the first node is the only root and that parents come first,
/// so the nodes form one tree.
fn validate_tree(nodes: &[NodeData]) -> EngineResult<()> {
    let invalid = |reason: String| EngineError::Scene(format!("invalid scene: {reason}"));
    let Some(root) = nodes.first() else {
        return Err(invalid("it has no nodes".to_owned()));
    };
    if root.parent.is_some() {
        return Err(invalid(format!(
            "the first node, \"{}\", is the root and cannot have a parent",
            root.name
        )));
    }
    for (index, node) in nodes.iter().enumerate().skip(1) {
        match node.parent {
            Some(parent) if parent < index => {}
            Some(parent) => {
                return Err(invalid(format!(
                    "node {index} (\"{}\") must come after its parent, node {parent}",
                    node.name
                )));
            }
            None => {
                return Err(invalid(format!(
                    "node {index} (\"{}\") has no parent, but only the first node is the root",
                    node.name
                )));
            }
        }
    }
    Ok(())
}

fn read_node(node: NodeData) -> EngineResult<NodeOutline> {
    let has_transform = node.position.is_some() || node.rotation.is_some() || node.scale.is_some();
    let transform = has_transform.then(|| Transform2D {
        position: node.position.map_or(Vec2::ZERO, vec2),
        rotation: node.rotation.unwrap_or_default(),
        scale: node.scale.map_or(Vec2::ONE, vec2),
    });
    let sprite = node
        .sprite
        .as_ref()
        .map(read_sprite)
        .transpose()
        .map_err(|error| EngineError::Scene(format!("node \"{}\": {error}", node.name)))?;
    let named = |error: EngineError| EngineError::Scene(format!("node \"{}\": {error}", node.name));
    let body = node
        .body
        .as_ref()
        .map(BodyData::to_settings)
        .transpose()
        .map_err(named)?;
    let body_3d = node
        .body_3d
        .as_ref()
        .map(Body3dData::to_settings)
        .transpose()
        .map_err(named)?;
    let mesh = node
        .mesh
        .as_ref()
        .map(crate::node_3d_data::MeshTable::to_outline)
        .transpose()
        .map_err(named)?;
    let light = node
        .light
        .as_ref()
        .map(crate::node_3d_data::LightData::to_light)
        .transpose()
        .map_err(named)?;
    if node
        .class
        .as_deref()
        .is_some_and(|class| class.trim().is_empty())
    {
        return Err(EngineError::Scene(format!(
            "node \"{}\": its class has no name",
            node.name
        )));
    }
    Ok(NodeOutline {
        name: node.name,
        class: node.class,
        parent: node.parent,
        transform,
        sprite,
        body,
        transform_3d: node
            .transform_3d
            .as_ref()
            .map(crate::node_3d_data::Transform3dData::to_transform),
        mesh,
        light,
        body_3d,
    })
}

fn read_sprite(sprite: &SpriteData) -> EngineResult<SpriteOutline> {
    let region = sprite
        .region
        .map_or(TextureRegion::FULL, |[min_x, min_y, max_x, max_y]| {
            TextureRegion::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
        });
    Ok(SpriteOutline {
        texture: AssetPath::new(&sprite.texture)?,
        filter: sprite.filter.into(),
        size: vec2(sprite.size),
        region,
        anchor: sprite.anchor.map_or(Sprite::CENTER, vec2),
        color: sprite
            .color
            .map_or(Color::WHITE, |[r, g, b, a]| Color::rgba(r, g, b, a)),
        layer: sprite.layer,
    })
}

fn write_node(node: &NodeOutline) -> NodeData {
    NodeData {
        name: node.name.clone(),
        class: node.class.clone(),
        parent: node.parent,
        position: node.transform.map(|transform| array(transform.position)),
        rotation: node.transform.map(|transform| transform.rotation),
        scale: node.transform.map(|transform| array(transform.scale)),
        sprite: node.sprite.as_ref().map(write_sprite),
        body: node.body.as_ref().map(BodyData::from_settings),
        transform_3d: node
            .transform_3d
            .as_ref()
            .map(Transform3dData::from_transform),
        mesh: node.mesh.as_ref().map(MeshTable::from_outline),
        light: node.light.as_ref().map(LightData::from_light),
        body_3d: node.body_3d.as_ref().map(Body3dData::from_settings),
    }
}

fn write_sprite(sprite: &SpriteOutline) -> SpriteData {
    let region = sprite.region;
    let color = sprite.color;
    SpriteData {
        texture: sprite.texture.as_str().to_owned(),
        filter: sprite.filter.into(),
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
    }
}

pub(crate) const fn vec2([x, y]: [f32; 2]) -> Vec2 {
    Vec2::new(x, y)
}

const fn array(vector: Vec2) -> [f32; 2] {
    [vector.x, vector.y]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlines_name_images_by_path_and_fill_in_defaults() {
        let outline = SceneOutline::from_text(
            r#"
            format = 1

            [[node]]
            name = "Level"

            [[node]]
            name = "Hero"
            class = "Player"
            parent = 0
            position = [2.0, 0.5]

            [node.sprite]
            texture = "sprites/hero.png"
            filter = "pixelated"
            size = [1.0, 2.0]
            color = [1.0, 0.5, 0.5, 1.0]
            "#,
        )
        .unwrap();

        let [level, hero] = outline.nodes() else {
            panic!("two nodes expected");
        };
        assert_eq!((level.parent, level.transform), (None, None));
        assert_eq!(hero.parent, Some(0));
        assert_eq!(
            (level.class.as_deref(), hero.class.as_deref()),
            (None, Some("Player"))
        );
        let transform = hero.transform.unwrap();
        assert_eq!(transform.position, Vec2::new(2.0, 0.5));
        assert_eq!(transform.scale, Vec2::ONE);
        let sprite = hero.sprite.as_ref().unwrap();
        assert_eq!(sprite.texture.as_str(), "sprites/hero.png");
        assert_eq!(sprite.filter, TextureFilter::Pixelated);
        assert_eq!(sprite.region, TextureRegion::FULL);
        assert_eq!(sprite.anchor, Sprite::CENTER);
        assert_eq!(sprite.color, Color::rgba(1.0, 0.5, 0.5, 1.0));
    }

    #[test]
    fn files_are_read_from_the_asset_folder() {
        let root = std::env::temp_dir().join(format!("star_scene_outline_{}", std::process::id()));
        std::fs::create_dir_all(root.join("levels")).unwrap();
        std::fs::write(
            root.join("levels/one.scene"),
            "format = 1\n[[node]]\nname = \"One\"",
        )
        .unwrap();
        let assets = AssetFolder::new(&root);

        let outline = SceneOutline::read(&assets, &AssetPath::new("levels/one.scene").unwrap());
        let missing = SceneOutline::read(&assets, &AssetPath::new("levels/two.scene").unwrap());

        assert_eq!(outline.unwrap().nodes()[0].name, "One");
        assert!(
            missing
                .unwrap_err()
                .to_string()
                .contains("\"levels/two.scene\"")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn broken_trees_are_rejected() {
        for text in [
            "format = 1",
            "format = 1\n[[node]]\nname = \"A\"\nparent = 0",
            "format = 1\n[[node]]\nname = \"A\"\n[[node]]\nname = \"B\"",
            "format = 1\n[[node]]\nname = \"A\"\n[[node]]\nname = \"B\"\nparent = 1",
            "format = 1\n[[node]]\nname = \"A\"\nclass = \" \"",
        ] {
            assert!(SceneOutline::from_text(text).is_err(), "{text}");
        }
    }

    #[test]
    fn bodies_are_read_and_their_errors_name_their_node() {
        let body = "format = 1\n[[node]]\nname = \"Ball\"\n[node.body]\nshape = \"circle\"\n";

        let outline = SceneOutline::from_text(&format!("{body}radius = 0.5")).unwrap();
        let error = SceneOutline::from_text(body).unwrap_err();

        let settings = outline.nodes()[0].body.unwrap();
        assert_eq!(settings.kind, star_physics::BodyKind::Dynamic);
        assert!(error.to_string().contains("node \"Ball\""), "{error}");
    }

    const TREE: &str = "format = 1\n\
        [[node]]\nname = \"Root\"\n\
        [[node]]\nname = \"A\"\nparent = 0\nposition = [1.0, 2.0]\n\
        [[node]]\nname = \"B\"\nparent = 0\n\
        [[node]]\nname = \"A1\"\nparent = 1\n\
        [[node]]\nname = \"B1\"\nparent = 2\n\
        [node.sprite]\ntexture = \"b.png\"\nsize = [1.0, 1.0]\ncolor = [1.0, 0.0, 0.0, 1.0]\n";

    fn names_and_parents(outline: &SceneOutline) -> Vec<(&str, Option<usize>)> {
        outline
            .nodes()
            .iter()
            .map(|node| (node.name.as_str(), node.parent))
            .collect()
    }

    #[test]
    fn written_outlines_read_back_the_same() {
        let outline = SceneOutline::from_text(TREE).unwrap();

        let text = outline.to_text().unwrap();

        assert_eq!(SceneOutline::from_text(&text).unwrap(), outline);
    }

    #[test]
    fn nodes_are_added_as_the_last_child_of_their_parent() {
        let mut outline = SceneOutline::from_text(TREE).unwrap();
        let mut node = outline.nodes()[3].clone();
        node.name = "B2".to_owned();

        let index = outline.add_node(2, node.clone()).unwrap();

        assert_eq!(index, 5);
        assert_eq!(outline.nodes()[5].parent, Some(2));
        assert!(outline.add_node(9, node).is_err());
        assert!(SceneOutline::from_text(&outline.to_text().unwrap()).is_ok());
    }

    #[test]
    fn removing_a_node_removes_its_descendants_and_renumbers_the_rest() {
        let mut outline = SceneOutline::from_text(TREE).unwrap();

        outline.remove_node(1).unwrap();

        assert_eq!(
            names_and_parents(&outline),
            vec![("Root", None), ("B", Some(0)), ("B1", Some(1))]
        );
    }

    #[test]
    fn moved_nodes_take_their_descendants_after_the_rest() {
        let mut outline = SceneOutline::from_text(TREE).unwrap();

        let index = outline.move_node(1, 2).unwrap();

        assert_eq!(index, 3);
        assert_eq!(
            names_and_parents(&outline),
            vec![
                ("Root", None),
                ("B", Some(0)),
                ("B1", Some(1)),
                ("A", Some(1)),
                ("A1", Some(3)),
            ]
        );
        assert_eq!(
            outline.nodes()[3].transform,
            SceneOutline::from_text(TREE).unwrap().nodes()[1].transform
        );
        assert!(SceneOutline::from_text(&outline.to_text().unwrap()).is_ok());
    }

    #[test]
    fn nodes_cannot_move_into_themselves_and_the_root_cannot_move() {
        let mut outline = SceneOutline::from_text(TREE).unwrap();

        assert!(outline.move_node(1, 1).is_err());
        assert!(outline.move_node(1, 3).is_err());
        assert!(outline.move_node(0, 2).is_err());
        assert!(outline.move_node(1, 9).is_err());
        assert_eq!(outline, SceneOutline::from_text(TREE).unwrap());
    }

    #[test]
    fn the_root_cannot_be_removed() {
        let mut outline = SceneOutline::from_text(TREE).unwrap();

        assert!(outline.remove_node(0).is_err());
        assert!(outline.remove_node(5).is_err());
        assert_eq!(outline.nodes().len(), 5);
    }

    #[test]
    fn broken_trees_are_not_written() {
        let mut outline = SceneOutline::from_text(TREE).unwrap();
        outline.node_mut(1).unwrap().parent = Some(4);

        assert!(outline.to_text().is_err());
    }

    #[test]
    fn invalid_texture_paths_name_their_node() {
        let error = SceneOutline::from_text(
            "format = 1\n[[node]]\nname = \"Hero\"\n[node.sprite]\n\
             texture = \"../hero.png\"\nsize = [1.0, 1.0]",
        )
        .unwrap_err();

        assert!(error.to_string().contains("node \"Hero\""), "{error}");
    }
}
