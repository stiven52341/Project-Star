//! Poses: the values several animations give their properties at once,
//! mixed by weights, for the fades and blends of state machines (decision
//! 81).
//!
//! Each animation is added with a weight; a property that several change
//! gets the weighted mix of their values (decimals and colors blend, and
//! the rest takes the value of the animation that weighs the most), and a
//! property only one changes gets that one's value.

use crate::nodes::Host;
use crate::scene_fields::SceneValue;
use crate::tracks::{TrackClip, mix, set_track_value};

/// The values of the properties some animations change, mixed.
#[derive(Debug, Default)]
pub struct Pose<'a> {
    values: Vec<PoseValue<'a>>,
}

/// One property of a [`Pose`] and its value so far.
#[derive(Debug)]
struct PoseValue<'a> {
    /// The animation that changed it first, for warnings.
    animation: &'a str,
    /// The node's path from the animator's parent.
    node: &'a str,
    property: &'a str,
    value: SceneValue,
    /// How much the animations that changed it weigh together.
    weight: f32,
}

impl<'a> Pose<'a> {
    /// A pose without values.
    #[must_use]
    pub const fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Mixes in the values `clip` has `time` seconds into it, weighing
    /// `weight` against the animations added before. A weight of zero or
    /// less adds nothing.
    pub fn add(&mut self, clip: &'a TrackClip, time: f32, weight: f32) {
        if weight <= 0.0 {
            return;
        }
        for track in &clip.tracks {
            let Some(value) = track.sample(time) else {
                continue;
            };
            let same = |known: &&mut PoseValue<'a>| {
                known.node == track.node && known.property == track.property
            };
            if let Some(known) = self.values.iter_mut().find(same) {
                let total = known.weight + weight;
                known.value = mix(&known.value, &value, weight / total);
                known.weight = total;
            } else {
                self.values.push(PoseValue {
                    animation: &clip.name,
                    node: &track.node,
                    property: &track.property,
                    value,
                    weight,
                });
            }
        }
    }

    /// Gives each property its mixed value, on `root` (the animator's
    /// parent) and the nodes under it.
    pub fn apply<H: Host>(&self, root: &H::Node) {
        for known in &self.values {
            set_track_value::<H>(
                root,
                known.animation,
                known.node,
                known.property,
                &known.value,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::nodes;
    use crate::test_host::{TestHost, TestNode};
    use crate::tracks::{Ease, Track, TrackKey};

    const EPSILON: f32 = 1e-5;

    /// An animation of one second that holds `x`, and `frame` when given.
    fn holding(name: &str, x: f64, frame: Option<i64>) -> TrackClip {
        let constant = |property: &str, value: SceneValue| Track {
            node: "Arm".to_owned(),
            property: property.to_owned(),
            ease: Ease::Linear,
            keys: vec![TrackKey { time: 0.0, value }],
        };
        let mut tracks = vec![constant("x", SceneValue::Float(x))];
        tracks.extend(frame.map(|frame| constant("frame", SceneValue::Int(frame))));
        TrackClip {
            name: name.to_owned(),
            length: 1.0,
            looping: true,
            tracks,
            calls: Vec::new(),
        }
    }

    fn arm_x(first: (&TrackClip, f32), second: (&TrackClip, f32)) -> f32 {
        let hero = Rc::new(TestNode::new("Hero"));
        let arm = Rc::new(TestNode::new("Arm"));
        nodes::add_child::<TestHost>(&hero, Rc::clone(&arm));
        let mut pose = Pose::new();
        pose.add(first.0, 0.0, first.1);
        pose.add(second.0, 0.0, second.1);
        pose.apply::<TestHost>(&hero);
        arm.node.x.get()
    }

    #[test]
    fn decimals_are_mixed_by_weight() {
        let walk = holding("walk", 0.0, None);
        let run = holding("run", 10.0, None);

        assert!((arm_x((&walk, 0.75), (&run, 0.25)) - 2.5).abs() < EPSILON);
        assert!((arm_x((&walk, 1.0), (&run, 0.0)) - 0.0).abs() < EPSILON);
        assert!((arm_x((&walk, 0.0), (&run, 0.5)) - 10.0).abs() < EPSILON);
    }

    #[test]
    fn values_that_cannot_blend_take_the_heavier_animation() {
        let walk = holding("walk", 0.0, Some(1));
        let run = holding("run", 0.0, Some(7));
        let frame = |run_weight: f32| {
            let mut pose = Pose::new();
            pose.add(&walk, 0.0, 1.0 - run_weight);
            pose.add(&run, 0.0, run_weight);
            pose.values
                .iter()
                .find(|known| known.property == "frame")
                .map(|known| known.value.clone())
        };

        assert_eq!(frame(0.4), Some(SceneValue::Int(1)));
        assert_eq!(frame(0.5), Some(SceneValue::Int(7)));
    }

    #[test]
    fn properties_only_one_animation_changes_keep_its_value() {
        let walk = holding("walk", 4.0, None);
        let wave = TrackClip {
            name: "wave".to_owned(),
            length: 1.0,
            looping: false,
            tracks: Vec::new(),
            calls: Vec::new(),
        };

        assert!((arm_x((&walk, 0.2), (&wave, 0.8)) - 4.0).abs() < EPSILON);
    }
}
