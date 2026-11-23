//! “Console”: the messages the editor logs (scenes that cannot be read,
//! images that do not load, the GPU it draws with…), newest at the bottom.

use eframe::egui;
use log::Level;

use crate::editor_log::{LogEntry, LogHistory};
use crate::style::{ERROR_COLOR, WARNING_COLOR};
use crate::texts;

/// The levels the console can show from, most severe first.
const LEVELS: [Level; 4] = [Level::Error, Level::Warn, Level::Info, Level::Debug];

/// The log of the editor, filtered by level.
pub(crate) struct ConsolePanel {
    history: LogHistory,
    /// The least severe level shown.
    lowest_level: Level,
}

impl ConsolePanel {
    pub(crate) const fn new(history: LogHistory) -> Self {
        Self {
            history,
            lowest_level: Level::Info,
        }
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let texts = texts::current();
        ui.horizontal(|ui| {
            ui.weak(texts.show_levels);
            egui::ComboBox::from_id_salt("console_level")
                .selected_text(texts.levels_from(self.lowest_level))
                .show_ui(ui, |ui| {
                    for level in LEVELS {
                        ui.selectable_value(
                            &mut self.lowest_level,
                            level,
                            texts.levels_from(level),
                        );
                    }
                });
            if ui.small_button(texts.clear).clicked() {
                self.history.clear();
            }
        });
        ui.separator();
        let lowest_level = self.lowest_level;
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                self.history.read(|entries| {
                    let mut any_shown = false;
                    for entry in entries.iter().filter(|entry| entry.level <= lowest_level) {
                        show_entry(ui, entry);
                        any_shown = true;
                    }
                    if !any_shown {
                        ui.weak(texts.console_empty);
                    }
                });
            });
    }
}

/// One message: when, how severe, from where and what it says.
fn show_entry(ui: &mut egui::Ui, entry: &LogEntry) {
    ui.horizontal_wrapped(|ui| {
        ui.weak(egui::RichText::new(time_text(entry)).monospace());
        let level = egui::RichText::new(texts::current().level_name(entry.level)).strong();
        match entry.level {
            Level::Error => ui.colored_label(ERROR_COLOR, level),
            Level::Warn => ui.colored_label(WARNING_COLOR, level),
            Level::Info | Level::Debug | Level::Trace => ui.weak(level),
        };
        ui.weak(&entry.target);
        ui.label(&entry.message);
    });
}

/// Seconds since the editor started, to a tenth: `12.3s`.
fn time_text(entry: &LogEntry) -> String {
    format!("{:.1}s", entry.time.as_secs_f32())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn times_are_shown_in_seconds_to_a_tenth() {
        let entry = LogEntry {
            level: Level::Info,
            target: "test".to_owned(),
            message: String::new(),
            time: Duration::from_millis(12_340),
        };

        assert_eq!(time_text(&entry), "12.3s");
    }
}
