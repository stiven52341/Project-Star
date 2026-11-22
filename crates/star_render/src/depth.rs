//! Depth buffer: lets nearer triangles hide the ones behind them.

/// Format of the depth buffer, also declared by every pipeline that uses it.
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Depth value of the far plane; the buffer is cleared to it every frame.
pub(crate) const FARTHEST_DEPTH: f32 = 1.0;

/// Per-pixel distance to the camera, matching the size of the render target.
pub(crate) struct DepthBuffer {
    view: wgpu::TextureView,
}

impl DepthBuffer {
    pub(crate) fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth buffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
        }
    }

    pub(crate) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
