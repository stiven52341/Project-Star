//! Writes StarScript text with indentation, remembering what each piece
//! of it was written for.

use super::{Origin, OriginRange};

/// Spaces per level of indentation.
const INDENTATION: &str = "    ";

/// StarScript text being written.
pub(super) struct SourceWriter {
    text: String,
    depth: usize,
    origins: Vec<OriginRange>,
}

impl SourceWriter {
    pub(super) const fn new() -> Self {
        Self {
            text: String::new(),
            depth: 0,
            origins: Vec::new(),
        }
    }

    /// Byte offset where the next text goes.
    pub(super) fn position(&self) -> usize {
        self.text.len()
    }

    /// Writes `text` as it is.
    pub(super) fn write(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Starts a line at the current indentation.
    pub(super) fn indent(&mut self) {
        for _ in 0..self.depth {
            self.text.push_str(INDENTATION);
        }
    }

    /// Ends the current line.
    pub(super) fn newline(&mut self) {
        self.text.push('\n');
    }

    /// A whole line at the current indentation.
    pub(super) fn line(&mut self, text: &str) {
        self.indent();
        self.write(text);
        self.newline();
    }

    /// Lines written from now on are one level deeper.
    pub(super) const fn open(&mut self) {
        self.depth += 1;
    }

    /// Lines written from now on are one level shallower.
    pub(super) const fn close(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// The text written since `start` comes from `origin`.
    pub(super) fn record(&mut self, start: usize, origin: Origin) {
        self.origins.push(OriginRange {
            start,
            end: self.position(),
            origin,
        });
    }

    pub(super) fn finish(self) -> (String, Vec<OriginRange>) {
        (self.text, self.origins)
    }
}
