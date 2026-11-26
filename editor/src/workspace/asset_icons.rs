//! The icons of the assets panel: a folder, or a page with a colored tag
//! saying what kind of file it is. Images show themselves instead (see
//! `thumbnails`), with a page as long as they cannot.

use eframe::egui;
use star_engine::prelude::AssetPath;
use star_engine::star_assets::{ImageFormat, SoundFormat};

use crate::project::{
    BLUEPRINT_EXTENSION, FRAMES_EXTENSION, KEYFRAMES_EXTENSION, RUST_EXTENSION, SCENE_EXTENSION,
    SCRIPT_EXTENSION,
};

/// Most letters of an extension shown on a page's tag.
const MAX_TAG_LETTERS: usize = 5;

const FOLDER_COLOR: egui::Color32 = egui::Color32::from_rgb(222, 180, 85);
const FOLDER_TAB_COLOR: egui::Color32 = egui::Color32::from_rgb(190, 150, 65);
const PAGE_COLOR: egui::Color32 = egui::Color32::from_rgb(210, 212, 218);
const PAGE_FOLD_COLOR: egui::Color32 = egui::Color32::from_rgb(160, 162, 170);
const TAG_TEXT_COLOR: egui::Color32 = egui::Color32::WHITE;
const SCENE_COLOR: egui::Color32 = egui::Color32::from_rgb(70, 130, 220);
const BLUEPRINT_COLOR: egui::Color32 = egui::Color32::from_rgb(150, 95, 210);
const SCRIPT_COLOR: egui::Color32 = egui::Color32::from_rgb(70, 170, 100);
const RUST_COLOR: egui::Color32 = egui::Color32::from_rgb(210, 110, 55);
const IMAGE_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 80, 140);
const FRAMES_COLOR: egui::Color32 = egui::Color32::from_rgb(225, 150, 40);
const KEYFRAMES_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 90, 160);
const SOUND_COLOR: egui::Color32 = egui::Color32::from_rgb(40, 160, 170);
const OTHER_COLOR: egui::Color32 = egui::Color32::from_rgb(120, 122, 130);

/// Parts of an icon, as fractions of its size.
const PAGE_WIDTH: f32 = 0.72;
const PAGE_FOLD: f32 = 0.22;
const TAG_HEIGHT: f32 = 0.26;
/// How far up from the page's bottom its tag is.
const TAG_LIFT: f32 = 0.12;
const TAG_TEXT_SIZE: f32 = 0.17;
const FOLDER_HEIGHT: f32 = 0.68;
const FOLDER_TAB_WIDTH: f32 = 0.4;
const FOLDER_TAB_HEIGHT: f32 = 0.1;
const ROUNDING: f32 = 3.0;

/// What a file of the assets folder is, by its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AssetKind {
    Folder,
    Scene,
    Blueprint,
    Script,
    Rust,
    Frames,
    Keyframes,
    Image,
    Sound,
    Other,
}

impl AssetKind {
    /// The kind of the file called `name`.
    pub(super) fn of_file(name: &str) -> Self {
        let lower = name.to_lowercase();
        let path = AssetPath::new(name);
        if lower.ends_with(SCENE_EXTENSION) {
            Self::Scene
        } else if lower.ends_with(BLUEPRINT_EXTENSION) {
            Self::Blueprint
        } else if lower.ends_with(SCRIPT_EXTENSION) {
            Self::Script
        } else if lower.ends_with(RUST_EXTENSION) {
            Self::Rust
        } else if lower.ends_with(FRAMES_EXTENSION) {
            Self::Frames
        } else if lower.ends_with(KEYFRAMES_EXTENSION) {
            Self::Keyframes
        } else if path
            .as_ref()
            .is_ok_and(|path| ImageFormat::of(path).is_ok())
        {
            Self::Image
        } else if path
            .as_ref()
            .is_ok_and(|path| SoundFormat::of(path).is_ok())
        {
            Self::Sound
        } else {
            Self::Other
        }
    }

    const fn color(self) -> egui::Color32 {
        match self {
            Self::Folder => FOLDER_COLOR,
            Self::Scene => SCENE_COLOR,
            Self::Blueprint => BLUEPRINT_COLOR,
            Self::Script => SCRIPT_COLOR,
            Self::Rust => RUST_COLOR,
            Self::Frames => FRAMES_COLOR,
            Self::Keyframes => KEYFRAMES_COLOR,
            Self::Image => IMAGE_COLOR,
            Self::Sound => SOUND_COLOR,
            Self::Other => OTHER_COLOR,
        }
    }
}

/// What the tag of blueprints says: their extension is too long to fit.
const BLUEPRINT_TAG: &str = "BP";

/// The text on the tag of the page of the file `name` of `kind`: its
/// extension in capitals, short enough to fit.
fn tag_of(kind: AssetKind, name: &str) -> String {
    if kind == AssetKind::Blueprint {
        return BLUEPRINT_TAG.to_owned();
    }
    name.rsplit_once('.')
        .map(|(_, extension)| extension.chars().take(MAX_TAG_LETTERS).collect::<String>())
        .unwrap_or_default()
        .to_uppercase()
}

/// Paints the icon of the file `name` of `kind` filling `rect`, a square.
pub(super) fn paint_icon(painter: &egui::Painter, rect: egui::Rect, kind: AssetKind, name: &str) {
    if kind == AssetKind::Folder {
        paint_folder(painter, rect);
    } else {
        paint_page(painter, rect, kind.color(), &tag_of(kind, name));
    }
}

fn paint_folder(painter: &egui::Painter, rect: egui::Rect) {
    let size = rect.width();
    let body = egui::Rect::from_center_size(rect.center(), egui::vec2(size, size * FOLDER_HEIGHT));
    let tab = egui::Rect::from_min_size(
        body.left_top() - egui::vec2(0.0, size * FOLDER_TAB_HEIGHT),
        egui::vec2(size * FOLDER_TAB_WIDTH, size * FOLDER_TAB_HEIGHT * 2.0),
    );
    painter.rect_filled(tab, ROUNDING, FOLDER_TAB_COLOR);
    painter.rect_filled(body, ROUNDING, FOLDER_COLOR);
}

/// A page with its top right corner folded and a tag of `color` with
/// `tag` on it.
fn paint_page(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32, tag: &str) {
    let size = rect.width();
    let page = egui::Rect::from_center_size(rect.center(), egui::vec2(size * PAGE_WIDTH, size));
    let fold = size * PAGE_FOLD;
    let outline = vec![
        page.left_top(),
        page.right_top() - egui::vec2(fold, 0.0),
        page.right_top() + egui::vec2(0.0, fold),
        page.right_bottom(),
        page.left_bottom(),
    ];
    painter.add(egui::Shape::convex_polygon(
        outline,
        PAGE_COLOR,
        egui::Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![
            page.right_top() - egui::vec2(fold, 0.0),
            page.right_top() + egui::vec2(-fold, fold),
            page.right_top() + egui::vec2(0.0, fold),
        ],
        PAGE_FOLD_COLOR,
        egui::Stroke::NONE,
    ));
    let band = egui::Rect::from_min_max(
        egui::pos2(rect.left(), page.bottom() - size * (TAG_LIFT + TAG_HEIGHT)),
        egui::pos2(rect.right(), page.bottom() - size * TAG_LIFT),
    );
    painter.rect_filled(band, ROUNDING, color);
    painter.text(
        band.center(),
        egui::Align2::CENTER_CENTER,
        tag,
        egui::FontId::proportional(size * TAG_TEXT_SIZE),
        TAG_TEXT_COLOR,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_come_from_the_name() {
        let kinds = [
            "level.scene",
            "Player.blueprint",
            "enemy.star",
            "main.rs",
            "hero.PNG",
            "jump.ogg",
            "notes.txt",
        ]
        .map(AssetKind::of_file);

        assert_eq!(
            kinds,
            [
                AssetKind::Scene,
                AssetKind::Blueprint,
                AssetKind::Script,
                AssetKind::Rust,
                AssetKind::Image,
                AssetKind::Sound,
                AssetKind::Other
            ]
        );
    }

    #[test]
    fn tags_are_short_capital_extensions() {
        assert_eq!(tag_of(AssetKind::Scene, "level.scene"), "SCENE");
        assert_eq!(tag_of(AssetKind::Blueprint, "Player.blueprint"), "BP");
        assert_eq!(tag_of(AssetKind::Other, "archive.backup"), "BACKU");
        assert_eq!(tag_of(AssetKind::Other, "README"), "");
    }
}
