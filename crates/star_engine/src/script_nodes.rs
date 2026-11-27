//! The nodes of a StarScript game, mirrored as nodes of the engine's world.
//!
//! StarScript objects are reference-counted Rust values, not entities: the
//! script's tree of `Node` objects is the source of truth while the game
//! runs. After the script's code ran each frame, the generated game describes
//! every node of that tree here, parents first, and [`ScriptNodes`] keeps a
//! node of the world in step with each one: same name, parent, transform and
//! sprite. The engine then draws them like any other node. Rust games use the
//! scene API directly and do not need this.
//!
//! Script nodes with a `Body` get a [`Body2D`], or a [`Body3D`] if they are
//! `Node3D`s. The simulation moves them after the script's frame, so before
//! the next one the generated game reads back where they are, how fast they
//! move and what they touch ([`ScriptNodes::read_bodies`] and
//! [`ScriptNodes::body_state`]).
//!
//! A `Node3D` gets a [`Transform3D`] instead of a [`Transform2D`]; a `Mesh`
//! a [`MeshFile`], a `Model` a [`ModelNode`] and a `Light` a [`LightNode`].

use std::collections::HashMap;
use std::ops::Add;

use star_assets::{AssetPath, ModelFormat};
use star_core::EngineResult;
use star_ecs::{Entity, World};
use star_math::{Mat4, Vec2, Vec3};
use star_physics::{BodySettings, ContactPhase};
use star_render::{Camera, Color, Sprite, SpriteSheet, TextureFilter};
use star_scene::{
    Body2D, Body3D, ClipAt, GlobalTransform2D, GlobalTransform3D, Hidden, Hierarchy, Light2dNode,
    LightNode, ModelNode, ModelPose, Name, NodeBody, NodeContact, PhysicsNodes, PhysicsNodes3d,
    SpriteLook, Transform2D, Transform3D, shaped_by_sprite,
};

pub use crate::script_3d::{
    CUBE, DEFAULT_RANGE, DIRECTIONAL, PLANE, POINT, SPOT, ScriptCamera3D, ScriptLight, ScriptMesh,
};
pub use crate::script_body::{
    BOX, CAPSULE, CIRCLE, DEFAULT_RADIUS, DEFAULT_SIZE, DYNAMIC, KINEMATIC, RECTANGLE, SPHERE,
    STATIC, ScriptBody, ScriptBodyState,
};
use crate::{MeshFile, SpriteFile};

/// One node of the script's tree, as the generated game describes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptNode<'a> {
    /// Identifies the script object for as long as it lives.
    pub id: u64,
    /// The script object of the parent; `None` for the root.
    pub parent: Option<u64>,
    /// Its name.
    pub name: &'a str,
    /// Whether it is drawn; a hidden node hides its children too.
    pub visible: bool,
    /// Where it is, relative to its parent.
    pub transform: Transform2D,
    /// For `Node3D` objects, where it is in 3D, relative to its parent:
    /// then `transform` is not used, and its body is a 3D one.
    pub transform_3d: Option<Transform3D>,
    /// The image it shows, for `Sprite` objects.
    pub sprite: Option<ScriptSprite<'a>>,
    /// Its physics body, if it has one.
    pub body: Option<ScriptBody<'a>>,
    /// The mesh it shows, for `Mesh` objects.
    pub mesh: Option<ScriptMesh<'a>>,
    /// The model it shows, for `Model` objects.
    pub model: Option<ScriptModel<'a>>,
    /// The light it shines, for `Light` objects.
    pub light: Option<ScriptLight<'a>>,
    /// The light it shines on the sprites, for `Light2D` objects.
    pub light_2d: Option<ScriptLight<'a>>,
}

/// What a StarScript `Model` shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptModel<'a> {
    /// Asset path of the model file, as the script wrote it.
    pub file: &'a str,
    /// The animations of the file its animator shows, at their times and
    /// weights; none stands the model at rest.
    pub clips: [Option<ClipAt>; MAX_POSED_CLIPS],
}

/// The most animations a script's model shows at once.
pub const MAX_POSED_CLIPS: usize = 4;

/// What a StarScript `Sprite` shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptSprite<'a> {
    /// Asset path of the image, as the script wrote it.
    pub texture: &'a str,
    /// Width and height in world units, before the node's scale.
    pub size: Vec2,
    /// Sprites in higher layers are drawn on top of lower ones.
    pub layer: i32,
    /// Keeps the pixels sharp, for pixel art.
    pub pixelated: bool,
    /// Multiplied by the image's colors.
    pub color: Color,
    /// How the image is cut into cells, and the one shown.
    pub sheet: SpriteSheet,
}

/// A StarScript `Camera2D`, as the generated game describes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptCamera {
    /// The script object, as in [`ScriptNode::id`].
    pub id: u64,
    /// Where it is, relative to its parent.
    pub position: Vec2,
    /// World units shown from the bottom to the top of the window.
    pub view_height: f32,
}

/// The smallest view a script camera shows, in world units: a zero,
/// negative or not-a-number height would turn the whole frame invisible or
/// upside down.
const MIN_VIEW_HEIGHT: f32 = 0.001;

/// The world node that mirrors each script node.
#[derive(Debug, Default)]
pub struct ScriptNodes {
    mirrored: HashMap<u64, Mirrored>,
    /// The script node of each world node.
    ids: HashMap<Entity, u64>,
    /// The script nodes destroyed since the bodies were last read: the
    /// contacts that ended with them still name their world nodes.
    departed: HashMap<Entity, u64>,
    /// Counts the frames, to find the nodes the script no longer has.
    frame: u64,
}

#[derive(Debug)]
struct Mirrored {
    entity: Entity,
    seen_in_frame: u64,
    /// The last texture path that was not a valid asset path, so the
    /// warning is logged once.
    invalid_texture: Option<String>,
    /// Why the script's body cannot be simulated, logged once.
    invalid_body: Option<String>,
    /// Why the script's mesh or light cannot be shown, logged once.
    invalid_look: Option<String>,
    /// The velocity the script was last told, to notice when it sets one.
    told_velocity: Vec3,
    touching: Vec<u64>,
    just_touched: Vec<u64>,
    just_separated: Vec<u64>,
}

impl Mirrored {
    fn new(entity: Entity, frame: u64) -> Self {
        Self {
            entity,
            seen_in_frame: frame,
            invalid_texture: None,
            invalid_body: None,
            invalid_look: None,
            told_velocity: Vec3::ZERO,
            touching: Vec::new(),
            just_touched: Vec::new(),
            just_separated: Vec::new(),
        }
    }

    fn note_contact(&mut self, other: u64, phase: ContactPhase) {
        match phase {
            ContactPhase::Started => {
                if !self.touching.contains(&other) {
                    self.touching.push(other);
                }
                self.just_touched.push(other);
            }
            ContactPhase::Ended => {
                self.touching.retain(|touching| *touching != other);
                self.just_separated.push(other);
            }
        }
    }
}

impl ScriptNodes {
    /// A mirror without nodes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts describing the script's tree for a new frame.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Makes the world node of `node` match it, creating it the first time.
    /// Parents must be described before their children. A problem (an
    /// invalid texture path, a full world) is logged, not returned: the
    /// script's frame already happened, and the other nodes still need to be
    /// mirrored.
    pub fn mirror(&mut self, world: &mut World, node: &ScriptNode<'_>) {
        if let Err(error) = self.try_mirror(world, node) {
            log::error!("cannot mirror the node \"{}\": {error}", node.name);
        }
    }

    /// Destroys the world nodes of the script nodes that were not described
    /// since [`ScriptNodes::begin_frame`]: they left the script's tree.
    pub fn finish_frame(&mut self, world: &mut World) {
        let frame = self.frame;
        let Self {
            mirrored,
            ids,
            departed,
            ..
        } = self;
        mirrored.retain(|&id, mirrored| {
            let still_there = mirrored.seen_in_frame == frame;
            if !still_there {
                world.despawn_node(mirrored.entity);
                ids.remove(&mirrored.entity);
                departed.insert(mirrored.entity, id);
            }
            still_there
        });
    }

    /// Learns what the simulation did to the bodies of the script nodes
    /// during the last frame: their velocities and the contacts that
    /// started and ended. Called before the script's frame, after which
    /// [`ScriptNodes::body_state`] tells it to each node.
    pub fn read_bodies(&mut self, world: &World) {
        for mirrored in self.mirrored.values_mut() {
            mirrored.just_touched.clear();
            mirrored.just_separated.clear();
            if let Some(body) = world.get::<Body2D>(mirrored.entity) {
                let velocity = body.velocity();
                mirrored.told_velocity = Vec3::new(velocity.x, velocity.y, 0.0);
            } else if let Some(body) = world.get::<Body3D>(mirrored.entity) {
                mirrored.told_velocity = body.velocity();
            }
        }
        if let Some(nodes) = world.resource::<PhysicsNodes>() {
            self.note_contacts(nodes.contacts());
        }
        if let Some(nodes) = world.resource::<PhysicsNodes3d>() {
            self.note_contacts(nodes.contacts());
        }
        self.departed.clear();
    }

    fn note_contacts(&mut self, contacts: &[NodeContact]) {
        for contact in contacts {
            let (Some(first), Some(second)) = (
                self.script_id(contact.first),
                self.script_id(contact.second),
            ) else {
                continue;
            };
            for (node, other) in [(first, second), (second, first)] {
                if let Some(mirrored) = self.mirrored.get_mut(&node) {
                    mirrored.note_contact(other, contact.phase);
                }
            }
        }
    }

    /// What the simulation did to the body of the script node `id`, as of
    /// the last [`ScriptNodes::read_bodies`]; `None` if it has no body in
    /// the simulation yet.
    #[must_use]
    pub fn body_state<'a>(&'a self, world: &World, id: u64) -> Option<ScriptBodyState<'a>> {
        let mirrored = self.mirrored.get(&id)?;
        let (is_3d, position, angles, velocity) =
            if let Some(body) = world.get::<Body3D>(mirrored.entity) {
                body.id()?;
                let transform = world.get::<Transform3D>(mirrored.entity)?;
                (
                    true,
                    transform.position,
                    transform.rotation.to_euler(),
                    body.velocity(),
                )
            } else {
                let body = world.get::<Body2D>(mirrored.entity)?;
                body.id()?;
                let transform = world.get::<Transform2D>(mirrored.entity)?;
                let velocity = body.velocity();
                (
                    false,
                    Vec3::new(transform.position.x, transform.position.y, 0.0),
                    Vec3::new(0.0, 0.0, transform.rotation),
                    Vec3::new(velocity.x, velocity.y, 0.0),
                )
            };
        Some(ScriptBodyState {
            is_3d,
            position,
            angles,
            velocity,
            touching: &mirrored.touching,
            just_touched: &mirrored.just_touched,
            just_separated: &mirrored.just_separated,
        })
    }

    fn script_id(&self, entity: Entity) -> Option<u64> {
        self.ids
            .get(&entity)
            .or_else(|| self.departed.get(&entity))
            .copied()
    }

    /// The world node that mirrors the script node `id`.
    #[must_use]
    pub fn entity(&self, id: u64) -> Option<Entity> {
        self.mirrored.get(&id).map(|mirrored| mirrored.entity)
    }

    /// The 2D camera that shows the world from `camera`: centered on its
    /// node's place in the world, so it follows its parents, or on its own
    /// position if it is not in the game's tree. Rotation and scale are
    /// ignored, so the view never tilts.
    #[must_use]
    pub fn camera(&self, world: &World, camera: &ScriptCamera) -> Camera {
        let center = self
            .entity(camera.id)
            .and_then(|entity| {
                world
                    .get::<GlobalTransform2D>(entity)
                    .map(|global| global.position())
            })
            .unwrap_or(camera.position);
        Camera::centered_2d(center, camera.view_height.max(MIN_VIEW_HEIGHT))
    }

    /// The 3D camera that shows the world from `camera`: from where its
    /// node is in the world, looking along the node's -z axis, so it
    /// follows and turns with its parents; from its own place if it is not
    /// in the game's tree. Its roll is ignored, so the view never tilts.
    #[must_use]
    pub fn camera_3d(&self, world: &World, camera: &ScriptCamera3D) -> Camera {
        let matrix = self
            .entity(camera.id)
            .and_then(|entity| {
                world
                    .get::<GlobalTransform3D>(entity)
                    .map(|global| global.matrix())
            })
            .unwrap_or_else(|| {
                Mat4::from_translation(camera.position) * camera.rotation().to_mat4()
            });
        Camera::along_transform(matrix, camera.shown_field_of_view())
    }

    fn try_mirror(&mut self, world: &mut World, node: &ScriptNode<'_>) -> EngineResult<()> {
        let parent = node.parent.and_then(|parent| self.entity(parent));
        let entity = match self.mirrored.get_mut(&node.id) {
            Some(mirrored) if world.is_alive(mirrored.entity) => {
                mirrored.seen_in_frame = self.frame;
                mirrored.entity
            }
            stale => {
                if let Some(stale) = stale {
                    self.ids.remove(&stale.entity);
                }
                let entity = world.spawn_node(node.name, parent)?;
                self.mirrored
                    .insert(node.id, Mirrored::new(entity, self.frame));
                self.ids.insert(entity, node.id);
                entity
            }
        };
        if world.parent(entity) != parent {
            world.set_parent(entity, parent)?;
        }
        update_name(world, entity, node.name)?;
        update_visible(world, entity, node.visible)?;
        if let Some(transform) = node.transform_3d {
            world.remove::<Transform2D>(entity);
            replace(world, entity, transform)?;
        } else {
            world.remove::<Transform3D>(entity);
            replace(world, entity, node.transform)?;
        }
        self.update_body(world, node, entity)?;
        self.update_mesh(world, node, entity)?;
        self.update_model(world, node, entity)?;
        self.update_light(world, node, entity)?;
        self.update_light_2d(world, node, entity)?;
        if let Some(sprite) = node.sprite {
            self.update_sprite(world, node.id, entity, sprite)
        } else {
            world.remove::<SpriteFile>(entity);
            Ok(())
        }
    }

    fn update_body(
        &mut self,
        world: &mut World,
        node: &ScriptNode<'_>,
        entity: Entity,
    ) -> EngineResult<()> {
        let Some(mirrored) = self.mirrored.get_mut(&node.id) else {
            return Ok(());
        };
        let is_3d = node.transform_3d.is_some();
        if is_3d {
            world.remove::<Body2D>(entity);
        } else {
            world.remove::<Body3D>(entity);
        }
        let Some(script_body) = node.body else {
            world.remove::<Body2D>(entity);
            world.remove::<Body3D>(entity);
            return Ok(());
        };
        let velocity = (script_body.velocity != mirrored.told_velocity
            && is_finite(script_body.velocity))
        .then_some(script_body.velocity);
        let impulse = (script_body.impulse != Vec3::ZERO && is_finite(script_body.impulse))
            .then_some(script_body.impulse);
        let flat = |vector: Vec3| Vec2::new(vector.x, vector.y);
        let synced = if is_3d {
            script_body.to_settings_3d().and_then(|settings| {
                sync_body::<_, star_physics::BodyId3d, Transform3D, Vec3>(
                    world, entity, settings, velocity, impulse,
                )
            })
        } else {
            script_body.to_settings().and_then(|settings| {
                // Script sprites are drawn around their node.
                let settings = match node.sprite {
                    Some(sprite) => shaped_by_sprite(
                        settings,
                        SpriteLook {
                            texture: sprite.texture,
                            size: sprite.size,
                            anchor: Sprite::CENTER,
                        },
                    ),
                    None => settings,
                };
                sync_body::<_, star_physics::BodyId, Transform2D, Vec2>(
                    world,
                    entity,
                    settings,
                    velocity.map(flat),
                    impulse.map(flat),
                )
            })
        };
        match synced {
            Ok(replaced) => {
                mirrored.invalid_body = None;
                if replaced {
                    mirrored.told_velocity = Vec3::ZERO;
                }
                if let Some(velocity) = velocity {
                    mirrored.told_velocity = velocity;
                }
                Ok(())
            }
            Err(star_core::EngineError::Physics(reason)) => {
                world.remove::<Body2D>(entity);
                world.remove::<Body3D>(entity);
                if mirrored.invalid_body.as_ref() != Some(&reason) {
                    log::warn!("the node \"{}\" has no body: {reason}", node.name);
                    mirrored.invalid_body = Some(reason);
                }
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn update_mesh(
        &mut self,
        world: &mut World,
        node: &ScriptNode<'_>,
        entity: Entity,
    ) -> EngineResult<()> {
        let Some(mesh) = node.mesh.filter(|_| node.transform_3d.is_some()) else {
            world.remove::<MeshFile>(entity);
            return Ok(());
        };
        let shape = match mesh.to_shape() {
            Ok(shape) => shape,
            Err(error) => {
                world.remove::<MeshFile>(entity);
                self.reject_look(node, &error.to_string());
                return Ok(());
            }
        };
        let texture = if mesh.texture.is_empty() {
            None
        } else {
            match AssetPath::new(mesh.texture) {
                Ok(path) => Some(path),
                Err(error) => {
                    self.reject_texture(node.id, mesh.texture, &error.to_string());
                    None
                }
            }
        };
        let filter = filter_of(mesh.pixelated);
        let updated = world.get_mut::<MeshFile>(entity).map(|mut file| {
            file.shape = shape;
            file.color = mesh.color;
            file.metallic = mesh.metallic;
            file.roughness = mesh.roughness;
            file.texture.clone_from(&texture);
            file.filter = filter;
        });
        if updated.is_none() {
            let mut file = MeshFile::new(shape)
                .with_color(mesh.color)
                .with_metallic(mesh.metallic)
                .with_roughness(mesh.roughness);
            if let Some(path) = texture {
                file = file.with_texture(path, filter);
            }
            world.insert(entity, file)?;
        }
        Ok(())
    }

    /// Gives `entity` the [`ModelNode`] of the script's `Model`, changing
    /// it only when the script names another file.
    fn update_model(
        &mut self,
        world: &mut World,
        node: &ScriptNode<'_>,
        entity: Entity,
    ) -> EngineResult<()> {
        let Some(model) = node.model.filter(|_| node.transform_3d.is_some()) else {
            world.remove::<ModelNode>(entity);
            world.remove::<ModelPose>(entity);
            return Ok(());
        };
        update_pose(world, entity, &model.clips)?;
        let file = model.file;
        let same = world
            .get::<ModelNode>(entity)
            .is_some_and(|shown| shown.file.as_str() == file);
        if same {
            return Ok(());
        }
        let checked = AssetPath::new(file).and_then(|path| {
            ModelFormat::of(&path)?;
            Ok(path)
        });
        match checked {
            Ok(file) => replace(world, entity, ModelNode { file }),
            Err(error) => {
                world.remove::<ModelNode>(entity);
                self.reject_look(node, &error.to_string());
                Ok(())
            }
        }
    }

    fn update_light(
        &mut self,
        world: &mut World,
        node: &ScriptNode<'_>,
        entity: Entity,
    ) -> EngineResult<()> {
        let Some(light) = node.light.filter(|_| node.transform_3d.is_some()) else {
            world.remove::<LightNode>(entity);
            return Ok(());
        };
        match light.to_light() {
            Ok(light) => replace(world, entity, light),
            Err(error) => {
                world.remove::<LightNode>(entity);
                self.reject_look(node, &error.to_string());
                Ok(())
            }
        }
    }

    fn update_light_2d(
        &mut self,
        world: &mut World,
        node: &ScriptNode<'_>,
        entity: Entity,
    ) -> EngineResult<()> {
        let Some(light) = node.light_2d.filter(|_| node.transform_3d.is_none()) else {
            world.remove::<Light2dNode>(entity);
            return Ok(());
        };
        match light.to_light_2d() {
            Ok(light) => replace(world, entity, light),
            Err(error) => {
                world.remove::<Light2dNode>(entity);
                self.reject_look(node, &error.to_string());
                Ok(())
            }
        }
    }

    /// Logs once why the mesh or light of `node` is not shown.
    fn reject_look(&mut self, node: &ScriptNode<'_>, reason: &str) {
        if let Some(mirrored) = self.mirrored.get_mut(&node.id)
            && mirrored.invalid_look.as_deref() != Some(reason)
        {
            mirrored.invalid_look = Some(reason.to_owned());
            log::warn!("the node \"{}\" shows nothing: {reason}", node.name);
        }
    }

    /// Logs once that the image `texture` of the script node `id` is not
    /// a valid asset path.
    fn reject_texture(&mut self, id: u64, texture: &str, reason: &str) {
        if let Some(mirrored) = self.mirrored.get_mut(&id)
            && mirrored.invalid_texture.as_deref() != Some(texture)
        {
            mirrored.invalid_texture = Some(texture.to_owned());
            log::warn!("the image \"{texture}\" is not shown: {reason}");
        }
    }

    fn update_sprite(
        &mut self,
        world: &mut World,
        id: u64,
        entity: Entity,
        sprite: ScriptSprite<'_>,
    ) -> EngineResult<()> {
        let filter = filter_of(sprite.pixelated);
        let updated = world.get_mut::<SpriteFile>(entity).is_some_and(|mut file| {
            let same_image = file.path.as_str() == sprite.texture;
            if same_image {
                file.filter = filter;
                file.size = sprite.size;
                file.layer = sprite.layer;
                file.color = sprite.color;
                file.sheet = sprite.sheet;
            }
            same_image
        });
        if updated {
            return Ok(());
        }
        match AssetPath::new(sprite.texture) {
            Ok(path) => {
                let file = SpriteFile::new(path, sprite.size)
                    .with_filter(filter)
                    .with_layer(sprite.layer)
                    .with_color(sprite.color)
                    .with_sheet(sprite.sheet);
                world.insert(entity, file)?;
            }
            Err(error) => {
                world.remove::<SpriteFile>(entity);
                self.reject_texture(id, sprite.texture, &error.to_string());
            }
        }
        Ok(())
    }
}

/// Gives the node `entity` a body as `settings` say (keeping the one it
/// has if it is the same), with the `velocity` and `impulse` the script
/// gave it; whether its body was replaced.
fn sync_body<Shape, Id, Transform, Vector>(
    world: &mut World,
    entity: Entity,
    settings: BodySettings<Shape>,
    velocity: Option<Vector>,
    impulse: Option<Vector>,
) -> EngineResult<bool>
where
    Shape: Copy + PartialEq + 'static,
    Id: Copy + 'static,
    Transform: Copy + PartialEq + 'static,
    Vector: Copy + Default + PartialEq + Add<Output = Vector> + 'static,
{
    let kept = world
        .get::<NodeBody<Shape, Id, Transform, Vector>>(entity)
        .is_some_and(|body| *body.settings() == settings);
    if !kept {
        world.insert(
            entity,
            NodeBody::<Shape, Id, Transform, Vector>::new(settings),
        )?;
    }
    if let Some(mut body) = world.get_mut::<NodeBody<Shape, Id, Transform, Vector>>(entity) {
        if let Some(velocity) = velocity {
            body.set_velocity(velocity);
        }
        if let Some(impulse) = impulse {
            body.apply_impulse(impulse);
        }
    }
    Ok(!kept)
}

fn is_finite(vector: Vec3) -> bool {
    vector.x.is_finite() && vector.y.is_finite() && vector.z.is_finite()
}

const fn filter_of(pixelated: bool) -> TextureFilter {
    if pixelated {
        TextureFilter::Pixelated
    } else {
        TextureFilter::Smooth
    }
}

fn update_name(world: &mut World, entity: Entity, name: &str) -> EngineResult<()> {
    let renamed = world
        .get_mut::<Name>(entity)
        .map(|mut current| {
            if current.as_str() != name {
                name.clone_into(&mut current.0);
            }
        })
        .is_some();
    if !renamed {
        world.insert(entity, Name(name.to_owned()))?;
    }
    Ok(())
}

/// Gives `entity` the component `value`, in place if it has one.
/// Marks `entity` [`Hidden`] unless it is `visible`.
fn update_visible(world: &mut World, entity: Entity, visible: bool) -> EngineResult<()> {
    if visible {
        world.remove::<Hidden>(entity);
    } else if !world.has::<Hidden>(entity) {
        world.insert(entity, Hidden)?;
    }
    Ok(())
}

/// Gives `entity` the pose of `clips`, reusing the room of the one it has.
fn update_pose(
    world: &mut World,
    entity: Entity,
    clips: &[Option<ClipAt>; MAX_POSED_CLIPS],
) -> EngineResult<()> {
    let shown = clips.iter().flatten().copied();
    if let Some(mut pose) = world.get_mut::<ModelPose>(entity) {
        pose.clips.clear();
        pose.clips.extend(shown);
        return Ok(());
    }
    world.insert(
        entity,
        ModelPose {
            clips: shown.collect(),
        },
    )?;
    Ok(())
}

fn replace<T: 'static>(world: &mut World, entity: Entity, value: T) -> EngineResult<()> {
    if let Some(mut current) = world.get_mut::<T>(entity) {
        *current = value;
        return Ok(());
    }
    world.insert(entity, value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use star_math::Quat;
    use star_physics::{BodyKind, BodySettings, Physics2d, Physics3d, Shape2d, Shape3d};
    use star_render::{DEFAULT_SPOT_ANGLE, DEFAULT_SPOT_BLEND, Projection};
    use star_scene::{TransformPropagation, step_node_physics, step_node_physics_3d};

    use super::*;

    /// Seconds of a test frame: a 60 Hz screen.
    const FRAME: f32 = 1.0 / 60.0;
    const TOLERANCE: f32 = 1e-3;

    fn node(id: u64, parent: Option<u64>, name: &str) -> ScriptNode<'_> {
        ScriptNode {
            id,
            parent,
            name,
            visible: true,
            transform: Transform2D::IDENTITY,
            transform_3d: None,
            sprite: None,
            body: None,
            mesh: None,
            model: None,
            light: None,
            light_2d: None,
        }
    }

    fn sprite(texture: &str) -> ScriptSprite<'_> {
        ScriptSprite {
            texture,
            size: Vec2::new(2.0, 1.0),
            layer: 3,
            pixelated: true,
            color: Color::rgb(1.0, 0.5, 0.0),
            sheet: SpriteSheet::new(4, 1, 2),
        }
    }

    /// Mirrors one frame of a script tree, given parents first.
    fn frame(mirror: &mut ScriptNodes, world: &mut World, nodes: &[ScriptNode<'_>]) {
        mirror.begin_frame();
        for node in nodes {
            mirror.mirror(world, node);
        }
        mirror.finish_frame(world);
    }

    #[test]
    fn script_nodes_become_world_nodes_with_their_tree_and_transform() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let mut hero = node(2, Some(1), "Hero");
        hero.transform = Transform2D::from_position(Vec2::new(3.0, 1.0));

        frame(&mut mirror, &mut world, &[node(1, None, "Game"), hero]);

        let root = mirror.entity(1).unwrap();
        let hero_entity = mirror.entity(2).unwrap();
        assert_eq!(world.children(root), [hero_entity]);
        assert_eq!(world.get::<Name>(hero_entity).unwrap().as_str(), "Hero");
        assert_eq!(
            world.get::<Transform2D>(hero_entity).unwrap().position,
            Vec2::new(3.0, 1.0)
        );
    }

    #[test]
    fn changes_reach_the_same_world_node() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        frame(&mut mirror, &mut world, &[node(1, None, "Hero")]);
        let entity = mirror.entity(1).unwrap();

        let mut renamed = node(1, None, "Knight");
        renamed.transform.rotation = 1.0;
        frame(&mut mirror, &mut world, &[renamed]);

        assert_eq!(mirror.entity(1), Some(entity));
        assert_eq!(world.get::<Name>(entity).unwrap().as_str(), "Knight");
        assert_eq!(world.get::<Transform2D>(entity).unwrap().rotation, 1.0);
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn hidden_script_nodes_hide_their_world_node() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let hidden = ScriptNode {
            visible: false,
            ..node(1, None, "Ghost")
        };
        frame(&mut mirror, &mut world, &[hidden]);
        let entity = mirror.entity(1).unwrap();
        assert!(world.has::<Hidden>(entity));

        frame(&mut mirror, &mut world, &[node(1, None, "Ghost")]);

        assert!(!world.has::<Hidden>(entity));
    }

    #[test]
    fn nodes_move_between_parents() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let tree = [
            node(1, None, "Game"),
            node(2, Some(1), "A"),
            node(3, Some(1), "B"),
        ];
        frame(&mut mirror, &mut world, &tree);

        frame(
            &mut mirror,
            &mut world,
            &[tree[0], tree[1], node(3, Some(2), "B")],
        );

        let a = mirror.entity(2).unwrap();
        assert_eq!(world.children(a), [mirror.entity(3).unwrap()]);
    }

    #[test]
    fn nodes_the_script_dropped_are_destroyed() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        frame(
            &mut mirror,
            &mut world,
            &[node(1, None, "Game"), node(2, Some(1), "Enemy")],
        );

        frame(&mut mirror, &mut world, &[node(1, None, "Game")]);

        assert_eq!(mirror.entity(2), None);
        assert_eq!(world.entity_count(), 1);
    }

    fn script_camera(id: u64, view_height: f32) -> ScriptCamera {
        ScriptCamera {
            id,
            position: Vec2::new(5.0, 6.0),
            view_height,
        }
    }

    #[test]
    fn cameras_in_the_tree_follow_their_parents() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let mut ship = node(1, None, "Ship");
        ship.transform = Transform2D::from_position(Vec2::new(3.0, 1.0));
        let mut eye = node(2, Some(1), "Eye");
        eye.transform = Transform2D::from_position(Vec2::new(0.0, 0.5));
        frame(&mut mirror, &mut world, &[ship, eye]);
        TransformPropagation::new().run(&mut world).unwrap();

        let camera = mirror.camera(&world, &script_camera(2, 4.0));

        assert_eq!(
            camera.position,
            Vec3::new(3.0, 1.5, Camera::new_2d(4.0).position.z)
        );
        assert_eq!(camera.target, Vec3::new(3.0, 1.5, 0.0));
        assert_eq!(
            camera.projection,
            Projection::Orthographic { view_height: 4.0 }
        );
    }

    #[test]
    fn cameras_outside_the_tree_use_their_own_position() {
        let camera = ScriptNodes::new().camera(&World::new(), &script_camera(7, 2.0));

        assert_eq!(camera.target, Vec3::new(5.0, 6.0, 0.0));
    }

    #[test]
    fn views_never_collapse() {
        let mirror = ScriptNodes::new();
        let world = World::new();

        for height in [0.0, -3.0, f32::NAN] {
            let camera = mirror.camera(&world, &script_camera(1, height));
            assert_eq!(
                camera.projection,
                Projection::Orthographic {
                    view_height: MIN_VIEW_HEIGHT
                }
            );
        }
    }

    fn ball(radius: f32) -> ScriptBody<'static> {
        ScriptBody {
            radius,
            ..ScriptBody::from_settings(&BodySettings::new(
                BodyKind::Dynamic,
                Shape2d::circle(1.0).unwrap(),
            ))
        }
    }

    fn physics_world() -> World {
        let mut world = World::new();
        world.insert_resource(Physics2d::new(Vec2::ZERO));
        world.insert_resource(PhysicsNodes::new());
        world.insert_resource(Physics3d::new(Vec3::ZERO));
        world.insert_resource(PhysicsNodes3d::new());
        world
    }

    /// Runs the engine's part of a frame, as the runtime does after the
    /// script's, then reads the bodies back for the next one.
    fn simulate(mirror: &mut ScriptNodes, world: &mut World) {
        TransformPropagation::new().run(world).unwrap();
        step_node_physics(world, FRAME).unwrap();
        step_node_physics_3d(world, FRAME).unwrap();
        mirror.read_bodies(world);
    }

    #[test]
    fn bodies_move_their_nodes_and_keep_the_scripts_velocity() {
        let mut world = physics_world();
        let mut mirror = ScriptNodes::new();
        let mut shot = node(1, None, "Shot");
        shot.body = Some(ScriptBody {
            velocity: Vec3::new(6.0, 0.0, 0.0),
            ..ball(0.25)
        });

        frame(&mut mirror, &mut world, &[shot]);
        assert_eq!(mirror.body_state(&world, 1), None);
        simulate(&mut mirror, &mut world);
        for _ in 0..9 {
            let state = mirror.body_state(&world, 1).unwrap();
            assert!(!state.is_3d);
            shot.transform.position = Vec2::new(state.position.x, state.position.y);
            shot.transform.rotation = state.angles.z;
            shot.body = Some(ScriptBody {
                velocity: state.velocity,
                ..ball(0.25)
            });
            frame(&mut mirror, &mut world, &[shot]);
            simulate(&mut mirror, &mut world);
        }

        let state = mirror.body_state(&world, 1).unwrap();
        assert!((state.velocity.x - 6.0).abs() < TOLERANCE, "{state:?}");
        assert!((state.position.x - 1.0).abs() < TOLERANCE, "{state:?}");
    }

    #[test]
    fn contacts_name_the_script_nodes() {
        let mut world = physics_world();
        let mut mirror = ScriptNodes::new();
        let mut coin = node(1, None, "Coin");
        coin.body = Some(ScriptBody {
            kind: STATIC,
            sensor: true,
            ..ball(1.0)
        });
        let mut hero = node(2, None, "Hero");
        hero.body = Some(ball(0.5));

        frame(&mut mirror, &mut world, &[coin, hero]);
        simulate(&mut mirror, &mut world);

        let state = mirror.body_state(&world, 1).unwrap();
        assert_eq!((state.touching, state.just_touched), (&[2][..], &[2][..]));
        assert_eq!(mirror.body_state(&world, 2).unwrap().touching, [1]);

        frame(&mut mirror, &mut world, &[coin, hero]);
        simulate(&mut mirror, &mut world);

        let state = mirror.body_state(&world, 1).unwrap();
        assert_eq!((state.touching, state.just_touched), (&[2][..], &[][..]));

        frame(&mut mirror, &mut world, &[coin]);
        simulate(&mut mirror, &mut world);

        let state = mirror.body_state(&world, 1).unwrap();
        let lists = (state.touching, state.just_touched, state.just_separated);
        assert_eq!(lists, (&[][..], &[][..], &[2][..]));
    }

    #[test]
    fn impossible_bodies_are_left_out_and_changed_ones_replaced() {
        let mut world = physics_world();
        let mut mirror = ScriptNodes::new();
        let mut crate_node = node(1, None, "Crate");
        crate_node.body = Some(ScriptBody {
            shape: "triangle",
            ..ball(1.0)
        });

        frame(&mut mirror, &mut world, &[crate_node]);
        let entity = mirror.entity(1).unwrap();
        assert!(!world.has::<Body2D>(entity));

        crate_node.body = Some(ball(1.0));
        frame(&mut mirror, &mut world, &[crate_node]);
        simulate(&mut mirror, &mut world);
        let first = world.get::<Body2D>(entity).unwrap().id();

        crate_node.body = Some(ball(2.0));
        frame(&mut mirror, &mut world, &[crate_node]);
        simulate(&mut mirror, &mut world);
        assert_ne!(world.get::<Body2D>(entity).unwrap().id(), first);

        crate_node.body = None;
        frame(&mut mirror, &mut world, &[crate_node]);
        assert!(!world.has::<Body2D>(entity));
    }

    fn node_3d(id: u64, name: &str, position: Vec3) -> ScriptNode<'_> {
        ScriptNode {
            transform_3d: Some(Transform3D::from_position(position)),
            ..node(id, None, name)
        }
    }

    fn crate_mesh(shape: &str) -> ScriptMesh<'_> {
        ScriptMesh {
            shape,
            width: 2.0,
            height: 1.0,
            depth: 1.0,
            radius: 0.5,
            color: Color::rgb(0.8, 0.6, 0.4),
            metallic: 0.0,
            roughness: 0.8,
            texture: "crate.png",
            pixelated: true,
        }
    }

    #[test]
    fn models_get_their_file_and_the_pose_of_their_animator() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let walking = ClipAt {
            animation: 1,
            time: 0.5,
            weight: 1.0,
        };
        let mut knight = node_3d(1, "Knight", Vec3::ZERO);
        knight.model = Some(ScriptModel {
            file: "knight.glb",
            clips: [Some(walking), None, None, None],
        });

        frame(&mut mirror, &mut world, &[knight]);

        let entity = mirror.entity(1).unwrap();
        assert_eq!(
            world.get::<ModelNode>(entity).unwrap().file.as_str(),
            "knight.glb"
        );
        assert_eq!(world.get::<ModelPose>(entity).unwrap().clips, [walking]);

        knight.model = Some(ScriptModel {
            file: "knight.obj",
            clips: [None; MAX_POSED_CLIPS],
        });
        frame(&mut mirror, &mut world, &[knight]);
        assert!(!world.has::<ModelNode>(entity), "not a model file");
        knight.model = None;
        frame(&mut mirror, &mut world, &[knight]);
        assert!(!world.has::<ModelPose>(entity));
    }

    #[test]
    fn nodes_3d_get_3d_transforms_meshes_and_lights() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let mut crate_node = node_3d(1, "Crate", Vec3::new(1.0, 2.0, 3.0));
        crate_node.mesh = Some(crate_mesh(CUBE));
        let mut lamp = node_3d(2, "Lamp", Vec3::UP);
        lamp.light = Some(ScriptLight {
            kind: POINT,
            color: Color::WHITE,
            intensity: 3.0,
            range: 6.0,
            angle: DEFAULT_SPOT_ANGLE,
            blend: DEFAULT_SPOT_BLEND,
        });

        frame(&mut mirror, &mut world, &[crate_node, lamp]);

        let entity = mirror.entity(1).unwrap();
        assert!(!world.has::<Transform2D>(entity));
        assert_eq!(
            world.get::<Transform3D>(entity).unwrap().position,
            Vec3::new(1.0, 2.0, 3.0)
        );
        let file = world.get::<MeshFile>(entity).unwrap().clone();
        assert_eq!(
            file.shape,
            star_render::MeshShape::Cube {
                size: Vec3::new(2.0, 1.0, 1.0)
            }
        );
        assert_eq!(file.texture.unwrap().as_str(), "crate.png");
        assert_eq!(file.filter, TextureFilter::Pixelated);
        let light = *world.get::<LightNode>(mirror.entity(2).unwrap()).unwrap();
        assert!(matches!(light, LightNode::Point { range: 6.0, .. }));

        crate_node.mesh = Some(crate_mesh("teapot"));
        lamp.light = None;
        frame(&mut mirror, &mut world, &[crate_node, lamp]);

        assert!(!world.has::<MeshFile>(entity));
        assert!(!world.has::<LightNode>(mirror.entity(2).unwrap()));
    }

    #[test]
    fn meshes_and_lights_need_3d_nodes() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let mut flat = node(1, None, "Flat");
        flat.mesh = Some(crate_mesh(CUBE));

        frame(&mut mirror, &mut world, &[flat]);

        assert!(!world.has::<MeshFile>(mirror.entity(1).unwrap()));
    }

    #[test]
    fn bodies_3d_move_their_nodes_in_space() {
        let mut world = physics_world();
        let mut mirror = ScriptNodes::new();
        let mut ball = node_3d(1, "Ball", Vec3::ZERO);
        let sphere = ScriptBody::from_settings_3d(&BodySettings::new(
            BodyKind::Dynamic,
            Shape3d::sphere(0.5).unwrap(),
        ));
        ball.body = Some(ScriptBody {
            velocity: Vec3::new(0.0, 0.0, -3.0),
            ..sphere
        });

        frame(&mut mirror, &mut world, &[ball]);
        simulate(&mut mirror, &mut world);
        for _ in 0..9 {
            let state = mirror.body_state(&world, 1).unwrap();
            assert!(state.is_3d);
            ball.transform_3d = Some(
                Transform3D::from_position(state.position)
                    .with_rotation(Quat::from_euler(state.angles)),
            );
            ball.body = Some(ScriptBody {
                velocity: state.velocity,
                ..sphere
            });
            frame(&mut mirror, &mut world, &[ball]);
            simulate(&mut mirror, &mut world);
        }

        let entity = mirror.entity(1).unwrap();
        assert!(world.has::<Body3D>(entity) && !world.has::<Body2D>(entity));
        let state = mirror.body_state(&world, 1).unwrap();
        assert!((state.position.z + 0.5).abs() < TOLERANCE, "{state:?}");

        ball.body = Some(ScriptBody {
            shape: CIRCLE,
            ..sphere
        });
        frame(&mut mirror, &mut world, &[ball]);
        assert!(!world.has::<Body3D>(entity));
    }

    #[test]
    fn cameras_3d_look_along_their_nodes() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let mut rig = node_3d(1, "Rig", Vec3::new(0.0, 0.0, 5.0));
        rig.transform_3d = rig
            .transform_3d
            .map(|place| place.with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)));
        let eye = ScriptNode {
            parent: Some(1),
            ..node_3d(2, "Eye", Vec3::ZERO)
        };
        frame(&mut mirror, &mut world, &[rig, eye]);
        TransformPropagation::new().run(&mut world).unwrap();
        let camera = ScriptCamera3D {
            id: 2,
            position: Vec3::ZERO,
            angles: Vec3::ZERO,
            field_of_view: 1.0,
        };

        let view = mirror.camera_3d(&world, &camera);

        assert!((view.position - Vec3::new(0.0, 0.0, 5.0)).length() < TOLERANCE);
        // Turned a quarter left, -z becomes -x.
        let forward = view.target - view.position;
        assert!(
            (forward - Vec3::new(-1.0, 0.0, 0.0)).length() < TOLERANCE,
            "{forward:?}"
        );
        assert_eq!(
            view.projection,
            Projection::Perspective { vertical_fov: 1.0 }
        );
    }

    #[test]
    fn sprites_name_their_image_and_follow_its_changes() {
        let mut world = World::new();
        let mut mirror = ScriptNodes::new();
        let mut hero = node(1, None, "Hero");
        hero.sprite = Some(sprite("sprites/hero.png"));
        frame(&mut mirror, &mut world, &[hero]);
        let entity = mirror.entity(1).unwrap();

        let file = world.get::<SpriteFile>(entity).unwrap().clone();
        assert_eq!(file.path.as_str(), "sprites/hero.png");
        assert_eq!(file.filter, TextureFilter::Pixelated);
        assert_eq!((file.size, file.layer), (Vec2::new(2.0, 1.0), 3));
        assert_eq!(file.color, Color::rgb(1.0, 0.5, 0.0));
        assert_eq!(file.sheet, SpriteSheet::new(4, 1, 2));

        let mut faded = sprite("sprites/hero.png");
        faded.color = Color::rgba(1.0, 1.0, 1.0, 0.5);
        faded.sheet = SpriteSheet::new(4, 1, 3);
        hero.sprite = Some(faded);
        frame(&mut mirror, &mut world, &[hero]);
        let file = world.get::<SpriteFile>(entity).unwrap().clone();
        assert_eq!((file.color, file.sheet), (faded.color, faded.sheet));

        hero.sprite = Some(sprite("sprites/knight.png"));
        frame(&mut mirror, &mut world, &[hero]);
        assert_eq!(
            world.get::<SpriteFile>(entity).unwrap().path.as_str(),
            "sprites/knight.png"
        );

        hero.sprite = Some(sprite("../outside.png"));
        frame(&mut mirror, &mut world, &[hero]);
        assert!(!world.has::<SpriteFile>(entity));
    }
}
