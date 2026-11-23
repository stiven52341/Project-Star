//! The bar to find and replace text in a script, over its code. `Ctrl+F`
//! opens it with the selected text, `Enter` and `Shift+Enter` go to the
//! next and previous match, and `Esc` closes it with the match chosen
//! selected in the code.

use std::ops::Range;

use eframe::egui;
use egui::text::{CCursor, CCursorRange};

use super::find::{Query, first_from, replace_all, replace_range};
use crate::texts;

/// The keys that open the bar.
pub(super) const FIND_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::F);
/// How wide the fields of the text to find and its replacement are.
const FIELD_WIDTH: f32 = 220.0;
const MATCH_CASE_LABEL: &str = "Aa";
const PREVIOUS_LABEL: &str = "‹";
const NEXT_LABEL: &str = "›";
const CLOSE_LABEL: &str = "×";

/// What is being found and replaced, kept between frames.
#[derive(Default)]
pub(super) struct FindBar {
    open: bool,
    query: String,
    replacement: String,
    match_case: bool,
    /// The place of the match chosen among those found.
    current: usize,
    /// The query and case the match was chosen for, to choose again when
    /// they change.
    chosen_for: Option<(String, bool)>,
    /// Whether the field of the text to find takes the keyboard on the
    /// next frame.
    focus_query: bool,
}

/// What the bar found and did on a frame.
pub(super) struct Found {
    /// Where the text looked for is.
    pub(super) matches: Vec<Range<usize>>,
    /// The place of the match chosen among them, if there is any.
    pub(super) current: Option<usize>,
    /// Whether the match chosen changed and has to be brought into sight.
    pub(super) moved: bool,
    /// Whether a replacement changed the text.
    pub(super) changed: bool,
    /// Whether the bar was closed.
    pub(super) closed: bool,
}

impl Found {
    /// The characters of the match chosen.
    pub(super) fn current_range(&self) -> Option<Range<usize>> {
        self.matches.get(self.current?).cloned()
    }
}

/// A move to another match.
#[derive(Clone, Copy)]
enum Step {
    Previous,
    Next,
}

impl FindBar {
    /// Whether it is shown.
    pub(super) const fn is_open(&self) -> bool {
        self.open
    }

    /// Shows it with the keyboard on its text to find, which becomes
    /// `selected` if it is one line of text.
    pub(super) fn open(&mut self, selected: Option<&str>) {
        if let Some(selected) = selected.filter(|selected| !selected.contains('\n')) {
            selected.clone_into(&mut self.query);
        }
        self.open = true;
        self.focus_query = true;
    }

    /// Shows the bar over the code of the text edit `code` with `text`,
    /// whose cursor is at the character `cursor`, and does what the user
    /// asks on it.
    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        text: &mut String,
        code: egui::Id,
        cursor: usize,
    ) -> Found {
        let mut found = Found {
            matches: Vec::new(),
            current: None,
            moved: false,
            changed: false,
            closed: false,
        };
        ui.horizontal(|ui| self.show_find_row(ui, text, code, cursor, &mut found));
        ui.horizontal(|ui| self.show_replace_row(ui, text, code, &mut found));
        found.current = (!found.matches.is_empty()).then_some(self.current);
        if found.closed {
            self.open = false;
        }
        found
    }

    /// The text to find, its case, the moves between its matches, how many
    /// there are, and closing.
    fn show_find_row(
        &mut self,
        ui: &mut egui::Ui,
        text: &str,
        code: egui::Id,
        cursor: usize,
        found: &mut Found,
    ) {
        let texts = texts::current();
        let field_id = code.with("find");
        let field = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .id(field_id)
                .hint_text(texts.find_hint())
                .desired_width(FIELD_WIDTH),
        );
        if std::mem::take(&mut self.focus_query) {
            select_all(ui.ctx(), field_id, &self.query);
            field.request_focus();
        }
        let mut step = None;
        if field.lost_focus() {
            step = keep_focus_on_enter(ui, &field, found);
        }
        ui.toggle_value(&mut self.match_case, MATCH_CASE_LABEL)
            .on_hover_text(texts.match_case());
        found.matches = self.query().matches_in(text);
        self.choose_from(cursor, found);
        if ui
            .button(PREVIOUS_LABEL)
            .on_hover_text(texts.previous_match())
            .clicked()
        {
            step = Some(Step::Previous);
        }
        if ui
            .button(NEXT_LABEL)
            .on_hover_text(texts.next_match())
            .clicked()
        {
            step = Some(Step::Next);
        }
        if let Some(step) = step {
            self.step(step, found);
        }
        self.show_count(ui, found);
        if ui
            .button(CLOSE_LABEL)
            .on_hover_text(texts.close_find())
            .clicked()
        {
            found.closed = true;
        }
    }

    /// The replacement, and replacing the match chosen or all of them.
    fn show_replace_row(
        &mut self,
        ui: &mut egui::Ui,
        text: &mut String,
        code: egui::Id,
        found: &mut Found,
    ) {
        let texts = texts::current();
        let field = ui.add(
            egui::TextEdit::singleline(&mut self.replacement)
                .id(code.with("replace"))
                .hint_text(texts.replace_hint())
                .desired_width(FIELD_WIDTH),
        );
        let mut replace_one = false;
        if field.lost_focus() {
            replace_one = keep_focus_on_enter(ui, &field, found).is_some();
        }
        replace_one |= ui.button(texts.replace()).clicked();
        if replace_one {
            self.replace_current(text, found);
        }
        if ui.button(texts.replace_all()).clicked() {
            replace_all(text, self.query(), &self.replacement);
            found.matches = self.query().matches_in(text);
            found.changed = true;
        }
    }

    /// How many matches there are and which one is chosen.
    fn show_count(&self, ui: &mut egui::Ui, found: &Found) {
        let texts = texts::current();
        if self.query.is_empty() {
            return;
        }
        if found.matches.is_empty() {
            ui.weak(texts.no_matches());
        } else {
            ui.weak(texts.match_count(self.current + 1, found.matches.len()));
        }
    }

    fn query(&self) -> Query<'_> {
        Query {
            text: &self.query,
            match_case: self.match_case,
        }
    }

    /// Chooses the first match from `cursor` when what is looked for
    /// changed, and keeps the one chosen among those there are.
    fn choose_from(&mut self, cursor: usize, found: &mut Found) {
        let looked_for = (self.query.clone(), self.match_case);
        if self.chosen_for.as_ref() != Some(&looked_for) {
            self.chosen_for = Some(looked_for);
            self.current = first_from(&found.matches, cursor);
            found.moved = !found.matches.is_empty();
        }
        if self.current >= found.matches.len() {
            self.current = 0;
        }
    }

    /// Chooses the match before or after the one chosen, going round.
    fn step(&mut self, step: Step, found: &mut Found) {
        let count = found.matches.len();
        if count == 0 {
            return;
        }
        self.current = match step {
            Step::Previous => (self.current + count - 1) % count,
            Step::Next => (self.current + 1) % count,
        };
        found.moved = true;
    }

    /// Replaces the match chosen, and chooses the one after it.
    fn replace_current(&mut self, text: &mut String, found: &mut Found) {
        let Some(range) = found.matches.get(self.current).cloned() else {
            return;
        };
        replace_range(text, range, &self.replacement);
        found.matches = self.query().matches_in(text);
        // The next match takes the place of the one replaced.
        if self.current >= found.matches.len() {
            self.current = 0;
        }
        found.changed = true;
        found.moved = !found.matches.is_empty();
    }
}

/// What the keys that made `field` lose the keyboard ask for: `Esc` closes
/// the bar, `Enter` steps to the next match (`Shift+Enter`, to the previous)
/// and gives the keyboard back to `field`.
fn keep_focus_on_enter(ui: &egui::Ui, field: &egui::Response, found: &mut Found) -> Option<Step> {
    let (enter, escape, shift) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::Enter),
            input.key_pressed(egui::Key::Escape),
            input.modifiers.shift,
        )
    });
    if escape {
        found.closed = true;
        return None;
    }
    if !enter {
        return None;
    }
    field.request_focus();
    Some(if shift { Step::Previous } else { Step::Next })
}

/// Selects all of `text`, the text of the field `id`.
fn select_all(context: &egui::Context, id: egui::Id, text: &str) {
    let mut state = egui::text_edit::TextEditState::load(context, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(0),
        CCursor::new(text.chars().count()),
    )));
    state.store(context, id);
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "size = size + size";

    fn nothing_found() -> Found {
        Found {
            matches: Vec::new(),
            current: None,
            moved: false,
            changed: false,
            closed: false,
        }
    }

    /// A bar looking for `query` in `text`, with its match chosen from
    /// `cursor`.
    fn looking_for(query: &str, text: &str, cursor: usize) -> (FindBar, Found) {
        let mut bar = FindBar::default();
        bar.open(Some(query));
        let mut found = nothing_found();
        found.matches = bar.query().matches_in(text);
        bar.choose_from(cursor, &mut found);
        (bar, found)
    }

    #[test]
    fn opening_takes_the_selected_line() {
        let mut bar = FindBar::default();

        bar.open(Some("size"));
        bar.open(Some("two\nlines"));

        assert!(bar.is_open());
        assert_eq!(bar.query, "size");
    }

    #[test]
    fn the_match_from_the_cursor_is_chosen_first() {
        let (bar, found) = looking_for("size", TEXT, 3);

        assert_eq!(bar.current, 1);
        assert!(found.moved);
    }

    #[test]
    fn steps_go_round_the_matches() {
        let (mut bar, mut found) = looking_for("size", TEXT, 0);

        bar.step(Step::Previous, &mut found);
        assert_eq!(bar.current, 2);
        bar.step(Step::Next, &mut found);
        assert_eq!(bar.current, 0);
    }

    #[test]
    fn replacing_chooses_the_next_match() {
        let mut text = TEXT.to_owned();
        let (mut bar, mut found) = looking_for("size", &text, 3);
        "scale".clone_into(&mut bar.replacement);

        bar.replace_current(&mut text, &mut found);

        assert_eq!(text, "size = scale + size");
        assert!(found.changed);
        assert_eq!(found.current_range(), None);
        assert_eq!(found.matches.get(bar.current), Some(&(15..19)));
    }

    #[test]
    fn replacing_the_last_match_chooses_the_first() {
        let mut text = TEXT.to_owned();
        let (mut bar, mut found) = looking_for("size", &text, 14);

        bar.replace_current(&mut text, &mut found);

        assert_eq!(text, "size = size + ");
        assert_eq!(bar.current, 0);
    }
}
