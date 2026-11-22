//! Writes indented lines of code.

/// Spaces per indentation level, as `rustfmt` uses.
const INDENT: &str = "    ";

/// Accumulates generated code line by line, keeping the indentation.
#[derive(Debug, Default)]
pub(super) struct Writer {
    text: String,
    depth: usize,
}

impl Writer {
    /// Writes `line` at the current indentation.
    pub(super) fn line(&mut self, line: &str) {
        if line.is_empty() {
            self.text.push('\n');
            return;
        }
        for _ in 0..self.depth {
            self.text.push_str(INDENT);
        }
        self.text.push_str(line);
        self.text.push('\n');
    }

    /// Writes `line` (which opens a block) and indents what follows.
    pub(super) fn open(&mut self, line: &str) {
        self.line(line);
        self.depth += 1;
    }

    /// Ends the indentation of a block and writes `line`, which closes it.
    pub(super) fn close(&mut self, line: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.line(line);
    }

    /// Writes `line`, which closes a block and opens the next one
    /// (`} else {`), at the indentation of the first.
    pub(super) fn middle(&mut self, line: &str) {
        self.close(line);
        self.depth += 1;
    }

    /// The code written so far.
    pub(super) fn finish(self) -> String {
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_indented() {
        let mut writer = Writer::default();
        writer.open("fn f() {");
        writer.line("g();");
        writer.close("}");

        assert_eq!(writer.finish(), "fn f() {\n    g();\n}\n");
    }
}
