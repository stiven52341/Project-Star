//! The mistakes found in a script, marked on its text: a wavy red line
//! under the code each one is about, and its message under the pointer.

use eframe::egui;
use egui::text::CCursor;
use star_script::Position;

use super::code_view::char_index_of;
use crate::style::ERROR_COLOR;

/// How wide a mark is when its mistake is at the end of a line.
const MIN_MARK_WIDTH: f32 = 8.0;
/// Horizontal distance between the peaks of the wavy line.
const WAVE_STEP: f32 = 3.0;
/// How far the wavy line goes up and down.
const WAVE_HEIGHT: f32 = 1.5;
const WAVE_WIDTH: f32 = 1.0;
const QUOTE: char = '"';

/// A mistake of a script: where it starts, and what is wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Mistake<'message> {
    pub(super) position: Position,
    pub(super) message: &'message str,
}

/// The characters of the text a mistake is marked under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Mark<'message> {
    /// First character, and the one after the last; the same when the
    /// mistake is at the end of a line.
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) line: usize,
    pub(super) message: &'message str,
}

/// Where each of `mistakes` is marked in `text`: under the name, number or
/// string it starts at, or under its one character.
pub(super) fn marks<'message>(text: &str, mistakes: &[Mistake<'message>]) -> Vec<Mark<'message>> {
    mistakes
        .iter()
        .map(|mistake| {
            let start = char_index_of(text, mistake.position);
            let rest = text.chars().skip(start);
            Mark {
                start,
                end: start + marked_length(rest),
                line: mistake.position.line,
                message: mistake.message,
            }
        })
        .collect()
}

/// How many of the characters of `rest` a mistake at its start covers.
fn marked_length(mut rest: impl Iterator<Item = char>) -> usize {
    let is_word = |character: char| character.is_alphanumeric() || character == '_';
    match rest.next() {
        None | Some('\n') => 0,
        Some(QUOTE) => {
            let line: Vec<char> = rest.take_while(|character| *character != '\n').collect();
            let closing = line.iter().position(|character| *character == QUOTE);
            1 + closing.map_or(line.len(), |closing| closing + 1)
        }
        Some(first) if is_word(first) => {
            1 + rest.take_while(|character| is_word(*character)).count()
        }
        Some(_) => 1,
    }
}

/// Paints a wavy line under each mark of the text laid out in `output`.
/// Returns the message of the one under the pointer, if any.
pub(super) fn paint_marks<'message>(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    marks: &[Mark<'message>],
) -> Option<&'message str> {
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let offset = output.galley_pos.to_vec2();
    let mut hovered = None;
    for mark in marks {
        let start = output
            .galley
            .pos_from_cursor(CCursor::new(mark.start))
            .translate(offset);
        let end = output
            .galley
            .pos_from_cursor(CCursor::new(mark.end))
            .translate(offset);
        let same_row = (end.min.y - start.min.y).abs() < f32::EPSILON;
        let right = if same_row && end.min.x > start.min.x {
            end.min.x
        } else {
            start.min.x + MIN_MARK_WIDTH
        };
        let area = egui::Rect::from_min_max(start.min, egui::pos2(right, start.max.y));
        ui.painter().add(egui::Shape::line(
            wave(area.left(), area.right(), area.bottom() - WAVE_HEIGHT),
            egui::Stroke::new(WAVE_WIDTH, ERROR_COLOR),
        ));
        if pointer.is_some_and(|pointer| area.contains(pointer)) {
            hovered = Some(mark.message);
        }
    }
    hovered
}

/// The points of a wavy line from `left` to `right`, around `middle`.
fn wave(left: f32, right: f32, middle: f32) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    let mut x = left;
    let mut up = true;
    while x < right {
        let y = if up {
            middle - WAVE_HEIGHT
        } else {
            middle + WAVE_HEIGHT
        };
        points.push(egui::pos2(x, y));
        x += WAVE_STEP;
        up = !up;
    }
    points.push(egui::pos2(right, middle));
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "var speed = @\nvar name = \"Bob\" + 2\nend";

    fn marked(text: &str, line: usize, column: usize) -> &str {
        let mistake = Mistake {
            position: Position { line, column },
            message: "wrong",
        };
        let mark = marks(text, &[mistake])[0];
        let start = text
            .char_indices()
            .nth(mark.start)
            .map_or(text.len(), |(byte, _)| byte);
        let end = text
            .char_indices()
            .nth(mark.end)
            .map_or(text.len(), |(byte, _)| byte);
        &text[start..end]
    }

    #[test]
    fn names_numbers_and_strings_are_marked_whole() {
        assert_eq!(marked(TEXT, 1, 5), "speed");
        assert_eq!(marked(TEXT, 2, 12), "\"Bob\"");
        assert_eq!(marked(TEXT, 2, 20), "2");
    }

    #[test]
    fn other_characters_are_marked_alone() {
        assert_eq!(marked(TEXT, 1, 13), "@");
        assert_eq!(marked(TEXT, 2, 18), "+");
    }

    #[test]
    fn mistakes_at_the_end_of_a_line_mark_nothing_of_the_next() {
        assert_eq!(marked(TEXT, 1, 14), "");
        assert_eq!(marked(TEXT, 3, 4), "");
    }

    #[test]
    fn unclosed_strings_are_marked_to_the_end_of_their_line() {
        assert_eq!(marked("x = \"open\ny", 1, 5), "\"open");
    }

    #[test]
    fn waves_go_from_left_to_right() {
        let points = wave(0.0, 10.0, 5.0);

        assert_eq!(points.first().map(|point| point.x), Some(0.0));
        assert_eq!(points.last().map(|point| point.x), Some(10.0));
    }
}
