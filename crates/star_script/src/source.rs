//! Locations inside a StarScript source file.

use std::iter;

/// Byte range `start..end` of a piece of source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// Creates the span between two byte offsets, in either order.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    /// Byte offset of the first character.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Byte offset just past the last character.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// The same span, `offset` bytes later: where it is once its file is
    /// placed at `offset` inside a program of several files.
    #[must_use]
    pub(crate) const fn shifted(self, offset: usize) -> Self {
        Self {
            start: self.start + offset,
            end: self.end + offset,
        }
    }

    /// The code this span covers, or `""` if it does not belong to `source`.
    #[must_use]
    pub fn text(self, source: &str) -> &str {
        source.get(self.start..self.end).unwrap_or_default()
    }
}

/// Human-readable location: both numbers start at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Line number.
    pub line: usize,
    /// Column number, counted in characters (not bytes).
    pub column: usize,
}

/// Converts byte offsets into lines and columns for error messages.
#[derive(Debug, Clone)]
pub struct LineIndex<'source> {
    source: &'source str,
    line_starts: Vec<usize>,
}

impl<'source> LineIndex<'source> {
    /// Indexes the lines of `source`.
    #[must_use]
    pub fn new(source: &'source str) -> Self {
        let line_starts = iter::once(0)
            .chain(source.match_indices('\n').map(|(offset, _)| offset + 1))
            .collect();
        Self {
            source,
            line_starts,
        }
    }

    /// Line and column of the character at byte `offset`.
    ///
    /// Offsets past the end of the source map to the end of the last line.
    #[must_use]
    pub fn position(&self, offset: usize) -> Position {
        let offset = offset.min(self.source.len());
        let line_index = self
            .line_starts
            .partition_point(|&line_start| line_start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts.get(line_index).copied().unwrap_or(0);
        let characters_before = self
            .source
            .get(line_start..offset)
            .map_or(0, |text| text.chars().count());
        Position {
            line: line_index + 1,
            column: characters_before + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_orders_its_offsets() {
        let span = Span::new(7, 3);

        assert_eq!((span.start(), span.end()), (3, 7));
    }

    #[test]
    fn span_text_slices_the_source() {
        assert_eq!(Span::new(4, 9).text("var speed = 2"), "speed");
    }

    #[test]
    fn span_outside_the_source_has_no_text() {
        assert_eq!(Span::new(10, 20).text("short"), "");
    }

    #[test]
    fn positions_start_at_line_and_column_one() {
        let index = LineIndex::new("class A {}");

        assert_eq!(index.position(0), Position { line: 1, column: 1 });
    }

    #[test]
    fn positions_count_lines_and_columns() {
        let index = LineIndex::new("class A {\n    var x\n}");

        assert_eq!(index.position(14), Position { line: 2, column: 5 });
        assert_eq!(index.position(20), Position { line: 3, column: 1 });
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        let source = "\"ñandú\" x";
        let index = LineIndex::new(source);
        let x_offset = source.find('x').expect("x is in the source");

        assert_eq!(index.position(x_offset), Position { line: 1, column: 9 });
    }

    #[test]
    fn offsets_past_the_end_map_to_the_end() {
        let index = LineIndex::new("ab");

        assert_eq!(index.position(100), Position { line: 1, column: 3 });
    }
}
