//! The text of a script, shown to edit: colored, with the number of each
//! line beside it, and where the cursor is below it.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use eframe::egui;
use egui::text::{CCursor, CCursorRange, LayoutJob};
use star_script::Position;

use super::editing::{Action, Selection, apply};
use super::highlight::colored;
use crate::texts;

/// Space between the line numbers and the edges of their column.
const GUTTER_PADDING: f32 = 8.0;
/// Fewest digits the line numbers' column has room for.
const MIN_GUTTER_DIGITS: usize = 3;
const DECIMAL_RADIX: usize = 10;

/// The text's cursor and colors, kept between frames.
#[derive(Default)]
pub(super) struct CodeView {
    /// Where the cursor has to be put on the next frame.
    jump: Option<Position>,
    /// Where the cursor is.
    cursor: Option<Position>,
    /// The colored text last laid out, by a hash of the text and the theme.
    colors: Option<(u64, LayoutJob)>,
}

impl CodeView {
    /// Puts the cursor at `position` on the next frame, with its line in
    /// sight.
    pub(super) const fn jump_to(&mut self, position: Position) {
        self.jump = Some(position);
    }

    /// Shows `text` to edit. Returns whether the user changed it.
    pub(super) fn show(&mut self, ui: &mut egui::Ui, text: &mut String, id: egui::Id) -> bool {
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let jumped = self.jump.take().map(|position| {
            let index = char_index_of(text, position);
            put_cursor(ui.ctx(), id, index);
            index
        });
        let footer_height = ui.spacing().interact_size.y;
        let mut changed = apply_writing_aids(ui.ctx(), text, id);
        egui::ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink(false)
            .max_height((ui.available_height() - footer_height).max(0.0))
            .show(ui, |ui| {
                changed |= self.show_lines(ui, text, id, &font, jumped);
            });
        ui.separator();
        if let Some(position) = self.cursor {
            ui.weak(texts::current().cursor_position(position.line, position.column));
        }
        changed
    }

    /// The column of line numbers and the text beside it; `jumped` is where
    /// the cursor was just put, to scroll to.
    fn show_lines(
        &mut self,
        ui: &mut egui::Ui,
        text: &mut String,
        id: egui::Id,
        font: &egui::FontId,
        jumped: Option<usize>,
    ) -> bool {
        let digits = digit_count(text.lines().count()).max(MIN_GUTTER_DIGITS);
        let digit_width = ui.fonts_mut(|fonts| fonts.glyph_width(font, '0'));
        #[allow(clippy::cast_precision_loss)] // A few digits.
        let gutter_width = digit_width * digits as f32 + 2.0 * GUTTER_PADDING;
        // A whole number of rows that fit, at least one.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let visible_rows = (ui.available_height() / ui.fonts_mut(|fonts| fonts.row_height(font)))
            .floor()
            .max(1.0) as usize;
        let dark_mode = ui.visuals().dark_mode;
        let colors = &mut self.colors;
        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = colored_cached(colors, buffer.as_str(), font, dark_mode);
            job.wrap.max_width = wrap_width;
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let output = ui
            .horizontal_top(|ui| {
                ui.add_space(gutter_width);
                egui::TextEdit::multiline(text)
                    .id(id)
                    .code_editor()
                    .font(font.clone())
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY)
                    // Clicking below the last line still reaches the text.
                    .desired_rows(visible_rows)
                    .layouter(&mut layouter)
                    .show(ui)
            })
            .inner;
        let cursor = output.cursor_range.map(|range| range.primary.index.0);
        self.cursor = cursor.map(|index| position_of(text, index));
        paint_line_numbers(ui, &output, font, self.cursor);
        if let Some(index) = jumped {
            let rect = output
                .galley
                .pos_from_cursor(CCursor::new(index))
                .translate(output.galley_pos.to_vec2());
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
        }
        output.response.changed()
    }
}

/// The colored layout of `text`, made again only when the text or the
/// theme changed since the last time.
fn colored_cached(
    cache: &mut Option<(u64, LayoutJob)>,
    text: &str,
    font: &egui::FontId,
    dark_mode: bool,
) -> LayoutJob {
    let mut hasher = DefaultHasher::new();
    (text, dark_mode, font).hash(&mut hasher);
    let key = hasher.finish();
    match cache {
        Some((cached, job)) if *cached == key => job.clone(),
        _ => {
            let mut job = colored(text, font, dark_mode);
            // Spaces typed at the end of a line must be seen.
            job.keep_trailing_whitespace = true;
            *cache = Some((key, job.clone()));
            job
        }
    }
}

/// Paints the number of each line left of its first row, the cursor's
/// brighter.
fn paint_line_numbers(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    font: &egui::FontId,
    cursor: Option<Position>,
) {
    let painter = ui.painter();
    let right = output.galley_pos.x - GUTTER_PADDING;
    let visuals = ui.visuals();
    let mut line = 1;
    let mut starts_line = true;
    for row in &output.galley.rows {
        if starts_line {
            let color = if cursor.is_some_and(|cursor| cursor.line == line) {
                visuals.strong_text_color()
            } else {
                visuals.weak_text_color()
            };
            let top = output.galley_pos.y + row.rect().min.y;
            painter.text(
                egui::pos2(right, top),
                egui::Align2::RIGHT_TOP,
                line.to_string(),
                font.clone(),
                color,
            );
            line += 1;
        }
        starts_line = row.ends_with_newline;
    }
    painter.vline(
        output.galley_pos.x - GUTTER_PADDING / 2.0,
        output.response.rect.y_range(),
        visuals.widgets.noninteractive.bg_stroke,
    );
}

/// Hands the keys that writing aids handle to them, before the text edit
/// `id` sees them, while it has the keyboard. Returns whether the text
/// changed.
fn apply_writing_aids(context: &egui::Context, text: &mut String, id: egui::Id) -> bool {
    if !context.memory(|memory| memory.has_focus(id)) {
        return false;
    }
    let Some(mut state) = egui::text_edit::TextEditState::load(context, id) else {
        return false;
    };
    let Some(range) = state.cursor.char_range() else {
        return false;
    };
    let mut selection = Selection {
        anchor: range.secondary.index.0,
        cursor: range.primary.index.0,
    };
    let mut changed = false;
    // Once the text edit has to handle a key, it handles those after it
    // too, so they reach the text in the order they were typed.
    let mut passing = false;
    context.input_mut(|input| {
        input.events.retain(|event| {
            if passing {
                return true;
            }
            let handled = action_of(event).and_then(|action| apply(text, selection, action));
            if let Some(new_selection) = handled {
                selection = new_selection;
                changed = true;
                return false;
            }
            passing = changes_text(event);
            true
        });
    });
    if changed {
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(selection.anchor),
            CCursor::new(selection.cursor),
        )));
        state.store(context, id);
    }
    changed
}

/// What a writing aid may do for `event`.
fn action_of(event: &egui::Event) -> Option<Action> {
    match event {
        // With an input method on (as on Linux while typing), characters
        // arrive as what it commits.
        egui::Event::Text(typed) | egui::Event::Ime(egui::ImeEvent::Commit(typed)) => {
            let mut characters = typed.chars();
            match (characters.next(), characters.next()) {
                (Some(character), None) => Some(Action::Type(character)),
                _ => None,
            }
        }
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => match key {
            egui::Key::Enter if modifiers.is_none() => Some(Action::NewLine),
            egui::Key::Tab if modifiers.is_none() => Some(Action::Indent),
            egui::Key::Tab if modifiers.matches_exact(egui::Modifiers::SHIFT) => {
                Some(Action::Outdent)
            }
            egui::Key::Backspace if modifiers.is_none() => Some(Action::Backspace),
            egui::Key::Slash if modifiers.command => Some(Action::ToggleComment),
            _ => None,
        },
        _ => None,
    }
}

/// Whether the text edit would change the text for `event`.
const fn changes_text(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::Text(_)
            | egui::Event::Ime(_)
            | egui::Event::Paste(_)
            | egui::Event::Cut
            | egui::Event::Key { pressed: true, .. }
    )
}

/// Puts the cursor of the text edit `id` before the character at `index`,
/// and gives it the keyboard.
fn put_cursor(context: &egui::Context, id: egui::Id, index: usize) {
    let mut state = egui::text_edit::TextEditState::load(context, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(index))));
    state.store(context, id);
    context.memory_mut(|memory| memory.request_focus(id));
}

/// How many decimal digits `number` has.
const fn digit_count(mut number: usize) -> usize {
    let mut digits = 1;
    while number >= DECIMAL_RADIX {
        number /= DECIMAL_RADIX;
        digits += 1;
    }
    digits
}

/// The line and column (from 1) of the character at `index` of `text`.
fn position_of(text: &str, index: usize) -> Position {
    let mut position = Position { line: 1, column: 1 };
    for character in text.chars().take(index) {
        if character == '\n' {
            position.line += 1;
            position.column = 1;
        } else {
            position.column += 1;
        }
    }
    position
}

/// The index of the character at `position` of `text`: the end of its line
/// if the line is shorter, the end of the text if it has fewer lines.
fn char_index_of(text: &str, position: Position) -> usize {
    let mut current = Position { line: 1, column: 1 };
    for (index, character) in text.chars().enumerate() {
        let past_line = current.line > position.line;
        let at_column = current.line == position.line
            && (current.column >= position.column || character == '\n');
        if past_line || at_column {
            return index;
        }
        if character == '\n' {
            current.line += 1;
            current.column = 1;
        } else {
            current.column += 1;
        }
    }
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "class A {\n  var ñ = 1\n}";

    fn at(line: usize, column: usize) -> Position {
        Position { line, column }
    }

    #[test]
    fn positions_count_characters_from_one() {
        assert_eq!(position_of(TEXT, 0), at(1, 1));
        assert_eq!(position_of(TEXT, 10), at(2, 1));
        assert_eq!(position_of(TEXT, 17), at(2, 8));
        assert_eq!(position_of(TEXT, 99), at(3, 2));
    }

    #[test]
    fn positions_and_indices_go_both_ways() {
        for index in 0..=TEXT.chars().count() {
            assert_eq!(char_index_of(TEXT, position_of(TEXT, index)), index);
        }
    }

    #[test]
    fn positions_past_a_line_stay_on_it() {
        assert_eq!(char_index_of(TEXT, at(1, 99)), 9);
        assert_eq!(char_index_of(TEXT, at(99, 1)), TEXT.chars().count());
    }

    #[test]
    fn typed_characters_reach_the_writing_aids_however_they_arrive() {
        let typed = egui::Event::Text("(".to_owned());
        let committed = egui::Event::Ime(egui::ImeEvent::Commit("{".to_owned()));
        let pasted = egui::Event::Text("ab".to_owned());

        assert_eq!(action_of(&typed), Some(Action::Type('(')));
        assert_eq!(action_of(&committed), Some(Action::Type('{')));
        assert_eq!(action_of(&pasted), None);
    }

    #[test]
    fn digits_are_counted() {
        assert_eq!(digit_count(0), 1);
        assert_eq!(digit_count(9), 1);
        assert_eq!(digit_count(10), 2);
        assert_eq!(digit_count(1234), 4);
    }
}
