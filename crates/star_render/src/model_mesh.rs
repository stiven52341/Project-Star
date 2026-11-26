//! 3D models on the GPU (decision 83): the meshes and textures of a
//! [`Model`], uploaded once, ready to be drawn at a node's place.

use star_core::{EngineResult, Image, Model};
use star_math::Mat4;

use crate::{Color, Material, Mesh3dId, MeshData, TextureId};

/// A mesh of a model on the GPU, with its material and place in the model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelMeshPart {
    /// The mesh, as uploaded.
    pub mesh: Mesh3dId,
    /// How its surface looks.
    pub material: Material,
    /// Where it is in the model: drawn at `node's matrix × transform`.
    pub transform: Mat4,
}

/// Uploads the images of `model` with `texture` and its meshes with
/// `mesh`, each image once, and gives its parts. A model's colors are in
/// linear space, as glTF gives them; materials take them in sRGB.
///
/// # Errors
///
/// Why a mesh or an image could not be uploaded.
pub fn upload_model(
    model: &Model,
    mut texture: impl FnMut(&Image) -> EngineResult<TextureId>,
    mut mesh: impl FnMut(&MeshData) -> EngineResult<Mesh3dId>,
) -> EngineResult<Vec<ModelMeshPart>> {
    let textures = model
        .images()
        .iter()
        .map(&mut texture)
        .collect::<EngineResult<Vec<TextureId>>>()?;
    model
        .parts()
        .iter()
        .map(|part| {
            let [r, g, b, a] = part.material.base_color;
            Ok(ModelMeshPart {
                mesh: mesh(&MeshData::from_model_part(part)?)?,
                material: Material {
                    base_color: Color::from_linear(Color::rgba(r, g, b, a)),
                    base_color_texture: part
                        .material
                        .texture
                        .and_then(|index| textures.get(index).copied()),
                    metallic: part.material.metallic,
                    roughness: part.material.roughness,
                },
                transform: Mat4::from_columns(part.transform),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use star_core::model::IDENTITY_COLUMNS;
    use star_core::{ModelMaterial, ModelPart, ModelVertex};

    use super::*;

    fn part(texture: Option<usize>) -> ModelPart {
        let corner = |x: f32| ModelVertex::new([x, 0.0, x], [0.0, 1.0, 0.0], [0.0, 0.0]);
        ModelPart {
            vertices: vec![corner(0.0), corner(1.0), corner(2.0)],
            indices: vec![0, 1, 2],
            material: ModelMaterial {
                base_color: [1.0, 0.0, 0.0, 1.0],
                texture,
                ..ModelMaterial::default()
            },
            transform: IDENTITY_COLUMNS,
            skinned: false,
        }
    }

    #[test]
    fn images_are_uploaded_once_and_parts_show_theirs() {
        let image = Image::from_rgba8(1, 1, vec![255; 4]).unwrap();
        let model =
            Model::new(vec![part(Some(0)), part(None), part(Some(0))], vec![image]).unwrap();
        let mut images = 0;
        let mut meshes = 0;

        let parts = upload_model(
            &model,
            |_| {
                images += 1;
                Ok(TextureId::from_index(7))
            },
            |_| {
                meshes += 1;
                Ok(Mesh3dId::from_index(meshes))
            },
        )
        .unwrap();

        assert_eq!((images, meshes), (1, 3));
        assert_eq!(
            parts[0].material.base_color_texture,
            Some(TextureId::from_index(7))
        );
        assert_eq!(parts[1].material.base_color_texture, None);
        let red = parts[0].material.base_color;
        assert!((red.r - 1.0).abs() < 1e-5 && red.g.abs() < 1e-5, "{red:?}");
    }
}
