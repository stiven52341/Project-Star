//! The layout of `.scene` files: TOML with a flat list of nodes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use star_core::{EngineError, EngineResult};
use star_render::TextureFilter;

use crate::FieldValue;
use crate::body_data::{Body3dData, BodyData};
use crate::camera_node::{Camera2dData, Camera3dData};
use crate::light_2d_node::Light2dData;
use crate::node_3d_data::{LightData, MeshTable, Transform3dData};

/// The version of the layout this engine writes and reads.
pub(crate) const FORMAT_VERSION: u32 = 1;

/// A whole `.scene` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SceneFile {
    pub(crate) format: u32,
    /// Parents always come before their children; the first node is the root.
    #[serde(rename = "node", default)]
    pub(crate) nodes: Vec<NodeData>,
}

/// One `[[node]]` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeData {
    pub(crate) name: String,
    /// The StarScript class the node is built as, by games written in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) class: Option<String>,
    /// Words that groups of nodes share, to find them all at once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) tags: Vec<String>,
    /// Position of the parent in the list; only the root has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parent: Option<usize>,
    /// Any of the three gives the node a transform; the others take their
    /// default values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) position: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) rotation: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) scale: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) sprite: Option<SpriteData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<BodyData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) transform_3d: Option<Transform3dData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mesh: Option<MeshTable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) light: Option<LightData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) body_3d: Option<Body3dData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) camera: Option<Camera2dData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) light_2d: Option<Light2dData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) camera_3d: Option<Camera3dData>,
    /// `[node.fields]`: the values of the `export` fields of its class.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) fields: BTreeMap<String, FieldValue>,
}

/// A `[node.sprite]` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpriteData {
    /// Asset path of the image.
    pub(crate) texture: String,
    #[serde(default)]
    pub(crate) filter: FilterData,
    pub(crate) size: [f32; 2],
    /// Top-left and bottom-right corners: `[min_x, min_y, max_x, max_y]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) region: Option<[f32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) anchor: Option<[f32; 2]>,
    /// Red, green, blue and alpha, from 0 to 1, in sRGB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) color: Option<[f32; 4]>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub(crate) layer: i32,
    /// The image cut into `columns` x `rows` cells, showing `frame`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) columns: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) rows: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) frame: Option<u32>,
}

/// How a texture is sampled, as written in files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FilterData {
    #[default]
    Smooth,
    Pixelated,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
fn is_zero(layer: &i32) -> bool {
    *layer == 0
}

impl SceneFile {
    pub(crate) fn parse(text: &str) -> EngineResult<Self> {
        let file: Self = toml::from_str(text)
            .map_err(|error| EngineError::Scene(format!("invalid scene file: {error}")))?;
        if file.format != FORMAT_VERSION {
            return Err(EngineError::Scene(format!(
                "the scene file has format {}, but this engine reads format {FORMAT_VERSION}",
                file.format
            )));
        }
        Ok(file)
    }

    pub(crate) fn to_text(&self) -> EngineResult<String> {
        toml::to_string(self)
            .map_err(|error| EngineError::Scene(format!("cannot write the scene: {error}")))
    }
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
    use super::*;

    #[test]
    fn files_are_read_with_their_optional_fields() {
        let file = SceneFile::parse(
            r#"
            format = 1

            [[node]]
            name = "Level"

            [[node]]
            name = "Hero"
            parent = 0
            position = [2.0, 0.5]

            [node.sprite]
            texture = "sprites/hero.png"
            filter = "pixelated"
            size = [1.0, 2.0]
            "#,
        )
        .unwrap();

        assert_eq!(file.nodes.len(), 2);
        let hero = &file.nodes[1];
        assert_eq!(hero.parent, Some(0));
        assert_eq!(hero.position, Some([2.0, 0.5]));
        assert_eq!(hero.rotation, None);
        let sprite = hero.sprite.as_ref().unwrap();
        assert_eq!(sprite.filter, FilterData::Pixelated);
        assert_eq!(sprite.layer, 0);
    }

    #[test]
    fn written_files_read_back_the_same() {
        let file = SceneFile {
            format: FORMAT_VERSION,
            nodes: vec![NodeData {
                name: "Hero".to_owned(),
                class: Some("Player".to_owned()),
                tags: vec!["hero".to_owned()],
                parent: None,
                position: Some([1.0, 2.0]),
                rotation: Some(0.5),
                scale: Some([1.0, 1.0]),
                sprite: Some(SpriteData {
                    texture: "hero.png".to_owned(),
                    filter: FilterData::Smooth,
                    size: [1.0, 1.0],
                    region: Some([0.0, 0.0, 0.5, 1.0]),
                    anchor: None,
                    color: None,
                    layer: 3,
                    columns: Some(4),
                    rows: None,
                    frame: Some(2),
                }),
                body: None,
                transform_3d: None,
                mesh: None,
                light: None,
                body_3d: None,
                camera: None,
                light_2d: None,
                camera_3d: None,
                fields: BTreeMap::new(),
            }],
        };

        let text = file.to_text().unwrap();

        assert_eq!(SceneFile::parse(&text).unwrap(), file);
    }

    #[test]
    fn unknown_formats_and_fields_are_scene_errors() {
        for text in [
            "format = 2",
            "format = 1\n[[node]]\nname = \"A\"\ncolour = 1",
            "not toml at all",
        ] {
            let error = SceneFile::parse(text).unwrap_err();

            assert!(matches!(error, EngineError::Scene(_)), "{text}: {error}");
        }
    }
}
