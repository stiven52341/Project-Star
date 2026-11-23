//! What a `.scene` file describes, before any image is loaded.

use star_assets::{AssetFolder, AssetPath};
use star_core::{EngineError, EngineResult};
use star_math::Vec2;
use star_render::{Color, Sprite, TextureFilter, TextureRegion};

use star_physics::BodySettings;

use crate::body_data::BodyData;
use crate::node_3d_data::MeshOutline;
use crate::scene_file::{NodeData, SceneFile, SpriteData};
use crate::{LightNode, Transform2D, Transform3D};

/// The nodes of a `.scene` file, checked to form one tree, with their images
/// named by asset path instead of loaded. A [`Scene`](crate::Scene) is built
/// from it by loading those images; code that builds its own nodes, such as
/// StarScript games, reads it directly.
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

pub(crate) const fn vec2([x, y]: [f32; 2]) -> Vec2 {
    Vec2::new(x, y)
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
