//! A program run by “Play” (Cargo, then the game), with its output sent
//! line by line as it comes, and stopped when asked.

use std::io::{self, BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use super::{OutputKind, OutputLine, RunEvent};

/// How often a running program is checked for having ended or for a stop
/// request: short enough that “Stop” feels immediate.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Names of the threads that read a program's output.
const READER_THREAD_NAME: &str = "play output";

/// How a program run with [`run`] ended.
#[derive(Debug)]
pub(super) enum Ended {
    /// It ended on its own.
    Exited(ExitStatus),
    /// It was stopped because the user asked.
    Stopped,
}

/// Where the run's events go, and what tells the editor to look at them.
#[derive(Clone)]
pub(super) struct Events {
    pub(super) sender: Sender<RunEvent>,
    pub(super) notify: Arc<dyn Fn() + Send + Sync>,
}

impl Events {
    /// Sends `event` and tells the editor. A closed editor is not waiting
    /// for it, and that is fine.
    pub(super) fn send(&self, event: RunEvent) {
        let _ = self.sender.send(event);
        (self.notify)();
    }
}

/// Runs `command`, sending each line it writes as output of `kind`, until it
/// ends or `stop` is set.
///
/// # Errors
///
/// Returns the error of the operating system if the program cannot start or
/// be waited for.
pub(super) fn run(
    mut command: Command,
    kind: OutputKind,
    events: &Events,
    stop: &AtomicBool,
) -> io::Result<Ended> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let readers = [
        child.stdout.take().map(|out| read_lines(out, kind, events)),
        child.stderr.take().map(|err| read_lines(err, kind, events)),
    ];
    let ended = supervise(&mut child, stop)?;
    if matches!(ended, Ended::Exited(_)) {
        // Every line arrives before the end is told. A stopped program is
        // not waited for: what it started may still hold its output open.
        for reader in readers.into_iter().flatten() {
            let _ = reader.map(thread::JoinHandle::join);
        }
    }
    Ok(ended)
}

/// Waits for `child` to end, or kills it when `stop` is set.
fn supervise(child: &mut Child, stop: &AtomicBool) -> io::Result<Ended> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Ended::Exited(status));
        }
        if stop.load(Ordering::Relaxed) {
            // It may have ended meanwhile: then there is nothing to kill.
            let _ = child.kill();
            child.wait()?;
            return Ok(Ended::Stopped);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Sends each line of `stream` on a thread of its own. Text that is not
/// UTF-8 is shown as well as it can be.
fn read_lines(
    stream: impl Read + Send + 'static,
    kind: OutputKind,
    events: &Events,
) -> io::Result<thread::JoinHandle<()>> {
    let events = events.clone();
    thread::Builder::new()
        .name(READER_THREAD_NAME.to_owned())
        .spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut bytes = Vec::new();
            while matches!(reader.read_until(b'\n', &mut bytes), Ok(read) if read > 0) {
                let text = String::from_utf8_lossy(&bytes).trim_end().to_owned();
                events.send(RunEvent::Output(OutputLine { kind, text }));
                bytes.clear();
            }
        })
}
