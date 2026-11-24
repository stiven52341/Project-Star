//! A script or blueprint renamed in “Assets” renames its class, when the
//! class is called as the file was, and every use of the class in the
//! project: the scenes built from it, the scripts that name it and the
//! blueprints that extend it, call it or use it as a type.
//!
//! Scripts are changed token by token, so a word in a text or a comment
//! is never touched.

use std::fs;
use std::path::Path;

use star_blueprint::{Blueprint, CallTarget};
use star_engine::prelude::SceneOutline;
use star_script::{TokenKind, scan};

use crate::file_tree::{file_paths, scan_directory};
use crate::project::{BLUEPRINT_EXTENSION, SCENE_EXTENSION, SCRIPT_EXTENSION};

/// A class to rename everywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClassRename {
    pub(super) old: String,
    pub(super) new: String,
}

impl ClassRename {
    /// The class to rename after the file at `from` became `to`, both in
    /// the `assets` folder: if the file is a script or a blueprint whose
    /// class was called as the file, and the new name can be a class.
    pub(super) fn after_moving(assets: &Path, from: &str, to: &str) -> Option<Self> {
        let extension = [SCRIPT_EXTENSION, BLUEPRINT_EXTENSION]
            .into_iter()
            .find(|extension| to.ends_with(extension))?;
        let old = file_stem(from, extension)?;
        let new = file_stem(to, extension)?;
        if old == new || !is_class_name(new) {
            return None;
        }
        let text = fs::read_to_string(assets.join(to)).ok()?;
        let declares = if extension == SCRIPT_EXTENSION {
            star_script::parse_source(&text)
                .is_ok_and(|source| source.classes.iter().any(|class| class.name.text == old))
        } else {
            Blueprint::from_text(&text).is_ok_and(|blueprint| blueprint.class == old)
        };
        declares.then(|| Self {
            old: old.to_owned(),
            new: new.to_owned(),
        })
    }

    /// Renames the class in every script, blueprint and scene of the
    /// `assets` folder but those in `skipped`. Returns the paths of the
    /// files changed and those it could not write.
    pub(super) fn apply(&self, assets: &Path, skipped: &[String]) -> (Vec<String>, Vec<String>) {
        let paths = scan_directory(assets)
            .map(|nodes| file_paths(&nodes))
            .unwrap_or_default();
        let mut changed = Vec::new();
        let mut failed = Vec::new();
        for path in paths.into_iter().filter(|path| !skipped.contains(path)) {
            let file = assets.join(&path);
            let Some(renamed) = fs::read_to_string(&file)
                .ok()
                .and_then(|text| self.in_file(&path, &text))
            else {
                continue;
            };
            match fs::write(&file, renamed) {
                Ok(()) => changed.push(path),
                Err(error) => {
                    log::warn!("Cannot rename the class {} in {path}: {error}", self.old);
                    failed.push(path);
                }
            }
        }
        (changed, failed)
    }

    /// Whether the file at `path` of the `assets` folder, as it is on
    /// disk, uses the class.
    pub(super) fn uses(&self, assets: &Path, path: &str) -> bool {
        fs::read_to_string(assets.join(path))
            .ok()
            .and_then(|text| self.in_file(path, &text))
            .is_some()
    }

    /// The text of the file at `path` with the class renamed; `None` if it
    /// does not use the class or is not a kind of file that can.
    fn in_file(&self, path: &str, text: &str) -> Option<String> {
        if path.ends_with(SCRIPT_EXTENSION) {
            self.in_code(text)
        } else if path.ends_with(BLUEPRINT_EXTENSION) {
            self.in_blueprint(text)
        } else if path.ends_with(SCENE_EXTENSION) {
            self.in_scene(text)
        } else {
            None
        }
    }

    /// StarScript code (a script, or a type such as `[Rock]?`) with every
    /// name of the class replaced.
    fn in_code(&self, code: &str) -> Option<String> {
        let mut renamed = String::with_capacity(code.len());
        let mut copied = 0;
        for token in scan(code).tokens {
            if matches!(&token.kind, TokenKind::Identifier(name) if *name == self.old) {
                renamed.push_str(code.get(copied..token.span.start())?);
                renamed.push_str(&self.new);
                copied = token.span.end();
            }
        }
        if copied == 0 {
            return None;
        }
        renamed.push_str(code.get(copied..)?);
        Some(renamed)
    }

    fn in_blueprint(&self, text: &str) -> Option<String> {
        let mut blueprint = Blueprint::from_text(text).ok()?;
        let mut changed = false;
        let mut rename = |name: &mut String| {
            if *name == self.old {
                self.new.clone_into(name);
                changed = true;
            }
        };
        rename(&mut blueprint.class);
        if let Some(base) = &mut blueprint.extends {
            rename(base);
        }
        for graph in &mut blueprint.graphs {
            for node in &mut graph.nodes {
                if let star_blueprint::NodeKind::Call(call) = &mut node.kind
                    && let CallTarget::Class(class) = &mut call.target
                {
                    rename(class);
                }
            }
        }
        let types = blueprint
            .variables
            .iter_mut()
            .filter_map(|variable| variable.ty.as_mut())
            .chain(blueprint.graphs.iter_mut().flat_map(|graph| {
                graph
                    .parameters
                    .iter_mut()
                    .map(|parameter| &mut parameter.ty)
                    .chain(graph.returns.as_mut())
            }));
        for ty in types {
            if let Some(renamed) = self.in_code(ty) {
                *ty = renamed;
                changed = true;
            }
        }
        if !changed {
            return None;
        }
        blueprint
            .to_text()
            .inspect_err(|error| log::warn!("{error}"))
            .ok()
    }

    fn in_scene(&self, text: &str) -> Option<String> {
        let mut outline = SceneOutline::from_text(text).ok()?;
        let mut changed = false;
        for index in 0..outline.nodes().len() {
            if let Some(class) = outline
                .node_mut(index)
                .and_then(|node| node.class.as_mut())
                .filter(|class| **class == self.old)
            {
                self.new.clone_into(class);
                changed = true;
            }
        }
        if !changed {
            return None;
        }
        outline
            .to_text()
            .inspect_err(|error| log::warn!("{error}"))
            .ok()
    }
}

/// The name of the file at `path`, without its folders and `extension`.
fn file_stem<'path>(path: &'path str, extension: &str) -> Option<&'path str> {
    path.rsplit('/').next()?.strip_suffix(extension)
}

/// Whether `name` can be the name of a class: a single name of code.
fn is_class_name(name: &str) -> bool {
    let scanned = scan(name);
    let tokens: Vec<_> = scanned
        .tokens
        .iter()
        .filter(|token| token.kind != TokenKind::EndOfFile)
        .collect();
    scanned.errors.is_empty()
        && matches!(
            tokens.as_slice(),
            [token] if matches!(&token.kind, TokenKind::Identifier(found) if found == name)
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    fn rock() -> ClassRename {
        ClassRename {
            old: "Rock".to_owned(),
            new: "Stone".to_owned(),
        }
    }

    #[test]
    fn scripts_rename_the_class_but_not_texts_comments_or_longer_names() {
        let code = "// Rock\nclass Rock extends Node {\n    var other: Rock? = null\n    \
                    var name = \"Rock\"\n    var rocks: [Rock] = []\n    var Rocky = 1\n}\n";

        assert_eq!(
            rock().in_code(code).unwrap(),
            "// Rock\nclass Stone extends Node {\n    var other: Stone? = null\n    \
             var name = \"Rock\"\n    var rocks: [Stone] = []\n    var Rocky = 1\n}\n"
        );
        assert_eq!(rock().in_code("class Boulder {}"), None);
    }

    #[test]
    fn blueprints_rename_their_class_base_calls_and_types() {
        let text = "format = 1\nclass = \"Boulder\"\nextends = \"Rock\"\n\n\
                    [[variable]]\nname = \"twin\"\ntype = \"Rock?\"\n\n\
                    [[graph]]\nname = \"start\"\n\n\
                    [[graph.node]]\nid = 1\nkind = \"entry\"\n\n\
                    [[graph.node]]\nid = 2\nkind = \"call\"\ntarget = { class = \"Rock\" }\n\
                    method = \"make\"\n";

        let renamed = Blueprint::from_text(&rock().in_blueprint(text).unwrap()).unwrap();

        assert_eq!(renamed.class, "Boulder");
        assert_eq!(renamed.extends.as_deref(), Some("Stone"));
        assert_eq!(renamed.variables[0].ty.as_deref(), Some("Stone?"));
        assert!(matches!(
            &renamed.graphs[0].nodes[1].kind,
            star_blueprint::NodeKind::Call(call)
                if call.target == CallTarget::Class("Stone".to_owned())
        ));
    }

    #[test]
    fn scenes_rename_the_classes_of_their_nodes() {
        let text = "format = 1\n[[node]]\nname = \"Root\"\nclass = \"Rock\"\n";

        let renamed = SceneOutline::from_text(&rock().in_scene(text).unwrap()).unwrap();

        assert_eq!(renamed.nodes()[0].class.as_deref(), Some("Stone"));
        assert_eq!(
            rock().in_scene("format = 1\n[[node]]\nname = \"Root\"\n"),
            None
        );
    }

    #[test]
    fn only_classes_called_as_their_file_are_renamed() {
        let assets = TestDirectory::new("class_rename_after");
        fs::create_dir_all(assets.path().join("rocks")).unwrap();
        fs::write(assets.path().join("rocks/Stone.star"), "class Rock {}").unwrap();
        fs::write(assets.path().join("Other.star"), "class Pebble {}").unwrap();

        assert_eq!(
            ClassRename::after_moving(assets.path(), "rocks/Rock.star", "rocks/Stone.star"),
            Some(rock())
        );
        assert_eq!(
            ClassRename::after_moving(assets.path(), "Boulder.star", "Other.star"),
            None
        );
        assert_eq!(
            ClassRename::after_moving(assets.path(), "Rock.star", "rocks/Stone.star"),
            Some(rock())
        );
    }

    #[test]
    fn names_that_cannot_be_classes_keep_the_class() {
        let assets = TestDirectory::new("class_rename_bad_name");
        fs::create_dir_all(assets.path()).unwrap();
        fs::write(assets.path().join("My rock.star"), "class Rock {}").unwrap();

        assert_eq!(
            ClassRename::after_moving(assets.path(), "Rock.star", "My rock.star"),
            None
        );
    }

    #[test]
    fn the_whole_project_is_renamed_but_what_is_skipped() {
        let assets = TestDirectory::new("class_rename_apply");
        fs::create_dir_all(assets.path()).unwrap();
        fs::write(
            assets.path().join("Stone.star"),
            "class Rock extends Node {}",
        )
        .unwrap();
        fs::write(
            assets.path().join("Pile.star"),
            "class Pile { var top: Rock? }",
        )
        .unwrap();
        fs::write(
            assets.path().join("Open.star"),
            "class Open { var a: Rock? }",
        )
        .unwrap();
        fs::write(assets.path().join("notes.txt"), "Rock").unwrap();

        let (changed, failed) = rock().apply(assets.path(), &["Open.star".to_owned()]);

        let mut changed = changed;
        changed.sort();
        assert_eq!(changed, ["Pile.star", "Stone.star"]);
        assert_eq!(failed, Vec::<String>::new());
        let read = |name: &str| fs::read_to_string(assets.path().join(name)).unwrap();
        assert_eq!(read("Stone.star"), "class Stone extends Node {}");
        assert_eq!(read("Open.star"), "class Open { var a: Rock? }");
        assert_eq!(read("notes.txt"), "Rock");
    }
}
