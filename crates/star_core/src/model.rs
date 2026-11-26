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

/// The joints a vertex follows at most.
pub const JOINTS_PER_VERTEX: usize = 4;

/// A 3D model: the meshes it is made of, each with its material and place
/// in the model, the images their materials show, and the skeleton that
/// bends its skinned meshes with the animations that move it.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    parts: Vec<ModelPart>,
    images: Vec<Image>,
    skeleton: Option<Skeleton>,
    animations: Vec<ModelAnimation>,
}

/// The joints of a model, each after its parent.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    /// The joints; a joint's parent comes before it.
    pub joints: Vec<Joint>,
}

/// A joint of a [`Skeleton`].
#[derive(Debug, Clone, PartialEq)]
pub struct Joint {
    /// Its name in the file (`"hand.L"`).
    pub name: String,
    /// The joint it hangs from, before it in the skeleton; `None` for a
    /// root.
    pub parent: Option<usize>,
    /// Where it is relative to its parent when no animation moves it.
    pub rest: JointPose,
    /// What takes a vertex from the model's space to the joint's, as it was
    /// when the mesh was bound to it: a matrix as 4 columns.
    pub inverse_bind: [[f32; 4]; 4],
    /// For a root, where the nodes above it in the file put it: a matrix
    /// as 4 columns (the identity for the others).
    pub base: [[f32; 4]; 4],
}

/// Where a joint is relative to its parent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointPose {
    /// Its offset.
    pub translation: [f32; 3],
    /// Its turn, as a quaternion: `x`, `y`, `z`, `w`.
    pub rotation: [f32; 4],
    /// Its stretch along each axis.
    pub scale: [f32; 3],
}

/// An animation of a model: channels that move its joints over time.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelAnimation {
    /// What it is played by (`"run"`).
    pub name: String,
    /// How long it lasts, in seconds: its last key's time.
    pub length: f32,
    /// What it moves.
    pub channels: Vec<JointChannel>,
}

/// One property of one joint, from key to key.
#[derive(Debug, Clone, PartialEq)]
pub struct JointChannel {
    /// The joint, as a position in [`Skeleton::joints`].
    pub joint: usize,
    /// What it changes.
    pub property: JointProperty,
    /// Whether it jumps from key to key instead of going between them.
    pub step: bool,
    /// The times of its keys, in seconds, in order.
    pub times: Vec<f32>,
    /// The value at each key: a translation or scale (`x`, `y`, `z`, and an
    /// unused 0) or a rotation (a quaternion `x`, `y`, `z`, `w`).
    pub values: Vec<[f32; 4]>,
}

/// What a [`JointChannel`] changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointProperty {
    /// [`JointPose::translation`]
    Translation,
    /// [`JointPose::rotation`]
    Rotation,
    /// [`JointPose::scale`]
    Scale,
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
    /// Where it is in the model: a matrix as 4 columns. A skinned part
    /// ignores it: its joints place it.
    pub transform: [[f32; 4]; 4],
    /// Whether the skeleton bends it, by its vertices' joints and weights.
    pub skinned: bool,
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
    /// The joints of the skeleton it follows, as positions in
    /// [`Skeleton::joints`].
    pub joints: [u16; JOINTS_PER_VERTEX],
    /// How much it follows each of `joints`; they add up to 1 in skinned
    /// parts.
    pub weights: [f32; JOINTS_PER_VERTEX],
}

impl ModelVertex {
    /// A corner that follows no joint.
    #[must_use]
    pub const fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            uv,
            joints: [0; JOINTS_PER_VERTEX],
            weights: [0.0; JOINTS_PER_VERTEX],
        }
    }
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
        Ok(Self {
            parts,
            images,
            skeleton: None,
            animations: Vec::new(),
        })
    }

    /// The model with `skeleton`, which bends its skinned parts, and the
    /// `animations` that move it.
    ///
    /// # Errors
    ///
    /// [`EngineError::Asset`] if a joint's parent does not come before it,
    /// a vertex follows a joint that is not there, or a channel moves a
    /// joint that is not there or has keys out of order or without values.
    pub fn with_skeleton(
        mut self,
        skeleton: Skeleton,
        animations: Vec<ModelAnimation>,
    ) -> EngineResult<Self> {
        let count = skeleton.joints.len();
        for (index, joint) in skeleton.joints.iter().enumerate() {
            if joint.parent.is_some_and(|parent| parent >= index) {
                return Err(invalid("a joint of a model comes before its parent"));
            }
        }
        let follows_missing = self.parts.iter().filter(|part| part.skinned).any(|part| {
            part.vertices
                .iter()
                .flat_map(|vertex| vertex.joints)
                .any(|joint| usize::from(joint) >= count)
        });
        if follows_missing {
            return Err(invalid(
                "a vertex of a model follows a joint it does not have",
            ));
        }
        for channel in animations.iter().flat_map(|animation| &animation.channels) {
            channel.validate(count)?;
        }
        self.skeleton = Some(skeleton);
        self.animations = animations;
        Ok(self)
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

    /// Its skeleton, if it has one.
    #[must_use]
    pub const fn skeleton(&self) -> Option<&Skeleton> {
        self.skeleton.as_ref()
    }

    /// The animations that move its skeleton.
    #[must_use]
    pub fn animations(&self) -> &[ModelAnimation] {
        &self.animations
    }
}

impl JointChannel {
    fn validate(&self, joints: usize) -> EngineResult<()> {
        if self.joint >= joints {
            return Err(invalid(
                "an animation of a model moves a joint it does not have",
            ));
        }
        if self.times.is_empty() || self.times.len() != self.values.len() {
            return Err(invalid(
                "a channel of an animation of a model needs a value for each key",
            ));
        }
        let ordered = self
            .times
            .windows(2)
            .all(|pair| pair[0].is_finite() && pair[0] <= pair[1]);
        if !ordered {
            return Err(invalid(
                "a channel of an animation of a model has keys out of order",
            ));
        }
        Ok(())
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
        let corner = |x: f32, y: f32| ModelVertex::new([x, y, 0.0], [0.0, 0.0, 1.0], [x, y]);
        ModelPart {
            vertices: vec![corner(0.0, 0.0), corner(1.0, 0.0), corner(0.0, 1.0)],
            indices,
            material,
            transform: IDENTITY_COLUMNS,
            skinned: false,
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

    fn joint(name: &str, parent: Option<usize>) -> Joint {
        Joint {
            name: name.to_owned(),
            parent,
            rest: JointPose {
                translation: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
            },
            inverse_bind: IDENTITY_COLUMNS,
            base: IDENTITY_COLUMNS,
        }
    }

    fn turning(joint: usize, times: Vec<f32>) -> ModelAnimation {
        let values = times.iter().map(|_| [0.0, 0.0, 0.0, 1.0]).collect();
        ModelAnimation {
            name: "turn".to_owned(),
            length: 1.0,
            channels: vec![JointChannel {
                joint,
                property: JointProperty::Rotation,
                step: false,
                times,
                values,
            }],
        }
    }

    #[test]
    fn skeletons_keep_parents_first_and_animations_their_joints() {
        let model = || {
            let mut skinned = triangle(vec![0, 1, 2], ModelMaterial::default());
            skinned.skinned = true;
            skinned.vertices[2].joints = [1, 0, 0, 0];
            Model::new(vec![skinned], Vec::new()).unwrap()
        };
        let arm = || Skeleton {
            joints: vec![joint("root", None), joint("arm", Some(0))],
        };

        let posed = model()
            .with_skeleton(arm(), vec![turning(1, vec![0.0, 1.0])])
            .unwrap();
        assert_eq!(posed.skeleton().map(|bones| bones.joints.len()), Some(2));
        assert_eq!(posed.animations()[0].name, "turn");

        let backwards = Skeleton {
            joints: vec![joint("arm", Some(1)), joint("root", None)],
        };
        assert!(model().with_skeleton(backwards, Vec::new()).is_err());
        let one = Skeleton {
            joints: vec![joint("root", None)],
        };
        assert!(
            model().with_skeleton(one, Vec::new()).is_err(),
            "a vertex follows joint 1"
        );
        assert!(
            model()
                .with_skeleton(arm(), vec![turning(5, vec![0.0])])
                .is_err()
        );
        assert!(
            model()
                .with_skeleton(arm(), vec![turning(1, vec![1.0, 0.0])])
                .is_err()
        );
    }
}
