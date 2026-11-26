//! Coroutines: the `async` methods of StarScript, run on the game's thread.
//!
//! An `async` method becomes a [`Task`], a Rust future. Calling it alone
//! [`start`]s it: it runs at once until its first wait, and is kept here.
//! Once per frame, after every `update`, [`resume`] lets each coroutine
//! whose wait is over go on until its next one. What coroutines wait for is
//! the game's own clock, which the game moves with [`frame_started`].
//!
//! Nothing here wakes futures: every waiting coroutine is simply polled
//! again each frame, which costs a check of its wait. Games have few
//! coroutines, and that keeps the runtime without dependencies.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

/// How long a console program sleeps between two turns of its coroutines.
const CONSOLE_FRAME: Duration = Duration::from_millis(16);

/// A running `async` method, which gives a `T` when it finishes.
pub struct Task<T> {
    future: Pin<Box<dyn Future<Output = T>>>,
    /// Whether the coroutine may go on; `None` for those that always can.
    alive: Option<Box<dyn Fn() -> bool>>,
}

impl<T> Task<T> {
    /// A task that runs `future` until it finishes.
    pub fn new(future: impl Future<Output = T> + 'static) -> Self {
        Self {
            future: Box::pin(future),
            alive: None,
        }
    }

    /// A task that, once started as a coroutine, ends as soon as `alive`
    /// says no: the coroutines of a node, which end when it leaves the tree.
    pub fn owned(
        future: impl Future<Output = T> + 'static,
        alive: impl Fn() -> bool + 'static,
    ) -> Self {
        Self {
            future: Box::pin(future),
            alive: Some(Box::new(alive)),
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    /// Awaited from another coroutine, a task runs as part of it: only the
    /// owner of the coroutine that was started decides whether it goes on.
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<T> {
        self.future.as_mut().poll(context)
    }
}

/// A started coroutine, whose value is no longer wanted.
struct Coroutine {
    future: Pin<Box<dyn Future<Output = ()>>>,
    alive: Option<Box<dyn Fn() -> bool>>,
}

impl Coroutine {
    fn is_alive(&self) -> bool {
        self.alive.as_ref().is_none_or(|alive| alive())
    }

    /// Runs it until its next wait; whether it finished.
    fn step(&mut self) -> bool {
        let mut context = Context::from_waker(Waker::noop());
        self.future.as_mut().poll(&mut context).is_ready()
    }
}

thread_local! {
    /// The coroutines waiting to go on.
    static WAITING: RefCell<Vec<Coroutine>> = const { RefCell::new(Vec::new()) };
    /// Seconds of the game: the sum of the frames' `delta`s.
    static NOW: Cell<f64> = const { Cell::new(0.0) };
    /// Seconds of the frame going on: the `delta` its `update`s get.
    static DELTA: Cell<f32> = const { Cell::new(0.0) };
    /// Frames whose coroutines already went on.
    static FRAME: Cell<u64> = const { Cell::new(0) };
}

/// Starts `task` as a coroutine: it runs at once until its first wait, and
/// then once per frame, until it finishes or its owner is gone.
pub fn start<T: 'static>(task: Task<T>) {
    let Task { future, alive } = task;
    let mut coroutine = Coroutine {
        future: Box::pin(async move {
            future.await;
        }),
        alive,
    };
    if !coroutine.step() {
        WAITING.with(|waiting| waiting.borrow_mut().push(coroutine));
    }
}

/// Moves the game's clock: a frame of `delta` seconds begins.
pub fn frame_started(delta: f32) {
    NOW.with(|now| now.set(now.get() + f64::from(delta)));
    DELTA.with(|current| current.set(delta));
}

/// Lets every coroutine whose wait is over go on until its next one.
/// Those of an owner that is gone end without going on. Coroutines started
/// meanwhile already ran their first part, so they wait for the next frame.
pub fn resume() {
    let mut running = WAITING.with(|waiting| std::mem::take(&mut *waiting.borrow_mut()));
    running.retain_mut(|coroutine| coroutine.is_alive() && !coroutine.step());
    WAITING.with(|waiting| {
        let mut waiting = waiting.borrow_mut();
        let started = std::mem::replace(&mut *waiting, running);
        waiting.extend(started);
    });
    FRAME.with(|frame| frame.set(frame.get() + 1));
}

/// How many coroutines are waiting.
#[must_use]
pub fn waiting() -> usize {
    WAITING.with(|waiting| waiting.borrow().len())
}

/// Drops every waiting coroutine without letting it go on: those of a
/// program that stopped.
pub fn clear() {
    let dropped = WAITING.with(|waiting| std::mem::take(&mut *waiting.borrow_mut()));
    // Dropped outside the borrow: a coroutine may own others' tasks.
    drop(dropped);
}

/// For console programs: lets the coroutines go on, with the clock moved
/// by the real time that passes, until none is left.
pub fn run_until_finished() {
    run_until_finished_or(|| false);
}

/// [`run_until_finished`], but it also returns as soon as `stop` holds
/// after a turn of coroutines: the interpreter stops there when one of
/// them failed.
pub fn run_until_finished_or(stop: impl Fn() -> bool) {
    let mut last = Instant::now();
    while waiting() > 0 {
        std::thread::sleep(CONSOLE_FRAME);
        let now = Instant::now();
        #[allow(clippy::cast_possible_truncation)]
        let delta = now.duration_since(last).as_secs_f64() as f32;
        last = now;
        frame_started(delta);
        resume();
        if stop() {
            return;
        }
    }
}

/// `Time.now()`: the seconds of the game so far, the sum of the frames'
/// `delta`s: the clock `Time.wait` counts with.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // `float` is how StarScript keeps decimals.
pub fn now() -> f32 {
    NOW.with(Cell::get) as f32
}

/// `Time.delta()`: the seconds of the frame going on, the same `delta`
/// that `update` gets, so code away from `update` can read it; 0 before
/// the first frame.
#[must_use]
pub fn delta() -> f32 {
    DELTA.with(Cell::get)
}

/// `Time.wait(seconds)`: waits until `seconds` of the game have passed,
/// counted from when it is awaited.
#[must_use]
pub fn wait(seconds: f32) -> Task<()> {
    Task::new(async move {
        let until = NOW.with(Cell::get) + f64::from(seconds);
        Until(move || NOW.with(Cell::get) >= until).await;
    })
}

/// `Time.next_frame()`: waits until the coroutines of the next frame go on.
#[must_use]
pub fn next_frame() -> Task<()> {
    Task::new(async {
        let frame = FRAME.with(Cell::get);
        Until(move || FRAME.with(Cell::get) > frame).await;
    })
}

/// A future that is ready once its condition holds.
struct Until<F>(F);

impl<F: Fn() -> bool + Unpin> Future for Until<F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<()> {
        if (self.0)() {
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

    /// A coroutine that writes each step it reaches into `log`.
    fn logged(
        log: &Rc<RefCell<Vec<&'static str>>>,
        steps: &'static [(&'static str, f32)],
    ) -> Task<()> {
        let log = Rc::clone(log);
        Task::new(async move {
            for (step, seconds) in steps {
                log.borrow_mut().push(step);
                wait(*seconds).await;
            }
            log.borrow_mut().push("end");
        })
    }

    /// One frame of `delta` seconds, as a game runs it.
    fn frame(delta: f32) {
        frame_started(delta);
        resume();
    }

    #[test]
    fn the_clock_is_the_sum_of_the_frames() {
        let start = now();

        frame(0.25);
        frame(0.5);

        assert!((now() - start - 0.75).abs() < 1e-6);
    }

    #[test]
    fn the_delta_is_the_last_frame_s() {
        frame(0.25);
        assert!((delta() - 0.25).abs() < f32::EPSILON);

        frame(0.5);
        assert!((delta() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn coroutines_run_at_once_until_their_first_wait() {
        clear();
        let log = Rc::new(RefCell::new(Vec::new()));

        start(logged(&log, &[("first", 1.0)]));

        assert_eq!(*log.borrow(), ["first"]);
        assert_eq!(waiting(), 1);
    }

    #[test]
    fn waits_end_in_the_first_frame_after_their_time() {
        clear();
        let log = Rc::new(RefCell::new(Vec::new()));
        start(logged(&log, &[("first", 0.5)]));

        frame(0.25);
        frame(0.2);
        assert_eq!(*log.borrow(), ["first"]);
        frame(0.1);

        assert_eq!(*log.borrow(), ["first", "end"]);
        assert_eq!(waiting(), 0);
    }

    #[test]
    fn next_frame_waits_for_the_next_turn() {
        clear();
        let steps = Rc::new(Cell::new(0));
        let counted = Rc::clone(&steps);
        start(Task::new(async move {
            loop {
                counted.set(counted.get() + 1);
                next_frame().await;
            }
        }));

        // Started during a frame: its turn of this frame is not a new frame.
        resume();
        assert_eq!(steps.get(), 1);
        frame(0.1);
        frame(0.1);

        assert_eq!(steps.get(), 3);
    }

    #[test]
    fn coroutines_whose_owner_is_gone_end_without_going_on() {
        clear();
        let log = Rc::new(RefCell::new(Vec::new()));
        let alive = Rc::new(Cell::new(true));
        let owner = Rc::clone(&alive);
        let Task { future, .. } = logged(&log, &[("first", 0.1)]);
        start(Task::owned(future, move || owner.get()));

        alive.set(false);
        frame(1.0);

        assert_eq!(*log.borrow(), ["first"]);
        assert_eq!(waiting(), 0);
    }

    #[test]
    fn awaited_tasks_give_their_value() {
        clear();
        let result = Rc::new(Cell::new(0));
        let written = Rc::clone(&result);
        start(Task::new(async move {
            let doubled = Task::new(async {
                wait(0.1).await;
                21 * 2
            });
            written.set(doubled.await);
        }));

        frame(0.2);

        assert_eq!(result.get(), 42);
    }

    #[test]
    fn coroutines_started_by_coroutines_wait_for_the_next_frame() {
        clear();
        let log = Rc::new(RefCell::new(Vec::new()));
        let inner = Rc::clone(&log);
        start(Task::new(async move {
            next_frame().await;
            start(logged(&inner, &[("inner", 0.05)]));
        }));

        // The first turn is the one of the frame it was started in.
        resume();
        frame(0.1);
        assert_eq!(*log.borrow(), ["inner"]);
        frame(0.1);

        assert_eq!(*log.borrow(), ["inner", "end"]);
    }

    #[test]
    fn cleared_coroutines_never_go_on() {
        clear();
        let log = Rc::new(RefCell::new(Vec::new()));
        start(logged(&log, &[("first", 0.1)]));

        clear();
        frame(1.0);

        assert_eq!(*log.borrow(), ["first"]);
        assert_eq!(waiting(), 0);
    }

    #[test]
    fn console_loops_stop_when_asked() {
        clear();
        let turns = Rc::new(Cell::new(0));
        let counted = Rc::clone(&turns);
        start(Task::new(async move {
            loop {
                counted.set(counted.get() + 1);
                next_frame().await;
            }
        }));
        let checks = Cell::new(0);

        run_until_finished_or(|| {
            checks.set(checks.get() + 1);
            checks.get() == 3
        });

        // Its first turn was the one it was started in.
        assert_eq!(turns.get(), 3);
        clear();
    }

    #[test]
    fn console_programs_run_their_coroutines_until_they_finish() {
        clear();
        let log = Rc::new(RefCell::new(Vec::new()));
        start(logged(&log, &[("first", 0.0), ("second", 0.0)]));

        run_until_finished();

        assert_eq!(*log.borrow(), ["first", "second", "end"]);
    }
}
