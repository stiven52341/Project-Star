//! Undo and redo in a scene: copies of the scene, and of the keyframe
//! animations of its animators, as they were before each change.
//!
//! A change can last several frames (dragging a handle, typing a number),
//! so changes are only marked while they happen, and a step is closed once
//! the user lets go: one drag or one number typed is undone at once.

use std::collections::BTreeMap;

use star_engine::prelude::SceneOutline;
use star_engine::star_scene::TrackAnimations;

/// The `.anim` files the scene's animators play, by their path in the
/// assets folder, once read.
pub(super) type KeyframeFiles = BTreeMap<String, TrackAnimations>;

/// How many changes can be undone; older ones are forgotten.
const MAXIMUM_STEPS: usize = 100;

/// The scene, its animations and the node selected, at one moment.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Snapshot {
    pub(super) outline: SceneOutline,
    pub(super) animations: KeyframeFiles,
    pub(super) selected: Option<usize>,
}

/// The scene as it is now, to copy only when a step closes.
#[derive(Debug, Clone, Copy)]
pub(super) struct Now<'scene> {
    pub(super) outline: &'scene SceneOutline,
    pub(super) animations: &'scene KeyframeFiles,
    pub(super) selected: Option<usize>,
}

impl Now<'_> {
    fn snapshot(self) -> Snapshot {
        Snapshot {
            outline: self.outline.clone(),
            animations: self.animations.clone(),
            selected: self.selected,
        }
    }
}

/// A snapshot and the number that tells it apart, to know whether the
/// scene is back to how it was saved.
#[derive(Debug, Clone)]
struct Step {
    snapshot: Snapshot,
    revision: u64,
}

/// What can be undone and redone in one scene.
#[derive(Debug)]
pub(super) struct SceneHistory {
    /// Older states, the newest last.
    past: Vec<Step>,
    /// States undone, the one undone last at the end.
    future: Vec<Step>,
    /// The scene as it was when the last step closed.
    settled: Step,
    /// Whether the scene changed after `settled`.
    pending: bool,
    /// The number the next step gets.
    next_revision: u64,
    /// The revision written to the file, if one is.
    saved: Option<u64>,
}

impl SceneHistory {
    /// The history of a scene just read from its file.
    pub(super) fn new(snapshot: Snapshot) -> Self {
        Self {
            past: Vec::new(),
            future: Vec::new(),
            settled: Step {
                snapshot,
                revision: 0,
            },
            pending: false,
            next_revision: 1,
            saved: Some(0),
        }
    }

    /// The scene has changed: a new step opens if none is open.
    pub(super) const fn mark_changed(&mut self) {
        self.pending = true;
    }

    /// The user selected `selected` without changing the scene: undoing
    /// the next change selects it again.
    pub(super) const fn follow_selection(&mut self, selected: Option<usize>) {
        if !self.pending {
            self.settled.snapshot.selected = selected;
        }
    }

    /// The file at `path` was read as `animations`: every moment kept had
    /// it so, as it was not changed before.
    pub(super) fn adopt_animations(&mut self, path: &str, animations: &TrackAnimations) {
        let steps = self
            .past
            .iter_mut()
            .chain(&mut self.future)
            .chain(std::iter::once(&mut self.settled));
        for step in steps {
            step.snapshot
                .animations
                .entry(path.to_owned())
                .or_insert_with(|| animations.clone());
        }
    }

    /// Closes the open step, if one is, with the scene as it is now.
    pub(super) fn close_step(&mut self, now: Now<'_>) {
        if !self.pending {
            return;
        }
        self.pending = false;
        let current = Step {
            snapshot: now.snapshot(),
            revision: self.take_revision(),
        };
        let before = std::mem::replace(&mut self.settled, current);
        self.past.push(before);
        if self.past.len() > MAXIMUM_STEPS {
            self.past.remove(0);
        }
        self.future.clear();
    }

    /// Goes one step back from `now`. Returns the scene to show, or `None`
    /// if there is nothing to undo.
    pub(super) fn undo(&mut self, now: Now<'_>) -> Option<Snapshot> {
        self.close_step(now);
        let previous = self.past.pop()?;
        let left = std::mem::replace(&mut self.settled, previous);
        self.future.push(left);
        Some(self.settled.snapshot.clone())
    }

    /// Goes one step forward again from `now`. Returns the scene to show,
    /// or `None` if there is nothing to redo.
    pub(super) fn redo(&mut self, now: Now<'_>) -> Option<Snapshot> {
        self.close_step(now);
        let next = self.future.pop()?;
        let left = std::mem::replace(&mut self.settled, next);
        self.past.push(left);
        Some(self.settled.snapshot.clone())
    }

    /// Whether there is a change to undo.
    pub(super) const fn can_undo(&self) -> bool {
        self.pending || !self.past.is_empty()
    }

    /// Whether there is a change undone to redo.
    pub(super) const fn can_redo(&self) -> bool {
        !self.pending && !self.future.is_empty()
    }

    /// `now` was written to the file.
    pub(super) fn mark_saved(&mut self, now: Now<'_>) {
        self.close_step(now);
        self.saved = Some(self.settled.revision);
    }

    /// Whether the scene is as it was written to the file.
    pub(super) fn is_saved(&self) -> bool {
        !self.pending && self.saved == Some(self.settled.revision)
    }

    const fn take_revision(&mut self) -> u64 {
        let revision = self.next_revision;
        self.next_revision += 1;
        revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scene whose root is called `name`.
    fn scene(name: &str, selected: Option<usize>) -> Snapshot {
        let text = format!("format = 1\n[[node]]\nname = \"{name}\"");
        Snapshot {
            outline: SceneOutline::from_text(&text).unwrap(),
            animations: KeyframeFiles::new(),
            selected,
        }
    }

    fn now(snapshot: &Snapshot) -> Now<'_> {
        Now {
            outline: &snapshot.outline,
            animations: &snapshot.animations,
            selected: snapshot.selected,
        }
    }

    /// A history where the root was renamed from `A` to `B`, then `C`.
    fn renamed_twice() -> SceneHistory {
        let mut history = SceneHistory::new(scene("A", None));
        history.mark_changed();
        history.close_step(now(&scene("B", None)));
        history.mark_changed();
        history.close_step(now(&scene("C", None)));
        history
    }

    #[test]
    fn a_new_history_has_nothing_to_undo_or_redo() {
        let history = SceneHistory::new(scene("A", None));

        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert!(history.is_saved());
    }

    #[test]
    fn undo_goes_back_one_step_at_a_time() {
        let mut history = renamed_twice();

        assert_eq!(history.undo(now(&scene("C", None))), Some(scene("B", None)));
        assert_eq!(history.undo(now(&scene("B", None))), Some(scene("A", None)));
        assert_eq!(history.undo(now(&scene("A", None))), None);
    }

    #[test]
    fn redo_goes_forward_again() {
        let mut history = renamed_twice();
        history.undo(now(&scene("C", None)));
        history.undo(now(&scene("B", None)));

        assert_eq!(history.redo(now(&scene("A", None))), Some(scene("B", None)));
        assert_eq!(history.redo(now(&scene("B", None))), Some(scene("C", None)));
        assert_eq!(history.redo(now(&scene("C", None))), None);
    }

    #[test]
    fn a_new_change_forgets_what_was_undone() {
        let mut history = renamed_twice();
        history.undo(now(&scene("C", None)));

        history.mark_changed();
        history.close_step(now(&scene("D", None)));

        assert!(!history.can_redo());
        assert_eq!(history.undo(now(&scene("D", None))), Some(scene("B", None)));
    }

    #[test]
    fn a_change_still_open_is_undone_whole() {
        let mut history = SceneHistory::new(scene("A", None));
        // A drag that changed the scene over several frames.
        history.mark_changed();
        history.mark_changed();

        assert!(history.can_undo());
        assert_eq!(history.undo(now(&scene("B", None))), Some(scene("A", None)));
        assert_eq!(history.redo(now(&scene("A", None))), Some(scene("B", None)));
    }

    #[test]
    fn undo_selects_what_was_selected_before_the_change() {
        let mut history = SceneHistory::new(scene("A", None));
        history.follow_selection(Some(0));
        history.mark_changed();
        // Selecting while the change is open does not move the step back.
        history.follow_selection(None);

        let undone = history.undo(now(&scene("B", None))).unwrap();

        assert_eq!(undone.selected, Some(0));
    }

    #[test]
    fn going_back_to_the_saved_scene_is_saved_again() {
        let mut history = SceneHistory::new(scene("A", None));
        history.mark_changed();
        assert!(!history.is_saved());

        history.undo(now(&scene("B", None)));
        assert!(history.is_saved());

        history.redo(now(&scene("A", None)));
        assert!(!history.is_saved());
        history.mark_saved(now(&scene("B", None)));
        assert!(history.is_saved());
    }

    #[test]
    fn animations_read_later_are_how_every_step_had_them() {
        let mut history = renamed_twice();
        let read = TrackAnimations::default();

        history.adopt_animations("knight.anim", &read);

        let undone = history.undo(now(&scene("C", None))).unwrap();
        assert_eq!(undone.animations.get("knight.anim"), Some(&read));
    }

    #[test]
    fn only_the_newest_steps_are_kept() {
        let mut history = SceneHistory::new(scene("0", None));
        for step in 1..=MAXIMUM_STEPS + 1 {
            history.mark_changed();
            history.close_step(now(&scene(&step.to_string(), None)));
        }
        let mut current = scene(&(MAXIMUM_STEPS + 1).to_string(), None);
        let mut undone = 0;
        while let Some(previous) = history.undo(now(&current)) {
            current = previous;
            undone += 1;
        }

        assert_eq!(undone, MAXIMUM_STEPS);
        assert_eq!(current, scene("1", None));
    }
}
