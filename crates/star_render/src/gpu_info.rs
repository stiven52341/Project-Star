//! Description of the GPU in use.

use std::fmt;

use crate::GraphicsBackend;

/// Which GPU and graphics API the renderer ended up using.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuInfo {
    /// Name of the GPU, as reported by the driver.
    pub adapter_name: String,
    /// Name of the graphics API in use.
    pub backend_name: &'static str,
    /// Whether rendering runs on the CPU (software emulation), which is
    /// much slower and usually means the GPU driver is missing or incomplete.
    pub is_software: bool,
}

impl GpuInfo {
    pub(crate) fn from_wgpu(info: &wgpu::AdapterInfo) -> Self {
        let backend_name = GraphicsBackend::from_wgpu(info.backend)
            .map_or(info.backend.to_str(), |backend| backend.name());
        Self {
            adapter_name: info.name.clone(),
            backend_name,
            is_software: info.device_type == wgpu::DeviceType::Cpu,
        }
    }
}

impl fmt::Display for GpuInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.adapter_name, self.backend_name)
    }
}
