//! The animations a node of a scene plays, as StarScript's `Animator`
//! (decision 79): its `.anim` file and the animation it starts with.

use serde::{Deserialize, Serialize};
use star_assets::AssetPath;
use star_core::EngineResult;

/// The animations an animator node plays on its parent and the nodes under
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimatorOutline {
    /// The `.anim` file, in the asset folder.
    pub animations: AssetPath,
    /// The animation it plays from the start, if any.
    pub animation: Option<String>,
    /// The `.states` file of the machine that chooses its animations, if
    /// any (decision 81).
    pub states: Option<AssetPath>,
}

/// A `[node.animator]` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AnimatorData {
    animations: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    animation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    states: Option<String>,
}

impl AnimatorData {
    pub(crate) fn to_outline(&self) -> EngineResult<AnimatorOutline> {
        Ok(AnimatorOutline {
            animations: AssetPath::new(&self.animations)?,
            animation: self.animation.clone(),
            states: self.states.as_deref().map(AssetPath::new).transpose()?,
        })
    }

    pub(crate) fn from_outline(animator: &AnimatorOutline) -> Self {
        Self {
            animations: animator.animations.as_str().to_owned(),
            animation: animator.animation.clone(),
            states: animator
                .states
                .as_ref()
                .map(|states| states.as_str().to_owned()),
        }
    }
}
