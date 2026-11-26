//! `parallel` functions: work on the other cores of the processor.
//!
//! [`run`] copies the arguments ([`Plain`]), runs the function on one of a
//! few threads kept for the whole program, and gives a [`Task`] that a
//! coroutine awaits: it is ready, with the result copied back, in the first
//! turn of the coroutines after the work ends. The threads start the first
//! time one is needed, one fewer than the processor's cores (at least one),
//! so the game's own thread keeps a core.
//!
//! A runtime error in the work stops the game from its own thread, when the
//! result is awaited (see [`fault`]).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use crate::fault::{WorkerFault, mark_worker_thread};
use crate::{Plain, Task, fault};

/// A piece of work for one of the threads.
type Job = Box<dyn FnOnce() + Send>;

/// What the work gives: its result, or why it stopped.
type Outcome<T> = Result<T, String>;

/// Name of the threads, as debuggers and crash reports show them.
const THREAD_NAME: &str = "starscript-parallel";

/// Where jobs go; `None` if no thread could be started.
static JOBS: OnceLock<Option<Sender<Job>>> = OnceLock::new();

/// Runs `work` on another thread with `arguments` (copies of plain values)
/// and gives a task with its result, copied back to this thread.
pub fn run<A, R>(arguments: A, work: fn(A) -> R) -> Task<R>
where
    A: Send + 'static,
    R: Plain + 'static,
{
    let (sender, receiver) = channel::<Outcome<R::Sent>>();
    let job: Job = Box::new(move || {
        let outcome = catch_unwind(AssertUnwindSafe(|| work(arguments).send()))
            .map_err(|payload| reason(&*payload));
        // The coroutine that waited may be gone: then nobody wants it.
        let _ = sender.send(outcome);
    });
    if let Err(job) = send(job) {
        // No thread could be started: the work runs here instead.
        job();
    }
    Task::new(Finished { receiver }.into_result())
}

/// Gives `job` to the threads, starting them the first time; gives it back
/// if there are none.
fn send(job: Job) -> Result<(), Job> {
    match JOBS.get_or_init(start_threads) {
        Some(jobs) => jobs.send(job).map_err(|error| error.0),
        None => Err(job),
    }
}

/// The stack of each thread: what a desktop's main thread gets, so the
/// calls [`crate::calls::MAX_CALL_DEPTH`] allows fit in it, even interpreted.
const WORKER_STACK: usize = 8 * 1024 * 1024;

/// Starts the threads, all taking jobs from one queue.
fn start_threads() -> Option<Sender<Job>> {
    let count = thread::available_parallelism()
        .map_or(1, |cores| cores.get().saturating_sub(1))
        .max(1);
    let (sender, receiver) = channel::<Job>();
    let receiver = Arc::new(Mutex::new(receiver));
    let started = (0..count)
        .filter(|_| {
            let receiver = Arc::clone(&receiver);
            thread::Builder::new()
                .name(THREAD_NAME.to_owned())
                .stack_size(WORKER_STACK)
                .spawn(move || work_on(&receiver))
                .is_ok()
        })
        .count();
    (started > 0).then_some(sender)
}

/// What each thread does: runs jobs until the program ends.
fn work_on(jobs: &Mutex<Receiver<Job>>) {
    mark_worker_thread();
    loop {
        // A job never unwinds past `catch_unwind`, so the lock is never
        // poisoned; if it were, the queue itself is still sound.
        let job = match jobs.lock() {
            Ok(jobs) => jobs.recv(),
            Err(poisoned) => poisoned.into_inner().recv(),
        };
        match job {
            Ok(job) => job(),
            Err(_) => return,
        }
    }
}

/// Why a job stopped: the message of its fault, or of another panic.
fn reason(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(WorkerFault(message)) = payload.downcast_ref::<WorkerFault>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "a `parallel` function stopped unexpectedly".to_owned()
    }
}

/// The end of a job, seen from the game's thread.
struct Finished<T> {
    receiver: Receiver<Outcome<T>>,
}

impl<T> Finished<T> {
    /// The result once the job ends; a fault of the job stops the game here.
    async fn into_result<R: Plain<Sent = T>>(self) -> R {
        loop {
            match self.receiver.try_recv() {
                Ok(Ok(sent)) => return R::receive(sent),
                Ok(Err(message)) => fault(&message),
                Err(TryRecvError::Disconnected) => {
                    fault("a `parallel` function stopped without a result")
                }
                Err(TryRecvError::Empty) => crate::tasks::next_frame().await,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::List;
    use crate::tasks::{frame_started, resume, start};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    /// How long a test waits for the threads before giving up.
    const PATIENCE: Duration = Duration::from_secs(10);

    fn sum((values,): (Vec<i64>,)) -> i64 {
        List::<i64>::receive(values).to_vec().iter().sum()
    }

    fn out_of_range((values,): (Vec<i64>,)) -> i64 {
        List::<i64>::receive(values).get(5)
    }

    /// Starts a coroutine that awaits `task` and keeps its value, and turns
    /// frames until it has one.
    fn awaited<R: 'static>(task: Task<R>) -> R {
        let result = Rc::new(RefCell::new(None));
        let written = Rc::clone(&result);
        start(Task::new(async move {
            *written.borrow_mut() = Some(task.await);
        }));
        let begun = Instant::now();
        loop {
            if let Some(value) = result.borrow_mut().take() {
                return value;
            }
            assert!(begun.elapsed() < PATIENCE, "the work never ended");
            thread::sleep(Duration::from_millis(1));
            frame_started(0.0);
            resume();
        }
    }

    #[test]
    fn work_runs_on_another_thread_with_copies() {
        let numbers = List::from_vec(vec![1_i64, 2, 3]);

        let total = awaited(run((numbers.send(),), sum));

        assert_eq!(total, 6);
    }

    #[test]
    fn results_are_copied_back() {
        fn doubled((values,): (Vec<i64>,)) -> List<i64> {
            List::from_vec(values.iter().map(|value| value * 2).collect())
        }

        let result = awaited(run((vec![1, 2],), doubled));

        assert_eq!(result.to_vec(), [2, 4]);
    }

    #[test]
    #[should_panic(expected = "StarScript runtime error: index 5 is out of range")]
    fn faults_stop_the_game_when_the_result_is_awaited() {
        awaited(run((vec![1],), out_of_range));
    }
}
