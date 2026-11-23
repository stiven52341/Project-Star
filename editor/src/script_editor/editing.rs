//! Writing aids of the code editor: brackets and quotes that close
//! themselves, new lines that keep (or add) indentation, `Tab` and
//! `Shift+Tab` over several lines, and lines commented out and back.
//!
//! Each aid is a change of the text and its selection, free of egui, so
//! it is tested on its own; the code view hands it the keys it handles.

/// One level of indentation, as blueprints are translated to StarScript.
const INDENT: &str = "    ";
const INDENT_WIDTH: usize = INDENT.len();
const LINE_COMMENT: &str = "//";
/// Brackets and quotes that close themselves, with what closes them.
const PAIRS: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"')];
const QUOTE: char = '"';

/// The selected text, as character indices: where it started and where the
/// cursor is. Both are the same when nothing is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Selection {
    pub(super) anchor: usize,
    pub(super) cursor: usize,
}

/// What the user did that a writing aid may handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Type(char),
    NewLine,
    Indent,
    Outdent,
    Backspace,
    ToggleComment,
}

/// Changes `text` as `action` asks, with the help of a writing aid, and
/// returns the new selection; `None` if no aid applies, so the text edit
/// does what it always does.
pub(super) fn apply(text: &mut String, selection: Selection, action: Action) -> Option<Selection> {
    let mut edit = Edit::new(text, selection);
    let handled = match action {
        Action::Type(character) => edit.type_character(character),
        Action::NewLine => {
            edit.new_line();
            true
        }
        Action::Indent => {
            edit.indent();
            true
        }
        Action::Outdent => {
            edit.change_lines(outdented);
            true
        }
        Action::Backspace => edit.backspace(),
        Action::ToggleComment => {
            edit.toggle_comment();
            true
        }
    };
    handled.then(|| edit.selection())
}

/// A text being changed, with its selection in bytes.
struct Edit<'text> {
    text: &'text mut String,
    anchor: usize,
    cursor: usize,
}

impl<'text> Edit<'text> {
    fn new(text: &'text mut String, selection: Selection) -> Self {
        let anchor = byte_of(text, selection.anchor);
        let cursor = byte_of(text, selection.cursor);
        Self {
            text,
            anchor,
            cursor,
        }
    }

    /// The selection, back in characters.
    fn selection(&self) -> Selection {
        let char_of = |byte: usize| self.text[..byte].chars().count();
        Selection {
            anchor: char_of(self.anchor),
            cursor: char_of(self.cursor),
        }
    }

    fn start(&self) -> usize {
        self.anchor.min(self.cursor)
    }

    fn end(&self) -> usize {
        self.anchor.max(self.cursor)
    }

    fn has_selection(&self) -> bool {
        self.anchor != self.cursor
    }

    fn next_character(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }

    fn previous_character(&self) -> Option<char> {
        self.text[..self.cursor].chars().next_back()
    }

    /// The start of the line the byte `index` is on.
    fn line_start(&self, index: usize) -> usize {
        self.text[..index]
            .rfind('\n')
            .map_or(0, |newline| newline + 1)
    }

    /// The line of the cursor, up to the cursor.
    fn line_before_cursor(&self) -> &str {
        &self.text[self.line_start(self.cursor)..self.cursor]
    }

    /// Puts `inserted` in place of the selection, with the cursor after it.
    fn replace_selection(&mut self, inserted: &str) {
        let start = self.start();
        self.text.replace_range(start..self.end(), inserted);
        self.anchor = start + inserted.len();
        self.cursor = self.anchor;
    }

    fn type_character(&mut self, character: char) -> bool {
        if let Some(&(opening, closing)) = PAIRS.iter().find(|(opening, _)| *opening == character) {
            if self.has_selection() {
                self.surround(opening, closing);
                return true;
            }
            if self.next_character() == Some(closing) && closing == QUOTE {
                return self.step_over();
            }
            return self.open_pair(opening, closing);
        }
        if PAIRS.iter().any(|(_, closing)| *closing == character) && !self.has_selection() {
            if self.next_character() == Some(character) {
                return self.step_over();
            }
            if character == '}' {
                return self.close_block();
            }
        }
        false
    }

    /// Moves the cursor over the closing character typed again.
    fn step_over(&mut self) -> bool {
        let Some(next) = self.next_character() else {
            return false;
        };
        self.cursor += next.len_utf8();
        self.anchor = self.cursor;
        true
    }

    /// Puts `opening` before the selection and `closing` after it, which
    /// stays selected.
    fn surround(&mut self, opening: char, closing: char) {
        let (start, end) = (self.start(), self.end());
        self.text.insert(end, closing);
        self.text.insert(start, opening);
        self.anchor = start + opening.len_utf8();
        self.cursor = end + opening.len_utf8();
    }

    /// Types `opening` and `closing` with the cursor between them, where
    /// nothing follows that the pair would wrap.
    fn open_pair(&mut self, opening: char, closing: char) -> bool {
        let follows_word = self
            .previous_character()
            .is_some_and(|previous| previous.is_alphanumeric() || previous == '_');
        let inside_string = self.line_before_cursor().matches(QUOTE).count() % 2 == 1;
        if opening == QUOTE && (follows_word || inside_string) {
            return false;
        }
        let free_after = self.next_character().is_none_or(|next| {
            next.is_whitespace() || PAIRS.iter().any(|(_, closing)| *closing == next) || next == ','
        });
        if !free_after {
            return false;
        }
        let at = self.cursor;
        self.text.insert(at, closing);
        self.text.insert(at, opening);
        self.cursor = at + opening.len_utf8();
        self.anchor = self.cursor;
        true
    }

    /// Types `}` one level of indentation back, on a line that only has
    /// indentation before it.
    fn close_block(&mut self) -> bool {
        let before = self.line_before_cursor();
        if before.is_empty() || !before.chars().all(char::is_whitespace) {
            return false;
        }
        let removed = indentation_to_remove(before);
        let start = self.cursor - removed;
        self.text.replace_range(start..self.cursor, "}");
        self.cursor = start + 1;
        self.anchor = self.cursor;
        true
    }

    /// Breaks the line keeping its indentation, one level more after an
    /// opening bracket; between a bracket and its closing one, the closing
    /// one goes to a line of its own.
    fn new_line(&mut self) {
        let start = self.start();
        self.text.replace_range(start..self.end(), "");
        self.cursor = start;
        let line = self.line_before_cursor();
        let indentation: String = line
            .chars()
            .take_while(|character| *character == ' ' || *character == '\t')
            .collect();
        let opening = line.trim_end().chars().next_back();
        let pair = PAIRS
            .iter()
            .find(|(open, _)| *open != QUOTE && Some(*open) == opening);
        let inserted = match pair {
            Some((_, closing)) if self.next_character() == Some(*closing) => {
                let inner = format!("\n{indentation}{INDENT}");
                let text = format!("{inner}\n{indentation}");
                self.text.insert_str(start, &text);
                self.cursor = start + inner.len();
                self.anchor = self.cursor;
                return;
            }
            Some(_) => format!("\n{indentation}{INDENT}"),
            None => format!("\n{indentation}"),
        };
        self.replace_selection(&inserted);
    }

    /// Indents the selected lines, or types one level of indentation.
    fn indent(&mut self) {
        if self.text[self.start()..self.end()].contains('\n') {
            self.change_lines(|line| format!("{INDENT}{line}"));
        } else {
            self.replace_selection(INDENT);
        }
    }

    /// Deletes an empty pair around the cursor, or one level of
    /// indentation before it.
    fn backspace(&mut self) -> bool {
        if self.has_selection() {
            return false;
        }
        let (Some(previous), Some(next)) = (self.previous_character(), self.next_character())
        else {
            return self.delete_indentation();
        };
        if PAIRS.contains(&(previous, next)) {
            let start = self.cursor - previous.len_utf8();
            self.text
                .replace_range(start..self.cursor + next.len_utf8(), "");
            self.cursor = start;
            self.anchor = start;
            return true;
        }
        self.delete_indentation()
    }

    fn delete_indentation(&mut self) -> bool {
        let before = self.line_before_cursor();
        if before.len() < 2 || !before.chars().all(|character| character == ' ') {
            return false;
        }
        let removed = indentation_to_remove(before);
        self.text
            .replace_range(self.cursor - removed..self.cursor, "");
        self.cursor -= removed;
        self.anchor = self.cursor;
        true
    }

    /// Comments out the selected lines, or uncomments them if they all are.
    fn toggle_comment(&mut self) {
        let (first, last) = self.selected_lines();
        let lines: Vec<&str> = self.text[first..last].split('\n').collect();
        let written: Vec<&&str> = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let commented = !written.is_empty()
            && written
                .iter()
                .all(|line| line.trim_start().starts_with(LINE_COMMENT));
        if commented {
            self.change_lines(uncommented);
        } else {
            let column = written
                .iter()
                .map(|line| line.len() - line.trim_start().len())
                .min()
                .unwrap_or(0);
            self.change_lines(|line| commented_at(line, column));
        }
    }

    /// The start of the first selected line and the end of the last one. A
    /// selection that ends at the start of a line leaves that line out.
    fn selected_lines(&self) -> (usize, usize) {
        let first = self.line_start(self.start());
        let mut end = self.end();
        if end > self.start() && self.text[..end].ends_with('\n') {
            end -= 1;
        }
        let last = self.text[end..]
            .find('\n')
            .map_or(self.text.len(), |newline| end + newline);
        (first, last)
    }

    /// Rewrites each selected line with `change`. A selection grows to the
    /// whole lines; a cursor alone keeps its place in the text.
    fn change_lines(&mut self, change: impl Fn(&str) -> String) {
        let (first, last) = self.selected_lines();
        let old = &self.text[first..last];
        let new = old.split('\n').map(change).collect::<Vec<_>>().join("\n");
        let shift = |index: usize| {
            let moved = index.saturating_add(new.len()).saturating_sub(old.len());
            moved.max(first)
        };
        let (anchor, cursor) = if self.has_selection() {
            (first, first + new.len())
        } else {
            (shift(self.anchor), shift(self.cursor))
        };
        self.text.replace_range(first..last, &new);
        self.anchor = anchor;
        self.cursor = cursor;
    }
}

/// How many bytes of the indentation `before` (spaces, or tabs) the next
/// level back removes.
fn indentation_to_remove(before: &str) -> usize {
    if before.ends_with('\t') {
        return 1;
    }
    let spaces = before.len() - before.trim_end_matches(' ').len();
    match spaces % INDENT_WIDTH {
        0 => spaces.min(INDENT_WIDTH),
        extra => extra,
    }
}

/// `line` with one level of indentation less.
fn outdented(line: &str) -> String {
    let removed = line
        .strip_prefix('\t')
        .or_else(|| line.strip_prefix(INDENT))
        .unwrap_or_else(|| line.trim_start_matches(' '));
    removed.to_owned()
}

/// `line` with `// ` at `column`, blank lines as they are.
fn commented_at(line: &str, column: usize) -> String {
    if line.trim().is_empty() {
        return line.to_owned();
    }
    let (indentation, code) = line.split_at(column.min(line.len()));
    format!("{indentation}{LINE_COMMENT} {code}")
}

/// `line` without its first `//` (and the space after it).
fn uncommented(line: &str) -> String {
    let code = line.trim_start();
    let indentation = &line[..line.len() - code.len()];
    let Some(rest) = code.strip_prefix(LINE_COMMENT) else {
        return line.to_owned();
    };
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    format!("{indentation}{rest}")
}

/// The byte where the character at `index` of `text` starts.
fn byte_of(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes `text` with `|` for the cursor, or `[` and `]` around a
    /// selection, applies `action` and returns the result written the
    /// same way; `None` if no aid applies.
    fn after(text: &str, action: Action) -> Option<String> {
        let (mut plain, selection) = parse(text);
        let selection = apply(&mut plain, selection, action)?;
        Some(render(&plain, selection))
    }

    fn parse(marked: &str) -> (String, Selection) {
        let mut plain = String::new();
        let (mut anchor, mut cursor) = (0, 0);
        for character in marked.chars() {
            let index = plain.chars().count();
            match character {
                '|' => (anchor, cursor) = (index, index),
                '[' => anchor = index,
                ']' => cursor = index,
                _ => plain.push(character),
            }
        }
        (plain, Selection { anchor, cursor })
    }

    fn render(plain: &str, selection: Selection) -> String {
        let mut marked = String::new();
        let empty = selection.anchor == selection.cursor;
        for (index, character) in plain.chars().chain(['\0']).enumerate() {
            if empty && index == selection.cursor {
                marked.push('|');
            } else if index == selection.anchor.min(selection.cursor) && !empty {
                marked.push('[');
            } else if index == selection.anchor.max(selection.cursor) && !empty {
                marked.push(']');
            }
            if character != '\0' {
                marked.push(character);
            }
        }
        marked
    }

    #[test]
    fn brackets_close_themselves() {
        assert_eq!(
            after("a = |", Action::Type('(')).as_deref(),
            Some("a = (|)")
        );
        assert_eq!(after("f(|)", Action::Type('{')).as_deref(), Some("f({|})"));
        assert_eq!(
            after("x = |", Action::Type('"')).as_deref(),
            Some("x = \"|\"")
        );
    }

    #[test]
    fn brackets_before_code_do_not_close() {
        assert_eq!(after("|speed", Action::Type('(')), None);
        assert_eq!(after("name|", Action::Type('"')), None);
        assert_eq!(after("\"open |", Action::Type('"')), None);
    }

    #[test]
    fn closing_characters_step_over_the_ones_already_there() {
        assert_eq!(after("f(a|)", Action::Type(')')).as_deref(), Some("f(a)|"));
        assert_eq!(
            after("\"hi|\"", Action::Type('"')).as_deref(),
            Some("\"hi\"|")
        );
        assert_eq!(after("f(a|", Action::Type(')')), None);
    }

    #[test]
    fn selections_are_surrounded() {
        assert_eq!(
            after("x = [a + b]", Action::Type('(')).as_deref(),
            Some("x = ([a + b])")
        );
    }

    #[test]
    fn new_lines_keep_the_indentation() {
        assert_eq!(
            after("    var a = 1|", Action::NewLine).as_deref(),
            Some("    var a = 1\n    |")
        );
    }

    #[test]
    fn new_lines_after_an_opening_bracket_go_one_level_in() {
        assert_eq!(
            after("    if a {|", Action::NewLine).as_deref(),
            Some("    if a {\n        |")
        );
        assert_eq!(
            after("func f() {|}", Action::NewLine).as_deref(),
            Some("func f() {\n    |\n}")
        );
    }

    #[test]
    fn closing_braces_go_one_level_back() {
        assert_eq!(
            after("    if a {\n        |", Action::Type('}')).as_deref(),
            Some("    if a {\n    }|")
        );
        assert_eq!(after("a |", Action::Type('}')), None);
    }

    #[test]
    fn tab_indents_selected_lines_or_types_an_indentation() {
        assert_eq!(after("a|", Action::Indent).as_deref(), Some("a    |"));
        assert_eq!(
            after("[a\nb]", Action::Indent).as_deref(),
            Some("[    a\n    b]")
        );
    }

    #[test]
    fn shift_tab_takes_a_level_out() {
        assert_eq!(
            after("        a|", Action::Outdent).as_deref(),
            Some("    a|")
        );
        assert_eq!(
            after("[    a\n\tb\n  c]", Action::Outdent).as_deref(),
            Some("[a\nb\nc]")
        );
        assert_eq!(after("  |a", Action::Outdent).as_deref(), Some("|a"));
    }

    #[test]
    fn backspace_deletes_empty_pairs_and_indentation_levels() {
        assert_eq!(after("f(|)", Action::Backspace).as_deref(), Some("f|"));
        assert_eq!(
            after("x\n        |", Action::Backspace).as_deref(),
            Some("x\n    |")
        );
        assert_eq!(
            after("x\n      |", Action::Backspace).as_deref(),
            Some("x\n    |")
        );
        assert_eq!(after("ab|", Action::Backspace), None);
    }

    #[test]
    fn comments_are_toggled_on_the_selected_lines() {
        let commented = after("[    a\n\n    b]", Action::ToggleComment);
        assert_eq!(commented.as_deref(), Some("[    // a\n\n    // b]"));
        let back = after("[    // a\n\n    // b]", Action::ToggleComment);
        assert_eq!(back.as_deref(), Some("[    a\n\n    b]"));
        assert_eq!(
            after("var a|", Action::ToggleComment).as_deref(),
            Some("// var a|")
        );
    }

    #[test]
    fn selections_ending_at_a_line_start_leave_that_line_out() {
        assert_eq!(
            after("[a\n]b", Action::Indent).as_deref(),
            Some("[    a]\nb")
        );
    }

    #[test]
    fn characters_beyond_ascii_are_counted_once() {
        assert_eq!(
            after("ñ = |", Action::Type('(')).as_deref(),
            Some("ñ = (|)")
        );
    }
}
