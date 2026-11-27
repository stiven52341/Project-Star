//! Bodies that take the shape of their sprite (decision 86): a 2D body on
//! a node whose sprite shows an image the engine brings is that shape, as
//! big as the sprite and where it is drawn.

use star_assets::BuiltinImage;
use star_math::Vec2;
use star_physics::{BodySettings, Shape2d};

/// What a sprite shows, as a body takes its shape from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteLook<'a> {
    /// Its image, as an asset path.
    pub texture: &'a str,
    /// Its width and height, before the node's scale.
    pub size: Vec2,
    /// Its point at the node, from `(0, 0)` (bottom-left corner) to
    /// `(1, 1)` (top-right corner).
    pub anchor: Vec2,
}

/// `body` as a node whose sprite looks as `sprite` holds it: with the
/// shape of the image the engine brings that the sprite shows, as big as
/// the sprite and centered where it is drawn. The body as it is if the
/// sprite shows another image, or its size makes no shape.
#[must_use]
pub fn shaped_by_sprite(body: BodySettings, sprite: SpriteLook<'_>) -> BodySettings {
    let Some(image) = BuiltinImage::of(sprite.texture) else {
        return body;
    };
    let size = Vec2::new(sprite.size.x.abs(), sprite.size.y.abs());
    let shape = match image {
        BuiltinImage::Square => Shape2d::rectangle(size),
        BuiltinImage::Circle => Shape2d::circle(size.x.max(size.y) / 2.0),
    };
    let center = Vec2::new(
        (0.5 - sprite.anchor.x) * sprite.size.x,
        (0.5 - sprite.anchor.y) * sprite.size.y,
    );
    shape
        .and_then(|shape| shape.with_offset(center))
        .map_or(body, |shape| BodySettings { shape, ..body })
}

#[cfg(test)]
mod tests {
    use star_assets::{CIRCLE_IMAGE, SQUARE_IMAGE};
    use star_physics::{BodyKind, ShapeKind};

    use super::*;

    fn body() -> BodySettings {
        BodySettings::new(BodyKind::Static, Shape2d::circle(0.1).unwrap())
    }

    fn look(texture: &str, size: Vec2) -> SpriteLook<'_> {
        SpriteLook {
            texture,
            size,
            anchor: Vec2::new(0.5, 0.5),
        }
    }

    #[test]
    fn squares_make_rectangles_of_their_size() {
        let floor = shaped_by_sprite(body(), look(SQUARE_IMAGE, Vec2::new(10.0, 1.0)));

        assert_eq!(
            floor.shape.kind(),
            ShapeKind::Rectangle {
                size: Vec2::new(10.0, 1.0)
            }
        );
        assert_eq!(floor.kind, BodyKind::Static, "the rest stays");
    }

    #[test]
    fn circles_hold_their_largest_side_and_follow_the_anchor() {
        let sprite = SpriteLook {
            anchor: Vec2::new(0.5, 0.0),
            ..look(CIRCLE_IMAGE, Vec2::new(2.0, 3.0))
        };

        let ball = shaped_by_sprite(body(), sprite);

        assert_eq!(ball.shape.kind(), ShapeKind::Circle { radius: 1.5 });
        assert_eq!(ball.shape.offset(), Vec2::new(0.0, 1.5));
    }

    #[test]
    fn other_images_and_empty_sprites_keep_the_body() {
        assert_eq!(
            shaped_by_sprite(body(), look("hero.png", Vec2::ONE)),
            body()
        );
        assert_eq!(
            shaped_by_sprite(body(), look(SQUARE_IMAGE, Vec2::ZERO)),
            body()
        );
    }
}
