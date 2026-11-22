//! RGBA colors.

/// Upper bound of the linear segment of the sRGB transfer curve.
const SRGB_LINEAR_THRESHOLD: f32 = 0.040_45;
/// Slope of the linear segment of the sRGB transfer curve.
const SRGB_LINEAR_SLOPE: f32 = 12.92;
/// Offset of the exponential segment of the sRGB transfer curve.
const SRGB_OFFSET: f32 = 0.055;
/// Exponent of the exponential segment of the sRGB transfer curve.
const SRGB_GAMMA: f32 = 2.4;

/// A color in sRGB space (the space color pickers use), with alpha.
///
/// Components range from `0.0` to `1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red component.
    pub r: f32,
    /// Green component.
    pub g: f32,
    /// Blue component.
    pub b: f32,
    /// Opacity: `0.0` is transparent, `1.0` is opaque.
    pub a: f32,
}

impl Color {
    /// Opaque black.
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    /// Opaque white.
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    /// Creates an opaque color.
    #[must_use]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// Creates a color with explicit opacity.
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Converts to linear space, which the GPU expects for sRGB render targets.
    /// Alpha is already linear and is kept unchanged.
    #[must_use]
    pub fn to_linear(self) -> Self {
        Self::rgba(
            srgb_to_linear(self.r),
            srgb_to_linear(self.g),
            srgb_to_linear(self.b),
            self.a,
        )
    }

    /// The color as a render target of `format` expects it: linear for sRGB
    /// targets (the GPU encodes it again), unchanged for the others.
    pub(crate) fn in_target_space(self, format: wgpu::TextureFormat) -> Self {
        if format.is_srgb() {
            self.to_linear()
        } else {
            self
        }
    }

    pub(crate) fn to_wgpu(self) -> wgpu::Color {
        wgpu::Color {
            r: f64::from(self.r),
            g: f64::from(self.g),
            b: f64::from(self.b),
            a: f64::from(self.a),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

fn srgb_to_linear(component: f32) -> f32 {
    if component <= SRGB_LINEAR_THRESHOLD {
        component / SRGB_LINEAR_SLOPE
    } else {
        ((component + SRGB_OFFSET) / (1.0 + SRGB_OFFSET)).powf(SRGB_GAMMA)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;

    #[test]
    fn black_and_white_are_unchanged_in_linear_space() {
        assert_eq!(Color::BLACK.to_linear(), Color::BLACK);
        assert_eq!(Color::WHITE.to_linear(), Color::WHITE);
    }

    #[test]
    fn mid_gray_becomes_darker_in_linear_space() {
        let linear = Color::rgba(0.5, 0.5, 0.5, 0.5).to_linear();

        assert!((linear.r - 0.214_04).abs() < TOLERANCE);
        assert!((linear.a - 0.5).abs() < TOLERANCE);
    }
}
