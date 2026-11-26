//! 3D models in memory: what model files (glTF) decode to, and the meshes
//! and textures the renderer makes of them.

use crate::{EngineError, EngineResult, Image};

/// Corners per triangle: indices come in threes.
const VERTICES_PER_TRIANGLE: usize = 3;

/// The matrix that leaves points where they are, as columns.
pub const IDENTITY_COLUMNS: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// A 3D model: the meshes it is made of, each with its material and place
/// in the model, and the images their materials show.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    parts: Vec<ModelPart>,
    images: Vec<Image>,
}

/// One mesh of a [`Model`], with its material and where it is in the
/// model.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPart {
    /// The corners of its triangles, in its own space.
    pub vertices: Vec<ModelVertex>,
    /// Every 3 indices of `vertices` one triangle, counterclockwise seen
    /// from the side it shows.
    pub indices: Vec<u32>,
    /// How its surface looks.
    pub material: ModelMaterial,
    /// Where it is in the model: a matrix as 4 columns.
    pub transform: [[f32; 4]; 4],
}

/// A corner of a [`ModelPart`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelVertex {
    /// Where it is.
    pub position: [f32; 3],
    /// The direction the surface faces there, of length 1.
    pub normal: [f32; 3],
    /// Where it is on its texture: `(0, 0)` is the image's top-left corner.
    pub uv: [f32; 2],
}

/// How the surface of a [`ModelPart`] looks: metallic-roughness, as glTF
/// and the engine's materials have it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelMaterial {
    /// Its color, red, green, blue and alpha from 0 to 1, multiplied by its
    /// texture's.
    pub base_color: [f32; 4],
    /// How metallic it is, from 0 to 1.
    pub metallic: f32,
    /// How rough it is, from 0 to 1.
    pub roughness: f32,
    /// The image it shows, as a position in [`Model::images`].
    pub texture: Option<usize>,
}

impl Default for ModelMaterial {
    /// White, not metallic and fully rough: glTF's material when a mesh
    /// names none.
    fn default() -> Self {
        Self {
            base_color: [1.0; 4],
            metallic: 0.0,
            roughness: 1.0,
            texture: None,
        }
    }
}

impl Model {
    /// A model of `parts`, whose materials show `images`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Asset`] if there are no parts, a part has no whole
    /// triangles or an index without a vertex, or a material names an image
    /// that is not there.
    pub fn new(parts: Vec<ModelPart>, images: Vec<Image>) -> EngineResult<Self> {
        if parts.is_empty() {
            return Err(invalid("a model needs at least one mesh"));
        }
        for part in &parts {
            part.validate(images.len())?;
        }
        Ok(Self { parts, images })
    }

    /// Its meshes.
    #[must_use]
    pub fn parts(&self) -> &[ModelPart] {
        &self.parts
    }

    /// The images its materials show.
    #[must_use]
    pub fn images(&self) -> &[Image] {
        &self.images
    }
}

impl ModelPart {
    fn validate(&self, images: usize) -> EngineResult<()> {
        if self.indices.is_empty() || self.indices.len() % VERTICES_PER_TRIANGLE != 0 {
            return Err(invalid(&format!(
                "a mesh of a model needs a non-zero multiple of {VERTICES_PER_TRIANGLE} indices, got {}",
                self.indices.len()
            )));
        }
        let count = self.vertices.len();
        if let Some(index) = self
            .indices
            .iter()
            .find(|&&index| usize::try_from(index).map_or(true, |index| index >= count))
        {
            return Err(invalid(&format!(
                "the index {index} of a mesh of a model has no vertex: there are {count}"
            )));
        }
        if self
            .material
            .texture
            .is_some_and(|texture| texture >= images)
        {
            return Err(invalid(
                "a material of a model shows an image it does not have",
            ));
        }
        Ok(())
    }
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Asset(reason.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle(indices: Vec<u32>, material: ModelMaterial) -> ModelPart {
        let corner = |x: f32, y: f32| ModelVertex {
            position: [x, y, 0.0],
            normal: [0.0, 0.0, 1.0],
            uv: [x, y],
        };
        ModelPart {
            vertices: vec![corner(0.0, 0.0), corner(1.0, 0.0), corner(0.0, 1.0)],
            indices,
            material,
            transform: IDENTITY_COLUMNS,
        }
    }

    #[test]
    fn models_keep_their_parts_and_images() {
        let image = Image::from_rgba8(1, 1, vec![255; 4]).unwrap();
        let material = ModelMaterial {
            texture: Some(0),
            ..ModelMaterial::default()
        };

        let model = Model::new(vec![triangle(vec![0, 1, 2], material)], vec![image]).unwrap();

        assert_eq!(model.parts().len(), 1);
        assert_eq!(model.images().len(), 1);
    }

    #[test]
    fn models_need_whole_triangles_of_their_vertices_and_images() {
        let plain = ModelMaterial::default();
        assert!(Model::new(Vec::new(), Vec::new()).is_err());
        assert!(Model::new(vec![triangle(vec![0, 1], plain)], Vec::new()).is_err());
        assert!(Model::new(vec![triangle(vec![0, 1, 3], plain)], Vec::new()).is_err());
        let textured = ModelMaterial {
            texture: Some(0),
            ..plain
        };
        assert!(Model::new(vec![triangle(vec![0, 1, 2], textured)], Vec::new()).is_err());
    }
}
