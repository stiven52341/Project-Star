//! Selection of the native graphics API.

use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// Native graphics API used to talk to the GPU.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GraphicsBackend {
    /// Let the engine pick the best API available on the platform.
    #[default]
    Auto,
    /// Vulkan (Windows, Linux, Android).
    Vulkan,
    /// DirectX 12 (Windows).
    DirectX12,
    /// Metal (macOS, iOS).
    Metal,
    /// OpenGL / OpenGL ES (fallback for older hardware).
    OpenGl,
}

impl GraphicsBackend {
    /// Human-readable name of the API.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Vulkan => "Vulkan",
            Self::DirectX12 => "DirectX 12",
            Self::Metal => "Metal",
            Self::OpenGl => "OpenGL",
        }
    }

    pub(crate) fn to_wgpu(self) -> wgpu::Backends {
        match self {
            Self::Auto => wgpu::Backends::all(),
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::DirectX12 => wgpu::Backends::DX12,
            Self::Metal => wgpu::Backends::METAL,
            Self::OpenGl => wgpu::Backends::GL,
        }
    }

    pub(crate) fn from_wgpu(backend: wgpu::Backend) -> Option<Self> {
        match backend {
            wgpu::Backend::Vulkan => Some(Self::Vulkan),
            wgpu::Backend::Dx12 => Some(Self::DirectX12),
            wgpu::Backend::Metal => Some(Self::Metal),
            wgpu::Backend::Gl => Some(Self::OpenGl),
            _ => None,
        }
    }
}

impl fmt::Display for GraphicsBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

impl FromStr for GraphicsBackend {
    type Err = UnknownBackendError;

    /// Parses names such as `vulkan`, `dx12`, `metal` or `opengl` (case-insensitive).
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "vulkan" | "vk" => Ok(Self::Vulkan),
            "dx12" | "directx12" | "d3d12" => Ok(Self::DirectX12),
            "metal" => Ok(Self::Metal),
            "opengl" | "gl" | "gles" => Ok(Self::OpenGl),
            _ => Err(UnknownBackendError(text.to_owned())),
        }
    }
}

/// Returned when a text does not name a known [`GraphicsBackend`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownBackendError(String);

impl fmt::Display for UnknownBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unknown graphics backend \"{}\" (expected auto, vulkan, dx12, metal or opengl)",
            self.0
        )
    }
}

impl Error for UnknownBackendError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_aliases_case_insensitively() {
        assert_eq!("Vulkan".parse(), Ok(GraphicsBackend::Vulkan));
        assert_eq!("DX12".parse(), Ok(GraphicsBackend::DirectX12));
        assert_eq!("gl".parse(), Ok(GraphicsBackend::OpenGl));
    }

    #[test]
    fn rejects_unknown_names() {
        assert!("directx9".parse::<GraphicsBackend>().is_err());
    }

    #[test]
    fn specific_backends_map_both_ways_to_wgpu() {
        for (backend, wgpu_backend) in [
            (GraphicsBackend::Vulkan, wgpu::Backend::Vulkan),
            (GraphicsBackend::DirectX12, wgpu::Backend::Dx12),
            (GraphicsBackend::Metal, wgpu::Backend::Metal),
            (GraphicsBackend::OpenGl, wgpu::Backend::Gl),
        ] {
            assert_eq!(backend.to_wgpu(), wgpu::Backends::from(wgpu_backend));
            assert_eq!(GraphicsBackend::from_wgpu(wgpu_backend), Some(backend));
        }
    }
}
