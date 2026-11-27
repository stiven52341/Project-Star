//! The groups of sounds of the project and the volumes they start at
//! (decision 87): `[audio.groups]` of `project.toml`, which the mixer tab
//! edits.
//!
//! The file is the user's: the rest of it is kept as it is, and the table
//! is written in place of the one it had, or at the end.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use super::MANIFEST_FILE_NAME;

/// The header of the table of groups.
const GROUPS_HEADER: &str = "[audio.groups]";

/// The groups of the project in `directory`, with their volumes, by name;
/// none if `project.toml` has none or cannot be read.
pub(crate) fn read_sound_groups(directory: &Path) -> Vec<(String, f32)> {
    fs::read_to_string(directory.join(MANIFEST_FILE_NAME))
        .ok()
        .map(|text| sound_groups_of(&text))
        .unwrap_or_default()
}

/// Makes `groups` the groups of the project in `directory`.
///
/// # Errors
///
/// Fails if `project.toml` cannot be read or written, or the groups could
/// not be written in it (it then stays as it was).
pub(crate) fn set_sound_groups(directory: &Path, groups: &[(String, f32)]) -> io::Result<()> {
    let file = directory.join(MANIFEST_FILE_NAME);
    let text = fs::read_to_string(&file)?;
    fs::write(&file, with_sound_groups(&text, groups)?)
}

fn sound_groups_of(text: &str) -> Vec<(String, f32)> {
    let Ok(table) = toml::from_str::<toml::Table>(text) else {
        return Vec::new();
    };
    table
        .get("audio")
        .and_then(|audio| audio.get("groups"))
        .and_then(toml::Value::as_table)
        .map(|groups| {
            groups
                .iter()
                .filter_map(|(name, volume)| {
                    // Volumes are written with a few digits.
                    #[allow(clippy::cast_possible_truncation)]
                    let volume = volume.as_float().map(|volume| volume as f32);
                    volume.map(|volume| (name.clone(), volume))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `text` with `groups` as its table of groups: in place of the one it
/// had, or at its end; without one if there are no groups.
fn with_sound_groups(text: &str, groups: &[(String, f32)]) -> io::Result<String> {
    let lines: Vec<&str> = text.lines().collect();
    let header = lines.iter().position(|line| line.trim() == GROUPS_HEADER);
    let (before, after) = match header {
        Some(header) => {
            let end = lines
                .iter()
                .skip(header + 1)
                .position(|line| line.trim_start().starts_with('['))
                .map_or(lines.len(), |end| header + 1 + end);
            (&lines[..header], &lines[end..])
        }
        None => (&lines[..], &[][..]),
    };
    let mut new_text = before.join("\n").trim_end().to_owned();
    if !groups.is_empty() {
        new_text.push_str("\n\n");
        new_text.push_str(GROUPS_HEADER);
        for (name, volume) in groups {
            let key = toml::Value::String(name.clone());
            // Writing to a `String` cannot fail.
            let _ = write!(new_text, "\n{key} = {volume:?}");
        }
    }
    let rest = after.join("\n");
    if !rest.trim().is_empty() {
        new_text.push_str("\n\n");
        new_text.push_str(rest.trim_start());
    }
    new_text.push('\n');
    if sound_groups_of(&new_text).len() == groups.len() {
        Ok(new_text)
    } else {
        Err(io::Error::other(
            "the sound groups could not be written in project.toml",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str =
        "# Mine.\n[project]\nname = \"Demo\"\n\n[input.actions]\njump = [\"Space\"]\n";

    fn groups(pairs: &[(&str, f32)]) -> Vec<(String, f32)> {
        pairs
            .iter()
            .map(|(name, volume)| ((*name).to_owned(), *volume))
            .collect()
    }

    #[test]
    fn groups_are_written_at_the_end_and_read_back() {
        let text =
            with_sound_groups(MANIFEST, &groups(&[("music", 0.5), ("effects", 1.0)])).unwrap();

        assert_eq!(
            text,
            "# Mine.\n[project]\nname = \"Demo\"\n\n[input.actions]\njump = [\"Space\"]\n\n\
             [audio.groups]\n\"music\" = 0.5\n\"effects\" = 1.0\n"
        );
        assert_eq!(
            sound_groups_of(&text),
            groups(&[("effects", 1.0), ("music", 0.5)])
        );
    }

    #[test]
    fn new_groups_replace_the_old_table_and_keep_the_rest() {
        let first = "[project]\nname = \"Demo\"\n\n[audio.groups]\nmusic = 0.5\n\n[input.actions]\njump = [\"Space\"]\n";

        let second = with_sound_groups(first, &groups(&[("voices", 0.75)])).unwrap();
        let none = with_sound_groups(&second, &[]).unwrap();

        assert_eq!(sound_groups_of(&second), groups(&[("voices", 0.75)]));
        assert!(second.contains("[input.actions]\njump"), "{second}");
        assert!(!none.contains("[audio.groups]"), "{none}");
        assert!(none.contains("name = \"Demo\""), "{none}");
    }
}
