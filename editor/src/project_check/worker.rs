//! A thread that checks a project while scripts are written: it lives as
//! long as the project is open and takes one request after another, so no
//! thread is started on every pause in the typing.

use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use super::{CheckGoal, OpenTexts, ProjectCheck, check_project_with};

/// Name of the thread, as debuggers and crash reports show it.
const THREAD_NAME: &str = "live project check";

/// The thread that checks a project for [`CheckGoal::Writing`], and what
/// goes to it and comes back.
#[derive(Debug)]
pub(crate) struct CheckWorker {
    /// The texts open in the editor of each check asked for. Dropping it
    /// ends the thread.
    requests: Sender<OpenTexts>,
    results: Receiver<ProjectCheck>,
}

impl CheckWorker {
    /// Starts the thread that checks the project in `directory`;
    /// `on_finish` runs on it after each check (e.g. to redraw the editor).
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system if it refused the thread.
    pub(crate) fn start(
        directory: PathBuf,
        on_finish: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        let (requests, requested) = mpsc::channel::<OpenTexts>();
        let (finished, results) = mpsc::channel();
        thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || {
                while let Ok(mut open) = requested.recv() {
                    // Only the newest texts matter: older ones are skipped.
                    while let Ok(newer) = requested.try_recv() {
                        open = newer;
                    }
                    let check = check_project_with(&directory, &open, CheckGoal::Writing);
                    if finished.send(check).is_err() {
                        break;
                    }
                    on_finish();
                }
            })?;
        Ok(Self { requests, results })
    }

    /// Asks for a check with the texts `open` in the editor. Returns
    /// whether the thread is still there to do it.
    pub(crate) fn check(&self, open: OpenTexts) -> bool {
        self.requests.send(open).is_ok()
    }

    /// The newest check finished since the last call, without waiting.
    pub(crate) fn latest(&self) -> Option<ProjectCheck> {
        self.results.try_iter().last()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use super::*;
    use crate::project::{ASSETS_DIRECTORY, MANIFEST_FILE_NAME, TestDirectory};

    /// Long enough for a tiny project on a slow machine.
    const TIMEOUT: Duration = Duration::from_secs(30);

    #[test]
    fn one_thread_checks_request_after_request() {
        let directory = TestDirectory::new("check_worker");
        fs::create_dir_all(directory.path().join(ASSETS_DIRECTORY)).unwrap();
        fs::write(directory.path().join(MANIFEST_FILE_NAME), "").unwrap();
        // Open texts stand for files that are there.
        fs::write(directory.path().join(ASSETS_DIRECTORY).join("a.star"), "").unwrap();
        let worker = CheckWorker::start(directory.path().to_path_buf(), || {}).unwrap();
        let script = |text: &str| OpenTexts::from([("a.star".to_owned(), text.to_owned())]);

        assert!(worker.check(script("class A {\n  var x = @\n}\n")));
        let first = worker.results.recv_timeout(TIMEOUT).unwrap();
        assert!(worker.check(script("class A {\n  var x = 1\n}\n")));
        let second = worker.results.recv_timeout(TIMEOUT).unwrap();

        assert_eq!(first.problems.len(), 1);
        assert_eq!(second.problems, []);
        assert!(second.classes.is_some());
        // Checks while writing never translate the program.
        assert!(second.program.is_none());
    }
}
