//! GPU renderer bound to a window.

use star_core::{EngineError, EngineResult};
use star_platform::{Window, WindowSize};

use crate::mesh::Topology;
use crate::scene_renderer::SceneRenderer;
use crate::{Frame, GpuInfo, GraphicsBackend, Image, MeshId, TextureFilter, TextureId, Vertex};

/// Settings that control how the renderer is created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendererConfig {
    /// Native graphics API to use.
    pub backend: GraphicsBackend,
    /// Synchronize frames with the display refresh rate to avoid tearing.
    pub vsync: bool,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            backend: GraphicsBackend::Auto,
            vsync: true,
        }
    }
}

/// Draws frames into a window using the GPU.
pub struct Renderer {
    window: Window,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    gpu_info: GpuInfo,
    scene: SceneRenderer,
}

impl Renderer {
    /// Connects to the GPU and prepares `window` for drawing.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnsupportedPlatform`] if the requested backend
    /// does not exist on this platform, and [`EngineError::Initialization`] if
    /// no GPU supports it or the window cannot be drawn into.
    pub fn new(window: &Window, config: RendererConfig) -> EngineResult<Self> {
        ensure_backend_is_available(config.backend)?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: config.backend.to_wgpu(),
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(window.clone()))
        });
        let surface = create_surface(&instance, window)?;
        let adapter = request_adapter(&instance, &surface, config.backend)?;
        let (device, queue) = request_device(&adapter)?;
        let surface_config =
            surface_configuration(&surface, &adapter, window.size(), config.vsync)?;
        let scene = SceneRenderer::new(
            device,
            queue,
            surface_config.format,
            surface_config.width,
            surface_config.height,
        );

        let renderer = Self {
            window: window.clone(),
            instance,
            surface,
            surface_config,
            gpu_info: GpuInfo::from_wgpu(&adapter.get_info()),
            scene,
        };
        renderer.configure_surface();
        Ok(renderer)
    }

    /// GPU and graphics API in use.
    #[must_use]
    pub fn gpu_info(&self) -> &GpuInfo {
        &self.gpu_info
    }

    /// Uploads a triangle list to the GPU: every 3 vertices form a triangle.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Rendering`] if `vertices` is empty or not a
    /// multiple of 3.
    pub fn create_mesh(&mut self, vertices: &[Vertex]) -> EngineResult<MeshId> {
        self.scene.create_mesh(vertices, Topology::Triangles)
    }

    /// Uploads line segments to the GPU: every 2 vertices form a segment.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Rendering`] if `vertices` is empty or odd.
    pub fn create_lines(&mut self, vertices: &[Vertex]) -> EngineResult<MeshId> {
        self.scene.create_mesh(vertices, Topology::Lines)
    }

    /// Uploads `image` to the GPU, to be drawn with `filter`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Rendering`](star_core::EngineError::Rendering)
    /// if the GPU does not support textures that big.
    pub fn create_texture(
        &mut self,
        image: &Image,
        filter: TextureFilter,
    ) -> EngineResult<TextureId> {
        self.scene.create_texture(image, filter)
    }

    /// Changes the image of the texture `id`, e.g. after its file changed;
    /// the sprites that use it show the new image from the next frame on.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Rendering`](star_core::EngineError::Rendering)
    /// if `id` is not a texture of this renderer or the GPU does not support
    /// textures that big.
    pub fn replace_texture(
        &mut self,
        id: TextureId,
        image: &Image,
        filter: TextureFilter,
    ) -> EngineResult<()> {
        self.scene.replace_texture(id, image, filter)
    }

    /// Adapts the render target to a new window size.
    pub fn resize(&mut self, size: WindowSize) {
        if size.is_empty() {
            return;
        }
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.configure_surface();
        self.scene.resize(size.width, size.height);
    }

    /// Draws `frame` and shows it on screen.
    ///
    /// Frames that cannot be drawn right now (minimized window, outdated
    /// surface) are skipped silently.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Rendering`] if the camera has no valid
    /// orientation, a mesh is unknown or the GPU reports an unrecoverable
    /// error, and [`EngineError::CapacityExceeded`] if the frame has too many draws.
    pub fn render(&mut self, frame: &Frame) -> EngineResult<()> {
        let Some(aspect_ratio) = self.window.size().aspect_ratio() else {
            return Ok(());
        };
        let Some(surface_texture) = self.acquire_frame()? else {
            return Ok(());
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.scene.render(&view, aspect_ratio, frame)?;
        self.scene.queue().present(surface_texture);
        Ok(())
    }

    fn acquire_frame(&mut self) -> EngineResult<Option<wgpu::SurfaceTexture>> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => Ok(Some(frame)),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                drop(frame);
                self.configure_surface();
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.recreate_surface()?;
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Validation => Err(EngineError::Rendering(
                "validation error while acquiring the next frame".to_owned(),
            )),
        }
    }

    fn configure_surface(&self) {
        self.surface
            .configure(self.scene.device(), &self.surface_config);
    }

    fn recreate_surface(&mut self) -> EngineResult<()> {
        self.surface = create_surface(&self.instance, &self.window)?;
        self.configure_surface();
        Ok(())
    }
}

fn initialization_error(reason: impl Into<String>) -> EngineError {
    EngineError::Initialization {
        subsystem: "renderer",
        reason: reason.into(),
    }
}

fn ensure_backend_is_available(backend: GraphicsBackend) -> EngineResult<()> {
    if wgpu::Instance::enabled_backend_features().intersects(backend.to_wgpu()) {
        Ok(())
    } else {
        Err(EngineError::UnsupportedPlatform(format!(
            "{backend} is not available on this platform"
        )))
    }
}

fn create_surface(
    instance: &wgpu::Instance,
    window: &Window,
) -> EngineResult<wgpu::Surface<'static>> {
    instance
        .create_surface(window.clone())
        .map_err(|error| initialization_error(error.to_string()))
}

fn request_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    backend: GraphicsBackend,
) -> EngineResult<wgpu::Adapter> {
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(surface),
        ..Default::default()
    };
    pollster::block_on(instance.request_adapter(&options)).map_err(|error| {
        initialization_error(format!("no GPU available for backend {backend}: {error}"))
    })
}

fn request_device(adapter: &wgpu::Adapter) -> EngineResult<(wgpu::Device, wgpu::Queue)> {
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("star_render device"),
        // Conservative limits keep older GPUs and OpenGL ES devices supported,
        // while still allowing textures as large as the screen.
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    };
    pollster::block_on(adapter.request_device(&descriptor))
        .map_err(|error| initialization_error(error.to_string()))
}

fn surface_configuration(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    size: WindowSize,
    vsync: bool,
) -> EngineResult<wgpu::SurfaceConfiguration> {
    let mut surface_config = surface
        .get_default_config(adapter, size.width.max(1), size.height.max(1))
        .ok_or_else(|| initialization_error("the GPU cannot draw into this window"))?;

    let capabilities = surface.get_capabilities(adapter);
    if let Some(srgb_format) = capabilities.formats.iter().find(|format| format.is_srgb()) {
        surface_config.format = *srgb_format;
    }
    surface_config.present_mode = if vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
    };
    Ok(surface_config)
}
