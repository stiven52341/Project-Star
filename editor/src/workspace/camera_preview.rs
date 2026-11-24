//! A small picture, in a corner of the view, of what a camera of the open
//! scene sees: the camera selected, or else the scene's main camera.
//!
//! It is drawn by the view's renderer, with the same textures and meshes,
//! into a target of its own, and shows the scene as the game shows it
//! before its code changes anything: without the editor's grid and marks.

use std::sync::Arc;

use eframe::egui;
use eframe::egui_wgpu;
use star_engine::prelude::{Camera, Color, EngineResult, Frame};
use star_render::interop::{OffscreenRenderer, OffscreenTarget, wgpu};

use super::viewport::to_pixels;
use super::viewport_assets::ViewportAssets;
use crate::scene_editor::{SceneDocument, game_aspect_ratio};
use crate::texts;

/// The width of the picture, in points, and the most of the view's width
/// it takes.
const PREVIEW_WIDTH: f32 = 240.0;
const MOST_OF_VIEW: f32 = 0.4;
/// Space between the picture and the view's right and bottom edges; the
/// bottom one leaves room for the view's hint.
const RIGHT_MARGIN: f32 = 10.0;
const BOTTOM_MARGIN: f32 = 36.0;
/// The camera's name above the picture.
const NAME_SIZE: f32 = 12.0;
const NAME_GAP: f32 = 4.0;
const BORDER_WIDTH: f32 = 1.0;
const FULL_TEXTURE: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

/// The target the camera's view is drawn into, shown by egui.
pub(crate) struct CameraPreview {
    target: OffscreenTarget,
    texture_id: egui::TextureId,
    device: wgpu::Device,
    ui_renderer: Arc<egui::mutex::RwLock<egui_wgpu::Renderer>>,
    frame: Frame,
    /// What was drawn last: the camera's name, or why it could not be.
    shown: Option<Result<String, String>>,
}

impl CameraPreview {
    /// Prepares a target for `renderer`, shown through egui's renderer.
    pub(crate) fn new(render_state: &egui_wgpu::RenderState, renderer: &OffscreenRenderer) -> Self {
        let target = OffscreenTarget::new(renderer);
        let texture_id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            target.color_view(),
            wgpu::FilterMode::Linear,
        );
        Self {
            target,
            texture_id,
            device: render_state.device.clone(),
            ui_renderer: Arc::clone(&render_state.renderer),
            frame: Frame::new(),
            shown: None,
        }
    }

    /// Draws what the camera to preview in `scene` sees, at the size the
    /// picture has in `view` (with `pixels_per_point`); nothing if the
    /// scene has no camera to preview.
    pub(crate) fn render(
        &mut self,
        renderer: &mut OffscreenRenderer,
        assets: &mut ViewportAssets,
        scene: Option<&SceneDocument>,
        view: egui::Rect,
        pixels_per_point: f32,
    ) {
        let Some((scene, (name, camera))) =
            scene.and_then(|scene| Some((scene, scene.preview_camera()?)))
        else {
            self.shown = None;
            return;
        };
        let size = preview_rect(view).size() * pixels_per_point;
        if self
            .target
            .resize(renderer, to_pixels(size.x), to_pixels(size.y))
        {
            self.ui_renderer
                .write()
                .update_egui_texture_from_wgpu_texture(
                    &self.device,
                    self.target.color_view(),
                    wgpu::FilterMode::Linear,
                    self.texture_id,
                );
        }
        let drawn = self.draw(renderer, assets, scene, camera);
        self.shown = Some(
            drawn
                .map(|()| name.to_owned())
                .map_err(|error| error.to_string()),
        );
    }

    fn draw(
        &mut self,
        renderer: &mut OffscreenRenderer,
        assets: &mut ViewportAssets,
        scene: &SceneDocument,
        camera: Camera,
    ) -> EngineResult<()> {
        self.frame.begin(Color::BLACK);
        self.frame.set_camera(camera);
        scene.draw_game_view(&mut assets.with(renderer), &mut self.frame);
        renderer.render_into(&self.target, &self.frame)
    }

    /// Paints the picture last drawn in a corner of `view`, with the
    /// camera's name above it.
    pub(crate) fn paint(&self, ui: &egui::Ui, painter: &egui::Painter, view: egui::Rect) {
        let Some(shown) = &self.shown else {
            return;
        };
        let rect = preview_rect(view);
        let visuals = ui.visuals();
        let name = match shown {
            Ok(name) => {
                painter.image(self.texture_id, rect, FULL_TEXTURE, egui::Color32::WHITE);
                name.clone()
            }
            Err(error) => {
                painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
                texts::current().viewport_error(error)
            }
        };
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(BORDER_WIDTH, visuals.widgets.noninteractive.bg_stroke.color),
            egui::StrokeKind::Outside,
        );
        painter.text(
            rect.left_top() - egui::vec2(0.0, NAME_GAP),
            egui::Align2::LEFT_BOTTOM,
            name,
            egui::FontId::proportional(NAME_SIZE),
            visuals.strong_text_color(),
        );
    }
}

impl Drop for CameraPreview {
    fn drop(&mut self) {
        self.ui_renderer.write().free_texture(&self.texture_id);
    }
}

/// Where the picture goes in `view`: in its bottom-right corner, with the
/// shape of the game's window.
fn preview_rect(view: egui::Rect) -> egui::Rect {
    let width = PREVIEW_WIDTH.min(view.width() * MOST_OF_VIEW);
    let size = egui::vec2(width, width / game_aspect_ratio());
    let bottom_right = view.right_bottom() - egui::vec2(RIGHT_MARGIN, BOTTOM_MARGIN);
    egui::Rect::from_min_size(bottom_right - size, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_sit_in_the_corner_with_the_game_s_shape() {
        let view = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 600.0));

        let rect = preview_rect(view);

        assert_eq!(rect.right_bottom(), egui::pos2(990.0, 564.0));
        assert!((rect.width() - PREVIEW_WIDTH).abs() < f32::EPSILON);
        assert!((rect.aspect_ratio() - game_aspect_ratio()).abs() < 1e-4);
    }

    #[test]
    fn narrow_views_get_smaller_previews() {
        let view = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 600.0));

        assert!((preview_rect(view).width() - 120.0).abs() < 1e-4);
    }
}
