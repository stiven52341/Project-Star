//! The timeline: the animations of the animator selected last (its `.anim`
//! file), in the “Animation” tab under the view. It shows their tracks of
//! keys and their calls over time, a playhead that previews them on the
//! scene, and edits them. Its changes are the scene's: undone with it and
//! saved with it.

mod calls;
mod edits;
mod lanes;
mod mark_values;
mod panel;
mod paths;
mod preview;
mod track_names;

use star_engine::prelude::SceneOutline;
use star_engine::star_scene::{FieldValue, NodeOutline, TrackAnimation, TrackAnimations};

use super::key_buttons::KeyButtons;

use self::edits::{Copied, Mark};
pub(super) use self::panel::{Change, TimelineFile};

/// What the timeline edits and how it shows it, besides the animations.
#[derive(Debug, Default)]
pub(super) struct Timeline {
    /// The animator whose animations it shows, by its place in the scene.
    animator: Option<usize>,
    /// The animation shown, by name.
    animation: Option<String>,
    /// Where the playhead is, in seconds.
    time: f32,
    /// Whether the playhead moves on by itself.
    playing: bool,
    /// The keys and calls selected.
    selection: Vec<Mark>,
    /// What was copied last.
    copied: Copied,
    /// The keys and calls being dragged, while they are.
    drag: Option<KeyDrag>,
    /// The new name being typed for the animation shown, while renaming.
    renaming: Option<String>,
    /// Whether its tab is shown, so the scene shows the animation.
    shown: bool,
    /// Whether it took an animator since it was last asked.
    just_bound: bool,
}

/// Keys and calls being dragged: the animation as it was when the drag
/// started, so they move from where they were, and none is lost on the way
/// over another key.
#[derive(Debug)]
struct KeyDrag {
    /// The time the pointer was at when it started.
    from: f32,
    original: TrackAnimation,
    marks: Vec<Mark>,
}

impl Timeline {
    /// Follows the node selected in `nodes`: an animator becomes the one
    /// shown. One that is no longer an animator is let go.
    pub(super) fn follow(&mut self, nodes: &[NodeOutline], selected: Option<usize>) {
        if self.animator.is_some_and(|animator| {
            nodes
                .get(animator)
                .is_none_or(|node| node.animator.is_none())
        }) {
            *self = Self {
                shown: self.shown,
                copied: std::mem::take(&mut self.copied),
                ..Self::default()
            };
        }
        let Some(selected) = selected.filter(|&selected| Some(selected) != self.animator) else {
            return;
        };
        let Some(animator) = nodes.get(selected).and_then(|node| node.animator.as_ref()) else {
            return;
        };
        *self = Self {
            animator: Some(selected),
            animation: animator.animation.clone(),
            shown: self.shown,
            copied: std::mem::take(&mut self.copied),
            just_bound: true,
            ..Self::default()
        };
    }

    /// Whether it took an animator since this was last asked, so its tab
    /// can be shown.
    pub(super) const fn take_just_bound(&mut self) -> bool {
        std::mem::replace(&mut self.just_bound, false)
    }

    /// Whether its tab is shown this frame.
    pub(super) const fn set_shown(&mut self, shown: bool) {
        self.shown = shown;
    }

    /// The `.anim` file of the animator shown, in `nodes`.
    pub(super) fn file<'nodes>(&self, nodes: &'nodes [NodeOutline]) -> Option<&'nodes str> {
        let animator = nodes.get(self.animator?)?.animator.as_ref()?;
        Some(animator.animations.as_str())
    }

    /// The scene as the animation shown leaves it at the playhead, while
    /// the tab is shown; `None` if there is nothing to preview.
    pub(super) fn previewed(
        &self,
        outline: &SceneOutline,
        animation: Option<&TrackAnimation>,
    ) -> Option<SceneOutline> {
        if !self.shown {
            return None;
        }
        let root = paths::animated_root(outline.nodes(), self.animator?)?;
        Some(preview::previewed(outline, root, animation?, self.time))
    }

    /// The name of the animation shown.
    pub(super) fn animation_name(&self) -> Option<&str> {
        self.animation.as_deref()
    }

    /// The key buttons of the node at `node` of `nodes`, while the tab is
    /// shown and the node is animated, with the node's path; `animations`
    /// are those of the animator's file.
    pub(super) fn key_buttons(
        &self,
        nodes: &[NodeOutline],
        node: usize,
        animations: &TrackAnimations,
    ) -> Option<(KeyButtons, String)> {
        if !self.shown {
            return None;
        }
        let root = paths::animated_root(nodes, self.animator?)?;
        let path = paths::path_of(nodes, root, node)?;
        let tracks = self
            .animation
            .as_deref()
            .and_then(|name| animations.animation(name))
            .map(|animation| animation.tracks.as_slice())
            .unwrap_or_default();
        let (mut tracked, mut keyed) = (Vec::new(), Vec::new());
        for track in tracks.iter().filter(|track| track.node == path) {
            tracked.push(track.property.clone());
            if edits::has_key_at(track, self.time) {
                keyed.push(track.property.clone());
            }
        }
        Some((KeyButtons::new(self.time, tracked, keyed), path))
    }

    /// Puts `keys` (properties and their values) of the node at `node`, a
    /// path, at the playhead of the animation shown, which is made if
    /// `animations` have none, and selects them. Returns whether the
    /// animations changed.
    pub(super) fn add_keys(
        &mut self,
        animations: &mut TrackAnimations,
        node: &str,
        keys: Vec<(String, FieldValue)>,
    ) -> bool {
        let mut made = false;
        if self
            .animation
            .as_deref()
            .and_then(|name| animations.animation(name))
            .is_none()
        {
            let name = edits::add_animation(animations);
            self.show_animation(name);
            made = true;
        }
        let Some(animation) = self.animation.as_deref().and_then(|name| {
            animations
                .animations
                .iter_mut()
                .find(|animation| animation.name == name)
        }) else {
            return made;
        };
        let marks: Vec<Mark> = keys
            .into_iter()
            .filter_map(|(property, value)| {
                edits::set_key(animation, node, &property, self.time, value)
            })
            .collect();
        if marks.is_empty() {
            return made;
        }
        self.selection = marks;
        true
    }
}
