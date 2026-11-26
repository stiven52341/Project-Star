//! The rows of a sprite's animations in the inspector: its `.frames` file
//! and the clip it plays from the start. Choosing them copies the file's
//! image and cells into the sprite, so the scene shows the frame the
//! sprite starts on. The file is only read when one is chosen, or while
//! the list of its clips is open.

use std::fs;
use std::path::Path;

use eframe::egui;
use star_engine::prelude::AssetPath;
use star_engine::star_render::SpriteSheet;
use star_engine::star_scene::{FrameClip, SpriteFrames, SpriteOutline};

use crate::texts;

/// The `.frames` files of the assets folder, to choose from.
#[derive(Clone, Copy)]
pub(crate) struct AnimationFiles<'files> {
    /// The assets folder.
    pub(crate) folder: &'files Path,
    /// The files, as paths inside it.
    pub(crate) paths: &'files [String],
}

impl AnimationFiles<'_> {
    /// The animations of the file at `path`; `None` if it cannot be read
    /// (the reason is logged).
    fn read(&self, path: &AssetPath) -> Option<SpriteFrames> {
        fs::read_to_string(self.folder.join(path.as_str()))
            .map_err(|error| error.to_string())
            .and_then(|text| SpriteFrames::from_text(&text).map_err(|error| error.to_string()))
            .inspect_err(|error| log::warn!("\"{path}\": {error}"))
            .ok()
    }
}

/// The sprite's `.frames` file and, if it has one, its starting clip.
/// Returns whether they changed.
pub(super) fn animation_rows(
    ui: &mut egui::Ui,
    sprite: &mut SpriteOutline,
    files: AnimationFiles<'_>,
) -> bool {
    let mut changed = frames_row(ui, sprite, files);
    if sprite.frames.is_some() {
        changed |= starting_clip_row(ui, sprite, files);
    }
    changed
}

fn frames_row(ui: &mut egui::Ui, sprite: &mut SpriteOutline, files: AnimationFiles<'_>) -> bool {
    let texts = texts::current();
    ui.weak(texts.animations())
        .on_hover_text(texts.animations_hint());
    let current = sprite.frames.as_ref().map(|path| path.as_str().to_owned());
    let mut chosen = None;
    egui::ComboBox::from_id_salt("sprite_frames")
        .selected_text(current.as_deref().unwrap_or(texts.no_animations()))
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(current.is_none(), texts.no_animations())
                .clicked()
            {
                chosen = Some(None);
            }
            for path in files.paths {
                if ui
                    .selectable_label(current.as_ref() == Some(path), path)
                    .clicked()
                {
                    chosen = Some(Some(path));
                }
            }
        });
    ui.end_row();
    let Some(chosen) = chosen else {
        return false;
    };
    let chosen = chosen.and_then(|path| AssetPath::new(path).ok());
    if chosen.as_ref().map(AssetPath::as_str) == current.as_deref() {
        return false;
    }
    sprite.animation = None;
    if let Some(path) = &chosen
        && let Some(frames) = files.read(path)
    {
        let first = frames.clips.first();
        take_sheet(sprite, &frames, first);
        sprite.animation = first.map(|clip| clip.name.clone());
    }
    sprite.frames = chosen;
    true
}

fn starting_clip_row(
    ui: &mut egui::Ui,
    sprite: &mut SpriteOutline,
    files: AnimationFiles<'_>,
) -> bool {
    let texts = texts::current();
    ui.weak(texts.starting_clip());
    let mut changed = false;
    egui::ComboBox::from_id_salt("sprite_starting_clip")
        .selected_text(
            sprite
                .animation
                .as_deref()
                .unwrap_or(texts.no_starting_clip()),
        )
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(sprite.animation.is_none(), texts.no_starting_clip())
                .clicked()
                && sprite.animation.is_some()
            {
                sprite.animation = None;
                changed = true;
            }
            let Some(frames) = sprite.frames.as_ref().and_then(|path| files.read(path)) else {
                return;
            };
            for clip in &frames.clips {
                let current = sprite.animation.as_deref() == Some(clip.name.as_str());
                if ui.selectable_label(current, &clip.name).clicked() && !current {
                    sprite.animation = Some(clip.name.clone());
                    take_sheet(sprite, &frames, Some(clip));
                    changed = true;
                }
            }
        });
    ui.end_row();
    changed
}

/// Gives `sprite` the image and cells of `frames`, showing the first frame
/// of `clip` (the first cell without one).
fn take_sheet(sprite: &mut SpriteOutline, frames: &SpriteFrames, clip: Option<&FrameClip>) {
    sprite.texture = frames.texture.clone();
    let first = clip
        .and_then(|clip| clip.frames.first())
        .copied()
        .unwrap_or(0);
    sprite.sheet = SpriteSheet::new(frames.columns, frames.rows, first);
}

#[cfg(test)]
mod tests {
    use star_engine::prelude::Vec2;

    use super::*;
    use crate::scene_editor::components::sprite_of;

    #[test]
    fn sprites_take_the_image_and_first_frame_of_their_clip() {
        let frames = SpriteFrames::from_text(
            "format = 1\ntexture = \"hero.png\"\ncolumns = 4\nrows = 2\n\
             [[clip]]\nname = \"run\"\nframes = [5, 6]\n",
        )
        .unwrap();
        let mut sprite = sprite_of(AssetPath::new("other.png").unwrap(), Vec2::ONE);

        take_sheet(&mut sprite, &frames, frames.clips.first());

        assert_eq!(sprite.texture.as_str(), "hero.png");
        assert_eq!(sprite.sheet, SpriteSheet::new(4, 2, 5));
        take_sheet(&mut sprite, &frames, None);
        assert_eq!(sprite.sheet.frame(), 0);
    }
}
