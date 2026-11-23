//! Checking a project on a thread of its own, so the editor keeps drawing
//! while big projects are read and checked.

use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use super::{OpenTexts, ProjectCheck, check_project_with};

/// Name of the thread, as debuggers and crash reports show it.
const THREAD_NAME: &str = "project check";

/// A check of a project that runs in the background.
#[derive(Debug)]
pub(crate) struct CheckJob {
    receiver: Receiver<ProjectCheck>,
}

/// Where a [`CheckJob`] is.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CheckStatus {
    /// Still checking.
    Running,
    /// Done, with what it found.
    Finished(ProjectCheck),
    /// The thread stopped without an answer.
    Lost,
}

impl CheckJob {
    /// Starts checking the project in `directory`, with the texts of the
    /// files `open` in the editor; `on_finish` runs on the job's thread
    /// once the result is ready (e.g. to redraw the editor).
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system if it refused the thread.
    pub(crate) fn start(
        directory: PathBuf,
        open: OpenTexts,
        on_finish: impl FnOnce() + Send + 'static,
    ) -> io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || {
                // The editor may have closed the project meanwhile: then
                // nobody waits for the result, and that is fine.
                let _ = sender.send(check_project_with(&directory, &open));
                on_finish();
            })?;
        Ok(Self { receiver })
    }

    /// Where the job is, without waiting for it.
    pub(crate) fn status(&self) -> CheckStatus {
        match self.receiver.try_recv() {
            Ok(check) => CheckStatus::Finished(check),
            Err(TryRecvError::Empty) => CheckStatus::Running,
            Err(TryRecvError::Disconnected) => CheckStatus::Lost,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    use super::*;
    use crate::project::{ASSETS_DIRECTORY, MANIFEST_FILE_NAME, TestDirectory};

    /// Long enough for a tiny project on a slow machine.
    const TIMEOUT: Duration = Duration::from_secs(30);

    #[test]
    fn the_result_arrives_after_on_finish_is_called() {
        let directory = TestDirectory::new("check_job");
        fs::create_dir_all(directory.path().join(ASSETS_DIRECTORY)).expect("assets folder");
        fs::write(directory.path().join(MANIFEST_FILE_NAME), "").expect("manifest");
        fs::write(
            directory.path().join(ASSETS_DIRECTORY).join("a.star"),
            "class A {\n  var x = @\n}\n",
        )
        .expect("script");
        let (finished, finish_signal) = mpsc::channel();

        let job = CheckJob::start(
            directory.path().to_path_buf(),
            OpenTexts::new(),
            move || {
                let _ = finished.send(());
            },
        )
        .expect("thread started");

        assert_ne!(
            finish_signal.recv_timeout(TIMEOUT),
            Err(RecvTimeoutError::Timeout)
        );
        match job.status() {
            CheckStatus::Finished(check) => {
                assert_eq!(check.scripts, 1);
                assert_eq!(check.problems.len(), 1);
            }
            other => panic!("expected a finished check, got {other:?}"),
        }
    }
}
