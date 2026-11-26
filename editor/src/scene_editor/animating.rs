//! A scene's timeline: the `.anim` files of its animators, read when an
//! animator that plays one is selected and written when the scene is saved,
//! and the “Animation” tab that edits them.

use std::fs;
use std::path::Path;

use eframe::egui;
use star_engine::star_scene::{FieldValue, TrackAnimations};
use star_script::semantic::ClassTable;

use super::exported_fields::exported_fields;
use super::history::KeyframeFiles;
use super::key_buttons::KeyButtons;
use super::{Change, SceneDocument, TimelineFile, animated_properties};

impl SceneDocument {
    /// Tells the timeline whether its tab is shown this frame: the scene
    /// previews its animation only then.
    pub(crate) const fn set_timeline_shown(&mut self, shown: bool) {
        self.timeline.set_shown(shown);
    }

    /// Whether an animator was selected since this was last asked, so its
    /// timeline can be shown.
    pub(crate) const fn take_animator_selected(&mut self) -> bool {
        self.timeline.take_just_bound()
    }

    /// The “Animation” tab: the timeline of the animator selected last,
    /// offering calls to the methods of `classes`.
    pub(crate) fn show_timeline(&mut self, ui: &mut egui::Ui, classes: Option<&ClassTable>) {
        let Ok(outline) = &mut self.content else {
            return;
        };
        let path = self.timeline.file(outline.nodes()).map(str::to_owned);
        let file = path.as_deref().map(|path| TimelineFile {
            path,
            animations: match self.animations.get_mut(path) {
                Some(animations) => Ok(animations),
                None => Err(self
                    .unreadable_animations
                    .get(path)
                    .map_or("", String::as_str)),
            },
        });
        let Some(change) = self.timeline.show(ui, outline, file, classes) else {
            return;
        };
        if let Some(path) = &path {
            rename_starting_animations(outline, path, &change);
        }
        self.mark_changed();
    }

    /// The key buttons of the node selected, with its path from the node
    /// animated, while the timeline of an animator that animates it is
    /// shown.
    pub(super) fn key_buttons(&self) -> Option<(KeyButtons, String)> {
        let outline = self.content.as_ref().ok()?;
        let file = self.animations.get(self.timeline.file(outline.nodes())?)?;
        self.timeline
            .key_buttons(outline.nodes(), self.selected?, file)
    }

    /// Puts keys with the values the node selected has now for the
    /// properties `pressed`, at the timeline's playhead. The node is at
    /// `path` from the node animated; an `export` field the scene gives no
    /// value is keyed with its class's, found in `classes`.
    pub(super) fn add_keys(
        &mut self,
        path: &str,
        pressed: &[String],
        classes: Option<&ClassTable>,
    ) {
        let (Ok(outline), Some(selected)) = (&self.content, self.selected) else {
            return;
        };
        let Some(node) = outline
            .nodes()
            .get(selected)
            .filter(|_| !pressed.is_empty())
        else {
            return;
        };
        let defaults = node
            .class
            .as_deref()
            .zip(classes)
            .and_then(|(class, classes)| exported_fields(classes, class))
            .unwrap_or_default();
        let keys: Vec<(String, FieldValue)> = pressed
            .iter()
            .filter_map(|property| {
                let value = animated_properties::value(node, property).or_else(|| {
                    defaults
                        .iter()
                        .find(|field| field.name == *property)
                        .map(|field| field.default.clone())
                })?;
                Some((property.clone(), value))
            })
            .collect();
        let Some(file) = self.timeline.file(outline.nodes()) else {
            return;
        };
        let Some(animations) = self.animations.get_mut(file) else {
            return;
        };
        if self.timeline.add_keys(animations, path, keys) {
            self.mark_changed();
        }
    }

    /// Follows the node selected with the timeline, and reads the file of
    /// the animator it shows if it was not read yet.
    pub(super) fn follow_timeline(&mut self) {
        let Ok(outline) = &self.content else {
            return;
        };
        self.timeline.follow(outline.nodes(), self.selected);
        let Some(path) = self.timeline.file(outline.nodes()).map(str::to_owned) else {
            return;
        };
        if self.animations.contains_key(&path) || self.unreadable_animations.contains_key(&path) {
            return;
        }
        match read_animations(&self.assets, &path) {
            Ok(animations) => {
                if let Some(history) = &mut self.history {
                    history.adopt_animations(&path, &animations);
                }
                self.written_animations
                    .insert(path.clone(), animations.clone());
                self.animations.insert(path, animations);
            }
            Err(error) => {
                self.unreadable_animations.insert(path, error);
            }
        }
    }
}

/// The animators of `outline` that play `path` keep starting with the
/// animation `change` renamed, or start with none once it is deleted.
fn rename_starting_animations(
    outline: &mut star_engine::prelude::SceneOutline,
    path: &str,
    change: &Change,
) {
    let (from, to) = match change {
        Change::Edited => return,
        Change::Renamed { from, to } => (from, Some(to)),
        Change::Deleted(name) => (name, None),
    };
    for index in 0..outline.nodes().len() {
        let Some(animator) = outline
            .node_mut(index)
            .and_then(|node| node.animator.as_mut())
            .filter(|animator| animator.animations.as_str() == path)
        else {
            continue;
        };
        if animator.animation.as_ref() == Some(from) {
            animator.animation = to.cloned();
        }
    }
}

/// Reads the `.anim` file at `path` of the `assets` folder.
fn read_animations(assets: &Path, path: &str) -> Result<TrackAnimations, String> {
    fs::read_to_string(assets.join(path))
        .map_err(|error| error.to_string())
        .and_then(|text| TrackAnimations::from_text(&text).map_err(|error| error.to_string()))
}

/// Writes the files of `animations` that are not as `written` says they
/// were last written, and updates `written`.
///
/// # Errors
///
/// Why a file could not be written, naming it.
pub(super) fn write_animations(
    assets: &Path,
    animations: &KeyframeFiles,
    written: &mut KeyframeFiles,
) -> Result<(), String> {
    for (path, file) in animations {
        if written.get(path) == Some(file) {
            continue;
        }
        file.to_text()
            .map_err(|error| error.to_string())
            .and_then(|text| fs::write(assets.join(path), text).map_err(|error| error.to_string()))
            .map_err(|error| format!("\"{path}\": {error}"))?;
        written.insert(path.clone(), file.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use star_engine::prelude::SceneOutline;
    use star_engine::star_scene::TrackAnimation;

    use super::*;

    const SCENE: &str = r#"format = 1

[[node]]
name = "Knight"

[[node]]
name = "Animator"
parent = 0

[node.animator]
animations = "knight.anim"
animation = "attack"

[[node]]
name = "Other"
parent = 0

[node.animator]
animations = "other.anim"
animation = "attack"
"#;

    fn starting(outline: &SceneOutline, node: usize) -> Option<&str> {
        outline.nodes()[node]
            .animator
            .as_ref()?
            .animation
            .as_deref()
    }

    #[test]
    fn animators_follow_their_starting_animation_renamed_or_deleted() {
        let mut outline = SceneOutline::from_text(SCENE).unwrap();
        let renamed = Change::Renamed {
            from: "attack".to_owned(),
            to: "slash".to_owned(),
        };

        rename_starting_animations(&mut outline, "knight.anim", &renamed);

        assert_eq!(starting(&outline, 1), Some("slash"));
        assert_eq!(starting(&outline, 2), Some("attack"));
        rename_starting_animations(
            &mut outline,
            "knight.anim",
            &Change::Deleted("slash".to_owned()),
        );
        assert_eq!(starting(&outline, 1), None);
    }

    #[test]
    fn only_files_changed_since_they_were_written_are_written() {
        let assets = std::env::temp_dir().join("star_editor_write_animations");
        fs::create_dir_all(&assets).unwrap();
        let mut file = TrackAnimations::default();
        let mut animations = KeyframeFiles::new();
        animations.insert("kept.anim".to_owned(), file.clone());
        let mut written = animations.clone();
        file.animations.push(TrackAnimation {
            name: "idle".to_owned(),
            length: 1.0,
            looping: true,
            tracks: Vec::new(),
            calls: Vec::new(),
        });
        animations.insert("changed.anim".to_owned(), file.clone());

        write_animations(&assets, &animations, &mut written).unwrap();

        assert!(!assets.join("kept.anim").exists());
        let read = read_animations(&assets, "changed.anim").unwrap();
        assert_eq!(read, file);
        assert_eq!(written, animations);
        fs::remove_dir_all(&assets).unwrap();
    }
}
