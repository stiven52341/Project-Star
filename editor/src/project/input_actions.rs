//! Adding input actions to `project.toml`, for what the editor adds that
//! reads them (the movement blueprints).
//!
//! The file is the user's: its comments and order are kept, so the lines
//! are written into its text rather than the whole file rewritten.

use std::fs;
use std::io;
use std::path::Path;

use super::MANIFEST_FILE_NAME;

/// The header of the table of actions.
const ACTIONS_HEADER: &str = "[input.actions]";

/// Adds to the `project.toml` of the project in `directory` each of
/// `actions` it does not have yet, with its bindings. Returns the names of
/// those added; the file is not touched if none is missing.
///
/// # Errors
///
/// Fails if the file cannot be read or written, or if its actions are
/// written in a way this cannot add to (it then stays as it was).
pub(crate) fn add_missing_actions(
    directory: &Path,
    actions: &[(&str, &[&str])],
) -> io::Result<Vec<String>> {
    let file = directory.join(MANIFEST_FILE_NAME);
    let text = fs::read_to_string(&file)?;
    let (new_text, added) = with_actions(&text, actions)?;
    if !added.is_empty() {
        fs::write(&file, new_text)?;
    }
    Ok(added)
}

/// `text` with the actions it lacks, and their names.
fn with_actions(text: &str, actions: &[(&str, &[&str])]) -> io::Result<(String, Vec<String>)> {
    let existing = action_names(text)?;
    let missing: Vec<_> = actions
        .iter()
        .filter(|(name, _)| !existing.iter().any(|existing| existing == name))
        .collect();
    if missing.is_empty() {
        return Ok((text.to_owned(), Vec::new()));
    }
    let lines: String = missing
        .iter()
        .map(|(name, bindings)| action_line(name, bindings))
        .collect();
    let new_text = match text.lines().position(|line| line.trim() == ACTIONS_HEADER) {
        Some(header) => {
            let mut result: Vec<&str> = text.lines().collect();
            result.insert(header + 1, lines.trim_end());
            result.join("\n") + "\n"
        }
        None => format!("{}\n\n{ACTIONS_HEADER}\n{lines}", text.trim_end()),
    };
    let now = action_names(&new_text)?;
    if missing
        .iter()
        .any(|(name, _)| !now.iter().any(|added| added == name))
    {
        return Err(io::Error::other(
            "the input actions of project.toml are written in a way the editor cannot add to",
        ));
    }
    let added = missing.iter().map(|(name, _)| (*name).to_owned()).collect();
    Ok((new_text, added))
}

/// `left = ["A", "ArrowLeft"]`
fn action_line(name: &str, bindings: &[&str]) -> String {
    let bindings: Vec<String> = bindings
        .iter()
        .map(|binding| format!("\"{binding}\""))
        .collect();
    format!("{name} = [{}]\n", bindings.join(", "))
}

/// The names of the actions declared in the text of `project.toml`.
fn action_names(text: &str) -> io::Result<Vec<String>> {
    let table: toml::Table = toml::from_str(text).map_err(io::Error::other)?;
    let actions = table
        .get("input")
        .and_then(|input| input.get("actions"))
        .and_then(toml::Value::as_table);
    Ok(actions.map_or_else(Vec::new, |actions| actions.keys().cloned().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOVE: [(&str, &[&str]); 2] = [("left", &["A"]), ("right", &["D", "ArrowRight"])];

    #[test]
    fn projects_without_actions_get_the_table() {
        let text = "# My game.\n[project]\nname = \"Demo\"\n";

        let (new_text, added) = with_actions(text, &MOVE).unwrap();

        assert_eq!(added, ["left", "right"]);
        assert_eq!(
            new_text,
            "# My game.\n[project]\nname = \"Demo\"\n\n[input.actions]\n\
             left = [\"A\"]\nright = [\"D\", \"ArrowRight\"]\n"
        );
    }

    #[test]
    fn only_the_missing_actions_are_added_to_the_table() {
        let text = "[project]\nname = \"Demo\"\n\n[input.actions]\n\
                    # Jumping.\njump = [\"Space\"]\nleft = [\"J\"]\n";

        let (new_text, added) = with_actions(text, &MOVE).unwrap();

        assert_eq!(added, ["right"]);
        assert_eq!(
            new_text,
            "[project]\nname = \"Demo\"\n\n[input.actions]\nright = [\"D\", \"ArrowRight\"]\n\
             # Jumping.\njump = [\"Space\"]\nleft = [\"J\"]\n"
        );
    }

    #[test]
    fn nothing_changes_when_every_action_is_there() {
        let text = "[input.actions]\nleft = [\"A\"]\nright = [\"D\"]\n";

        assert_eq!(
            with_actions(text, &MOVE).unwrap(),
            (text.to_owned(), Vec::new())
        );
    }

    #[test]
    fn actions_written_another_way_are_left_alone() {
        let text = "[input]\nactions = { jump = [\"Space\"] }\n";

        assert!(with_actions(text, &MOVE).is_err());
    }

    #[test]
    fn the_file_of_the_project_is_changed() {
        let project = crate::project::TestDirectory::new("input_actions_added");
        fs::create_dir_all(project.path()).unwrap();
        fs::write(project.path().join(MANIFEST_FILE_NAME), "[project]\n").unwrap();

        let added = add_missing_actions(project.path(), &MOVE).unwrap();
        let again = add_missing_actions(project.path(), &MOVE).unwrap();

        assert_eq!(added.len(), 2);
        assert_eq!(again, Vec::<String>::new());
    }
}
