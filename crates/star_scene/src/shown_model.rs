//! A 3D model ready to be drawn (decision 83): its meshes on the GPU, and
//! the skeleton and animations that pose its skinned meshes.

use star_core::{Model, ModelAnimation, Skeleton};
use star_math::Mat4;
use star_render::{Frame, ModelMeshPart};

use crate::{ClipAt, Poser};

/// A model's meshes as uploaded, with what poses them.
#[derive(Debug, Clone, PartialEq)]
pub struct ShownModel {
    /// Its meshes on the GPU.
    pub parts: Vec<ModelMeshPart>,
    /// Its skeleton, if a skeleton bends some of its meshes.
    pub skeleton: Option<Skeleton>,
    /// The animations that move its skeleton.
    pub animations: Vec<ModelAnimation>,
}

/// The working space of drawing posed models, kept between frames so it
/// does not allocate once it has room.
#[derive(Debug, Default)]
pub struct ModelDrawing {
    poser: Poser,
    joints: Vec<Mat4>,
}

impl ShownModel {
    /// `model`'s skeleton and animations with its uploaded `parts`.
    #[must_use]
    pub fn new(model: &Model, parts: Vec<ModelMeshPart>) -> Self {
        Self {
            parts,
            skeleton: model.skeleton().cloned(),
            animations: model.animations().to_vec(),
        }
    }

    /// The position among its animations of the one called `name`.
    #[must_use]
    pub fn animation(&self, name: &str) -> Option<usize> {
        self.animations
            .iter()
            .position(|animation| animation.name == name)
    }

    /// Adds to `frame` every part of it placed at `global`: its skinned
    /// parts bent by its skeleton posed by `clips` (at rest without them),
    /// and the others where the model puts them.
    ///
    /// # Errors
    ///
    /// Why a skinned part could not be drawn: a skeleton of too many
    /// joints. The other parts are drawn.
    pub fn draw(
        &self,
        global: Mat4,
        clips: &[ClipAt],
        drawing: &mut ModelDrawing,
        frame: &mut Frame,
    ) -> star_core::EngineResult<()> {
        if let Some(skeleton) = &self.skeleton {
            drawing
                .poser
                .pose(skeleton, &self.animations, clips, &mut drawing.joints);
        }
        let mut result = Ok(());
        for part in &self.parts {
            if part.skinned && self.skeleton.is_some() {
                let drawn =
                    frame.draw_skinned_mesh_3d(part.mesh, part.material, global, &drawing.joints);
                result = result.and(drawn);
            } else {
                frame.draw_mesh_3d(part.mesh, part.material, global * part.transform);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use star_core::model::IDENTITY_COLUMNS;
    use star_core::{Joint, JointPose};
    use star_render::{MAX_JOINTS, Material, Mesh3dId};

    use super::*;

    fn part(skinned: bool) -> ModelMeshPart {
        ModelMeshPart {
            mesh: Mesh3dId::from_index(0),
            material: Material::default(),
            transform: Mat4::IDENTITY,
            skinned,
        }
    }

    fn skeleton(joints: usize) -> Skeleton {
        let joint = Joint {
            name: String::new(),
            parent: None,
            rest: JointPose {
                translation: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
            },
            inverse_bind: IDENTITY_COLUMNS,
            base: IDENTITY_COLUMNS,
        };
        Skeleton {
            joints: vec![joint; joints],
        }
    }

    fn drawn(joints: usize) -> (star_core::EngineResult<()>, usize) {
        let model = ShownModel {
            parts: vec![part(true), part(false)],
            skeleton: Some(skeleton(joints)),
            animations: Vec::new(),
        };
        let mut frame = Frame::new();
        let result = model.draw(
            Mat4::IDENTITY,
            &[],
            &mut ModelDrawing::default(),
            &mut frame,
        );
        (result, frame.mesh_3d_count())
    }

    #[test]
    fn skinned_and_plain_parts_are_all_drawn() {
        let (result, count) = drawn(2);

        assert!(result.is_ok());
        assert_eq!(count, 2);
    }

    #[test]
    fn skeletons_of_too_many_joints_leave_out_only_their_skinned_parts() {
        let (result, count) = drawn(MAX_JOINTS + 1);

        assert!(result.is_err());
        assert_eq!(count, 1, "the plain part is drawn");
    }
}
