//! Touch screen state: the fingers on the screen this frame.

use star_math::Vec2;
use star_platform::TouchPhase;

/// Most fingers tracked at once; more simultaneous touches are ignored.
/// Ten covers every finger of both hands, more than any game needs.
pub const MAX_TOUCHES: usize = 10;

/// Identifies a finger while it touches the screen. When it is lifted, a
/// later touch may get the same id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TouchId(u64);

/// One finger on the screen, or one that left it during the current frame.
///
/// Positions are in physical pixels from the window's top-left corner, with
/// Y growing downwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Touch {
    id: TouchId,
    position: Vec2,
    start_position: Vec2,
    down: bool,
    started_this_frame: bool,
    ended_this_frame: bool,
}

impl Touch {
    fn start(id: TouchId, position: Vec2) -> Self {
        Self {
            id,
            position,
            start_position: position,
            down: true,
            started_this_frame: true,
            ended_this_frame: false,
        }
    }

    /// Identifies this finger while it touches the screen.
    #[must_use]
    pub const fn id(&self) -> TouchId {
        self.id
    }

    /// Where the finger is, or where it left the screen.
    #[must_use]
    pub const fn position(&self) -> Vec2 {
        self.position
    }

    /// Where the finger first touched the screen, e.g. to detect swipes.
    #[must_use]
    pub const fn start_position(&self) -> Vec2 {
        self.start_position
    }

    /// Whether the finger is still on the screen.
    #[must_use]
    pub const fn is_down(&self) -> bool {
        self.down
    }

    /// Whether the finger touched the screen during the current frame.
    #[must_use]
    pub const fn just_started(&self) -> bool {
        self.started_this_frame
    }

    /// Whether the finger left the screen during the current frame.
    #[must_use]
    pub const fn just_ended(&self) -> bool {
        self.ended_this_frame
    }

    fn end(&mut self) {
        self.down = false;
        self.ended_this_frame = true;
    }
}

/// The fingers on a touch screen during the current frame.
///
/// A finger that leaves the screen is still listed until the frame ends, so a
/// quick tap (it touches and leaves within one frame) is never lost.
/// Fixed capacity ([`MAX_TOUCHES`]): it never allocates memory in the game loop.
#[derive(Debug, Clone, PartialEq)]
pub struct Touches {
    slots: [Option<Touch>; MAX_TOUCHES],
}

impl Touches {
    /// No fingers on the screen.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_TOUCHES],
        }
    }

    /// Every finger on the screen, plus those that left it this frame.
    pub fn iter(&self) -> impl Iterator<Item = &Touch> {
        self.slots.iter().flatten()
    }

    /// The finger `id`, if it is on the screen or left it this frame.
    #[must_use]
    pub fn get(&self, id: TouchId) -> Option<&Touch> {
        self.iter().find(|touch| touch.id == id)
    }

    /// Whether at least one finger is on the screen.
    #[must_use]
    pub fn is_any_down(&self) -> bool {
        self.iter().any(Touch::is_down)
    }

    pub(crate) fn apply(&mut self, id: u64, phase: TouchPhase, position: Vec2) {
        let id = TouchId(id);
        match phase {
            TouchPhase::Started => self.start(id, position),
            TouchPhase::Moved => self.move_to(id, position),
            TouchPhase::Ended | TouchPhase::Cancelled => self.end(id, position),
        }
    }

    /// Lifts every finger, e.g. when the window loses focus.
    pub(crate) fn release_all(&mut self) {
        for touch in self.slots.iter_mut().flatten() {
            if touch.down {
                touch.end();
            }
        }
    }

    /// Forgets the fingers that left the screen and this frame's new touches.
    pub(crate) fn end_frame(&mut self) {
        for slot in &mut self.slots {
            if slot.is_some_and(|touch| !touch.down) {
                *slot = None;
            } else if let Some(touch) = slot {
                touch.started_this_frame = false;
            }
        }
    }

    /// A start for a finger that is already down is only a move.
    fn start(&mut self, id: TouchId, position: Vec2) {
        if self.held_mut(id).is_some() {
            self.move_to(id, position);
        } else if let Some(free) = self.slots.iter_mut().find(|slot| slot.is_none()) {
            *free = Some(Touch::start(id, position));
        }
    }

    fn move_to(&mut self, id: TouchId, position: Vec2) {
        if let Some(touch) = self.held_mut(id) {
            touch.position = position;
        }
    }

    fn end(&mut self, id: TouchId, position: Vec2) {
        if let Some(touch) = self.held_mut(id) {
            touch.position = position;
            touch.end();
        }
    }

    /// The finger `id` while it is on the screen.
    fn held_mut(&mut self, id: TouchId) -> Option<&mut Touch> {
        self.slots
            .iter_mut()
            .flatten()
            .find(|touch| touch.down && touch.id == id)
    }
}

impl Default for Touches {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FINGER: u64 = 7;
    const START: Vec2 = Vec2::new(10.0, 20.0);
    const END: Vec2 = Vec2::new(30.0, 40.0);

    fn touching() -> Touches {
        let mut touches = Touches::new();
        touches.apply(FINGER, TouchPhase::Started, START);
        touches
    }

    fn finger(touches: &Touches) -> Touch {
        *touches.get(TouchId(FINGER)).unwrap()
    }

    #[test]
    fn a_new_touch_is_down_and_just_started() {
        let touches = touching();

        let touch = finger(&touches);
        assert!(touch.is_down() && touch.just_started() && !touch.just_ended());
        assert_eq!(touch.position(), START);
        assert!(touches.is_any_down());
    }

    #[test]
    fn a_touch_only_just_started_in_its_first_frame() {
        let mut touches = touching();

        touches.end_frame();

        assert!(!finger(&touches).just_started());
        assert!(finger(&touches).is_down());
    }

    #[test]
    fn moving_keeps_the_start_position() {
        let mut touches = touching();

        touches.apply(FINGER, TouchPhase::Moved, END);

        assert_eq!(finger(&touches).position(), END);
        assert_eq!(finger(&touches).start_position(), START);
    }

    #[test]
    fn an_ended_touch_stays_until_the_frame_ends() {
        let mut touches = touching();
        touches.end_frame();

        touches.apply(FINGER, TouchPhase::Ended, END);

        let touch = finger(&touches);
        assert!(!touch.is_down() && touch.just_ended());
        assert_eq!(touch.position(), END);
        assert!(!touches.is_any_down());

        touches.end_frame();

        assert!(touches.get(TouchId(FINGER)).is_none());
    }

    #[test]
    fn a_tap_within_one_frame_is_not_lost() {
        let mut touches = touching();

        touches.apply(FINGER, TouchPhase::Ended, START);

        let touch = finger(&touches);
        assert!(touch.just_started() && touch.just_ended());
    }

    #[test]
    fn a_cancelled_touch_ends() {
        let mut touches = touching();

        touches.apply(FINGER, TouchPhase::Cancelled, START);

        assert!(finger(&touches).just_ended());
    }

    #[test]
    fn an_id_can_be_reused_after_its_touch_ended() {
        let mut touches = touching();
        touches.apply(FINGER, TouchPhase::Ended, START);

        touches.apply(FINGER, TouchPhase::Started, END);

        assert_eq!(touches.iter().count(), 2);
        assert!(touches.is_any_down());
    }

    #[test]
    fn touches_beyond_the_capacity_are_ignored() {
        let mut touches = Touches::new();

        for id in 0..=MAX_TOUCHES as u64 {
            touches.apply(id, TouchPhase::Started, START);
        }

        assert_eq!(touches.iter().count(), MAX_TOUCHES);
        assert!(touches.get(TouchId(MAX_TOUCHES as u64)).is_none());
    }

    #[test]
    fn events_for_unknown_fingers_are_ignored() {
        let mut touches = Touches::new();

        touches.apply(FINGER, TouchPhase::Moved, START);
        touches.apply(FINGER, TouchPhase::Ended, START);

        assert_eq!(touches.iter().count(), 0);
    }

    #[test]
    fn release_all_lifts_every_finger() {
        let mut touches = touching();
        touches.apply(FINGER + 1, TouchPhase::Started, END);

        touches.release_all();

        assert!(!touches.is_any_down());
        assert!(touches.iter().all(Touch::just_ended));
    }
}
