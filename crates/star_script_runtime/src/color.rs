//! StarScript's `Color`.

use crate::Field;

/// The fields of a color: its channels, from `0.0` to `1.0`.
pub struct ColorFields {
    /// `r`: red.
    pub r: Field<f32>,
    /// `g`: green.
    pub g: Field<f32>,
    /// `b`: blue.
    pub b: Field<f32>,
    /// `a`: how opaque it is.
    pub a: Field<f32>,
}

impl ColorFields {
    /// A color with the channels `r`, `g`, `b` and `a`.
    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: Field::new(r),
            g: Field::new(g),
            b: Field::new(b),
            a: Field::new(a),
        }
    }

    /// Its channels now, in the order `[r, g, b, a]`.
    #[must_use]
    pub fn channels(&self) -> [f32; 4] {
        [self.r.get(), self.g.get(), self.b.get(), self.a.get()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_are_read_in_order() {
        let color = ColorFields::new(0.1, 0.2, 0.3, 0.4);
        color.b.set(0.5);

        assert_eq!(color.channels(), [0.1, 0.2, 0.5, 0.4]);
    }
}
