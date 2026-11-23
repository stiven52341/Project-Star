//! Suggestions shown while writing: the rest of the chosen name in faint
//! text after the cursor, and the other names in a faint list under it.
//! `Tab` writes the chosen one and `Esc` hides them until the word changes.
//! The arrows keep moving the cursor until `Ctrl+Space` makes the list
//! active: then they choose in it, while that word is written.

use eframe::egui;
use egui::text::{CCursor, CCursorRange};

use super::completion::{Candidate, Completion, complete};
use super::scope::Known;
use crate::texts;

/// How many names the list shows at once.
const SHOWN_CANDIDATES: usize = 8;
/// How visible the faint rest of the chosen name is, from 0 to 255.
const GHOST_ALPHA: u8 = 110;
/// How visible the list is until `Ctrl+Space` makes it active, from 0 to 1.
const INACTIVE_LIST_OPACITY: f32 = 0.55;
/// The keys that make the list active, or show hidden suggestions again.
/// `Ctrl` on every system: `Cmd+Space` opens the search of macOS.
pub(super) const CHOOSE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Space);

/// What the user chose among the suggestions, kept between frames.
#[derive(Default)]
pub(super) struct Suggestions {
    /// The word (start and what is typed of it) the choice is for.
    word: Option<(usize, String)>,
    /// The chosen candidate.
    selected: usize,
    /// Whether `Esc` hid the suggestions for this word.
    dismissed: bool,
    /// Whether `Ctrl+Space` was pressed since the last frame.
    asked: bool,
    /// Where the word whose list is active starts: the arrows choose in it
    /// while that word is written.
    active_at: Option<usize>,
}

/// What a key did to the suggestions.
enum KeyUse {
    Accept,
    Choose(isize),
    Dismiss,
}

impl Suggestions {
    /// Shows the suggestions hidden with `Esc` again, and makes their list
    /// active: what `Ctrl+Space` does.
    pub(super) const fn ask(&mut self) {
        self.dismissed = false;
        self.asked = true;
    }

    /// The suggestions for the word that ends at `cursor` of `text`, unless
    /// they were hidden for it.
    pub(super) fn find(
        &mut self,
        text: &str,
        cursor: Option<usize>,
        knows: Known<'_>,
    ) -> Option<Completion> {
        let asked = std::mem::take(&mut self.asked);
        let Some(completion) = cursor.and_then(|cursor| complete(text, cursor, knows)) else {
            self.word = None;
            return None;
        };
        let word = (completion.start, completion.typed.clone());
        if self.word.as_ref() != Some(&word) {
            self.word = Some(word);
            self.selected = 0;
            self.dismissed = false;
        }
        if asked {
            self.active_at = Some(completion.start);
        }
        self.selected = self.selected.min(completion.candidates.len() - 1);
        (!self.dismissed).then_some(completion)
    }

    /// Takes the keys that use the suggestions before the text edit `id`
    /// sees them. Returns whether the text changed.
    pub(super) fn handle_keys(
        &mut self,
        context: &egui::Context,
        text: &mut String,
        id: egui::Id,
        completion: &Completion,
    ) -> bool {
        let choosing = completion.candidates.len() > 1 && self.is_active(completion);
        let mut used = None;
        context.input_mut(|input| {
            input.events.retain(|event| {
                let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                else {
                    return true;
                };
                if used.is_some() || !modifiers.is_none() {
                    return true;
                }
                used = key_use(*key, choosing);
                used.is_none()
            });
        });
        match used {
            Some(KeyUse::Accept) => {
                let Some(candidate) = completion.candidates.get(self.selected) else {
                    return false;
                };
                let cursor = accept(text, completion, candidate);
                let mut state =
                    egui::text_edit::TextEditState::load(context, id).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
                state.store(context, id);
                true
            }
            Some(KeyUse::Choose(step)) => {
                let count = completion.candidates.len();
                self.selected = (self.selected + count).wrapping_add_signed(step) % count;
                false
            }
            Some(KeyUse::Dismiss) => {
                self.dismissed = true;
                false
            }
            None => false,
        }
    }

    /// Paints the rest of the chosen name after the cursor of the text laid
    /// out in `output`, and the list of names under it.
    pub(super) fn paint(
        &self,
        ui: &egui::Ui,
        output: &egui::text_edit::TextEditOutput,
        font: &egui::FontId,
        text: &str,
        completion: &Completion,
    ) {
        let Some(chosen) = completion.candidates.get(self.selected) else {
            return;
        };
        let typed_length = completion.typed.chars().count();
        let cursor = completion.start + typed_length;
        let place = output
            .galley
            .pos_from_cursor(CCursor::new(cursor))
            .translate(output.galley_pos.to_vec2());
        let follows_name = text
            .chars()
            .nth(cursor)
            .is_some_and(|next| next.is_ascii_alphanumeric() || next == '_');
        if !follows_name {
            let rest: String = chosen.name.chars().skip(typed_length).collect();
            let color = ui.visuals().text_color().gamma_multiply_u8(GHOST_ALPHA);
            ui.painter()
                .text(place.min, egui::Align2::LEFT_TOP, rest, font.clone(), color);
        }
        if completion.candidates.len() > 1 {
            self.show_list(ui, output.response.id, place.left_bottom(), completion);
        }
    }

    /// Whether the arrows choose in the list of `completion`.
    fn is_active(&self, completion: &Completion) -> bool {
        self.active_at == Some(completion.start)
    }

    /// The names around the chosen one, under the word; faint until the
    /// list is active.
    fn show_list(&self, ui: &egui::Ui, id: egui::Id, at: egui::Pos2, completion: &Completion) {
        let active = self.is_active(completion);
        let first = self
            .selected
            .saturating_sub(SHOWN_CANDIDATES / 2)
            .min(completion.candidates.len().saturating_sub(SHOWN_CANDIDATES));
        egui::Area::new(id.with("suggestions"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .interactable(false)
            .show(ui.ctx(), |ui| {
                if !active {
                    ui.multiply_opacity(INACTIVE_LIST_OPACITY);
                }
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    egui::Grid::new(id.with("suggestion_rows")).show(ui, |ui| {
                        let shown = completion.candidates.iter().enumerate();
                        for (index, candidate) in shown.skip(first).take(SHOWN_CANDIDATES) {
                            show_row(ui, candidate, index == self.selected);
                        }
                    });
                    ui.weak(texts::current().suggestion_keys(active));
                });
            });
    }
}

/// What `key` does to the suggestions, if anything; the arrows only while
/// `choosing` in a list.
const fn key_use(key: egui::Key, choosing: bool) -> Option<KeyUse> {
    match key {
        egui::Key::Tab => Some(KeyUse::Accept),
        egui::Key::ArrowDown if choosing => Some(KeyUse::Choose(1)),
        egui::Key::ArrowUp if choosing => Some(KeyUse::Choose(-1)),
        egui::Key::Escape => Some(KeyUse::Dismiss),
        _ => None,
    }
}

/// A row of the list: the name, and its signature or what it is.
fn show_row(ui: &mut egui::Ui, candidate: &Candidate, selected: bool) {
    let name = egui::RichText::new(&candidate.name).monospace();
    ui.add(egui::Button::selectable(selected, name));
    let detail = candidate
        .signature
        .clone()
        .unwrap_or_else(|| texts::current().candidate_kind(candidate.kind).to_owned());
    ui.weak(egui::RichText::new(detail).monospace());
    ui.end_row();
}

/// Writes `candidate` in place of the word of `completion`. Returns where
/// the cursor goes.
fn accept(text: &mut String, completion: &Completion, candidate: &Candidate) -> usize {
    let byte = |index: usize| {
        text.char_indices()
            .nth(index)
            .map_or(text.len(), |(byte, _)| byte)
    };
    let start = byte(completion.start);
    let end = byte(completion.start + completion.typed.chars().count());
    let (inserted, cursor) = candidate.insertion();
    text.replace_range(start..end, &inserted);
    completion.start + cursor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepting_writes_the_whole_name_in_place_of_the_word() {
        let mut text = "var a = ro + 1".to_owned();
        let completion = Completion {
            start: 8,
            typed: "ro".to_owned(),
            candidates: Vec::new(),
        };

        let cursor = accept(&mut text, &completion, &Candidate::named("rock"));

        assert_eq!(text, "var a = rock + 1");
        assert_eq!(cursor, 12);
    }

    #[test]
    fn arrows_move_the_cursor_until_the_list_is_active() {
        assert!(key_use(egui::Key::ArrowDown, false).is_none());
        assert!(matches!(
            key_use(egui::Key::ArrowDown, true),
            Some(KeyUse::Choose(1))
        ));
        assert!(matches!(
            key_use(egui::Key::Tab, false),
            Some(KeyUse::Accept)
        ));
    }

    #[test]
    fn asking_makes_the_list_of_the_word_active_while_it_is_written() {
        let mut suggestions = Suggestions::default();
        let text = "var counter = 1
var country = 2
co";
        let end = text.chars().count();

        suggestions.ask();
        let asked = suggestions.find(text, Some(end), Known::default()).unwrap();
        assert!(suggestions.is_active(&asked));

        let longer = format!("{text}u");
        let typed = suggestions
            .find(&longer, Some(end + 1), Known::default())
            .unwrap();
        assert!(suggestions.is_active(&typed));

        let other = format!("{text} + co");
        let another = suggestions
            .find(&other, Some(end + 5), Known::default())
            .unwrap();
        assert!(!suggestions.is_active(&another));
    }

    #[test]
    fn asking_shows_hidden_suggestions_again() {
        let mut suggestions = Suggestions::default();
        let text = "var counter = 1
co";
        suggestions.find(text, Some(18), Known::default());
        suggestions.dismissed = true;

        suggestions.ask();

        assert!(suggestions.find(text, Some(18), Known::default()).is_some());
    }

    #[test]
    fn hidden_suggestions_come_back_for_another_word() {
        let mut suggestions = Suggestions::default();
        let text = "var counter = 1\nco";

        assert!(suggestions.find(text, Some(18), Known::default()).is_some());
        suggestions.dismissed = true;
        assert!(suggestions.find(text, Some(18), Known::default()).is_none());
        assert!(
            suggestions
                .find("var counter = 1\nc", Some(17), Known::default())
                .is_some()
        );
    }
}
