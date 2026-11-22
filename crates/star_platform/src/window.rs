//! Native window handle and its configuration.

use std::sync::Arc;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};

const DEFAULT_TITLE: &str = "ProjectStar";
const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 720;

/// Size of a window's drawable area, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl WindowSize {
    /// Creates a size from its dimensions.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Whether nothing can be drawn, e.g. because the window is minimized.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Width divided by height, or `None` if the size is empty.
    #[must_use]
    // Pixel dimensions are far below 2^24, so converting them to `f32` is exact.
    #[allow(clippy::cast_precision_loss)]
    pub fn aspect_ratio(self) -> Option<f32> {
        if self.is_empty() {
            None
        } else {
            Some(self.width as f32 / self.height as f32)
        }
    }
}

/// Settings used to create the main window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    /// Text shown in the title bar.
    pub title: String,
    /// Initial size of the drawable area, in logical pixels.
    pub size: WindowSize,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: DEFAULT_TITLE.to_owned(),
            size: WindowSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT),
        }
    }
}

/// A native window. Cheap to clone: all clones refer to the same window.
///
/// Exposes raw handles so rendering backends can draw into it without
/// depending on the windowing library.
#[derive(Debug, Clone)]
pub struct Window {
    native: Arc<winit::window::Window>,
}

impl Window {
    pub(crate) fn new(native: winit::window::Window) -> Self {
        Self {
            native: Arc::new(native),
        }
    }

    /// Current size of the drawable area, in physical pixels.
    #[must_use]
    pub fn size(&self) -> WindowSize {
        let size = self.native.inner_size();
        WindowSize::new(size.width, size.height)
    }

    pub(crate) fn request_redraw(&self) {
        self.native.request_redraw();
    }

    pub(crate) fn is(&self, id: winit::window::WindowId) -> bool {
        self.native.id() == id
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.native.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.native.display_handle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_with_a_zero_dimension_is_empty() {
        assert!(WindowSize::new(0, 720).is_empty());
        assert!(WindowSize::new(1280, 0).is_empty());
        assert!(!WindowSize::new(1280, 720).is_empty());
    }

    #[test]
    fn aspect_ratio_divides_width_by_height() {
        assert_eq!(WindowSize::new(1600, 800).aspect_ratio(), Some(2.0));
    }

    #[test]
    fn empty_size_has_no_aspect_ratio() {
        assert_eq!(WindowSize::new(1280, 0).aspect_ratio(), None);
    }
}
