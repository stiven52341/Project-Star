//! Tweens: a property of a node taken to a value over some seconds, with
//! an ease (decision 80). `Node.tween` and its kin start one: it is a
//! coroutine, awaited until it ends, or started alone to run by itself.
//!
//! A tween reads where the property is when it starts (after its delay),
//! and changes it once per turn of coroutines. A new tween of the same
//! property of the same node takes its place: the old one ends where it
//! was. A tween also ends when its node leaves the game's tree, or with
//! `stop_tweens()`.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::field::Field;
use crate::nodes::Host;
use crate::scene_fields::SceneValue;
use crate::tracks::Ease;
use crate::{RuntimeError, Task, properties, tasks};

/// `repeats` that never end.
pub const FOREVER: i32 = -1;

/// The fields of a `TweenOptions`: what a tween does besides going to its
/// value once.
#[derive(Debug)]
pub struct TweenOptionsFields {
    /// `delay`: seconds it waits before it starts.
    pub delay: Field<f32>,
    /// `back`: whether it comes back to where it started after reaching
    /// its value, taking as long again.
    pub back: Field<bool>,
    /// `repeats`: how many times it plays (there, or there and back), or
    /// [`FOREVER`].
    pub repeats: Field<i32>,
}

impl TweenOptionsFields {
    /// What `TweenOptions()` gives: no delay, no way back, once.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            delay: Field::new(0.0),
            back: Field::new(false),
            repeats: Field::new(1),
        }
    }
}

impl Default for TweenOptionsFields {
    fn default() -> Self {
        Self::new()
    }
}

/// Where a tween takes its property.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Goal {
    /// To this decimal (`tween`).
    To(f32),
    /// This much past where it starts (`tween_by`).
    By(f32),
    /// To this color, as its channels (`tween_color`).
    Color([f32; 4]),
}

/// A tween to start.
#[derive(Debug, Clone, PartialEq)]
pub struct Tween {
    /// The property, by the name `.anim` tracks give it.
    pub property: String,
    /// Where it takes it.
    pub goal: Goal,
    /// How long it takes to reach its value.
    pub seconds: f32,
    /// How it goes there, by the name `.anim` files give it.
    pub ease: String,
    /// Seconds it waits before it starts.
    pub delay: f32,
    /// Whether it comes back to where it started.
    pub back: bool,
    /// How many times it plays, or [`FOREVER`].
    pub repeats: i32,
}

impl Tween {
    /// A tween of `property` to `goal` in `seconds` with the ease `ease`,
    /// once, at once.
    #[must_use]
    pub const fn new(property: String, goal: Goal, seconds: f32, ease: String) -> Self {
        Self {
            property,
            goal,
            seconds,
            ease,
            delay: 0.0,
            back: false,
            repeats: 1,
        }
    }

    /// The tween with what `options` say.
    #[must_use]
    pub fn with_options(self, options: &TweenOptionsFields) -> Self {
        Self {
            delay: options.delay.get(),
            back: options.back.get(),
            repeats: options.repeats.get(),
            ..self
        }
    }
}

/// A tween going on: its node, its property and the number that tells it
/// apart from the one that may take its place.
struct Running {
    node: u64,
    property: String,
    id: u64,
}

thread_local! {
    /// The tweens going on.
    static RUNNING: RefCell<Vec<Running>> = const { RefCell::new(Vec::new()) };
    /// The number the next tween gets.
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
}

/// Starts `tween` on `node`, as `node.tween(…)` and its kin do: the task
/// changes the property until the tween ends, `alive` stops holding (its
/// node left the game's tree), or another tween takes its place.
///
/// # Errors
///
/// A [`RuntimeError`] if the tween cannot be: an unknown ease, a property
/// the node does not have or that is not a decimal (a color, for
/// `tween_color`), negative seconds or delay, or repeats below 1 that are
/// not [`FOREVER`].
pub fn start<H: Host>(
    node: &H::Node,
    tween: Tween,
    alive: impl Fn() -> bool + 'static,
) -> Result<Task<()>, RuntimeError> {
    let ease = check::<H>(node, &tween)?;
    let node_id = H::node(node).id();
    let id = NEXT_ID.with(|next| next.replace(next.get() + 1));
    RUNNING.with(|running| {
        let mut running = running.borrow_mut();
        running.retain(|other| other.node != node_id || other.property != tween.property);
        running.push(Running {
            node: node_id,
            property: tween.property.clone(),
            id,
        });
    });
    let node = H::downgrade(node);
    Ok(Task::new(async move {
        let started = tasks::now();
        let mut from = None;
        loop {
            let going_on = is_running(id) && alive();
            let Some(node) = H::upgrade(&node).filter(|_| going_on) else {
                break;
            };
            let elapsed = tasks::now() - started - tween.delay;
            if elapsed >= 0.0 {
                let from = from.get_or_insert_with(|| {
                    properties::property::<H>(&node, &tween.property)
                        .unwrap_or(SceneValue::Float(0.0))
                });
                let (progress, done) = progress_at(&tween, elapsed);
                // A tween ends right on its value, whatever its ease (a
                // step only jumps there at the end).
                let amount = if done { progress } else { ease.apply(progress) };
                let value = between(from, tween.goal, amount);
                // Checked when it started: the property takes this kind.
                let _ = properties::set_property::<H>(&node, &tween.property, &value);
                if done {
                    break;
                }
            }
            NextTurn::new().await;
        }
        RUNNING.with(|running| running.borrow_mut().retain(|other| other.id != id));
    }))
}

/// Ends every tween of `node`: `stop_tweens()`.
pub fn stop<H: Host>(node: &H::Node) {
    let node_id = H::node(node).id();
    RUNNING.with(|running| running.borrow_mut().retain(|other| other.node != node_id));
}

/// How many tweens are going on.
#[must_use]
pub fn running() -> usize {
    RUNNING.with(|running| running.borrow().len())
}

fn is_running(id: u64) -> bool {
    RUNNING.with(|running| running.borrow().iter().any(|other| other.id == id))
}

/// Checks that `tween` can change its property of `node`. Returns its ease.
fn check<H: Host>(node: &H::Node, tween: &Tween) -> Result<Ease, RuntimeError> {
    let ease = Ease::named(&tween.ease).ok_or_else(|| {
        let names: Vec<&str> = Ease::NAMES.iter().map(|(name, _)| *name).collect();
        RuntimeError::new(format!(
            "the ease \"{}\" of a tween is not one of: {}",
            tween.ease,
            names.join(", ")
        ))
    })?;
    if !(tween.seconds >= 0.0 && tween.delay >= 0.0) {
        return Err(RuntimeError::new(
            "a tween cannot last or wait a negative time",
        ));
    }
    if tween.repeats < 1 && tween.repeats != FOREVER {
        return Err(RuntimeError::new(format!(
            "a tween repeats at least once, or {FOREVER} to repeat forever, not {}",
            tween.repeats
        )));
    }
    let current = properties::property::<H>(node, &tween.property).map_err(|reason| {
        RuntimeError::new(format!(
            "a tween cannot change `{}`: {reason}",
            tween.property
        ))
    })?;
    let fits = match tween.goal {
        Goal::To(_) | Goal::By(_) => matches!(current, SceneValue::Float(_)),
        Goal::Color(_) => matches!(current, SceneValue::Color(_)),
    };
    if !fits {
        let kind = match tween.goal {
            Goal::Color(_) => "a color",
            Goal::To(_) | Goal::By(_) => "a decimal (`tween_color` changes colors)",
        };
        return Err(RuntimeError::new(format!(
            "a tween cannot change `{}`: it is not {kind}",
            tween.property
        )));
    }
    Ok(ease)
}

/// How far from where it started to its value `tween` is after `elapsed`
/// seconds of playing, `0.0` to `1.0` before its ease, and whether it
/// ended.
fn progress_at(tween: &Tween, elapsed: f32) -> (f32, bool) {
    let end = if tween.back { 0.0 } else { 1.0 };
    if tween.seconds <= 0.0 {
        return (end, true);
    }
    let round = if tween.back {
        2.0 * tween.seconds
    } else {
        tween.seconds
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "repeats are far below where f32 loses whole numbers"
    )]
    let total = (tween.repeats != FOREVER).then_some(round * tween.repeats as f32);
    if total.is_some_and(|total| elapsed >= total) {
        return (end, true);
    }
    let within = elapsed % round;
    let progress = if within > tween.seconds {
        1.0 - (within - tween.seconds) / tween.seconds
    } else {
        within / tween.seconds
    };
    (progress, false)
}

/// The value `amount` of the way from `from` to `goal`.
fn between(from: &SceneValue, goal: Goal, amount: f32) -> SceneValue {
    match (from, goal) {
        (SceneValue::Color(start), Goal::Color(end)) => {
            let mut mixed = *start;
            for (channel, end) in mixed.iter_mut().zip(end) {
                *channel += (end - *channel) * amount;
            }
            SceneValue::Color(mixed)
        }
        (SceneValue::Float(start), Goal::To(end)) => {
            SceneValue::Float(start + (f64::from(end) - start) * f64::from(amount))
        }
        (SceneValue::Float(start), Goal::By(by)) => {
            SceneValue::Float(start + f64::from(by) * f64::from(amount))
        }
        _ => from.clone(),
    }
}

/// Waits for the next turn of coroutines, without allocating.
struct NextTurn {
    after: u64,
}

impl NextTurn {
    fn new() -> Self {
        Self {
            after: tasks::turns(),
        }
    }
}

impl Future for NextTurn {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<()> {
        if tasks::turns() > self.after {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::node_classes::SpriteFields;
    use crate::test_host::{TestHost, TestNode};

    const EPSILON: f32 = 1e-4;

    fn frame(delta: f32) {
        tasks::frame_started(delta);
        tasks::resume();
    }

    fn slide(to: f32, seconds: f32) -> Tween {
        Tween::new("x".to_owned(), Goal::To(to), seconds, "linear".to_owned())
    }

    /// Starts `tween` by itself, as a node's `update` does: the turn of
    /// coroutines of that frame follows.
    fn start_alone(node: &Rc<TestNode>, tween: Tween) {
        tasks::start(start::<TestHost>(node, tween, || true).unwrap());
        tasks::resume();
    }

    #[test]
    fn tweens_go_to_their_value_over_their_seconds_and_end() {
        tasks::clear();
        let hero = Rc::new(TestNode::new("Hero"));

        start_alone(&hero, slide(4.0, 1.0));
        frame(0.25);
        let quarter = hero.node.x.get();
        for _ in 0..4 {
            frame(0.25);
        }

        assert!((quarter - 1.0).abs() < EPSILON, "{quarter}");
        assert!((hero.node.x.get() - 4.0).abs() < EPSILON);
        assert_eq!(running(), 0);
    }

    #[test]
    fn tweens_by_add_to_where_the_property_starts_and_wait_their_delay() {
        tasks::clear();
        let hero = Rc::new(TestNode::new("Hero"));
        hero.node.y.set(1.0);
        let options = TweenOptionsFields::new();
        options.delay.set(0.5);
        let rise = Tween::new("y".to_owned(), Goal::By(2.0), 1.0, "linear".to_owned());

        start_alone(&hero, rise.with_options(&options));
        frame(0.5);
        let waited = hero.node.y.get();
        frame(0.5);

        assert!((waited - 1.0).abs() < EPSILON);
        assert!((hero.node.y.get() - 2.0).abs() < EPSILON);
    }

    #[test]
    fn tweens_with_back_come_back_and_repeat() {
        tasks::clear();
        let hero = Rc::new(TestNode::new("Hero"));
        let options = TweenOptionsFields::new();
        options.back.set(true);
        options.repeats.set(2);

        start_alone(&hero, slide(2.0, 1.0).with_options(&options));
        frame(1.0);
        let there = hero.node.x.get();
        frame(1.5);
        let coming_back = hero.node.x.get();
        for _ in 0..3 {
            frame(1.0);
        }

        assert!((there - 2.0).abs() < EPSILON);
        assert!((coming_back - 1.0).abs() < EPSILON, "{coming_back}");
        assert!(hero.node.x.get().abs() < EPSILON);
        assert_eq!(running(), 0);
    }

    #[test]
    fn a_new_tween_of_a_property_takes_the_place_of_the_old_one() {
        tasks::clear();
        let hero = Rc::new(TestNode::new("Hero"));

        start_alone(&hero, slide(10.0, 1.0));
        frame(0.5);
        start_alone(&hero, slide(0.0, 1.0));
        frame(0.5);

        assert!(
            (hero.node.x.get() - 2.5).abs() < EPSILON,
            "{}",
            hero.node.x.get()
        );
        assert_eq!(running(), 1);
        stop::<TestHost>(&hero);
        frame(0.5);
        assert_eq!(running(), 0);
        assert!((hero.node.x.get() - 2.5).abs() < EPSILON);
    }

    #[test]
    fn colors_blend_and_tweens_end_when_their_node_leaves() {
        tasks::clear();
        let mut hero = TestNode::new("Hero");
        hero.sprite = Some(SpriteFields::new("hero.png".to_owned()));
        let hero = Rc::new(hero);
        let alive = Rc::new(Cell::new(true));
        let red = Tween::new(
            "color".to_owned(),
            Goal::Color([1.0, 0.0, 0.0, 1.0]),
            1.0,
            "linear".to_owned(),
        );

        let watched = Rc::clone(&alive);
        tasks::start(start::<TestHost>(&hero, red, move || watched.get()).unwrap());
        tasks::resume();
        frame(0.5);
        alive.set(false);
        frame(0.5);

        let color = hero.sprite.as_ref().unwrap().color.get();
        assert!((color[1] - 0.5).abs() < EPSILON, "{color:?}");
        assert_eq!(running(), 0);
    }

    #[test]
    fn tweens_that_cannot_be_are_runtime_errors() {
        let hero = Rc::new(TestNode::new("Hero"));
        let error = |tween: Tween| {
            start::<TestHost>(&hero, tween, || true)
                .err()
                .map(|error| error.message().to_owned())
                .unwrap_or_default()
        };

        let bouncy = Tween::new("x".to_owned(), Goal::To(1.0), 1.0, "bouncy".to_owned());
        assert!(error(bouncy).contains("ease_in_out"));
        assert!(error(slide(1.0, -1.0)).contains("negative"));
        let mut twice = slide(1.0, 1.0);
        twice.repeats = 0;
        assert!(error(twice).contains("at least once"));
        let mut depth = slide(1.0, 1.0);
        depth.property = "z".to_owned();
        assert!(error(depth).contains("`z`"));
        let mut shown = slide(1.0, 1.0);
        shown.property = "visible".to_owned();
        assert!(error(shown).contains("not a decimal"));
    }
}
