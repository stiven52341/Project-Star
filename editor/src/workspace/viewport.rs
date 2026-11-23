//! Central area where the engine renders the scene.
//!
//! The engine draws into an offscreen texture on the same GPU device as the
//! UI; egui then shows that texture like any image, with no CPU copies.
//! Over the grid it draws the open scene, as the game would show it, with
//! the node selected marked.

use std::path::Path;
use std::sync::Arc;

use eframe::egui;
use eframe::egui_wgpu;
use star_engine::prelude::{Color, EngineResult, Frame, Mat4, MeshId, Vec2};
use star_render::interop::{OffscreenRenderer, wgpu};

use super::editor_camera::EditorCamera;
use super::grid::grid_lines;
use super::viewport_assets::ViewportAssets;
use crate::project::ProjectDimension;
use crate::scene_editor::{Marks, Ray, SceneDocument, cross_lines, cube_lines, square_lines};
use crate::style::ERROR_COLOR;
use crate::texts;

const BACKGROUND: Color = Color::rgb(0.12, 0.12, 0.14);
const LABEL_MARGIN: f32 = 10.0;
const LABEL_SIZE: f32 = 14.0;
const FULL_TEXTURE: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

/// Scene view rendered by the engine, with a camera the user can move.
pub(crate) struct SceneViewport {
    renderer: OffscreenRenderer,
    device: wgpu::Device,
    ui_renderer: Arc<egui::mutex::RwLock<egui_wgpu::Renderer>>,
    texture_id: egui::TextureId,
    grid: MeshId,
    marks: Marks,
    assets: ViewportAssets,
    camera: EditorCamera,
    dimension: ProjectDimension,
    frame: Frame,
    render_error: Option<String>,
}

impl SceneViewport {
    /// Prepares the engine renderer on the UI's GPU device, to show scenes
    /// with the images of the `assets` folder.
    pub(crate) fn new(
        render_state: &egui_wgpu::RenderState,
        dimension: ProjectDimension,
        assets: &Path,
    ) -> EngineResult<Self> {
        let mut renderer =
            OffscreenRenderer::new(render_state.device.clone(), render_state.queue.clone());
        let grid = renderer.create_lines(&grid_lines(dimension))?;
        let marks = Marks {
            square: renderer.create_lines(&square_lines())?,
            cube: renderer.create_lines(&cube_lines())?,
            cross: renderer.create_lines(&cross_lines())?,
        };
        let texture_id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            renderer.color_view(),
            wgpu::FilterMode::Linear,
        );
        Ok(Self {
            renderer,
            device: render_state.device.clone(),
            ui_renderer: Arc::clone(&render_state.renderer),
            texture_id,
            grid,
            marks,
            assets: ViewportAssets::new(assets),
            camera: match dimension {
                ProjectDimension::TwoD => EditorCamera::new_2d(),
                ProjectDimension::ThreeD => EditorCamera::new_3d(),
            },
            dimension,
            frame: Frame::new(),
            render_error: None,
        })
    }

    /// Shows the view, with `scene` drawn in it if one is open; a click
    /// selects the node under the pointer.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, mut scene: Option<&mut SceneDocument>) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        self.handle_input(ui, &response, rect);
        if let Some(scene) = scene.as_deref_mut() {
            self.select_clicked(&response, rect, scene);
        }
        self.match_target_size(rect, ui.ctx().pixels_per_point());
        self.render_error = self
            .render(scene.as_deref())
            .err()
            .map(|error| error.to_string());

        let painter = ui.painter_at(rect);
        painter.image(self.texture_id, rect, FULL_TEXTURE, egui::Color32::WHITE);
        self.paint_overlay(ui, &painter, rect);
    }

    fn handle_input(&mut self, ui: &egui::Ui, response: &egui::Response, rect: egui::Rect) {
        if response.dragged() {
            let delta = response.drag_delta();
            self.camera.drag(Vec2::new(delta.x, delta.y), rect.height());
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > f32::EPSILON {
                self.camera.zoom(scroll);
            }
        }
    }

    /// Selects, in `scene`, the node under the pointer when the view is
    /// clicked.
    fn select_clicked(
        &self,
        response: &egui::Response,
        rect: egui::Rect,
        scene: &mut SceneDocument,
    ) {
        if !response.clicked() {
            return;
        }
        let Some(pointer) = response.interact_pointer_pos() else {
            return;
        };
        let ray = self
            .camera
            .to_camera()
            .view_projection(rect.aspect_ratio())
            .and_then(|view_projection| Ray::through(view_projection, to_screen(pointer, rect)));
        if let Some(ray) = ray {
            scene.select_at(ray);
        }
    }

    /// Resizes the engine's texture to the viewport's physical pixels and
    /// points egui at the new texture when it changes.
    fn match_target_size(&mut self, rect: egui::Rect, pixels_per_point: f32) {
        let size = rect.size() * pixels_per_point;
        if self.renderer.resize(to_pixels(size.x), to_pixels(size.y)) {
            self.ui_renderer
                .write()
                .update_egui_texture_from_wgpu_texture(
                    &self.device,
                    self.renderer.color_view(),
                    wgpu::FilterMode::Linear,
                    self.texture_id,
                );
        }
    }

    fn render(&mut self, scene: Option<&SceneDocument>) -> EngineResult<()> {
        self.frame.begin(BACKGROUND);
        self.frame.set_camera(self.camera.to_camera());
        self.frame.draw(self.grid, Mat4::IDENTITY);
        if let Some(scene) = scene {
            let mut gpu = self.assets.with(&mut self.renderer);
            scene.draw(&mut gpu, self.marks, &mut self.frame);
        }
        self.renderer.render(&self.frame)
    }

    fn paint_overlay(&self, ui: &egui::Ui, painter: &egui::Painter, rect: egui::Rect) {
        let font = egui::FontId::proportional(LABEL_SIZE);
        let top_left = rect.left_top() + egui::vec2(LABEL_MARGIN, LABEL_MARGIN);
        painter.text(
            top_left,
            egui::Align2::LEFT_TOP,
            texts::current().viewport_title(self.dimension.label()),
            font.clone(),
            ui.visuals().strong_text_color(),
        );
        let (hint, color) = match &self.render_error {
            Some(error) => (texts::current().viewport_error(error), ERROR_COLOR),
            None => (
                texts::current()
                    .viewport_controls(self.dimension)
                    .to_owned(),
                ui.visuals().weak_text_color(),
            ),
        };
        painter.text(
            rect.center_bottom() - egui::vec2(0.0, LABEL_MARGIN),
            egui::Align2::CENTER_BOTTOM,
            hint,
            font,
            color,
        );
    }
}

impl Drop for SceneViewport {
    fn drop(&mut self) {
        self.ui_renderer.write().free_texture(&self.texture_id);
    }
}

/// Where `pointer` is in the view `rect`: from `(-1, -1)` at its bottom
/// left to `(1, 1)` at its top right, as the GPU sees the screen.
fn to_screen(pointer: egui::Pos2, rect: egui::Rect) -> Vec2 {
    let across = (pointer - rect.left_top()) / rect.size();
    Vec2::new(across.x * 2.0 - 1.0, 1.0 - across.y * 2.0)
}

/// Rounds a non-negative on-screen length to whole pixels.
// Viewport sizes are small positive numbers, so the conversion cannot overflow
// or lose a sign.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_pixels(length: f32) -> u32 {
    length.max(0.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_are_rounded_to_whole_pixels() {
        assert_eq!(to_pixels(99.6), 100);
    }

    #[test]
    fn pointers_are_placed_on_the_gpu_screen() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 100.0));

        assert_eq!(to_screen(rect.left_top(), rect), Vec2::new(-1.0, 1.0));
        assert_eq!(to_screen(rect.center(), rect), Vec2::new(0.0, 0.0));
        assert_eq!(to_screen(rect.right_bottom(), rect), Vec2::new(1.0, -1.0));
    }

    #[test]
    fn negative_lengths_become_zero() {
        assert_eq!(to_pixels(-3.0), 0);
    }
}
