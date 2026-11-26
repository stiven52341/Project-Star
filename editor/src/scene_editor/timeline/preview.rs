//! An animation previewed on the scene: a copy of its nodes with the
//! values its tracks give them at a time. The scene itself is left as it
//! was.

use star_engine::prelude::SceneOutline;
use star_engine::star_scene::TrackAnimation;

use super::paths;
use crate::scene_editor::animated_properties;

/// The scene `outline` as `animation` leaves the node at `root` and those
/// under it `time` seconds in. Tracks of nodes that are not there, or with
/// values their property cannot take, change nothing.
pub(super) fn previewed(
    outline: &SceneOutline,
    root: usize,
    animation: &TrackAnimation,
    time: f32,
) -> SceneOutline {
    let mut previewed = outline.clone();
    for track in &animation.tracks {
        let Some(index) = paths::node_at(outline.nodes(), root, &track.node) else {
            continue;
        };
        let (Some(value), Some(node)) = (track.value_at(time), previewed.node_mut(index)) else {
            continue;
        };
        animated_properties::set_value(node, &track.property, &value);
    }
    previewed
}

#[cfg(test)]
mod tests {
    use star_engine::star_scene::{FieldValue, Interpolation, Key, PropertyTrack};

    use super::*;

    const SCENE: &str = r#"format = 1

[[node]]
name = "Knight"
position = [0.0, 0.0]

[[node]]
name = "Arm"
parent = 0
position = [1.0, 0.0]

[[node]]
name = "Animator"
parent = 0

[node.animator]
animations = "knight.anim"
"#;

    fn track(node: &str, property: &str, keys: [(f32, f64); 2]) -> PropertyTrack {
        PropertyTrack {
            node: node.to_owned(),
            property: property.to_owned(),
            interpolation: Interpolation::Linear,
            keys: keys
                .into_iter()
                .map(|(time, value)| Key {
                    time,
                    value: FieldValue::Float(value),
                })
                .collect(),
        }
    }

    #[test]
    fn previews_change_a_copy_of_the_animated_nodes() {
        let outline = SceneOutline::from_text(SCENE).unwrap();
        let animation = TrackAnimation {
            name: "wave".to_owned(),
            length: 1.0,
            looping: false,
            tracks: vec![
                track("Arm", "rotation", [(0.0, 0.0), (1.0, 2.0)]),
                track("Leg", "rotation", [(0.0, 0.0), (1.0, 2.0)]),
                track("", "x", [(0.0, 4.0), (1.0, 4.0)]),
            ],
            calls: Vec::new(),
        };

        let previewed = previewed(&outline, 0, &animation, 0.5);

        let arm = previewed.nodes()[1].transform.unwrap();
        assert!((arm.rotation - 1.0).abs() < 1e-6);
        assert_eq!(previewed.nodes()[0].transform.unwrap().position.x, 4.0);
        assert_eq!(outline.nodes()[1].transform.unwrap().rotation, 0.0);
    }
}
