//! The skeleton of a glTF model, from its first skin, and the animations
//! that move its joints.
//!
//! A skin names its joints in any order; the model's skeleton keeps each
//! joint after its parent, so the vertices' joints and the channels are
//! renumbered to that order. Nodes above a root joint that are not joints
//! place it (its `base`). Channels of nodes that are not joints, and of
//! morph targets, are left out.

use std::collections::HashMap;

use gltf::animation::util::ReadOutputs;
use gltf::animation::{Interpolation, Property};
use star_core::model::IDENTITY_COLUMNS;
use star_core::{
    EngineResult, Joint, JointChannel, JointPose, JointProperty, ModelAnimation, ModelPart,
    Skeleton,
};

use super::{invalid, multiply};

/// Values of a cubic spline key: in-tangent, value, out-tangent. Only the
/// value is kept: such keys are played as linear ones.
const SPLINE_VALUES: usize = 3;

/// A skeleton read from a skin, with what renumbers the file's joints.
pub(crate) struct ReadSkeleton {
    pub(crate) skeleton: Skeleton,
    /// The skeleton's position of each joint of the skin, in its order.
    of_skin: Vec<u16>,
    /// The skeleton's position of each node that is a joint.
    of_node: HashMap<usize, usize>,
}

/// Reads the skeleton of `skin`, a skin of `gltf` whose buffers are
/// `buffers`.
///
/// # Errors
///
/// [`invalid`] if the skin has more joints than vertices can name.
pub(crate) fn read_skeleton(
    gltf: &gltf::Gltf,
    skin: &gltf::Skin<'_>,
    buffers: &[Vec<u8>],
) -> EngineResult<ReadSkeleton> {
    let parents = parents_of(gltf);
    let nodes: Vec<gltf::Node<'_>> = skin.joints().collect();
    let in_skin: HashMap<usize, usize> = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.index(), position))
        .collect();
    let inverse_binds: Vec<[[f32; 4]; 4]> = skin
        .reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice))
        .read_inverse_bind_matrices()
        .map(Iterator::collect)
        .unwrap_or_default();
    let parent_joint = |node: usize| {
        let mut above = parents.get(&node).copied();
        while let Some(index) = above {
            if let Some(&joint) = in_skin.get(&index) {
                return Some(joint);
            }
            above = parents.get(&index).copied();
        }
        None
    };
    let depth = |mut position: usize| {
        let mut depth = 0;
        while let Some(parent) = nodes
            .get(position)
            .and_then(|node| parent_joint(node.index()))
        {
            depth += 1;
            position = parent;
        }
        depth
    };
    let mut order: Vec<usize> = (0..nodes.len()).collect();
    order.sort_by_key(|&position| depth(position));
    let mut of_skin = vec![0_u16; nodes.len()];
    for (new, &old) in order.iter().enumerate() {
        of_skin[old] = u16::try_from(new).map_err(|_| invalid("a skin has too many joints"))?;
    }
    let joints = order
        .iter()
        .map(|&old| {
            let node = &nodes[old];
            let parent = parent_joint(node.index()).map(|parent| usize::from(of_skin[parent]));
            let base = if parent.is_some() {
                IDENTITY_COLUMNS
            } else {
                base_of(gltf, &parents, node.index())
            };
            let (translation, rotation, scale) = node.transform().decomposed();
            Joint {
                name: node.name().unwrap_or_default().to_owned(),
                parent,
                rest: JointPose {
                    translation,
                    rotation,
                    scale,
                },
                inverse_bind: inverse_binds.get(old).copied().unwrap_or(IDENTITY_COLUMNS),
                base,
            }
        })
        .collect();
    let of_node = nodes
        .iter()
        .enumerate()
        .map(|(old, node)| (node.index(), usize::from(of_skin[old])))
        .collect();
    Ok(ReadSkeleton {
        skeleton: Skeleton { joints },
        of_skin,
        of_node,
    })
}

impl ReadSkeleton {
    /// Makes the joints of the vertices of the skinned `parts`, positions
    /// in the skin, positions in the skeleton.
    pub(crate) fn renumber(&self, parts: &mut [ModelPart]) {
        for part in parts.iter_mut().filter(|part| part.skinned) {
            for vertex in &mut part.vertices {
                for joint in &mut vertex.joints {
                    *joint = self.of_skin.get(usize::from(*joint)).copied().unwrap_or(0);
                }
            }
        }
    }

    /// The animations of `gltf` that move the joints of the skeleton.
    pub(crate) fn animations(&self, gltf: &gltf::Gltf, buffers: &[Vec<u8>]) -> Vec<ModelAnimation> {
        gltf.animations()
            .enumerate()
            .map(|(index, animation)| {
                let channels: Vec<JointChannel> = animation
                    .channels()
                    .filter_map(|channel| self.channel(&channel, buffers))
                    .collect();
                let length = channels
                    .iter()
                    .filter_map(|channel| channel.times.last().copied())
                    .fold(0.0, f32::max);
                ModelAnimation {
                    name: animation
                        .name()
                        .map_or_else(|| format!("animation{index}"), str::to_owned),
                    length,
                    channels,
                }
            })
            .collect()
    }

    fn channel(
        &self,
        channel: &gltf::animation::Channel<'_>,
        buffers: &[Vec<u8>],
    ) -> Option<JointChannel> {
        let joint = *self.of_node.get(&channel.target().node().index())?;
        let property = match channel.target().property() {
            Property::Translation => JointProperty::Translation,
            Property::Rotation => JointProperty::Rotation,
            Property::Scale => JointProperty::Scale,
            Property::MorphTargetWeights => return None,
        };
        let reader = channel.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
        let times: Vec<f32> = reader.read_inputs()?.collect();
        let values: Vec<[f32; 4]> = match reader.read_outputs()? {
            ReadOutputs::Translations(values) | ReadOutputs::Scales(values) => {
                values.map(|[x, y, z]| [x, y, z, 0.0]).collect()
            }
            ReadOutputs::Rotations(values) => values.into_f32().collect(),
            ReadOutputs::MorphTargetWeights(_) => return None,
        };
        let interpolation = channel.sampler().interpolation();
        let values = if interpolation == Interpolation::CubicSpline {
            values
                .chunks_exact(SPLINE_VALUES)
                .map(|key| key[1])
                .collect()
        } else {
            values
        };
        Some(JointChannel {
            joint,
            property,
            step: interpolation == Interpolation::Step,
            times,
            values,
        })
    }
}

/// The parent of every node of `gltf` that has one.
fn parents_of(gltf: &gltf::Gltf) -> HashMap<usize, usize> {
    gltf.nodes()
        .flat_map(|node| {
            node.children()
                .map(move |child| (child.index(), node.index()))
        })
        .collect()
}

/// Where the nodes above the node `index` put it: their matrices, from the
/// top one down.
fn base_of(gltf: &gltf::Gltf, parents: &HashMap<usize, usize>, index: usize) -> [[f32; 4]; 4] {
    let mut above = Vec::new();
    let mut current = parents.get(&index).copied();
    while let Some(parent) = current {
        above.push(parent);
        current = parents.get(&parent).copied();
    }
    above.iter().rev().fold(IDENTITY_COLUMNS, |place, &node| {
        gltf.nodes()
            .nth(node)
            .map_or(place, |node| multiply(place, node.transform().matrix()))
    })
}
