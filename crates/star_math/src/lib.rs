//! Math types for 2D and 3D games.
//!
//! Engine code depends on these types, never on an external math crate directly,
//! so the implementation can evolve (e.g. SIMD) without breaking users.

mod affine2;
mod mat4;
mod quat;
mod vec2;
mod vec3;

pub use affine2::Affine2;
pub use mat4::Mat4;
pub use quat::Quat;
pub use vec2::Vec2;
pub use vec3::Vec3;
