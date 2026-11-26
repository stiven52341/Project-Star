//! The fields of the engine's node classes beyond `Node` and `Node3D`
//! (those are in [`crate::nodes`]), with their first values, shared by
//! every way of running StarScript.

use std::cell::{Cell, RefCell};
use std::f32::consts::{FRAC_PI_3, FRAC_PI_6};

use crate::Field;
use crate::machines::MachineRun;
use crate::nodes::Host;

/// The fields of a `Sprite`: the image it shows and how.
pub struct SpriteFields<H: Host> {
    /// `texture`: the asset path of the image.
    pub texture: Field<String>,
    /// `width`
    pub width: Field<f32>,
    /// `height`
    pub height: Field<f32>,
    /// `layer`: sprites of higher layers are drawn over the others.
    pub layer: Field<i32>,
    /// `pixelated`
    pub pixelated: Field<bool>,
    /// `color`: what the image is tinted with.
    pub color: Field<H::Color>,
    /// `columns`: how many cells wide the image is, as a sprite sheet.
    pub columns: Field<i32>,
    /// `rows`: how many cells tall the image is, as a sprite sheet.
    pub rows: Field<i32>,
    /// `frame`: the cell shown, counted from the top-left one, row by row.
    pub frame: Field<i32>,
    /// `frames`: the asset path of the `.frames` file of its animations;
    /// empty for none.
    pub frames: Field<String>,
    /// The clip playing, or the last one played; empty before any.
    pub(crate) animation: Field<String>,
    /// Whether `animation` is playing.
    pub(crate) playing: Cell<bool>,
    /// Seconds `animation` has played.
    pub(crate) clip_time: Cell<f32>,
}

impl<H: Host> SpriteFields<H> {
    /// A sprite of 1 by 1 showing the whole `texture` as it is, in layer 0.
    #[must_use]
    pub fn new(texture: String) -> Self {
        Self {
            texture: Field::new(texture),
            width: Field::new(1.0),
            height: Field::new(1.0),
            layer: Field::new(0),
            pixelated: Field::new(false),
            color: Field::new(white::<H>()),
            columns: Field::new(1),
            rows: Field::new(1),
            frame: Field::new(0),
            frames: Field::new(String::new()),
            animation: Field::new(String::new()),
            playing: Cell::new(false),
            clip_time: Cell::new(0.0),
        }
    }
}

/// The fields of a `Camera2D`: how much of the world it shows, and whether
/// it becomes the game's camera when it enters the game's tree.
pub struct Camera2dFields {
    /// `view_height`, in world units.
    pub view_height: Field<f32>,
    /// `main`
    pub main: Field<bool>,
}

impl Camera2dFields {
    /// Showing 2 units of height, not the main camera.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            view_height: Field::new(2.0),
            main: Field::new(false),
        }
    }
}

impl Default for Camera2dFields {
    fn default() -> Self {
        Self::new()
    }
}

/// The fields of a `Light2D` or a `Light`: the kind of light it shines,
/// its color, how bright it is, how far it reaches and the cone of spots.
pub struct LightFields<H: Host> {
    /// `kind`
    pub kind: Field<String>,
    /// `color`
    pub color: Field<H::Color>,
    /// `intensity`
    pub intensity: Field<f32>,
    /// `range`
    pub range: Field<f32>,
    /// `angle`: half the cone of a spot, in radians.
    pub angle: Field<f32>,
    /// `blend`: how soft the cone's edge is.
    pub blend: Field<f32>,
}

/// How far a new `Light2D` reaches, in world units.
const LIGHT_2D_RANGE: f32 = 3.0;
/// How far a new `Light` reaches, in world units.
const LIGHT_3D_RANGE: f32 = 10.0;
/// How soft the edge of a new spot's cone is.
const SPOT_BLEND: f32 = 0.2;

impl<H: Host> LightFields<H> {
    /// A white `Light2D` of the kind `kind`.
    #[must_use]
    pub fn new_2d(kind: String) -> Self {
        Self::new(kind, LIGHT_2D_RANGE)
    }

    /// A white `Light` of the kind `kind`.
    #[must_use]
    pub fn new_3d(kind: String) -> Self {
        Self::new(kind, LIGHT_3D_RANGE)
    }

    fn new(kind: String, range: f32) -> Self {
        Self {
            kind: Field::new(kind),
            color: Field::new(white::<H>()),
            intensity: Field::new(1.0),
            range: Field::new(range),
            angle: Field::new(FRAC_PI_6),
            blend: Field::new(SPOT_BLEND),
        }
    }
}

/// The fields of a `Mesh`: the shape it shows, its sizes and its surface.
pub struct MeshFields<H: Host> {
    /// `shape`
    pub shape: Field<String>,
    /// `width`
    pub width: Field<f32>,
    /// `height`
    pub height: Field<f32>,
    /// `depth`
    pub depth: Field<f32>,
    /// `radius`
    pub radius: Field<f32>,
    /// `color`
    pub color: Field<H::Color>,
    /// `metallic`
    pub metallic: Field<f32>,
    /// `roughness`
    pub roughness: Field<f32>,
    /// `texture`: the asset path of its image; empty for none.
    pub texture: Field<String>,
    /// `pixelated`
    pub pixelated: Field<bool>,
}

/// How rough the surface of a new mesh is.
const MESH_ROUGHNESS: f32 = 0.5;
/// The radius of a new round mesh, so it is 1 across.
const MESH_RADIUS: f32 = 0.5;

impl<H: Host> MeshFields<H> {
    /// A white `shape` of 1 by 1 by 1, without an image.
    #[must_use]
    pub fn new(shape: String) -> Self {
        Self {
            shape: Field::new(shape),
            width: Field::new(1.0),
            height: Field::new(1.0),
            depth: Field::new(1.0),
            radius: Field::new(MESH_RADIUS),
            color: Field::new(white::<H>()),
            metallic: Field::new(0.0),
            roughness: Field::new(MESH_ROUGHNESS),
            texture: Field::new(String::new()),
            pixelated: Field::new(false),
        }
    }
}

/// The fields of a `Camera3D`: how wide it sees, and whether it becomes
/// the game's 3D camera when it enters the game's tree.
pub struct Camera3dFields {
    /// `fov`: the field of view, in radians.
    pub fov: Field<f32>,
    /// `main`
    pub main: Field<bool>,
}

impl Camera3dFields {
    /// Seeing 60 degrees, not the main camera.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fov: Field::new(FRAC_PI_3),
            main: Field::new(false),
        }
    }
}

impl Default for Camera3dFields {
    fn default() -> Self {
        Self::new()
    }
}

/// The fields of a `Model`: the glTF file it shows (decision 83).
pub struct ModelFields {
    /// `file`: the asset path of its `.glb` or `.gltf`.
    pub file: Field<String>,
}

impl ModelFields {
    /// A model of the file at `file`.
    #[must_use]
    pub const fn new(file: String) -> Self {
        Self {
            file: Field::new(file),
        }
    }
}

/// How long a new `Bone2D` is, in world units.
const BONE_LENGTH: f32 = 0.5;

/// The fields of a `Bone2D`: a joint of a cut-out character (decision 82).
pub struct BoneFields {
    /// `length`: how far it reaches along its rotation, in world units.
    pub length: Field<f32>,
}

impl BoneFields {
    /// A bone of the length bones start with.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            length: Field::new(BONE_LENGTH),
        }
    }
}

impl Default for BoneFields {
    fn default() -> Self {
        Self::new()
    }
}

/// The most animations an animator shows at once: two a state blends,
/// and two of the state it fades from.
pub const MAX_SHOWN_CLIPS: usize = 4;

/// An animation an animator showed in its last frame, at a time and
/// weighing in a mix: what poses the skeleton of a `Model` it animates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShownClip {
    /// The animation, as a position in its file.
    pub index: usize,
    /// Seconds into it.
    pub time: f32,
    /// How much it weighs against the others.
    pub weight: f32,
}

/// The time an animator has shown before showing any: below every call's.
pub(crate) const BEFORE_START: f32 = -1.0;

/// The fields of an `Animator`: the `.anim` file of its animations, and
/// the one it plays.
pub struct AnimatorFields {
    /// `animations`: the asset path of the `.anim` file; empty for none.
    pub animations: Field<String>,
    /// `states`: the asset path of the `.states` file of the machine that
    /// chooses its animations; empty for none.
    pub states: Field<String>,
    /// What its state machine is doing.
    pub(crate) machine: RefCell<MachineRun>,
    /// The animations it showed in its last frame, for the skeleton of a
    /// `Model` it animates.
    pub(crate) shown_clips: Cell<[Option<ShownClip>; MAX_SHOWN_CLIPS]>,
    /// The animation playing, or the last one played; empty before any.
    pub(crate) animation: Field<String>,
    /// Whether `animation` is playing.
    pub(crate) playing: Cell<bool>,
    /// Seconds `animation` has played.
    pub(crate) time: Cell<f32>,
    /// The time of `animation` shown in the last frame, so the calls
    /// between it and the next are made; below zero before the first.
    pub(crate) shown: Cell<f32>,
}

impl AnimatorFields {
    /// An animator without animations, playing nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            animations: Field::new(String::new()),
            states: Field::new(String::new()),
            machine: RefCell::new(MachineRun::new()),
            shown_clips: Cell::new([None; MAX_SHOWN_CLIPS]),
            animation: Field::new(String::new()),
            playing: Cell::new(false),
            time: Cell::new(0.0),
            shown: Cell::new(BEFORE_START),
        }
    }
}

impl Default for AnimatorFields {
    fn default() -> Self {
        Self::new()
    }
}

fn white<H: Host>() -> H::Color {
    H::color(1.0, 1.0, 1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_host::TestHost;

    #[test]
    fn new_nodes_start_white_and_unscaled() {
        let sprite = SpriteFields::<TestHost>::new("ship.png".to_owned());
        let mesh = MeshFields::<TestHost>::new("sphere".to_owned());

        assert_eq!(sprite.texture.get(), "ship.png");
        assert_eq!(sprite.color.get(), [1.0; 4]);
        assert_eq!(mesh.color.get(), [1.0; 4]);
        assert!((mesh.radius.get() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn lights_of_2d_reach_less_than_lights_of_3d() {
        let flat = LightFields::<TestHost>::new_2d("point".to_owned());
        let deep = LightFields::<TestHost>::new_3d("point".to_owned());

        assert!((flat.range.get() - 3.0).abs() < f32::EPSILON);
        assert!((deep.range.get() - 10.0).abs() < f32::EPSILON);
        assert_eq!(flat.kind.get(), "point");
    }

    #[test]
    fn new_cameras_are_not_main() {
        assert!(!Camera2dFields::new().main.get());
        assert!(!Camera3dFields::new().main.get());
        assert!((Camera2dFields::new().view_height.get() - 2.0).abs() < f32::EPSILON);
    }
}
