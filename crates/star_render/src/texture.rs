//! Images stored on the GPU, ready to be drawn.

use star_core::{EngineError, EngineResult};
use wgpu::util::DeviceExt as _;

use crate::Image;

/// How a texture is sampled when it is drawn bigger or smaller than its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureFilter {
    /// Blends neighboring pixels: soft results for painted art and photos.
    #[default]
    Smooth,
    /// Takes the nearest pixel: sharp, blocky edges for pixel art.
    Pixelated,
}

/// Handle to a texture uploaded with [`Renderer::create_texture`](crate::Renderer::create_texture).
///
/// Only valid for the renderer that created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub(crate) usize);

impl TextureId {
    /// The id of the `index`-th texture a renderer created. Ids normally come
    /// from the renderer; one it did not create is reported as an error when
    /// drawn, so this is meant for tests and tools that store ids.
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }
}

/// A texture together with the way it is sampled, as the shaders bind it.
pub(crate) struct GpuTexture {
    pub(crate) bind_group: wgpu::BindGroup,
    /// Width and height in pixels.
    pub(crate) size: [u32; 2],
}

/// What every texture shares: how shaders see it, its samplers and its format.
pub(crate) struct TextureBindings {
    layout: wgpu::BindGroupLayout,
    smooth_sampler: wgpu::Sampler,
    pixelated_sampler: wgpu::Sampler,
    format: wgpu::TextureFormat,
}

impl TextureBindings {
    /// Prepares textures for a target of `target_format`. Images hold sRGB
    /// values: sRGB targets get sRGB textures, which the GPU decodes to
    /// linear and encodes again; other targets get the values unchanged.
    pub(crate) fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        Self {
            layout: create_bind_group_layout(device),
            smooth_sampler: create_sampler(device, wgpu::FilterMode::Linear),
            pixelated_sampler: create_sampler(device, wgpu::FilterMode::Nearest),
            format: if target_format.is_srgb() {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
        }
    }

    /// Layout of the bind group of every texture: the texture, then its sampler.
    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// Copies `image` to the GPU.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Rendering`] if the GPU does not support images
    /// that big.
    pub(crate) fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &Image,
        filter: TextureFilter,
    ) -> EngineResult<GpuTexture> {
        ensure_fits(image, device.limits().max_texture_dimension_2d)?;
        let sampler = match filter {
            TextureFilter::Smooth => &self.smooth_sampler,
            TextureFilter::Pixelated => &self.pixelated_sampler,
        };
        let pixels = Pixels {
            width: image.width(),
            height: image.height(),
            rgba: image.pixels(),
        };
        Ok(self.create(device, queue, &pixels, sampler))
    }

    /// Copies `pixels`, which fit the GPU, to a texture sampled by `sampler`.
    fn create(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pixels: &Pixels<'_>,
        sampler: &wgpu::Sampler,
    ) -> GpuTexture {
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("texture"),
                size: wgpu::Extent3d {
                    width: pixels.width,
                    height: pixels.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            pixels.rgba,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        GpuTexture {
            bind_group,
            size: [pixels.width, pixels.height],
        }
    }
}

/// RGBA8 pixels in sRGB, row by row.
struct Pixels<'a> {
    width: u32,
    height: u32,
    rgba: &'a [u8],
}

impl TextureBindings {
    /// A texture of one white pixel, which shows a color as it is.
    pub(crate) fn upload_white(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> GpuTexture {
        let white = Pixels {
            width: 1,
            height: 1,
            rgba: &[u8::MAX; 4],
        };
        self.create(device, queue, &white, &self.pixelated_sampler)
    }
}

/// Checks that the GPU can hold `image`, whose sides may be at most `max_side` pixels.
fn ensure_fits(image: &Image, max_side: u32) -> EngineResult<()> {
    let (width, height) = (image.width(), image.height());
    if width > max_side || height > max_side {
        return Err(EngineError::Rendering(format!(
            "this GPU supports textures of up to {max_side}x{max_side} pixels, got {width}x{height}"
        )));
    }
    Ok(())
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("texture"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// Texture coordinates outside `0..1` repeat the edge pixels instead of
/// wrapping, so sprites cut from a sheet never bleed in their neighbors.
fn create_sampler(device: &wgpu::Device, filter: wgpu::FilterMode) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("texture sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter,
        min_filter: filter,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32) -> Image {
        Image::from_rgba8(width, height, vec![0; (width * height * 4) as usize]).unwrap()
    }

    #[test]
    fn images_within_the_limit_fit() {
        assert!(ensure_fits(&image(4, 2), 4).is_ok());
    }

    #[test]
    fn images_beyond_the_limit_are_rejected() {
        assert!(ensure_fits(&image(1, 5), 4).is_err());
    }
}
