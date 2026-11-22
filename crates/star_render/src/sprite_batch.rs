//! Groups the sprites of a frame into as few draw calls as possible.
//!
//! Every sprite becomes 4 vertices in one shared buffer; consecutive sprites
//! (in drawing order) with the same texture are drawn by a single call.

use std::ops::Range;

use star_core::{EngineError, EngineResult};

use crate::bytes::extend_with_f32s;
use crate::{Color, Sprite, TextureId};

/// Sprites allowed per frame; bounds the size of the sprite buffers (32 MiB of vertices).
pub(crate) const MAX_SPRITES_PER_FRAME: u32 = 1 << 18;
/// Corners of a sprite.
const VERTICES_PER_SPRITE: u32 = 4;
/// Two triangles per sprite.
const INDICES_PER_SPRITE: u32 = 6;
/// The two triangles of a sprite, as positions in its corners (counter-clockwise
/// from the bottom-left one, see [`Sprite::corners`]).
const SPRITE_TRIANGLES: [u32; INDICES_PER_SPRITE as usize] = [0, 1, 2, 0, 2, 3];
/// Position (2 floats), texture coordinates (2) and color (4), matching `sprite.wgsl`.
const FLOATS_PER_VERTEX: usize = 8;
/// Bytes of one vertex.
const VERTEX_BYTES: usize = FLOATS_PER_VERTEX * size_of::<f32>();
/// Bytes of one vertex, as the GPU measures buffers.
pub(crate) const VERTEX_STRIDE: u64 = VERTEX_BYTES as u64;

/// Consecutive sprites with the same texture, drawn in one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Batch {
    pub(crate) texture: TextureId,
    /// Range of the index buffer to draw.
    pub(crate) indices: Range<u32>,
}

/// The vertices and batches of a frame. Its storage is reused between
/// frames, so preparing them only allocates when a frame has more sprites
/// than any frame before.
#[derive(Default)]
pub(crate) struct SpriteBatcher {
    /// Positions in the frame's sprites, in drawing order.
    order: Vec<usize>,
    vertices: Vec<u8>,
    batches: Vec<Batch>,
    sprite_count: u32,
}

impl SpriteBatcher {
    /// Builds the vertices and batches of `sprites`, mapping each color with
    /// `to_target_space`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::CapacityExceeded`] if there are more than
    /// [`MAX_SPRITES_PER_FRAME`] sprites.
    pub(crate) fn prepare(
        &mut self,
        sprites: &[Sprite],
        to_target_space: impl Fn(Color) -> Color,
    ) -> EngineResult<()> {
        self.sprite_count = u32::try_from(sprites.len())
            .ok()
            .filter(|&count| count <= MAX_SPRITES_PER_FRAME)
            .ok_or(EngineError::CapacityExceeded {
                resource: "sprites per frame",
            })?;
        self.order.clear();
        self.order.extend(0..sprites.len());
        // Unstable sorting does not allocate; the position as second key keeps
        // the order in which sprites of the same layer were added.
        self.order
            .sort_unstable_by_key(|&index| (sprites[index].layer, index));
        self.vertices.clear();
        self.batches.clear();
        for (slot, &index) in (0..).zip(&self.order) {
            let sprite = &sprites[index];
            push_vertices(&mut self.vertices, sprite, to_target_space(sprite.color));
            let indices = slot * INDICES_PER_SPRITE..(slot + 1) * INDICES_PER_SPRITE;
            match self.batches.last_mut() {
                Some(batch) if batch.texture == sprite.texture => batch.indices.end = indices.end,
                _ => self.batches.push(Batch {
                    texture: sprite.texture,
                    indices,
                }),
            }
        }
        Ok(())
    }

    /// Vertex bytes of every prepared sprite, in drawing order.
    pub(crate) fn vertices(&self) -> &[u8] {
        &self.vertices
    }

    /// Draw calls of the prepared sprites, in order.
    pub(crate) fn batches(&self) -> &[Batch] {
        &self.batches
    }

    /// Number of prepared sprites.
    pub(crate) const fn sprite_count(&self) -> u32 {
        self.sprite_count
    }
}

fn push_vertices(bytes: &mut Vec<u8>, sprite: &Sprite, color: Color) {
    let Color { r, g, b, a } = color;
    for (position, texture_point) in sprite.corners() {
        extend_with_f32s(
            bytes,
            &[
                position.x,
                position.y,
                texture_point.x,
                texture_point.y,
                r,
                g,
                b,
                a,
            ],
        );
    }
}

/// Index bytes that draw `sprite_count` sprites, two triangles each.
pub(crate) fn sprite_indices(sprite_count: u32) -> Vec<u8> {
    (0..sprite_count)
        .flat_map(|sprite| SPRITE_TRIANGLES.map(|corner| sprite * VERTICES_PER_SPRITE + corner))
        .flat_map(u32::to_le_bytes)
        .collect()
}

#[cfg(test)]
mod tests {
    use star_math::Vec2;

    use super::*;

    const FLOAT_BYTES: usize = size_of::<f32>();

    fn sprite(texture: usize) -> Sprite {
        Sprite::new(TextureId(texture), Vec2::ZERO, Vec2::ONE)
    }

    fn prepared(sprites: &[Sprite]) -> SpriteBatcher {
        let mut batcher = SpriteBatcher::default();
        batcher.prepare(sprites, |color| color).unwrap();
        batcher
    }

    fn batch(texture: usize, sprites: Range<u32>) -> Batch {
        Batch {
            texture: TextureId(texture),
            indices: sprites.start * INDICES_PER_SPRITE..sprites.end * INDICES_PER_SPRITE,
        }
    }

    /// The red component of the first vertex of the sprite at `slot`.
    fn red_of_sprite(batcher: &SpriteBatcher, slot: usize) -> f32 {
        let start = slot * VERTICES_PER_SPRITE as usize * VERTEX_BYTES + 4 * FLOAT_BYTES;
        let bytes = batcher.vertices()[start..start + FLOAT_BYTES]
            .try_into()
            .unwrap();
        f32::from_le_bytes(bytes)
    }

    #[test]
    fn sprites_sharing_a_texture_are_drawn_together() {
        let batcher = prepared(&[sprite(0), sprite(0), sprite(1), sprite(0)]);

        assert_eq!(
            batcher.batches(),
            [batch(0, 0..2), batch(1, 2..3), batch(0, 3..4)]
        );
        assert_eq!(batcher.sprite_count(), 4);
    }

    #[test]
    fn every_sprite_has_four_vertices() {
        let batcher = prepared(&[sprite(0), sprite(1)]);

        assert_eq!(batcher.vertices().len(), 8 * VERTEX_BYTES);
    }

    #[test]
    fn higher_layers_are_drawn_later() {
        let top = sprite(0).with_layer(1).with_color(Color::WHITE);
        let bottom = sprite(1).with_layer(-1).with_color(Color::BLACK);

        let batcher = prepared(&[top, bottom]);

        assert_eq!(batcher.batches(), [batch(1, 0..1), batch(0, 1..2)]);
        assert!(red_of_sprite(&batcher, 0) < red_of_sprite(&batcher, 1));
    }

    #[test]
    fn sprites_of_one_layer_keep_their_order() {
        let sprites: Vec<_> = (0..20).map(sprite).collect();

        let batcher = prepared(&sprites);

        let textures: Vec<_> = batcher
            .batches()
            .iter()
            .map(|batch| batch.texture.0)
            .collect();
        assert_eq!(textures, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn colors_are_converted_to_target_space() {
        let mut batcher = SpriteBatcher::default();

        batcher.prepare(&[sprite(0)], |_| Color::BLACK).unwrap();

        assert_eq!(red_of_sprite(&batcher, 0), 0.0);
    }

    #[test]
    fn preparing_again_replaces_the_previous_sprites() {
        let mut batcher = prepared(&[sprite(0), sprite(1)]);

        batcher.prepare(&[], |color| color).unwrap();

        assert!(batcher.batches().is_empty() && batcher.vertices().is_empty());
    }

    #[test]
    fn indices_draw_two_triangles_per_sprite() {
        let indices: Vec<u32> = sprite_indices(2)
            .chunks_exact(4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
            .collect();

        assert_eq!(indices, [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }
}
