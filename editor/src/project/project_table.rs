//! Writing a key of the `[project]` table of `project.toml`.
//!
//! The file is the user's: its comments and order are kept, so the line is
//! written into its text rather than the whole file rewritten.

use std::io;

use super::MANIFEST_FILE_NAME;

/// The header of the table.
const PROJECT_HEADER: &str = "[project]";

/// `text` with `key = value` in its `[project]` table: in place of the line
/// of `key` it had, or after the table's last line.
///
/// # Errors
///
/// Fails if `text` has no `[project]` table.
pub(super) fn with_project_key(text: &str, key: &str, value: &toml::Value) -> io::Result<String> {
    let line = format!("{key} = {value}");
    let mut lines: Vec<&str> = text.lines().collect();
    let header = lines
        .iter()
        .position(|line| line.trim() == PROJECT_HEADER)
        .ok_or_else(|| {
            invalid(format!(
                "{MANIFEST_FILE_NAME} has no {PROJECT_HEADER} table"
            ))
        })?;
    let table_end = lines[header + 1..]
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .map_or(lines.len(), |end| header + 1 + end);
    let existing = lines[header + 1..table_end]
        .iter()
        .position(|line| is_line_of(line, key))
        .map(|at| header + 1 + at);
    if let Some(at) = existing {
        lines[at] = &line;
    } else {
        // After the table's last line, not after the blank lines that
        // separate it from the next one.
        let last = lines[header..table_end]
            .iter()
            .rposition(|line| !line.trim().is_empty())
            .map_or(header, |at| header + at);
        lines.insert(last + 1, &line);
    }
    Ok(lines.join("\n") + "\n")
}

fn is_line_of(line: &str, key: &str) -> bool {
    line.split_once('=')
        .is_some_and(|(written, _)| written.trim() == key)
}

pub(super) fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
