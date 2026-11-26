//! Bones as parts of nodes: the joints of a cut-out 2D character
//! (decision 82), whose sprites hang from them and turn with them, and
//! their `[node.bone]` table. A bone is not drawn while a game plays; the
//! editor draws it from the node's place along its rotation.

use serde::{Deserialize, Serialize};
use star_core::{EngineError, EngineResult};

/// How long a bone is when the scene does not say, in world units.
pub const DEFAULT_BONE_LENGTH: f32 = 0.5;

/// A 2D bone: a joint that turns the nodes under it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bone2dNode {
    /// How far it reaches along its rotation, in world units; above zero.
    /// Bones under it usually start where it ends.
    pub length: f32,
}

impl Default for Bone2dNode {
    fn default() -> Self {
        Self {
            length: DEFAULT_BONE_LENGTH,
        }
    }
}

/// A `[node.bone]` table; what it leaves out takes its default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoneData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    length: Option<f32>,
}

impl BoneData {
    pub(crate) fn to_bone(&self) -> EngineResult<Bone2dNode> {
        let length = self.length.unwrap_or(DEFAULT_BONE_LENGTH);
        if !(length.is_finite() && length > 0.0) {
            return Err(EngineError::Scene(format!(
                "a bone must have a positive length, not {length}"
            )));
        }
        Ok(Bone2dNode { length })
    }

    pub(crate) fn from_bone(bone: Bone2dNode) -> Self {
        Self {
            length: (bone.length != DEFAULT_BONE_LENGTH).then_some(bone.length),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(text: &str) -> EngineResult<Bone2dNode> {
        toml::from_str::<BoneData>(text).unwrap().to_bone()
    }

    #[test]
    fn bones_take_their_length_or_the_default() {
        assert_eq!(bone("").unwrap(), Bone2dNode::default());
        assert_eq!(bone("length = 1.25").unwrap().length, 1.25);
        assert_eq!(BoneData::from_bone(Bone2dNode::default()).length, None);
    }

    #[test]
    fn bones_need_a_positive_length() {
        assert!(bone("length = 0.0").is_err());
        assert!(bone("length = -1.0").is_err());
    }
}
