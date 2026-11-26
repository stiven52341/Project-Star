//! Posing the skeleton of a model (decision 83): each joint where its
//! animations put it at their times, mixed by their weights, or at rest,
//! and the joint matrices that bend the model's skinned meshes.

use star_core::{JointChannel, JointPose, JointProperty, ModelAnimation, Skeleton};
use star_math::{Mat4, Quat, Vec3};

/// An animation of a model at a time, weighing in a mix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipAt {
    /// The animation, as a position in the model's animations.
    pub animation: usize,
    /// Seconds into it.
    pub time: f32,
    /// How much it weighs against the others; zero or less is left out.
    pub weight: f32,
}

/// The animations a node's model plays this frame, as its animator says;
/// without it, or with no clips, the model stands at rest.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelPose {
    /// The animations at their times, and their weights.
    pub clips: Vec<ClipAt>,
}

/// Works out joint matrices, keeping its working space between frames so
/// posing does not allocate once it has room.
#[derive(Debug, Default)]
pub struct Poser {
    mixed: Vec<JointPose>,
    clip: Vec<JointPose>,
    globals: Vec<Mat4>,
}

impl Poser {
    /// The matrix of each joint of `skeleton` into `joints`: from the
    /// model's space as the meshes were bound to where the joint takes them.
    /// The joints are where `clips` of `animations` put them, mixed by
    /// their weights, or at rest without clips.
    pub fn pose(
        &mut self,
        skeleton: &Skeleton,
        animations: &[ModelAnimation],
        clips: &[ClipAt],
        joints: &mut Vec<Mat4>,
    ) {
        self.mix(skeleton, animations, clips);
        self.globals.clear();
        joints.clear();
        for (joint, local) in skeleton.joints.iter().zip(&self.mixed) {
            let above = joint
                .parent
                .and_then(|parent| self.globals.get(parent).copied())
                .unwrap_or_else(|| Mat4::from_columns(joint.base));
            let global = above * pose_matrix(local);
            self.globals.push(global);
            joints.push(global * Mat4::from_columns(joint.inverse_bind));
        }
    }

    /// Each joint's place relative to its parent, mixed from `clips`.
    fn mix(&mut self, skeleton: &Skeleton, animations: &[ModelAnimation], clips: &[ClipAt]) {
        let rest = skeleton.joints.iter().map(|joint| joint.rest);
        self.mixed.clear();
        self.mixed.extend(rest.clone());
        let mut total = 0.0;
        for clip in clips.iter().filter(|clip| clip.weight > 0.0) {
            let Some(animation) = animations.get(clip.animation) else {
                continue;
            };
            self.clip.clear();
            self.clip.extend(rest.clone());
            for channel in &animation.channels {
                if let Some(pose) = self.clip.get_mut(channel.joint) {
                    apply(channel, clip.time, pose);
                }
            }
            total += clip.weight;
            let amount = clip.weight / total;
            for (mixed, posed) in self.mixed.iter_mut().zip(&self.clip) {
                *mixed = blend(*mixed, *posed, amount);
            }
        }
    }
}

/// Gives `pose` the value `channel` has `time` seconds in.
fn apply(channel: &JointChannel, time: f32, pose: &mut JointPose) {
    let value = sample(channel, time);
    match channel.property {
        JointProperty::Translation => pose.translation = [value[0], value[1], value[2]],
        JointProperty::Scale => pose.scale = [value[0], value[1], value[2]],
        JointProperty::Rotation => pose.rotation = value,
    }
}

/// The value of `channel` at `time`: the first key's before it, the last
/// key's after it, and between two keys a step or a mix of them.
fn sample(channel: &JointChannel, time: f32) -> [f32; 4] {
    let after = channel.times.partition_point(|&key| key <= time);
    let value = |index: usize| channel.values.get(index).copied().unwrap_or_default();
    let (Some(before), Some(&next_time)) = (after.checked_sub(1), channel.times.get(after)) else {
        return value(
            after
                .saturating_sub(1)
                .min(channel.values.len().saturating_sub(1)),
        );
    };
    if channel.step {
        return value(before);
    }
    let before_time = channel.times.get(before).copied().unwrap_or(time);
    let span = next_time - before_time;
    let amount = if span > f32::EPSILON {
        (time - before_time) / span
    } else {
        0.0
    };
    match channel.property {
        JointProperty::Rotation => nlerp(value(before), value(after), amount),
        JointProperty::Translation | JointProperty::Scale => {
            lerp4(value(before), value(after), amount)
        }
    }
}

/// `amount` of the way from `from` to `to`.
fn blend(from: JointPose, to: JointPose, amount: f32) -> JointPose {
    let lerp3 = |a: [f32; 3], b: [f32; 3]| {
        let [x, y, z, _] = lerp4([a[0], a[1], a[2], 0.0], [b[0], b[1], b[2], 0.0], amount);
        [x, y, z]
    };
    JointPose {
        translation: lerp3(from.translation, to.translation),
        rotation: nlerp(from.rotation, to.rotation, amount),
        scale: lerp3(from.scale, to.scale),
    }
}

fn lerp4(from: [f32; 4], to: [f32; 4], amount: f32) -> [f32; 4] {
    let mut mixed = from;
    for (value, target) in mixed.iter_mut().zip(to) {
        *value += (target - *value) * amount;
    }
    mixed
}

/// Turns between two rotations the short way round, normalized.
fn nlerp(from: [f32; 4], to: [f32; 4], amount: f32) -> [f32; 4] {
    let dot: f32 = from.iter().zip(&to).map(|(a, b)| a * b).sum();
    let to = if dot < 0.0 { to.map(|part| -part) } else { to };
    let mixed = lerp4(from, to, amount);
    Quat::from_xyzw(mixed).map_or(from, Quat::to_xyzw)
}

/// The matrix of a joint's place relative to its parent.
fn pose_matrix(pose: &JointPose) -> Mat4 {
    let vector = |[x, y, z]: [f32; 3]| Vec3::new(x, y, z);
    Mat4::from_scale_rotation_translation(
        vector(pose.scale),
        Quat::from_xyzw(pose.rotation).unwrap_or_default(),
        vector(pose.translation),
    )
}

#[cfg(test)]
mod tests {
    use star_core::Joint;
    use star_core::model::IDENTITY_COLUMNS;

    use super::*;

    const EPSILON: f32 = 1e-4;

    fn joint(parent: Option<usize>, translation: [f32; 3]) -> Joint {
        Joint {
            name: String::new(),
            parent,
            rest: JointPose {
                translation,
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
            },
            inverse_bind: IDENTITY_COLUMNS,
            base: IDENTITY_COLUMNS,
        }
    }

    /// A root at the origin and an arm one unit up from it.
    fn arm() -> Skeleton {
        Skeleton {
            joints: vec![joint(None, [0.0; 3]), joint(Some(0), [0.0, 1.0, 0.0])],
        }
    }

    /// Moves the root from x = 0 to x = `to` in one second.
    fn slide(to: f32) -> ModelAnimation {
        ModelAnimation {
            name: "slide".to_owned(),
            length: 1.0,
            channels: vec![JointChannel {
                joint: 0,
                property: JointProperty::Translation,
                step: false,
                times: vec![0.0, 1.0],
                values: vec![[0.0; 4], [to, 0.0, 0.0, 0.0]],
            }],
        }
    }

    fn arm_at(clips: &[ClipAt], animations: &[ModelAnimation]) -> Vec3 {
        let mut joints = Vec::new();
        Poser::default().pose(&arm(), animations, clips, &mut joints);
        joints[1].translation()
    }

    #[test]
    fn joints_at_rest_follow_their_parents() {
        assert!((arm_at(&[], &[]) - Vec3::new(0.0, 1.0, 0.0)).length() < EPSILON);
    }

    #[test]
    fn animations_move_joints_between_their_keys() {
        let clip = |time| ClipAt {
            animation: 0,
            time,
            weight: 1.0,
        };

        let halfway = arm_at(&[clip(0.5)], &[slide(4.0)]);
        let after = arm_at(&[clip(3.0)], &[slide(4.0)]);

        assert!(
            (halfway - Vec3::new(2.0, 1.0, 0.0)).length() < EPSILON,
            "{halfway:?}"
        );
        assert!(
            (after - Vec3::new(4.0, 1.0, 0.0)).length() < EPSILON,
            "{after:?}"
        );
    }

    #[test]
    fn clips_are_mixed_by_their_weights() {
        let at = |animation, weight| ClipAt {
            animation,
            time: 1.0,
            weight,
        };

        let mixed = arm_at(&[at(0, 0.75), at(1, 0.25)], &[slide(0.0), slide(4.0)]);

        assert!(
            (mixed - Vec3::new(1.0, 1.0, 0.0)).length() < EPSILON,
            "{mixed:?}"
        );
    }

    #[test]
    fn rotations_turn_the_short_way() {
        let quarter = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_xyzw();
        let half = nlerp([0.0, 0.0, 0.0, 1.0], quarter, 0.5);
        let eighth = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4).to_xyzw();

        for (got, wanted) in half.iter().zip(eighth) {
            assert!((got - wanted).abs() < EPSILON, "{half:?}");
        }
        let flipped = quarter.map(|part| -part);
        let same = nlerp(quarter, flipped, 0.5);
        assert!(same.iter().all(|part| part.is_finite()));
    }
}
