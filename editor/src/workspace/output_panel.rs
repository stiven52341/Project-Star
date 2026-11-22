//! “Output”: the game being built and played, and what Cargo and the game
//! write while it is.

use std::collections::VecDeque;

use eframe::egui;

use crate::game_run::{GameRun, OutputKind, OutputLine, PlayRequest, RunEnd, RunEvent, RunPhase};
use crate::style::{ERROR_COLOR, SUCCESS_COLOR};
use crate::texts;

/// Most lines kept: a game that prints every frame never fills the memory.
const MAX_LINES: usize = 5000;

/// The game of the project while it is built and played, and its output.
pub(crate) struct OutputPanel {
    run: Option<GameRun>,
    phase: Option<RunPhase>,
    lines: VecDeque<OutputLine>,
    /// A message of the editor about the last “Play”, shown after the lines.
    note: Option<Note>,
}

/// A message of the editor, and whether it tells of something wrong.
struct Note {
    text: String,
    is_error: bool,
}

impl OutputPanel {
    pub(crate) const fn new() -> Self {
        Self {
            run: None,
            phase: None,
            lines: VecDeque::new(),
            note: None,
        }
    }

    /// Whether the game is being built or played.
    pub(crate) fn is_playing(&self) -> bool {
        matches!(self.phase, Some(RunPhase::Building | RunPhase::Running))
    }

    /// Builds and plays `request`, forgetting the last run's output.
    pub(crate) fn play(&mut self, request: PlayRequest, context: &egui::Context) {
        self.clear();
        let context = context.clone();
        match GameRun::start(request, move || context.request_repaint()) {
            Ok(run) => {
                self.run = Some(run);
                self.phase = Some(RunPhase::Building);
            }
            Err(error) => self.set_note(texts::current().play_not_started(&error), true),
        }
    }

    /// Stops the build or the game.
    pub(crate) fn stop(&self) {
        if let Some(run) = &self.run {
            run.stop();
        }
    }

    /// Says why “Play” did not start the game, instead of its output.
    pub(crate) fn refuse(&mut self, reason: &str) {
        self.clear();
        self.set_note(reason.to_owned(), true);
    }

    /// Takes what happened since the last frame.
    pub(crate) fn update(&mut self) {
        let Some(run) = &self.run else {
            return;
        };
        let events: Vec<RunEvent> = run.events().collect();
        for event in events {
            match event {
                RunEvent::Output(line) => self.push(line),
                RunEvent::Phase(RunPhase::Finished(end)) => self.finish(end),
                RunEvent::Phase(phase) => self.phase = Some(phase),
            }
        }
    }

    fn finish(&mut self, end: RunEnd) {
        let texts = texts::current();
        let (text, is_error) = match &end {
            RunEnd::Closed { success, code } => (texts.game_closed(*success, *code), !success),
            RunEnd::BuildFailed => (texts.game_build_failed.to_owned(), true),
            RunEnd::Stopped => (texts.game_stopped.to_owned(), false),
            RunEnd::Failed(reason) => (reason.clone(), true),
        };
        self.set_note(text, is_error);
        self.phase = Some(RunPhase::Finished(end));
        self.run = None;
    }

    fn push(&mut self, line: OutputLine) {
        if self.lines.len() == MAX_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    fn set_note(&mut self, text: String, is_error: bool) {
        self.note = Some(Note { text, is_error });
    }

    fn clear(&mut self) {
        self.run = None;
        self.phase = None;
        self.lines.clear();
        self.note = None;
    }

    /// One line about the game, for the status bar.
    pub(crate) fn show_summary(&self, ui: &mut egui::Ui) {
        match &self.phase {
            Some(RunPhase::Building) => {
                ui.spinner();
                ui.weak(texts::current().building_game);
            }
            Some(RunPhase::Running) => {
                ui.colored_label(SUCCESS_COLOR, texts::current().game_running);
            }
            Some(RunPhase::Finished(_)) | None => {}
        }
    }

    /// The lines written, newest at the bottom, followed by the editor's note.
    pub(crate) fn show(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.lines.is_empty() && self.note.is_none() && self.phase.is_none() {
                    ui.weak(texts::current().output_empty);
                }
                for line in &self.lines {
                    let text = egui::RichText::new(&line.text).monospace();
                    match line.kind {
                        OutputKind::Build => ui.weak(text),
                        OutputKind::Game => ui.label(text),
                    };
                }
                if self.phase == Some(RunPhase::Building) {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.weak(texts::current().building_game);
                    });
                }
                if let Some(note) = &self.note {
                    let color = if note.is_error {
                        ERROR_COLOR
                    } else {
                        SUCCESS_COLOR
                    };
                    ui.colored_label(color, &note.text);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> OutputLine {
        OutputLine {
            kind: OutputKind::Game,
            text: text.to_owned(),
        }
    }

    #[test]
    fn old_lines_make_room_for_new_ones() {
        let mut panel = OutputPanel::new();

        for number in 0..=MAX_LINES {
            panel.push(line(&number.to_string()));
        }

        assert_eq!(panel.lines.len(), MAX_LINES);
        assert_eq!(panel.lines.front(), Some(&line("1")));
    }

    #[test]
    fn the_end_of_a_run_is_noted() {
        let mut panel = OutputPanel::new();
        panel.phase = Some(RunPhase::Running);

        panel.finish(RunEnd::BuildFailed);

        assert!(!panel.is_playing());
        assert!(panel.note.as_ref().is_some_and(|note| note.is_error));
    }

    #[test]
    fn refusing_to_play_forgets_the_last_output() {
        let mut panel = OutputPanel::new();
        panel.push(line("old"));

        panel.refuse("no");

        assert!(panel.lines.is_empty());
        assert_eq!(
            panel.note.as_ref().map(|note| note.text.as_str()),
            Some("no")
        );
    }
}
