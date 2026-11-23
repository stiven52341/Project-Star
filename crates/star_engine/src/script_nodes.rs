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
//! Script nodes with a `Body` get a [`Body2D`]. The simulation moves them
//! after the script's frame, so before the next one the generated game reads
//! back where they are, how fast they move and what they touch
//! ([`ScriptNodes::read_bodies`] and [`ScriptNodes::body_state`]).

use std::collections::HashMap;

use star_assets::AssetPath;
use star_core::EngineResult;
use star_ecs::{Entity, World};
use star_math::Vec2;
use star_physics::ContactPhase;
use star_render::{Camera, Color, TextureFilter};
use star_scene::{Body2D, GlobalTransform2D, Hierarchy, Name, PhysicsNodes, Transform2D};

use crate::SpriteFile;
pub use crate::script_body::{
    CAPSULE, CIRCLE, DEFAULT_RADIUS, DEFAULT_SIZE, DYNAMIC, KINEMATIC, RECTANGLE, STATIC,
    ScriptBody, ScriptBodyState,
};

/// One node of the script's tree, as the generated game describes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptNode<'a> {
    /// Identifies the script object for as long as it lives.
    pub id: u64,
    /// The script object of the parent; `None` for the root.
    pub parent: Option<u64>,
    /// Its name.
    pub name: &'a str,
    /// Where it is, relative to its parent.
    pub transform: Transform2D,
    /// The image it shows, for `Sprite` objects.
    pub sprite: Option<ScriptSprite<'a>>,
    /// Its physics body, if it has one.
    pub body: Option<ScriptBody<'a>>,
}

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
}

/// A StarScript `Camera`, as the generated game describes it.
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
    /// The velocity the script was last told, to notice when it sets one.
    told_velocity: Vec2,
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
            told_velocity: Vec2::ZERO,
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
                mirrored.told_velocity = body.velocity();
            }
        }
        if let Some(nodes) = world.resource::<PhysicsNodes>() {
            for contact in nodes.contacts() {
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
        self.departed.clear();
    }

    /// What the simulation did to the body of the script node `id`, as of
    /// the last [`ScriptNodes::read_bodies`]; `None` if it has no body in
    /// the simulation yet.
    #[must_use]
    pub fn body_state<'a>(&'a self, world: &World, id: u64) -> Option<ScriptBodyState<'a>> {
        let mirrored = self.mirrored.get(&id)?;
        let body = world.get::<Body2D>(mirrored.entity)?;
        body.id()?;
        Some(ScriptBodyState {
            transform: *world.get::<Transform2D>(mirrored.entity)?,
            velocity: body.velocity(),
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
        let mut view = Camera::new_2d(camera.view_height.max(MIN_VIEW_HEIGHT));
        view.position.x = center.x;
        view.position.y = center.y;
        view.target.x = center.x;
        view.target.y = center.y;
        view
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
        update_transform(world, entity, node.transform)?;
        self.update_body(world, node, entity)?;
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
        let Some(script_body) = node.body else {
            world.remove::<Body2D>(entity);
            return Ok(());
        };
        let settings = match script_body.to_settings() {
            Ok(settings) => {
                mirrored.invalid_body = None;
                settings
            }
            Err(error) => {
                world.remove::<Body2D>(entity);
                let reason = error.to_string();
                if mirrored.invalid_body.as_ref() != Some(&reason) {
                    log::warn!("the node \"{}\" has no body: {reason}", node.name);
                    mirrored.invalid_body = Some(reason);
                }
                return Ok(());
            }
        };
        let kept = world
            .get::<Body2D>(entity)
            .is_some_and(|body| *body.settings() == settings);
        if !kept {
            world.insert(entity, Body2D::new(settings))?;
            mirrored.told_velocity = Vec2::ZERO;
        }
        if let Some(mut body) = world.get_mut::<Body2D>(entity) {
            if script_body.velocity != mirrored.told_velocity && is_finite(script_body.velocity) {
                body.set_velocity(script_body.velocity);
                mirrored.told_velocity = script_body.velocity;
            }
            if script_body.impulse != Vec2::ZERO && is_finite(script_body.impulse) {
                body.apply_impulse(script_body.impulse);
            }
        }
        Ok(())
    }

    fn update_sprite(
        &mut self,
        world: &mut World,
        id: u64,
        entity: Entity,
        sprite: ScriptSprite<'_>,
    ) -> EngineResult<()> {
        let filter = if sprite.pixelated {
            TextureFilter::Pixelated
        } else {
            TextureFilter::Smooth
        };
        let updated = world.get_mut::<SpriteFile>(entity).is_some_and(|mut file| {
            let same_image = file.path.as_str() == sprite.texture;
            if same_image {
                file.filter = filter;
                file.size = sprite.size;
                file.layer = sprite.layer;
                file.color = sprite.color;
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
                    .with_color(sprite.color);
                world.insert(entity, file)?;
            }
            Err(error) => {
                world.remove::<SpriteFile>(entity);
                if let Some(mirrored) = self.mirrored.get_mut(&id)
                    && mirrored.invalid_texture.as_deref() != Some(sprite.texture)
                {
                    mirrored.invalid_texture = Some(sprite.texture.to_owned());
                    log::warn!("the sprite \"{}\" shows nothing: {error}", sprite.texture);
                }
            }
        }
        Ok(())
    }
}

fn is_finite(vector: Vec2) -> bool {
    vector.x.is_finite() && vector.y.is_finite()
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

fn update_transform(world: &mut World, entity: Entity, transform: Transform2D) -> EngineResult<()> {
    let updated = world
        .get_mut::<Transform2D>(entity)
        .map(|mut current| *current = transform)
        .is_some();
    if !updated {
        world.insert(entity, transform)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use star_math::Vec3;
    use star_physics::{BodyKind, BodySettings, Physics2d, Shape2d};
    use star_render::Projection;
    use star_scene::{TransformPropagation, step_node_physics};

    use super::*;

    /// Seconds of a test frame: a 60 Hz screen.
    const FRAME: f32 = 1.0 / 60.0;
    const TOLERANCE: f32 = 1e-3;

    fn node(id: u64, parent: Option<u64>, name: &str) -> ScriptNode<'_> {
        ScriptNode {
            id,
            parent,
            name,
            transform: Transform2D::IDENTITY,
            sprite: None,
            body: None,
        }
    }

    fn sprite(texture: &str) -> ScriptSprite<'_> {
        ScriptSprite {
            texture,
            size: Vec2::new(2.0, 1.0),
            layer: 3,
            pixelated: true,
            color: Color::rgb(1.0, 0.5, 0.0),
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
        world
    }

    /// Runs the engine's part of a frame, as the runtime does after the
    /// script's, then reads the bodies back for the next one.
    fn simulate(mirror: &mut ScriptNodes, world: &mut World) {
        TransformPropagation::new().run(world).unwrap();
        step_node_physics(world, FRAME).unwrap();
        mirror.read_bodies(world);
    }

    #[test]
    fn bodies_move_their_nodes_and_keep_the_scripts_velocity() {
        let mut world = physics_world();
        let mut mirror = ScriptNodes::new();
        let mut shot = node(1, None, "Shot");
        shot.body = Some(ScriptBody {
            velocity: Vec2::new(6.0, 0.0),
            ..ball(0.25)
        });

        frame(&mut mirror, &mut world, &[shot]);
        assert_eq!(mirror.body_state(&world, 1), None);
        simulate(&mut mirror, &mut world);
        for _ in 0..9 {
            let state = mirror.body_state(&world, 1).unwrap();
            shot.transform = state.transform;
            shot.body = Some(ScriptBody {
                velocity: state.velocity,
                ..ball(0.25)
            });
            frame(&mut mirror, &mut world, &[shot]);
            simulate(&mut mirror, &mut world);
        }

        let state = mirror.body_state(&world, 1).unwrap();
        assert!((state.velocity.x - 6.0).abs() < TOLERANCE, "{state:?}");
        assert!(
            (state.transform.position.x - 1.0).abs() < TOLERANCE,
            "{state:?}"
        );
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

        let mut faded = sprite("sprites/hero.png");
        faded.color = Color::rgba(1.0, 1.0, 1.0, 0.5);
        hero.sprite = Some(faded);
        frame(&mut mirror, &mut world, &[hero]);
        assert_eq!(world.get::<SpriteFile>(entity).unwrap().color, faded.color);

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
