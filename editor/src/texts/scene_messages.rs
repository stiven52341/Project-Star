//! Texts about scenes and the values of their nodes, in each language.

use star_engine::prelude::{BodyKind, LightNode, MeshShape, ShapeKind, ShapeKind3d, TextureFilter};

use super::{Language, Texts};

impl Texts {
    pub(crate) fn scene_unreadable(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not open the scene: {reason}"),
            Language::Spanish => format!("No se pudo abrir la escena: {reason}"),
        }
    }

    pub(crate) const fn body_kind_name(&self, kind: BodyKind) -> &'static str {
        match kind {
            BodyKind::Dynamic => self.pick("Dynamic", "Dinámico"),
            BodyKind::Static => self.pick("Static", "Estático"),
            BodyKind::Kinematic => self.pick("Kinematic", "Cinemático"),
        }
    }

    pub(crate) const fn filter_name(&self, filter: TextureFilter) -> &'static str {
        match filter {
            TextureFilter::Smooth => self.pick("Smooth", "Suave"),
            TextureFilter::Pixelated => self.pick("Pixelated", "Pixelado"),
        }
    }

    /// A 2D shape with its sizes: `Circle, radius 0.5`.
    pub(crate) fn shape_description(&self, shape: ShapeKind) -> String {
        match shape {
            ShapeKind::Circle { radius } => {
                self.round_shape(self.pick("Circle", "Círculo"), radius)
            }
            ShapeKind::Rectangle { size } => format!(
                "{} {}",
                self.pick("Rectangle", "Rectángulo"),
                size_text(&[size.x, size.y])
            ),
            ShapeKind::Capsule { height, radius } => self.capsule(height, radius),
        }
    }

    /// A 3D shape with its sizes: `Box 1 × 2 × 1`.
    pub(crate) fn shape_3d_description(&self, shape: ShapeKind3d) -> String {
        match shape {
            ShapeKind3d::Sphere { radius } => {
                self.round_shape(self.pick("Sphere", "Esfera"), radius)
            }
            ShapeKind3d::Box { size } => format!(
                "{} {}",
                self.pick("Box", "Caja"),
                size_text(&[size.x, size.y, size.z])
            ),
            ShapeKind3d::Capsule { height, radius } => self.capsule(height, radius),
        }
    }

    /// A mesh shape with its sizes.
    pub(crate) fn mesh_description(&self, shape: MeshShape) -> String {
        match shape {
            MeshShape::Cube { size } => format!(
                "{} {}",
                self.pick("Cube", "Cubo"),
                size_text(&[size.x, size.y, size.z])
            ),
            MeshShape::Plane { size } => format!(
                "{} {}",
                self.pick("Plane", "Plano"),
                size_text(&[size.x, size.y])
            ),
            MeshShape::Sphere { radius, .. } => {
                self.round_shape(self.pick("Sphere", "Esfera"), radius)
            }
        }
    }

    pub(crate) const fn light_kind_name(&self, light: &LightNode) -> &'static str {
        match light {
            LightNode::Directional { .. } => self.pick("Directional", "Direccional"),
            LightNode::Point { .. } => self.pick("Point", "Puntual"),
        }
    }

    fn round_shape(&self, name: &str, radius: f32) -> String {
        format!(
            "{name}, {} {}",
            self.pick("radius", "radio"),
            number_text(radius)
        )
    }

    fn capsule(&self, height: f32, radius: f32) -> String {
        match self.language {
            Language::English => format!(
                "Capsule, height {}, radius {}",
                number_text(height),
                number_text(radius)
            ),
            Language::Spanish => format!(
                "Cápsula, altura {}, radio {}",
                number_text(height),
                number_text(radius)
            ),
        }
    }
}

/// Decimals shown of a number: enough for the values written in scenes.
const SHOWN_DECIMALS: usize = 3;

/// A number with no trailing zeros: `0.5`, `2`, `-1.25`.
pub(crate) fn number_text(value: f32) -> String {
    let text = format!("{value:.SHOWN_DECIMALS$}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    // `-0.0001` rounds to `-0`, which reads as a different number.
    if text == "-0" { "0" } else { text }.to_owned()
}

/// Sizes along each axis: `1 × 2`.
pub(crate) fn size_text(sizes: &[f32]) -> String {
    sizes
        .iter()
        .map(|&size| number_text(size))
        .collect::<Vec<_>>()
        .join(" × ")
}

#[cfg(test)]
mod tests {
    use star_engine::prelude::{Vec2, Vec3};

    use super::*;

    const ENGLISH: &Texts = Language::English.texts();
    const SPANISH: &Texts = Language::Spanish.texts();

    #[test]
    fn numbers_lose_their_trailing_zeros() {
        assert_eq!(number_text(0.5), "0.5");
        assert_eq!(number_text(2.0), "2");
        assert_eq!(number_text(-1.25), "-1.25");
        assert_eq!(number_text(-0.0001), "0");
    }

    #[test]
    fn shapes_are_described_with_their_sizes() {
        assert_eq!(
            ENGLISH.shape_description(ShapeKind::Rectangle {
                size: Vec2::new(1.0, 0.5)
            }),
            "Rectangle 1 × 0.5"
        );
        assert_eq!(
            SPANISH.shape_3d_description(ShapeKind3d::Capsule {
                height: 2.0,
                radius: 0.5
            }),
            "Cápsula, altura 2, radio 0.5"
        );
        assert_eq!(
            ENGLISH.mesh_description(MeshShape::Cube {
                size: Vec3::new(1.0, 2.0, 3.0)
            }),
            "Cube 1 × 2 × 3"
        );
    }
}
