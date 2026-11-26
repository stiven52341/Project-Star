//! Animators that play the keyframe animations of `.anim` files (decision
//! 79).
//!
//! An `Animator` names an `.anim` file in its `animations` field. Every
//! frame, after the animator's `update`, [`advance`] gives each property
//! its animation changes the value it has at that time, on the animator's
//! parent and the nodes under it. The files are read through the engine,
//! which sets the loader when the game starts ([`set_loader`]); each file
//! is read once. The game runs on one thread, so all of it is per thread.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::rc::Rc;

use crate::machines;
use crate::node_classes::{AnimatorFields, BEFORE_START};
use crate::nodes::{self, Host};
use crate::properties::set_property;
use crate::scene_fields::SceneValue;

/// The separator of the names in a node's path.
const PATH_SEPARATOR: char = '/';

/// The animations of an `.anim` file, with the values of scenes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tracks {
    /// The animations, each with a different name.
    pub animations: Vec<TrackClip>,
}

/// One animation of a [`Tracks`].
#[derive(Debug, Clone, PartialEq)]
pub struct TrackClip {
    /// What scripts play it by.
    pub name: String,
    /// How long it lasts, in seconds; above zero.
    pub length: f32,
    /// Whether it starts over at its end, or stays there.
    pub looping: bool,
    /// The properties it changes.
    pub tracks: Vec<Track>,
    /// The methods it calls.
    pub calls: Vec<TrackCall>,
}

/// A property of a node, changed from key to key.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    /// The node's path from the animator's parent: `""` is the parent,
    /// `"Arm/Hand"` the child `Hand` of its child `Arm`.
    pub node: String,
    /// The property, a field of the node's class.
    pub property: String,
    /// How the value goes from one key to the next.
    pub ease: Ease,
    /// The keys, in order of time; never empty.
    pub keys: Vec<TrackKey>,
}

/// The value a [`Track`] has at a time.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackKey {
    /// Seconds from the start of the animation.
    pub time: f32,
    /// The value then.
    pub value: SceneValue,
}

/// How a [`Track`] goes from a key to the next. Decimals and colors follow
/// it; whole numbers, `bool`s and texts always jump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ease {
    /// Keeps a key's value until the next key.
    Step,
    /// Changes at the same pace from key to key.
    Linear,
    /// Starts slowly and speeds up.
    In,
    /// Starts fast and slows down.
    Out,
    /// Starts and ends slowly.
    InOut,
}

/// A method called when the animation passes a time.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackCall {
    /// The node's path from the animator's parent, as a [`Track`]'s.
    pub node: String,
    /// Seconds from the start of the animation.
    pub time: f32,
    /// The method, which takes no arguments.
    pub method: String,
}

impl Tracks {
    pub(crate) fn position(&self, name: &str) -> Option<usize> {
        self.animations
            .iter()
            .position(|animation| animation.name == name)
    }
}

impl Ease {
    /// Every ease, with the word `.anim` files and tweens name it by.
    pub const NAMES: [(&'static str, Self); 5] = [
        ("step", Self::Step),
        ("linear", Self::Linear),
        ("ease_in", Self::In),
        ("ease_out", Self::Out),
        ("ease_in_out", Self::InOut),
    ];

    /// The ease called `name`, if one is.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .find_map(|&(word, ease)| (word == name).then_some(ease))
    }

    /// How far between two keys the value is, `0.0` to `1.0`, when the
    /// time is `progress` of the way.
    #[must_use]
    pub fn apply(self, progress: f32) -> f32 {
        let progress = progress.clamp(0.0, 1.0);
        match self {
            Self::Step => 0.0,
            Self::Linear => progress,
            Self::In => progress * progress,
            Self::Out => 1.0 - (1.0 - progress) * (1.0 - progress),
            Self::InOut if progress < 0.5 => 2.0 * progress * progress,
            Self::InOut => 1.0 - (2.0 - 2.0 * progress).powi(2) / 2.0,
        }
    }
}

impl Track {
    /// The value of the track `time` seconds into the animation: the first
    /// key's before it, the last key's after it, and between two keys what
    /// its ease gives.
    #[must_use]
    pub fn sample(&self, time: f32) -> Option<SceneValue> {
        let after = self.keys.partition_point(|key| key.time <= time);
        let Some(before) = after.checked_sub(1).and_then(|index| self.keys.get(index)) else {
            return self.keys.first().map(|key| key.value.clone());
        };
        let Some(next) = self.keys.get(after) else {
            return Some(before.value.clone());
        };
        let progress = (time - before.time) / (next.time - before.time);
        Some(blend(&before.value, &next.value, self.ease.apply(progress)))
    }
}

/// The value `amount` of the way from `from` to `to`: decimals and colors
/// blend; whole numbers, `bool`s and texts keep `from`.
fn blend(from: &SceneValue, to: &SceneValue, amount: f32) -> SceneValue {
    let number = |value: &SceneValue| match value {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an animated number is far below where f64 loses whole numbers"
        )]
        SceneValue::Int(number) => Some(*number as f64),
        SceneValue::Float(number) => Some(*number),
        _ => None,
    };
    let decimals = matches!(from, SceneValue::Float(_)) || matches!(to, SceneValue::Float(_));
    match (from, to) {
        (SceneValue::Color(start), SceneValue::Color(end)) => {
            let mut mixed = *start;
            for (channel, end) in mixed.iter_mut().zip(end) {
                *channel += (end - *channel) * amount;
            }
            SceneValue::Color(mixed)
        }
        _ if decimals => match (number(from), number(to)) {
            (Some(start), Some(end)) => {
                SceneValue::Float(start + (end - start) * f64::from(amount))
            }
            _ => from.clone(),
        },
        _ => from.clone(),
    }
}

/// The value `amount` of the way from `from` to `to` when two animations
/// are mixed: decimals and colors blend, and whole numbers, `bool`s and
/// texts jump to `to` halfway.
pub(crate) fn mix(from: &SceneValue, to: &SceneValue, amount: f32) -> SceneValue {
    let continuous = matches!(
        (from, to),
        (SceneValue::Color(_), SceneValue::Color(_))
            | (
                SceneValue::Float(_),
                SceneValue::Float(_) | SceneValue::Int(_)
            )
            | (SceneValue::Int(_), SceneValue::Float(_))
    );
    if continuous {
        blend(from, to, amount)
    } else if amount >= HALFWAY {
        to.clone()
    } else {
        from.clone()
    }
}

/// Where a mix of values that cannot blend jumps to the second.
const HALFWAY: f32 = 0.5;

/// Reads the `.anim` file at an asset path, or explains why it cannot.
type Loader = Box<dyn Fn(&str) -> Result<Tracks, String>>;

/// A read file, or `None` if it could not be read.
type Loaded = Option<Rc<Tracks>>;

thread_local! {
    static LOADER: RefCell<Option<Loader>> = const { RefCell::new(None) };
    static LOADED: RefCell<HashMap<String, Loaded>> = RefCell::new(HashMap::new());
    /// The warnings written, so a track that fails every frame writes once.
    static WARNED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Sets how `.anim` files are read. Called once, before the game is
/// created.
pub fn set_loader(loader: impl Fn(&str) -> Result<Tracks, String> + 'static) {
    LOADER.with(|current| {
        if let Ok(mut current) = current.try_borrow_mut() {
            *current = Some(Box::new(loader));
        }
    });
    LOADED.with(|loaded| {
        if let Ok(mut loaded) = loaded.try_borrow_mut() {
            loaded.clear();
        }
    });
    WARNED.with(|warned| {
        if let Ok(mut warned) = warned.try_borrow_mut() {
            warned.clear();
        }
    });
}

/// `func play`: plays the animation `name` of the animator's `animations`
/// from its start, unless it is already playing it.
pub fn play<H: Host>(node: &H::Node, name: &str) {
    if machines::runs_machine::<H>(node) {
        machines::jump::<H>(node, name);
        return;
    }
    H::with_animator(node, |animator| {
        let same = animator.playing.get() && animator.animation.read(|current| current == name);
        if !same {
            animator.animation.set(name.to_owned());
            animator.time.set(0.0);
            animator.shown.set(BEFORE_START);
            animator.playing.set(true);
        }
    });
}

/// `func stop`: stops the animation, leaving every property as it is.
pub fn stop<H: Host>(node: &H::Node) {
    H::with_animator(node, |animator| {
        animator.playing.set(false);
        animator.machine.borrow_mut().stop();
    });
}

/// `func is_playing`: whether the animation is playing; one that does not
/// loop stops at its end.
#[must_use]
pub fn is_playing<H: Host>(node: &H::Node) -> bool {
    let machine = machines::runs_machine::<H>(node);
    H::with_animator(node, |animator| {
        if machine {
            animator.machine.borrow().is_playing()
        } else {
            animator.playing.get()
        }
    })
    .unwrap_or(false)
}

/// `func animation`: the animation the animator plays, or played last;
/// empty before any.
#[must_use]
pub fn animation<H: Host>(node: &H::Node) -> String {
    let machine = machines::runs_machine::<H>(node);
    H::with_animator(node, |animator| {
        if machine {
            animator.machine.borrow().animation().to_owned()
        } else {
            animator.animation.get()
        }
    })
    .unwrap_or_default()
}

/// What an animator plays this frame.
struct Playing {
    file: Rc<Tracks>,
    index: usize,
    /// Seconds into the animation shown now.
    time: f32,
    /// Seconds into the animation shown in the last frame.
    shown: f32,
}

/// Moves the animation of `node`, if it is an animator playing one,
/// `delta` seconds on: each property it changes gets the value it has at
/// the time it is at, and the methods whose times it passed since the last
/// frame run (once each, however many times a long frame went round a
/// looping animation). Called after the node's `update`, so an animation
/// played there shows its start in the same frame. When an animation that
/// does not loop ends, the node's `animation_finished` runs with its name.
pub fn advance<H: Host>(node: &H::Node, delta: f32) {
    if machines::advance::<H>(node, delta) {
        return;
    }
    let Some(playing) = H::with_animator(node, playing).flatten() else {
        return;
    };
    let Some(clip) = playing.file.animations.get(playing.index) else {
        return;
    };
    let root = nodes::parent::<H>(node);
    if let Some(root) = &root {
        apply::<H>(root, clip, playing.time);
    }
    // The animator is moved on before any script runs, so a method called
    // here can play another animation without this one undoing it.
    let ended = H::with_animator(node, |animator| {
        let time = animator.time.get();
        let ended = !clip.looping && time >= clip.length;
        // A looping animation's time goes round its length, so it never
        // grows without end.
        let kept = if clip.looping { playing.time } else { time };
        animator.time.set(kept + delta);
        animator.shown.set(playing.time);
        if ended {
            animator.playing.set(false);
        }
        ended
    })
    .unwrap_or(false);
    if let Some(root) = &root {
        call::<H>(root, clip, playing.shown, playing.time);
    }
    if ended {
        H::animation_finished(node, &clip.name);
    }
}

/// The animation `animator` plays and the time it is at, if it plays one
/// that can be found.
fn playing(animator: &AnimatorFields) -> Option<Playing> {
    if !animator.playing.get() {
        return None;
    }
    let file = animator
        .animations
        .read(|path| if path.is_empty() { None } else { load(path) })?;
    let found = animator.animation.read(|name| {
        let index = file.position(name);
        if index.is_none() {
            animator.animations.read(|path| {
                warn(&format!(
                    "the animations \"{path}\" have no animation \"{name}\""
                ));
            });
        }
        index
    });
    let Some(index) = found else {
        animator.playing.set(false);
        return None;
    };
    let clip = file.animations.get(index)?;
    let time = animator.time.get();
    let time = if clip.looping {
        time.rem_euclid(clip.length)
    } else {
        time.min(clip.length)
    };
    let shown = animator.shown.get();
    Some(Playing {
        file,
        index,
        time,
        shown,
    })
}

/// Runs the methods `clip` calls after `shown` and up to `time`: past its
/// end and from its start when it went round.
pub(crate) fn call<H: Host>(root: &H::Node, clip: &TrackClip, shown: f32, time: f32) {
    let passed = |at: f32| {
        if time >= shown {
            shown < at && at <= time
        } else {
            shown < at || at <= time
        }
    };
    for method in clip.calls.iter().filter(|method| passed(method.time)) {
        let called = find::<H>(root, &method.node)
            .ok_or_else(|| format!("\"{}\" is not there", method.node))
            .and_then(|target| H::call_method(&target, &method.method));
        if let Err(reason) = called {
            warn(&format!(
                "the animation \"{}\" cannot call `{}`: {reason}",
                clip.name, method.method
            ));
        }
    }
}

/// Gives the properties `clip` changes, on `root` and the nodes under it,
/// their values `time` seconds into it.
pub(crate) fn apply<H: Host>(root: &H::Node, clip: &TrackClip, time: f32) {
    for track in &clip.tracks {
        if let Some(value) = track.sample(time) {
            set_track_value::<H>(root, &clip.name, &track.node, &track.property, &value);
        }
    }
}

/// Gives `property` of the node at `path` from `root` the value an
/// animation called `animation` has for it, or warns why it cannot.
pub(crate) fn set_track_value<H: Host>(
    root: &H::Node,
    animation: &str,
    path: &str,
    property: &str,
    value: &SceneValue,
) {
    let Some(target) = find::<H>(root, path) else {
        warn(&format!(
            "the animation \"{animation}\" changes `{property}` of \"{path}\", which is not there"
        ));
        return;
    };
    if let Err(reason) = set_property::<H>(&target, property, value) {
        warn(&format!(
            "the animation \"{animation}\" cannot change `{property}`: {reason}"
        ));
    }
}

/// The node at `path` from `root`: `root` itself for an empty path.
fn find<H: Host>(root: &H::Node, path: &str) -> Option<H::Node> {
    if path.is_empty() {
        return Some(root.clone());
    }
    path.split(PATH_SEPARATOR)
        .try_fold(root.clone(), |node, name| {
            nodes::find_child::<H>(&node, name)
        })
}

/// The `.anim` file at `path`, read the first time it is needed. `None` if
/// it cannot be read: the reason is written once.
pub(crate) fn load(path: &str) -> Loaded {
    if let Some(loaded) = LOADED.with(|loaded| loaded.borrow().get(path).cloned()) {
        return loaded;
    }
    let result = LOADER.with(|loader| match &*loader.borrow() {
        Some(loader) => loader(path),
        None => Err("animations can only be loaded while a game runs".to_owned()),
    });
    let loaded = match result {
        Ok(tracks) => Some(Rc::new(tracks)),
        Err(reason) => {
            warn(&format!("cannot load the animations \"{path}\": {reason}"));
            None
        }
    };
    LOADED.with(|cache| {
        cache.borrow_mut().insert(path.to_owned(), loaded.clone());
    });
    loaded
}

/// Writes `message` as a warning, once.
pub(crate) fn warn(message: &str) {
    let first = WARNED.with(|warned| warned.borrow_mut().insert(message.to_owned()));
    if first {
        // Nothing can be reported if the error output itself fails.
        let _ = writeln!(std::io::stderr(), "warning: {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_host::{TestHost, TestNode};

    const EPSILON: f32 = 1e-5;

    fn key(time: f32, value: SceneValue) -> TrackKey {
        TrackKey { time, value }
    }

    fn track(node: &str, property: &str, ease: Ease, keys: Vec<TrackKey>) -> Track {
        Track {
            node: node.to_owned(),
            property: property.to_owned(),
            ease,
            keys,
        }
    }

    fn float(value: Option<SceneValue>) -> f64 {
        match value {
            Some(SceneValue::Float(number)) => number,
            other => panic!("not a decimal: {other:?}"),
        }
    }

    #[test]
    fn eases_start_and_end_on_the_keys() {
        for ease in [Ease::Linear, Ease::In, Ease::Out, Ease::InOut] {
            assert!(ease.apply(0.0).abs() < EPSILON, "{ease:?}");
            assert!((ease.apply(1.0) - 1.0).abs() < EPSILON, "{ease:?}");
        }
        assert!(Ease::In.apply(0.5) < 0.5);
        assert!(Ease::Out.apply(0.5) > 0.5);
        assert!((Ease::InOut.apply(0.5) - 0.5).abs() < EPSILON);
        assert!(Ease::Step.apply(0.9).abs() < EPSILON);
    }

    #[test]
    fn decimals_blend_between_keys_and_hold_outside_them() {
        let slide = track(
            "",
            "x",
            Ease::Linear,
            vec![
                key(1.0, SceneValue::Float(0.0)),
                key(3.0, SceneValue::Int(4)),
            ],
        );

        assert!(float(slide.sample(0.0)).abs() < 1e-6);
        assert!((float(slide.sample(2.0)) - 2.0).abs() < 1e-6);
        assert_eq!(slide.sample(5.0), Some(SceneValue::Int(4)));
    }

    #[test]
    fn whole_numbers_bools_and_steps_jump() {
        let frames = track(
            "",
            "frame",
            Ease::Linear,
            vec![key(0.0, SceneValue::Int(0)), key(1.0, SceneValue::Int(4))],
        );
        let stepped = track(
            "",
            "x",
            Ease::Step,
            vec![
                key(0.0, SceneValue::Float(0.0)),
                key(1.0, SceneValue::Float(4.0)),
            ],
        );

        assert_eq!(frames.sample(0.9), Some(SceneValue::Int(0)));
        assert_eq!(frames.sample(1.0), Some(SceneValue::Int(4)));
        assert_eq!(stepped.sample(0.9), Some(SceneValue::Float(0.0)));
    }

    #[test]
    fn colors_blend_channel_by_channel() {
        let fade = track(
            "",
            "color",
            Ease::Linear,
            vec![
                key(0.0, SceneValue::Color([1.0, 1.0, 1.0, 1.0])),
                key(1.0, SceneValue::Color([0.0, 1.0, 1.0, 0.0])),
            ],
        );

        assert_eq!(
            fade.sample(0.5),
            Some(SceneValue::Color([0.5, 1.0, 1.0, 0.5]))
        );
    }

    fn wave() -> Tracks {
        Tracks {
            animations: vec![
                TrackClip {
                    name: "wave".to_owned(),
                    length: 1.0,
                    looping: true,
                    tracks: vec![track(
                        "Arm",
                        "rotation",
                        Ease::Linear,
                        vec![
                            key(0.0, SceneValue::Float(0.0)),
                            key(1.0, SceneValue::Float(1.0)),
                        ],
                    )],
                    calls: vec![
                        TrackCall {
                            node: String::new(),
                            time: 0.0,
                            method: "start".to_owned(),
                        },
                        TrackCall {
                            node: "Arm".to_owned(),
                            time: 0.5,
                            method: "hit".to_owned(),
                        },
                        TrackCall {
                            node: "Arm".to_owned(),
                            time: 0.5,
                            method: "missing".to_owned(),
                        },
                    ],
                },
                TrackClip {
                    name: "fade".to_owned(),
                    length: 0.5,
                    looping: false,
                    tracks: vec![track(
                        "",
                        "visible",
                        Ease::Step,
                        vec![
                            key(0.0, SceneValue::Bool(true)),
                            key(0.5, SceneValue::Bool(false)),
                        ],
                    )],
                    calls: Vec::new(),
                },
            ],
        }
    }

    /// A hero with an arm and an animator of `wave.anim`.
    fn hero() -> (Rc<TestNode>, Rc<TestNode>, Rc<TestNode>) {
        set_loader(|path| match path {
            "wave.anim" => Ok(wave()),
            _ => Err("it does not exist".to_owned()),
        });
        let hero = Rc::new(TestNode::new("Hero"));
        let arm = Rc::new(TestNode::new("Arm"));
        let mut animator = TestNode::new("Animator");
        animator.animator = Some(AnimatorFields::new());
        let animator = Rc::new(animator);
        nodes::add_child::<TestHost>(&hero, Rc::clone(&arm));
        nodes::add_child::<TestHost>(&hero, Rc::clone(&animator));
        if let Some(fields) = &animator.animator {
            fields.animations.set("wave.anim".to_owned());
        }
        (hero, arm, animator)
    }

    #[test]
    fn animators_change_the_nodes_under_their_parent_and_loop() {
        let (_hero, arm, animator) = hero();

        play::<TestHost>(&animator, "wave");
        let rotations: Vec<f32> = (0..4)
            .map(|_| {
                advance::<TestHost>(&animator, 0.4);
                arm.node.rotation.get()
            })
            .collect();

        let expected = [0.0, 0.4, 0.8, 0.2];
        for (rotation, expected) in rotations.iter().zip(expected) {
            assert!((rotation - expected).abs() < EPSILON, "{rotations:?}");
        }
        assert!(is_playing::<TestHost>(&animator));
        assert_eq!(animation::<TestHost>(&animator), "wave");
    }

    #[test]
    fn animations_that_do_not_loop_end_on_their_last_values() {
        let (hero, _arm, animator) = hero();

        play::<TestHost>(&animator, "fade");
        advance::<TestHost>(&animator, 0.3);
        advance::<TestHost>(&animator, 0.3);
        assert!(hero.node.visible.get());
        assert_eq!(animator.calls(), Vec::<String>::new());
        advance::<TestHost>(&animator, 0.3);

        assert!(!hero.node.visible.get());
        assert!(!is_playing::<TestHost>(&animator));
        assert_eq!(animator.calls(), ["finished fade"]);
        advance::<TestHost>(&animator, 0.3);
        assert_eq!(animator.calls(), ["finished fade"], "it ends once");
    }

    #[test]
    fn playing_the_same_animation_again_goes_on_and_stop_keeps_the_values() {
        let (_hero, arm, animator) = hero();

        play::<TestHost>(&animator, "wave");
        advance::<TestHost>(&animator, 0.5);
        play::<TestHost>(&animator, "wave");
        advance::<TestHost>(&animator, 0.25);
        stop::<TestHost>(&animator);
        advance::<TestHost>(&animator, 0.25);

        assert!((arm.node.rotation.get() - 0.5).abs() < EPSILON);
        assert!(!is_playing::<TestHost>(&animator));
    }

    #[test]
    fn methods_run_when_the_animation_passes_their_time_also_going_round() {
        let (hero, arm, animator) = hero();

        play::<TestHost>(&animator, "wave");
        let mut calls = Vec::new();
        for _ in 0..4 {
            advance::<TestHost>(&animator, 0.4);
            calls.push((hero.calls().len(), arm.calls().len()));
        }

        // At 0.0, 0.4, 0.8 (past the hit) and 0.2 (round to the start).
        assert_eq!(calls, [(1, 0), (1, 0), (1, 1), (2, 1)]);
        assert_eq!(hero.calls(), ["call start", "call start"]);
        assert_eq!(arm.calls(), ["call hit"]);
    }

    #[test]
    fn playing_again_calls_the_methods_at_the_start_again() {
        let (hero, _arm, animator) = hero();

        play::<TestHost>(&animator, "wave");
        advance::<TestHost>(&animator, 0.1);
        play::<TestHost>(&animator, "fade");
        play::<TestHost>(&animator, "wave");
        advance::<TestHost>(&animator, 0.1);

        assert_eq!(hero.calls(), ["call start", "call start"]);
    }

    #[test]
    fn missing_animations_stop_the_animator() {
        let (_hero, _arm, animator) = hero();

        play::<TestHost>(&animator, "dance");
        advance::<TestHost>(&animator, 0.1);

        assert!(!is_playing::<TestHost>(&animator));
    }
}
