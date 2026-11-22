//! “Problems”: what the last “Compile” found in the project.

use std::path::Path;

use eframe::egui;
use star_script::codegen::GeneratedProgram;

use crate::project_check::{CheckJob, CheckStatus, Problem, ProjectCheck};
use crate::style::{ERROR_COLOR, SUCCESS_COLOR};
use crate::texts;

/// Space between the place of a problem and its message.
const PLACE_SPACING: f32 = 8.0;

/// The checks of the project and their result.
pub(crate) struct ProblemsPanel {
    state: CompileState,
}

/// How a check that has just finished went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckEnd {
    /// No problems.
    Clean,
    /// Problems were found, or the check gave no answer.
    WithProblems,
}

enum CompileState {
    NotCompiled,
    Running(CheckJob),
    Finished(ProjectCheck),
    /// The check could not start or stopped without an answer.
    Failed(String),
}

impl ProblemsPanel {
    pub(crate) const fn new() -> Self {
        Self {
            state: CompileState::NotCompiled,
        }
    }

    pub(crate) const fn is_compiling(&self) -> bool {
        matches!(self.state, CompileState::Running(_))
    }

    /// Starts checking the project in `directory`, unless a check is running.
    pub(crate) fn compile(&mut self, directory: &Path, context: &egui::Context) {
        if self.is_compiling() {
            return;
        }
        let context = context.clone();
        self.state = match CheckJob::start(directory.to_path_buf(), move || {
            context.request_repaint();
        }) {
            Ok(job) => CompileState::Running(job),
            Err(error) => CompileState::Failed(texts::current().check_not_started(&error)),
        };
    }

    /// Takes the result of the running check if it arrived. Returns how a
    /// check that has just finished went, so its problems can be shown.
    pub(crate) fn update(&mut self) -> Option<CheckEnd> {
        let CompileState::Running(job) = &self.state else {
            return None;
        };
        match job.status() {
            CheckStatus::Running => None,
            CheckStatus::Finished(check) => {
                let end = if check.problems.is_empty() {
                    CheckEnd::Clean
                } else {
                    CheckEnd::WithProblems
                };
                self.state = CompileState::Finished(check);
                Some(end)
            }
            CheckStatus::Lost => {
                self.state = CompileState::Failed(texts::current().check_lost.to_owned());
                Some(CheckEnd::WithProblems)
            }
        }
    }

    /// The game's Rust code, if the last check found no problems.
    pub(crate) fn program(&self) -> Option<&GeneratedProgram> {
        match &self.state {
            CompileState::Finished(check) => check.program.as_ref(),
            _ => None,
        }
    }

    /// The tab's title: “Problems”, with how many there are.
    pub(crate) fn title(&self) -> String {
        match &self.state {
            CompileState::Finished(check) if !check.problems.is_empty() => {
                texts::current().problems_title(check.problems.len())
            }
            _ => texts::current().problems.to_owned(),
        }
    }

    /// One line about the last check, for the status bar.
    pub(crate) fn show_summary(&self, ui: &mut egui::Ui) {
        match &self.state {
            CompileState::NotCompiled => {}
            CompileState::Running(_) => {
                ui.spinner();
                ui.weak(texts::current().compiling);
            }
            CompileState::Finished(check) if check.problems.is_empty() => {
                ui.colored_label(SUCCESS_COLOR, texts::current().check_passed(check.scripts));
            }
            CompileState::Finished(check) => {
                ui.colored_label(
                    ERROR_COLOR,
                    texts::current().problems_found(check.problems.len()),
                );
            }
            CompileState::Failed(_) => {
                ui.colored_label(ERROR_COLOR, texts::current().check_failed);
            }
        }
    }

    /// What the last check found; nothing if none finished.
    pub(crate) fn problems(&self) -> &[Problem] {
        match &self.state {
            CompileState::Finished(check) => &check.problems,
            _ => &[],
        }
    }

    /// Lists the problems. Returns the one in a blueprint the user clicked,
    /// to show it there.
    pub(crate) fn show(&self, ui: &mut egui::Ui) -> Option<Problem> {
        let mut clicked = None;
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| match &self.state {
                CompileState::NotCompiled => {
                    ui.weak(texts::current().not_compiled);
                }
                CompileState::Running(_) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.weak(texts::current().compiling);
                    });
                }
                CompileState::Finished(check) if check.problems.is_empty() => {
                    ui.weak(texts::current().check_passed(check.scripts));
                }
                CompileState::Finished(check) => {
                    for problem in &check.problems {
                        if show_problem(ui, problem) {
                            clicked = Some(problem.clone());
                        }
                    }
                }
                CompileState::Failed(reason) => {
                    ui.colored_label(ERROR_COLOR, reason);
                }
            });
        clicked
    }
}

/// One problem; the place of those in blueprints can be clicked. Returns
/// whether it was.
fn show_problem(ui: &mut egui::Ui, problem: &Problem) -> bool {
    let place = problem.position.map_or_else(
        || problem.file.clone(),
        |position| format!("{}:{}:{}", problem.file, position.line, position.column),
    );
    ui.horizontal_wrapped(|ui| {
        let place = egui::RichText::new(place).monospace().color(ERROR_COLOR);
        let clicked = if problem.origin.is_some() {
            ui.add(egui::Link::new(place))
                .on_hover_text(texts::current().show_in_blueprint)
                .clicked()
        } else {
            ui.label(place);
            false
        };
        ui.add_space(PLACE_SPACING);
        ui.label(&problem.message);
        clicked
    })
    .inner
}
