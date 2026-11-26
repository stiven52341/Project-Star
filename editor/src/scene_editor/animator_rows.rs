//! The rows of an animator in the inspector: its `.anim` file, the
//! animation it plays from the start (decision 79) and the `.states` file
//! of its state machine (decision 81). The file is only read while the
//! list of its animations is open.

use std::fs;

use eframe::egui;
use star_engine::prelude::AssetPath;
use star_engine::star_scene::{AnimatorOutline, TrackAnimations};

use super::animation_rows::AnimationFiles;
use crate::texts;

/// An animator of the first `.anim` file of `files`, playing nothing yet;
/// `None` if there is none.
pub(super) fn new_animator(files: AnimationFiles<'_>) -> Option<AnimatorOutline> {
    let animations = files
        .keyframes
        .iter()
        .find_map(|path| AssetPath::new(path).ok())?;
    Some(AnimatorOutline {
        animations,
        animation: None,
        states: None,
    })
}

/// The animator's file and starting animation. Returns whether they
/// changed.
pub(super) fn edit_animator(
    ui: &mut egui::Ui,
    animator: &mut AnimatorOutline,
    files: AnimationFiles<'_>,
) -> bool {
    file_row(ui, animator, files)
        | starting_animation_row(ui, animator, files)
        | states_row(ui, animator, files)
}

fn states_row(
    ui: &mut egui::Ui,
    animator: &mut AnimatorOutline,
    files: AnimationFiles<'_>,
) -> bool {
    let texts = texts::current();
    ui.weak(texts.animator_states())
        .on_hover_text(texts.animator_states_hint());
    let shown = animator.states.as_ref().map(AssetPath::as_str);
    let mut chosen: Option<Option<AssetPath>> = None;
    egui::ComboBox::from_id_salt("animator_states")
        .selected_text(shown.unwrap_or(texts.no_states()))
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(shown.is_none(), texts.no_states())
                .clicked()
                && shown.is_some()
            {
                chosen = Some(None);
            }
            for path in files.states {
                let current = shown == Some(path.as_str());
                if ui.selectable_label(current, path).clicked() && !current {
                    chosen = AssetPath::new(path).ok().map(Some);
                }
            }
        });
    ui.end_row();
    let Some(chosen) = chosen else {
        return false;
    };
    animator.states = chosen;
    true
}

fn file_row(ui: &mut egui::Ui, animator: &mut AnimatorOutline, files: AnimationFiles<'_>) -> bool {
    let texts = texts::current();
    ui.weak(texts.keyframes())
        .on_hover_text(texts.keyframes_hint());
    let mut chosen = None;
    egui::ComboBox::from_id_salt("animator_animations")
        .selected_text(animator.animations.as_str())
        .show_ui(ui, |ui| {
            for path in files.keyframes {
                let current = animator.animations.as_str() == path;
                if ui.selectable_label(current, path).clicked() && !current {
                    chosen = AssetPath::new(path).ok();
                }
            }
        });
    ui.end_row();
    let Some(chosen) = chosen else {
        return false;
    };
    animator.animations = chosen;
    // The animations of another file have other names.
    animator.animation = None;
    true
}

fn starting_animation_row(
    ui: &mut egui::Ui,
    animator: &mut AnimatorOutline,
    files: AnimationFiles<'_>,
) -> bool {
    let texts = texts::current();
    ui.weak(texts.starting_animation());
    let mut changed = false;
    egui::ComboBox::from_id_salt("animator_starting_animation")
        .selected_text(
            animator
                .animation
                .as_deref()
                .unwrap_or(texts.no_starting_clip()),
        )
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(animator.animation.is_none(), texts.no_starting_clip())
                .clicked()
                && animator.animation.is_some()
            {
                animator.animation = None;
                changed = true;
            }
            for name in animation_names(files, &animator.animations) {
                let current = animator.animation.as_deref() == Some(name.as_str());
                if ui.selectable_label(current, &name).clicked() && !current {
                    animator.animation = Some(name);
                    changed = true;
                }
            }
        });
    ui.end_row();
    changed
}

/// The names of the animations of the file at `path`; none if it cannot
/// be read (the reason is logged).
fn animation_names(files: AnimationFiles<'_>, path: &AssetPath) -> Vec<String> {
    fs::read_to_string(files.folder.join(path.as_str()))
        .map_err(|error| error.to_string())
        .and_then(|text| TrackAnimations::from_text(&text).map_err(|error| error.to_string()))
        .inspect_err(|error| log::warn!("\"{path}\": {error}"))
        .map(|file| {
            file.animations
                .into_iter()
                .map(|animation| animation.name)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn new_animators_take_the_first_file_and_play_nothing() {
        let keyframes = ["knight.anim".to_owned(), "door.anim".to_owned()];
        let files = AnimationFiles {
            folder: Path::new("assets"),
            paths: &[],
            keyframes: &keyframes,
            states: &[],
        };

        let animator = new_animator(files).unwrap();

        assert_eq!(animator.animations.as_str(), "knight.anim");
        assert_eq!(animator.animation, None);
        let none = AnimationFiles {
            keyframes: &[],
            ..files
        };
        assert_eq!(new_animator(none), None);
    }
}
