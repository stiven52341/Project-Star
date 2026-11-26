//! A state machine running: its state, its values and its fade, moved on
//! every frame.

use std::rc::Rc;

use super::{Condition, MachineState, Motion, StateMachine, Test, Transition, machine_of};
use crate::node_classes::{AnimatorFields, BEFORE_START};
use crate::nodes::{self, Host};
use crate::poses::Pose;
use crate::tracks::{self, TrackClip, Tracks, warn};

/// How far a blend is between its points when its value is exactly
/// halfway, where values that cannot blend jump.
const HALFWAY: f32 = 0.5;

/// What the state machine of an animator is doing.
#[derive(Debug, Default)]
pub struct MachineRun {
    /// The `.states` file it runs; another one starts it over.
    path: String,
    /// The state it is in; empty before it starts.
    state: String,
    /// The animation of the state that weighs the most, for `animation()`.
    animation: String,
    /// Rounds of the state's animations played: `1.0` is once.
    progress: f32,
    /// `progress` in the last frame; below zero before the first.
    shown: f32,
    /// Whether the state's animation that does not loop has ended, so
    /// `animation_finished` runs once.
    ended: bool,
    /// Whether `stop()` stopped it.
    stopped: bool,
    /// The state it is fading from, if it is.
    fading: Option<Fading>,
    numbers: Vec<(String, f32)>,
    flags: Vec<(String, bool)>,
    /// The triggers set since the last frame.
    triggers: Vec<String>,
}

/// The state a machine is fading from.
#[derive(Debug)]
struct Fading {
    state: String,
    /// Rounds of its animations played, still moving on.
    progress: f32,
    /// Seconds the fade has lasted.
    elapsed: f32,
    /// Seconds it lasts.
    length: f32,
}

/// Up to two animations, each with its time and weight.
type Timed = [Option<(usize, f32, f32)>; 2];

/// The animations a state plays now: one or two, with their weights.
#[derive(Debug, Clone, Copy, Default)]
struct Blend {
    first: Option<(usize, f32)>,
    second: Option<(usize, f32)>,
}

impl MachineRun {
    /// A machine that has not started.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            path: String::new(),
            state: String::new(),
            animation: String::new(),
            progress: 0.0,
            shown: BEFORE_START,
            ended: false,
            stopped: false,
            fading: None,
            numbers: Vec::new(),
            flags: Vec::new(),
            triggers: Vec::new(),
        }
    }

    /// Whether it plays: until `stop()`.
    #[must_use]
    pub const fn is_playing(&self) -> bool {
        !self.stopped
    }

    /// The animation of its state that weighs the most; empty before the
    /// first frame.
    #[must_use]
    pub fn animation(&self) -> &str {
        &self.animation
    }

    /// Stops it where it is, until a state is played.
    pub const fn stop(&mut self) {
        self.stopped = true;
    }

    pub(super) fn set_number(&mut self, name: &str, value: f32) {
        set_named(&mut self.numbers, name, value);
    }

    pub(super) fn set_flag(&mut self, name: &str, value: bool) {
        set_named(&mut self.flags, name, value);
    }

    pub(super) fn set_trigger(&mut self, name: &str) {
        if !self.triggers.iter().any(|known| known == name) {
            self.triggers.push(name.to_owned());
        }
    }

    fn number(&self, name: &str) -> f32 {
        named(&self.numbers, name).unwrap_or(0.0)
    }

    fn flag(&self, name: &str) -> bool {
        named(&self.flags, name).unwrap_or(false)
    }

    /// Starts the machine of the file at `path` over, in `start`; its
    /// values stay.
    fn restart(&mut self, path: &str, start: &str) {
        path.clone_into(&mut self.path);
        self.fading = None;
        self.enter(start);
    }

    fn enter(&mut self, state: &str) {
        state.clone_into(&mut self.state);
        self.progress = 0.0;
        self.shown = BEFORE_START;
        self.ended = false;
    }

    fn holds(&self, condition: &Condition) -> bool {
        match condition.test {
            Test::Above(limit) => self.number(&condition.value) > limit,
            Test::Below(limit) => self.number(&condition.value) < limit,
            Test::Is(value) => self.flag(&condition.value) == value,
            Test::Triggered => self.triggers.contains(&condition.value),
        }
    }

    /// Whether `transition` can be taken now: one that waits for the end
    /// once the state's last frame of a round was shown.
    fn can_take(&self, transition: &Transition) -> bool {
        let leaves = transition
            .from
            .as_ref()
            .map_or(transition.to != self.state, |from| *from == self.state);
        leaves
            && (!transition.at_end || self.shown >= 1.0)
            && transition
                .conditions
                .iter()
                .all(|condition| self.holds(condition))
    }

    /// Takes the first transition that can be taken, if one can.
    fn transition(&mut self, machine: &StateMachine) {
        let Some(transition) = machine
            .transitions
            .iter()
            .find(|transition| self.can_take(transition))
        else {
            return;
        };
        self.fading = (transition.fade > 0.0).then(|| Fading {
            state: std::mem::take(&mut self.state),
            progress: self.progress,
            elapsed: 0.0,
            length: transition.fade,
        });
        self.enter(&transition.to);
    }
}

fn named<T: Copy>(values: &[(String, T)], name: &str) -> Option<T> {
    values
        .iter()
        .find_map(|(known, value)| (known == name).then_some(*value))
}

fn set_named<T>(values: &mut Vec<(String, T)>, name: &str, value: T) {
    if let Some((_, known)) = values.iter_mut().find(|(known, _)| known == name) {
        *known = value;
    } else {
        values.push((name.to_owned(), value));
    }
}

/// `func play` of an animator with a machine: goes to the state `name`
/// at once, unless it is in it, and plays again if it was stopped.
pub(crate) fn jump<H: Host>(node: &H::Node, name: &str) {
    let Some(machine) = machine_of::<H>(node) else {
        return;
    };
    if machine.state(name).is_none() {
        warn(&format!("the states have no state \"{name}\""));
        return;
    }
    H::with_animator(node, |animator| {
        let mut run = animator.machine.borrow_mut();
        run.stopped = false;
        if run.state != name {
            run.fading = None;
            run.enter(name);
        }
    });
}

/// `func state`: the state the animator's machine is in; empty if it has
/// none, or before its first frame.
#[must_use]
pub fn state<H: Host>(node: &H::Node) -> String {
    H::with_animator(node, |animator| animator.machine.borrow().state.clone()).unwrap_or_default()
}

/// What a frame of a machine does once its values are worked out, so no
/// script runs while the machine is borrowed.
struct Frame {
    clips: Rc<Tracks>,
    /// The animations of the state faded from, with their times, and how
    /// much they weigh.
    from: Timed,
    /// The animations of the state, with their times and weights.
    to: Timed,
    /// The animation whose methods run, and the times they run between.
    calls: Option<(usize, f32, f32)>,
    /// The animation that ended, for `animation_finished`.
    finished: Option<String>,
}

/// Moves the state machine of `node` on, if it is an animator with one:
/// takes a transition, gives the properties the values of the state's
/// animations (mixed with the state it fades from), runs the methods the
/// heaviest animation passed, and `animation_finished` when one that does
/// not loop ends. Returns whether `node` runs a machine, so its single
/// animation is not played too.
pub(crate) fn advance<H: Host>(node: &H::Node, delta: f32) -> bool {
    let Some(machine) = machine_of::<H>(node) else {
        return false;
    };
    let clips = H::with_animator(node, |animator| {
        animator.animations.read(|path| {
            if path.is_empty() {
                None
            } else {
                tracks::load(path)
            }
        })
    })
    .flatten();
    let Some(clips) = clips else {
        warn("an animator with states has no animations to play");
        return true;
    };
    let frame = H::with_animator(node, |animator| step(animator, &machine, clips, delta)).flatten();
    if let Some(frame) = frame {
        show::<H>(node, &frame);
    }
    true
}

/// Moves the machine `delta` seconds on and says what to show.
fn step(
    animator: &AnimatorFields,
    machine: &StateMachine,
    clips: Rc<Tracks>,
    delta: f32,
) -> Option<Frame> {
    let mut run = animator.machine.borrow_mut();
    if !animator.states.read(|path| *path == run.path) {
        run.restart(&animator.states.get(), &machine.start);
    }
    if run.stopped {
        return None;
    }
    run.transition(machine);
    run.triggers.clear();
    let state = machine.state(&run.state)?;
    let blend = blend_of(state, &clips, &run);
    let heaviest = blend.heaviest();
    if let Some(name) = heaviest.and_then(|index| clips.animations.get(index)) {
        if run.animation != name.name {
            name.name.clone_into(&mut run.animation);
        }
    }
    let from = fading_from(&run, machine, &clips);
    let to = blend.timed(
        &clips,
        run.progress,
        from.as_ref().map_or(1.0, |from| from.1),
    );
    let calls = heaviest.and_then(|index| {
        let clip = clips.animations.get(index)?;
        let shown = if run.shown < 0.0 {
            BEFORE_START
        } else {
            clip_time(clip, run.shown)
        };
        Some((index, shown, clip_time(clip, run.progress)))
    });
    let finished = ended(
        &mut run,
        heaviest.and_then(|index| clips.animations.get(index)),
    );
    run.shown = run.progress;
    run.progress = kept_small(run.progress + delta / blend.length(&clips));
    move_fade(&mut run, machine, &clips, delta);
    Some(Frame {
        from: from.map_or([None, None], |(timed, _)| timed),
        to,
        calls,
        finished,
        clips,
    })
}

/// The animations of the state the machine fades from, with their times
/// and weights, and the weight left for the state it fades to.
fn fading_from(run: &MachineRun, machine: &StateMachine, clips: &Tracks) -> Option<(Timed, f32)> {
    let fading = run.fading.as_ref()?;
    let state = machine.state(&fading.state)?;
    let amount = (fading.elapsed / fading.length).clamp(0.0, 1.0);
    let timed = blend_of(state, clips, run).timed(clips, fading.progress, 1.0 - amount);
    Some((timed, amount))
}

/// Moves the fade `delta` seconds on, ending it when it lasted its length.
fn move_fade(run: &mut MachineRun, machine: &StateMachine, clips: &Tracks, delta: f32) {
    let length = run
        .fading
        .as_ref()
        .and_then(|fading| machine.state(&fading.state))
        .map(|state| blend_of(state, clips, run).length(clips));
    let Some(fading) = &mut run.fading else {
        return;
    };
    fading.elapsed += delta;
    if fading.elapsed >= fading.length {
        run.fading = None;
    } else if let Some(length) = length {
        fading.progress += delta / length;
    }
}

/// `progress` with whole rounds past the first taken away: the same
/// times of animations, and still at their end once played, without
/// growing until decimals lose their precision.
fn kept_small(progress: f32) -> f32 {
    if progress >= 2.0 {
        1.0 + progress.fract()
    } else {
        progress
    }
}

/// The name of the animation that does not loop that ended in this frame,
/// once per time the state is entered.
fn ended(run: &mut MachineRun, clip: Option<&TrackClip>) -> Option<String> {
    let clip = clip.filter(|clip| !clip.looping)?;
    if run.ended || run.progress < 1.0 {
        return None;
    }
    run.ended = true;
    Some(clip.name.clone())
}

/// Shows `frame` on the animator's parent and the nodes under it, runs
/// the methods it passed and tells the animator an animation ended.
fn show<H: Host>(node: &H::Node, frame: &Frame) {
    if let Some(root) = nodes::parent::<H>(node) {
        apply::<H>(&root, frame);
        if let Some((index, shown, time)) = frame.calls {
            if let Some(clip) = frame.clips.animations.get(index) {
                tracks::call::<H>(&root, clip, shown, time);
            }
        }
    }
    if let Some(name) = &frame.finished {
        H::animation_finished(node, name);
    }
}

/// Gives the properties the values of the frame's animations: those of the
/// only one, or else mixed in a pose.
fn apply<H: Host>(root: &H::Node, frame: &Frame) {
    let mut parts = frame.from.iter().chain(&frame.to).flatten();
    let clip = |index: usize| frame.clips.animations.get(index);
    if let ([Some((index, time, _)), None], [None, None]) = (frame.to, frame.from) {
        if let Some(clip) = clip(index) {
            tracks::apply::<H>(root, clip, time);
        }
        return;
    }
    let mut pose = Pose::new();
    for &(index, time, weight) in &mut parts {
        if let Some(clip) = clip(index) {
            pose.add(clip, time, weight);
        }
    }
    pose.apply::<H>(root);
}

/// The time of `clip` after `progress` rounds: going round if it loops,
/// staying at its end if it does not.
fn clip_time(clip: &TrackClip, progress: f32) -> f32 {
    if clip.looping {
        (progress * clip.length).rem_euclid(clip.length)
    } else {
        progress.min(1.0) * clip.length
    }
}

/// The animations `state` plays with the machine's values now.
fn blend_of(state: &MachineState, clips: &Tracks, run: &MachineRun) -> Blend {
    let find = |name: &str| {
        let index = clips.position(name);
        if index.is_none() {
            warn(&format!(
                "the state \"{}\" plays \"{name}\", which the animations do not have",
                state.name
            ));
        }
        index
    };
    match &state.motion {
        Motion::Animation(name) => Blend {
            first: find(name).map(|index| (index, 1.0)),
            second: None,
        },
        Motion::Blend { value, points } => {
            let value = run.number(value);
            let after = points.partition_point(|point| point.at <= value);
            let (Some(before), Some(next)) = (
                after.checked_sub(1).and_then(|index| points.get(index)),
                points.get(after),
            ) else {
                let only = points.get(after.saturating_sub(1));
                return Blend {
                    first: only
                        .and_then(|point| find(&point.animation))
                        .map(|index| (index, 1.0)),
                    second: None,
                };
            };
            let amount = (value - before.at) / (next.at - before.at);
            Blend {
                first: find(&before.animation).map(|index| (index, 1.0 - amount)),
                second: find(&next.animation).map(|index| (index, amount)),
            }
        }
    }
}

impl Blend {
    fn parts(self) -> impl Iterator<Item = (usize, f32)> {
        self.first.into_iter().chain(self.second)
    }

    /// The animation that weighs the most; the second when both weigh the
    /// same, as values that cannot blend jump halfway.
    fn heaviest(self) -> Option<usize> {
        match (self.first, self.second) {
            (Some((first, weight)), Some((second, _))) => {
                Some(if weight > HALFWAY { first } else { second })
            }
            (first, second) => first.or(second).map(|(index, _)| index),
        }
    }

    /// Seconds a round of the blend lasts: its animations' lengths by
    /// their weights, so they keep in step.
    fn length(self, clips: &Tracks) -> f32 {
        let (length, weight) = self
            .parts()
            .fold((0.0, 0.0), |(length, total), (index, weight)| {
                let clip = clips.animations.get(index).map_or(0.0, |clip| clip.length);
                (length + clip * weight, total + weight)
            });
        if weight > 0.0 && length > 0.0 {
            length / weight
        } else {
            1.0
        }
    }

    /// Its animations with their times after `progress` rounds, weighing
    /// `share` of the pose in all.
    fn timed(self, clips: &Tracks, progress: f32, share: f32) -> Timed {
        let time = |part: Option<(usize, f32)>| {
            let (index, weight) = part?;
            let clip = clips.animations.get(index)?;
            Some((index, clip_time(clip, progress), weight * share))
        };
        [time(self.first), time(self.second)]
    }
}
