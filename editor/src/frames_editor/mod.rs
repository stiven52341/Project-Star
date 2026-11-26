//! The frames editor: a `.frames` file of the assets folder, open in a tab
//! of the central area (decision 78). The middle shows its image cut into
//! cells, where clicking a cell adds it to the clip selected; the
//! inspector lists the clips, with their speed, loop, frames and a
//! preview.

mod sheet;

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_engine::prelude::{AssetFolder, AssetPath};
use star_engine::star_scene::{DEFAULT_CLIP_SPEED, FrameClip, SpriteFrames};

use crate::style::ERROR_COLOR;
use crate::texts;

/// Names “New clip” gives: `clip1`, `clip2`…
const NEW_CLIP_PREFIX: &str = "clip";
/// The most cells a sheet is cut into along a side in the editor: more
/// would be too small to click.
const MAX_CELLS_PER_SIDE: u16 = 64;
/// The fastest clip the editor sets, in frames per second.
const MAX_CLIP_SPEED: f32 = 120.0;
/// The slowest clip the editor sets, in frames per second.
const MIN_CLIP_SPEED: f32 = 0.1;
/// How much dragging the speed changes it.
const SPEED_DRAG_STEP: f64 = 0.1;

/// A `.frames` file open in the editor.
pub(crate) struct FramesTab {
    /// Where it is inside the assets folder: `sprites/hero.frames`.
    path: String,
    /// The assets folder, where its image is.
    assets: PathBuf,
    /// The animations, or why the file could not be read.
    content: Result<SpriteFrames, String>,
    /// The clip selected, whose cells clicks add.
    selected: usize,
    /// The image loaded for the sheet, and from where.
    image: Option<(AssetPath, Option<egui::TextureHandle>)>,
    /// Whether it changed since it was opened or saved.
    unsaved: bool,
    /// Why the last save failed, until one works.
    save_error: Option<String>,
}

impl FramesTab {
    /// Opens the `.frames` file at `path` inside the `assets` folder.
    pub(crate) fn open(assets: &Path, path: String) -> Self {
        let content = fs::read_to_string(assets.join(&path))
            .map_err(|error| error.to_string())
            .and_then(|text| SpriteFrames::from_text(&text).map_err(|error| error.to_string()));
        Self {
            path,
            assets: assets.to_path_buf(),
            content,
            selected: 0,
            image: None,
            unsaved: false,
            save_error: None,
        }
    }

    /// Where it is inside the assets folder.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The name of its file, for its tab, marked if it has unsaved changes.
    pub(crate) fn title(&self) -> String {
        let name = self.path.rsplit('/').next().unwrap_or(&self.path);
        if self.unsaved {
            texts::unsaved_title(name)
        } else {
            name.to_owned()
        }
    }

    /// Whether it changed since it was opened or saved.
    pub(crate) const fn is_unsaved(&self) -> bool {
        self.unsaved
    }

    /// Writes it to its file if it has unsaved changes. Returns whether it
    /// is saved now; if not, the tab shows why (a clip without frames, for
    /// one).
    pub(crate) fn save(&mut self) -> bool {
        let Ok(frames) = &self.content else {
            return true;
        };
        if !self.unsaved {
            return true;
        }
        let written = frames
            .to_text()
            .map_err(|error| error.to_string())
            .and_then(|text| {
                fs::write(self.assets.join(&self.path), text).map_err(|error| error.to_string())
            });
        match written {
            Ok(()) => {
                self.unsaved = false;
                self.save_error = None;
                true
            }
            Err(error) => {
                self.save_error = Some(error);
                false
            }
        }
    }

    /// The middle of the tab: the image and how it is cut, what is wrong
    /// with the file, and the sheet to click. `images` are those of the
    /// assets folder, to choose from.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, images: &[String]) {
        let frames = match &mut self.content {
            Ok(frames) => frames,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::current().frames_unreadable(error));
                return;
            }
        };
        if let Some(error) = &self.save_error {
            ui.colored_label(ERROR_COLOR, texts::current().frames_not_saved(error));
        }
        self.unsaved |= show_sheet_settings(ui, frames, images);
        if let Err(error) = frames.validate() {
            ui.colored_label(ERROR_COLOR, error.to_string());
        }
        let clip = frames.clips.get(self.selected);
        match clip {
            Some(clip) => ui.weak(texts::current().sheet_hint(&clip.name)),
            None => ui.weak(texts::current().no_clip_hint()),
        };
        ui.separator();
        let image = loaded_image(&mut self.image, &self.assets, &frames.texture, ui.ctx());
        if let Some(cell) = sheet::show_sheet(ui, image, frames, clip)
            && let Some(clip) = frames.clips.get_mut(self.selected)
        {
            clip.frames.push(cell);
            self.unsaved = true;
        }
    }

    /// The inspector of the tab: its clips, and the one selected to edit.
    pub(crate) fn show_inspector(&mut self, ui: &mut egui::Ui) {
        let Ok(frames) = &mut self.content else {
            return;
        };
        ui.strong(texts::current().clips());
        for (index, clip) in frames.clips.iter().enumerate() {
            if ui
                .selectable_label(index == self.selected, &clip.name)
                .clicked()
            {
                self.selected = index;
            }
        }
        if ui.button(texts::current().new_clip()).clicked() {
            frames.clips.push(new_clip(&frames.clips));
            self.selected = frames.clips.len() - 1;
            self.unsaved = true;
        }
        let Some(clip) = frames.clips.get_mut(self.selected) else {
            return;
        };
        ui.separator();
        let edit = edit_clip(ui, clip);
        self.unsaved |= edit.changed;
        let image = loaded_image(&mut self.image, &self.assets, &frames.texture, ui.ctx());
        if let Some(clip) = frames.clips.get(self.selected) {
            ui.separator();
            sheet::show_preview(ui, image, frames, clip);
        }
        if edit.delete {
            frames.clips.remove(self.selected);
            self.selected = self.selected.saturating_sub(1);
            self.unsaved = true;
        }
    }
}

/// The image of the sheet and how many cells it is cut into. Returns
/// whether they changed.
fn show_sheet_settings(ui: &mut egui::Ui, frames: &mut SpriteFrames, images: &[String]) -> bool {
    let texts = texts::current();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(texts.image);
        let current = frames.texture.as_str().to_owned();
        egui::ComboBox::from_id_salt("frames_image")
            .selected_text(&current)
            .show_ui(ui, |ui| {
                let others = images.iter().filter(|image| **image != current);
                for image in std::iter::once(&current).chain(others) {
                    if ui.selectable_label(*image == current, image).clicked()
                        && *image != current
                        && let Ok(path) = AssetPath::new(image)
                    {
                        frames.texture = path;
                        changed = true;
                    }
                }
            });
        ui.separator();
        ui.label(texts.sheet_columns);
        changed |= ui
            .add(egui::DragValue::new(&mut frames.columns).range(1..=MAX_CELLS_PER_SIDE))
            .changed();
        ui.label(texts.sheet_rows);
        changed |= ui
            .add(egui::DragValue::new(&mut frames.rows).range(1..=MAX_CELLS_PER_SIDE))
            .changed();
    });
    changed
}

/// What the user did to a clip in the inspector.
struct ClipEdit {
    changed: bool,
    delete: bool,
}

/// The name, speed, loop and frames of `clip`, and the button that
/// deletes it.
fn edit_clip(ui: &mut egui::Ui, clip: &mut FrameClip) -> ClipEdit {
    let texts = texts::current();
    let mut changed = false;
    egui::Grid::new("clip_properties")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label(texts.name);
            changed |= ui.text_edit_singleline(&mut clip.name).changed();
            ui.end_row();
            ui.label(texts.clip_speed());
            let speed = egui::DragValue::new(&mut clip.speed)
                .range(MIN_CLIP_SPEED..=MAX_CLIP_SPEED)
                .speed(SPEED_DRAG_STEP);
            changed |= ui.add(speed).changed();
            ui.end_row();
            ui.label(texts.clip_looping());
            changed |= ui.checkbox(&mut clip.looping, "").changed();
            ui.end_row();
        });
    ui.label(texts.clip_frames());
    let mut removed = None;
    ui.horizontal_wrapped(|ui| {
        for (index, frame) in clip.frames.iter().enumerate() {
            let button = ui
                .small_button(format!("{frame} ×"))
                .on_hover_text(texts.remove_frame());
            if button.clicked() {
                removed = Some(index);
            }
        }
    });
    if let Some(index) = removed {
        clip.frames.remove(index);
        changed = true;
    }
    if ui
        .add_enabled(
            !clip.frames.is_empty(),
            egui::Button::new(texts.clear_frames()),
        )
        .clicked()
    {
        clip.frames.clear();
        changed = true;
    }
    let delete = ui.button(texts.delete_clip()).clicked();
    ClipEdit { changed, delete }
}

/// A clip with a name no other of `clips` has, looping at the usual speed,
/// without frames yet: clicks on the sheet add them.
fn new_clip(clips: &[FrameClip]) -> FrameClip {
    let taken = |name: &str| clips.iter().any(|clip| clip.name == name);
    // One of as many names as there are clips, plus one, is always free.
    let name = (1..=clips.len() + 1)
        .map(|number| format!("{NEW_CLIP_PREFIX}{number}"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| NEW_CLIP_PREFIX.to_owned());
    FrameClip {
        name,
        frames: Vec::new(),
        speed: DEFAULT_CLIP_SPEED,
        looping: true,
    }
}

/// The texture of the image at `texture`, loaded into `image` the first
/// time and whenever the image changes; `None` if it cannot be loaded (the
/// reason is logged once).
fn loaded_image<'image>(
    image: &'image mut Option<(AssetPath, Option<egui::TextureHandle>)>,
    assets: &Path,
    texture: &AssetPath,
    ctx: &egui::Context,
) -> Option<&'image egui::TextureHandle> {
    if image.as_ref().is_none_or(|(path, _)| path != texture) {
        *image = Some((texture.clone(), load_texture(assets, texture, ctx)));
    }
    image.as_ref().and_then(|(_, loaded)| loaded.as_ref())
}

/// The image at `path` of the `assets` folder as a texture, kept sharp so
/// pixel art shows its cells as they are.
fn load_texture(
    assets: &Path,
    path: &AssetPath,
    ctx: &egui::Context,
) -> Option<egui::TextureHandle> {
    let image = AssetFolder::new(assets)
        .load_image(path)
        .inspect_err(|error| log::warn!("{error}"))
        .ok()?;
    let size = [
        usize::try_from(image.width()).ok()?,
        usize::try_from(image.height()).ok()?,
    ];
    Some(ctx.load_texture(
        format!("frames:{path}"),
        egui::ColorImage::from_rgba_unmultiplied(size, image.pixels()),
        egui::TextureOptions::NEAREST,
    ))
}

/// The text of a new `.frames` file of the image at `image`: the whole
/// image as one cell, without clips yet.
///
/// # Errors
///
/// The image's path is not a valid asset path.
pub(crate) fn new_frames_text(image: &str) -> Result<String, String> {
    let texture = AssetPath::new(image).map_err(|error| error.to_string())?;
    SpriteFrames::new(texture)
        .to_text()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    const HERO: &str = "format = 1\ntexture = \"hero.png\"\ncolumns = 4\nrows = 2\n\n\
                        [[clip]]\nname = \"run\"\nframes = [4, 5]\n";

    fn assets_with_hero(test_name: &str) -> TestDirectory {
        let assets = TestDirectory::new(test_name);
        fs::create_dir_all(assets.path()).expect("folder");
        fs::write(assets.path().join("hero.frames"), HERO).expect("file");
        assets
    }

    #[test]
    fn new_clips_get_names_of_their_own_and_no_frames() {
        let first = new_clip(&[]);
        let second = new_clip(std::slice::from_ref(&first));

        assert_eq!(
            (first.name.as_str(), second.name.as_str()),
            ("clip1", "clip2")
        );
        assert_eq!(first.frames, Vec::<u32>::new());
        assert!(first.looping);
    }

    #[test]
    fn frames_open_from_the_assets_folder_and_save_their_changes() {
        let assets = assets_with_hero("frames_tab_open");
        let mut tab = FramesTab::open(assets.path(), "hero.frames".to_owned());
        assert_eq!(tab.title(), "hero.frames");

        if let Ok(frames) = &mut tab.content {
            frames.clips[0].frames.push(6);
        }
        tab.unsaved = true;
        assert!(tab.title().starts_with("hero.frames "));
        assert!(tab.save());

        let text = fs::read_to_string(assets.path().join("hero.frames")).expect("saved");
        let saved = SpriteFrames::from_text(&text).expect("valid");
        assert_eq!(saved.clips[0].frames, [4, 5, 6]);
        assert!(!tab.is_unsaved());
    }

    #[test]
    fn invalid_frames_are_not_saved_and_say_why() {
        let assets = assets_with_hero("frames_tab_invalid");
        let mut tab = FramesTab::open(assets.path(), "hero.frames".to_owned());

        if let Ok(frames) = &mut tab.content {
            frames.clips.push(new_clip(&frames.clips));
        }
        tab.unsaved = true;

        assert!(!tab.save());
        let error = tab.save_error.clone().unwrap_or_default();
        assert!(
            error.contains("the clip \"clip1\" has no frames"),
            "{error}"
        );
        let text = fs::read_to_string(assets.path().join("hero.frames")).expect("kept");
        assert_eq!(text, HERO);
    }

    #[test]
    fn new_frames_files_name_their_image() {
        let text = new_frames_text("sprites/hero.png").expect("valid");

        let frames = SpriteFrames::from_text(&text).expect("readable");
        assert_eq!(frames.texture.as_str(), "sprites/hero.png");
        assert_eq!(frames.clips, Vec::new());
        assert!(new_frames_text("../outside.png").is_err());
    }
}
