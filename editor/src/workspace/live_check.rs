//! Checking the project while scripts are written: a moment after the user
//! stops typing, the project is checked by a thread of its own with the texts
//! open in the editor, and the code editor marks the mistakes found.
//!
//! It is separate from “Compile”: its problems are only shown in the code,
//! and it never keeps a program for “Play”.

use std::path::Path;
use std::time::{Duration, Instant};

use eframe::egui;

use star_script::{Position, Span};

use crate::project_check::{CheckWorker, OpenTexts, Problem, ProgramOutline};

/// How long after the last change the check starts, so it does not run on
/// every key.
const QUIET_TIME: Duration = Duration::from_millis(500);

/// The check of what is being written, and what the last one found.
pub(super) struct LiveCheck {
    /// When the texts changed, if they did since the last check started.
    changed_at: Option<Instant>,
    /// The thread that checks, once started.
    worker: Option<CheckWorker>,
    problems: Vec<Problem>,
    /// What the last check that could read the program found it declares:
    /// while a line is half written the program cannot be read, and
    /// suggestions still need it.
    outline: Option<ProgramOutline>,
}

impl LiveCheck {
    pub(super) const fn new() -> Self {
        Self {
            changed_at: None,
            worker: None,
            problems: Vec::new(),
            outline: None,
        }
    }

    /// Checks again once the user stops changing the texts.
    pub(super) fn request(&mut self) {
        self.changed_at = Some(Instant::now());
    }

    /// What the program declares, as the last check that could read it
    /// found it.
    pub(super) const fn outline(&self) -> Option<&ProgramOutline> {
        self.outline.as_ref()
    }

    /// The file (inside the assets folder), line and column of the
    /// program's place `span`, as the last check that could read the
    /// program found them.
    pub(super) fn locate(&self, span: Span) -> Option<(String, Position)> {
        self.outline.as_ref()?.locate(span)
    }

    /// The problems the last check found.
    pub(super) fn problems(&self) -> &[Problem] {
        &self.problems
    }

    /// Takes the result of the newest check that finished, and asks for
    /// the next one when it is time, with the texts `open` gives; redraws
    /// the editor when there is something to do later.
    pub(super) fn update(
        &mut self,
        context: &egui::Context,
        directory: &Path,
        open: impl FnOnce() -> OpenTexts,
    ) {
        if let Some(check) = self.worker.as_ref().and_then(CheckWorker::latest) {
            self.problems = check.problems;
            if check.outline.is_some() {
                self.outline = check.outline;
            }
        }
        let Some(changed_at) = self.changed_at else {
            return;
        };
        if let Some(left) = QUIET_TIME.checked_sub(changed_at.elapsed()) {
            context.request_repaint_after(left);
            return;
        }
        self.changed_at = None;
        let asked = self
            .worker(context, directory)
            .is_some_and(|worker| worker.check(open()));
        if !asked {
            // The thread stopped: a new one is started on the next change.
            self.worker = None;
        }
    }

    /// The thread that checks, started the first time it is needed.
    fn worker(&mut self, context: &egui::Context, directory: &Path) -> Option<&CheckWorker> {
        if self.worker.is_none() {
            let redraw = context.clone();
            self.worker = CheckWorker::start(directory.to_path_buf(), move || {
                redraw.request_repaint();
            })
            .inspect_err(|error| log::warn!("Could not check the scripts while writing: {error}"))
            .ok();
        }
        self.worker.as_ref()
    }
}
