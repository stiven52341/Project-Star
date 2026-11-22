//! The source files of a program made of several `.star` files.

use crate::diagnostic::format_report;
use crate::{CompileError, LineIndex, Position, Span};

/// Offsets left empty between two files, so the end of one file (where
/// "end of file" mistakes are reported) is never the start of the next.
const FILE_GAP: usize = 1;

/// The `.star` files of one program, each with the name it is reported by.
///
/// Every file is read on its own, so a mistake in one never spills into
/// another, but they share one range of byte offsets: each [`Span`] of the
/// program belongs to a single file, which [`SourceSet::locate`] finds.
#[derive(Debug, Clone, Default)]
pub struct SourceSet {
    files: Vec<SourceText>,
    next_start: usize,
}

/// One file of a [`SourceSet`].
#[derive(Debug, Clone)]
struct SourceText {
    name: String,
    text: String,
    /// Offset of its first byte in the program.
    start: usize,
}

/// Where a piece of code of a [`SourceSet`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation<'set> {
    /// The name its file was added with.
    pub file: &'set str,
    /// Its line and column inside that file.
    pub position: Position,
    /// Its byte offset inside that file, for tools that map the file's
    /// code back to where it came from (such as blueprints).
    pub offset: usize,
}

impl SourceSet {
    /// A program without files.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the file called `name` (as mistakes will name it, e.g.
    /// `"enemies/rock.star"`) with the source `text`.
    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) {
        let text = text.into();
        let start = self.next_start;
        self.next_start = start + text.len() + FILE_GAP;
        self.files.push(SourceText {
            name: name.into(),
            text,
            start,
        });
    }

    /// Whether no file was added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Each file's source and the offset where it starts in the program.
    pub(crate) fn texts(&self) -> impl Iterator<Item = (&str, usize)> {
        self.files
            .iter()
            .map(|file| (file.text.as_str(), file.start))
    }

    /// The file, line and column where `span` starts, or `None` if it does
    /// not belong to this program.
    #[must_use]
    pub fn locate(&self, span: Span) -> Option<SourceLocation<'_>> {
        let offset = span.start();
        let index = self
            .files
            .partition_point(|file| file.start <= offset)
            .checked_sub(1)?;
        let file = self.files.get(index)?;
        let local_offset = offset - file.start;
        if local_offset > file.text.len() {
            return None;
        }
        Some(SourceLocation {
            file: &file.name,
            position: LineIndex::new(&file.text).position(local_offset),
            offset: local_offset,
        })
    }

    /// One-line report of `error`, as [`CompileError::report`] writes it.
    #[must_use]
    pub fn report(&self, error: &CompileError) -> String {
        self.locate(error.span()).map_or_else(
            || format!("error: {error}"),
            |location| format_report(location.file, location.position, error),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_files() -> SourceSet {
        let mut sources = SourceSet::new();
        sources.add("a.star", "class A {}\n");
        sources.add("b.star", "class B {\n  var x = 1\n}");
        sources
    }

    fn start_of_second_file(sources: &SourceSet) -> usize {
        sources.texts().nth(1).map_or(0, |(_, start)| start)
    }

    #[test]
    fn files_do_not_share_offsets() {
        let sources = two_files();
        let starts: Vec<usize> = sources.texts().map(|(_, start)| start).collect();

        assert_eq!(starts, [0, "class A {}\n".len() + FILE_GAP]);
    }

    #[test]
    fn offsets_are_located_in_their_file() {
        let sources = two_files();
        let x_offset = start_of_second_file(&sources) + "class B {\n  ".len();

        let location = sources.locate(Span::new(x_offset, x_offset + 1));

        assert_eq!(
            location,
            Some(SourceLocation {
                file: "b.star",
                position: Position { line: 2, column: 3 },
                offset: "class B {\n  ".len(),
            })
        );
    }

    #[test]
    fn the_end_of_a_file_belongs_to_that_file() {
        let sources = two_files();
        let end_of_first = "class A {}\n".len();

        let location = sources.locate(Span::new(end_of_first, end_of_first));

        assert_eq!(location.map(|location| location.file), Some("a.star"));
    }

    #[test]
    fn offsets_past_every_file_are_not_located() {
        let sources = two_files();

        assert_eq!(sources.locate(Span::new(1_000, 1_001)), None);
        assert_eq!(SourceSet::new().locate(Span::new(0, 0)), None);
    }
}
