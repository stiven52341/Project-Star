//! Nodes that show a sprite where they are.

use star_core::EngineResult;
use star_ecs::World;
use star_math::Vec2;
use star_render::{Color, Frame, Sprite, TextureId, TextureRegion};

use crate::GlobalTransform2D;

/// Shows a texture at a 2D node: the engine draws it every frame where the
/// node's [`GlobalTransform2D`] places it, rotated and scaled with it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteNode {
    /// Image shown.
    pub texture: TextureId,
    /// Part of the texture shown.
    pub region: TextureRegion,
    /// Width and height in world units, before the node's scale.
    pub size: Vec2,
    /// Point of the sprite placed at the node, from `(0, 0)` (bottom-left
    /// corner) to `(1, 1)` (top-right corner).
    pub anchor: Vec2,
    /// Multiplied by the texture's colors.
    pub color: Color,
    /// Sprites in higher layers are drawn on top of lower ones.
    pub layer: i32,
}

impl SpriteNode {
    /// The whole `texture`, `size` world units big and centered on the node,
    /// with its own colors, in layer 0.
    #[must_use]
    pub const fn new(texture: TextureId, size: Vec2) -> Self {
        Self {
            texture,
            region: TextureRegion::FULL,
            size,
            anchor: Sprite::CENTER,
            color: Color::WHITE,
            layer: 0,
        }
    }

    /// The same sprite showing only `region` of its texture.
    #[must_use]
    pub const fn with_region(mut self, region: TextureRegion) -> Self {
        self.region = region;
        self
    }

    /// The same sprite placed by another `anchor`.
    #[must_use]
    pub const fn with_anchor(mut self, anchor: Vec2) -> Self {
        self.anchor = anchor;
        self
    }

    /// The same sprite tinted with `color`.
    #[must_use]
    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// The same sprite in `layer`.
    #[must_use]
    pub const fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    /// The sprite to draw for a node placed at `global`.
    #[must_use]
    pub fn to_sprite(self, global: GlobalTransform2D) -> Sprite {
        let scale = global.scale();
        let size = Vec2::new(self.size.x * scale.x, self.size.y * scale.y);
        Sprite::new(self.texture, global.position(), size)
            .with_region(self.region)
            .with_rotation(global.rotation())
            .with_anchor(self.anchor)
            .with_color(self.color)
            .with_layer(self.layer)
    }
}

/// Adds to `frame` the sprite of every node with a [`SpriteNode`] and a
/// [`GlobalTransform2D`].
///
/// # Errors
///
/// Returns [`EngineError::Ecs`](star_core::EngineError::Ecs) if their
/// components are being changed by a running query.
pub fn draw_sprite_nodes(world: &World, frame: &mut Frame) -> EngineResult<()> {
    world.each::<(&GlobalTransform2D, &SpriteNode)>(|_, (global, sprite)| {
        frame.draw_sprite(sprite.to_sprite(*global));
    })
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;
    use crate::{Hierarchy, Transform2D, TransformPropagation};

    const EPSILON: f32 = 1e-5;

    /// The frame only stores texture ids, so no renderer is needed.
    fn texture() -> TextureId {
        TextureId::from_index(3)
    }

    #[test]
    fn sprites_follow_the_global_transform_of_their_node() {
        let mut world = World::new();
        let ship = world.spawn_node("Ship", None).unwrap();
        let flag = world.spawn_node("Flag", Some(ship)).unwrap();
        world
            .insert(
                ship,
                Transform2D::from_position(Vec2::new(4.0, 0.0))
                    .with_rotation(FRAC_PI_2)
                    .with_scale(Vec2::new(2.0, -1.0)),
            )
            .unwrap();
        world.insert(flag, Transform2D::IDENTITY).unwrap();
        world
            .insert(
                flag,
                SpriteNode::new(texture(), Vec2::new(1.0, 3.0)).with_layer(2),
            )
            .unwrap();
        TransformPropagation::new().run(&mut world).unwrap();
        let mut frame = Frame::new();

        draw_sprite_nodes(&world, &mut frame).unwrap();

        let [sprite] = frame.sprites() else {
            panic!("one sprite expected, got {:?}", frame.sprites());
        };
        assert_eq!(sprite.position, Vec2::new(4.0, 0.0));
        assert!((sprite.rotation - FRAC_PI_2).abs() < EPSILON);
        assert!((sprite.size.x - 2.0).abs() < EPSILON);
        assert!((sprite.size.y + 3.0).abs() < EPSILON);
        assert_eq!(sprite.layer, 2);
    }

    #[test]
    fn nodes_without_a_global_transform_are_not_drawn() {
        let mut world = World::new();
        let hero = world.spawn_node("Hero", None).unwrap();
        world
            .insert(hero, SpriteNode::new(texture(), Vec2::ONE))
            .unwrap();
        let mut frame = Frame::new();

        draw_sprite_nodes(&world, &mut frame).unwrap();

        assert_eq!(frame.sprite_count(), 0);
    }
}
