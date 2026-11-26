//! The value a [`PropertyTrack`] gives its property at a time, as an
//! `Animator` plays it: the editor previews animations with it.

use crate::FieldValue;
use crate::track_animations::{Interpolation, PropertyTrack};

impl Interpolation {
    /// How far between two keys the value is, from 0 (the first key's) to
    /// 1 (the next key's), when `progress` of the time between them has
    /// passed.
    #[must_use]
    pub fn apply(self, progress: f32) -> f32 {
        let progress = progress.clamp(0.0, 1.0);
        match self {
            Self::Step => 0.0,
            Self::Linear => progress,
            Self::EaseIn => progress * progress,
            Self::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            Self::EaseInOut if progress < 0.5 => 2.0 * progress * progress,
            Self::EaseInOut => 1.0 - (2.0 - 2.0 * progress).powi(2) / 2.0,
        }
    }
}

impl PropertyTrack {
    /// The value `time` seconds into the animation: the first key's before
    /// it, the last key's after it, and between two keys what its
    /// interpolation gives. `None` if it has no keys.
    #[must_use]
    pub fn value_at(&self, time: f32) -> Option<FieldValue> {
        let after = self.keys.partition_point(|key| key.time <= time);
        let Some(before) = after.checked_sub(1).and_then(|index| self.keys.get(index)) else {
            return self.keys.first().map(|key| key.value.clone());
        };
        let Some(next) = self.keys.get(after) else {
            return Some(before.value.clone());
        };
        let progress = (time - before.time) / (next.time - before.time);
        Some(blend(
            &before.value,
            &next.value,
            self.interpolation.apply(progress),
        ))
    }
}

/// The value `amount` of the way from `from` to `to`: decimals and colors
/// blend; whole numbers, `bool`s and texts keep `from`.
fn blend(from: &FieldValue, to: &FieldValue, amount: f32) -> FieldValue {
    let number = |value: &FieldValue| match value {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an animated number is far below where f64 loses whole numbers"
        )]
        FieldValue::Int(number) => Some(*number as f64),
        FieldValue::Float(number) => Some(*number),
        _ => None,
    };
    let decimals = matches!(from, FieldValue::Float(_)) || matches!(to, FieldValue::Float(_));
    match (from, to) {
        (FieldValue::Color(start), FieldValue::Color(end)) => {
            let mut mixed = *start;
            for (channel, end) in mixed.iter_mut().zip(end) {
                *channel += (end - *channel) * amount;
            }
            FieldValue::Color(mixed)
        }
        _ if decimals => match (number(from), number(to)) {
            (Some(start), Some(end)) => {
                FieldValue::Float(start + (end - start) * f64::from(amount))
            }
            _ => from.clone(),
        },
        _ => from.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track_animations::Key;

    const EPSILON: f32 = 1e-6;

    fn track(interpolation: Interpolation, keys: Vec<(f32, FieldValue)>) -> PropertyTrack {
        PropertyTrack {
            node: String::new(),
            property: "x".to_owned(),
            interpolation,
            keys: keys
                .into_iter()
                .map(|(time, value)| Key { time, value })
                .collect(),
        }
    }

    fn float(value: Option<FieldValue>) -> f64 {
        match value {
            Some(FieldValue::Float(number)) => number,
            other => panic!("not a decimal: {other:?}"),
        }
    }

    #[test]
    fn interpolations_start_and_end_on_the_keys() {
        for ease in [
            Interpolation::Linear,
            Interpolation::EaseIn,
            Interpolation::EaseOut,
            Interpolation::EaseInOut,
        ] {
            assert!(ease.apply(0.0).abs() < EPSILON, "{ease:?}");
            assert!((ease.apply(1.0) - 1.0).abs() < EPSILON, "{ease:?}");
        }
        assert!(Interpolation::EaseIn.apply(0.5) < 0.5);
        assert!(Interpolation::EaseOut.apply(0.5) > 0.5);
        assert!((Interpolation::EaseInOut.apply(0.5) - 0.5).abs() < EPSILON);
        assert!(Interpolation::Step.apply(0.9).abs() < EPSILON);
    }

    #[test]
    fn decimals_blend_between_keys_and_hold_outside_them() {
        let slide = track(
            Interpolation::Linear,
            vec![(1.0, FieldValue::Float(0.0)), (3.0, FieldValue::Int(4))],
        );

        assert!(float(slide.value_at(0.0)).abs() < 1e-6);
        assert!((float(slide.value_at(2.0)) - 2.0).abs() < 1e-6);
        assert_eq!(slide.value_at(5.0), Some(FieldValue::Int(4)));
    }

    #[test]
    fn colors_blend_and_the_rest_jump() {
        let fade = track(
            Interpolation::Linear,
            vec![
                (0.0, FieldValue::Color([0.0, 0.0, 0.0, 1.0])),
                (1.0, FieldValue::Color([1.0, 0.5, 0.0, 1.0])),
            ],
        );
        let frames = track(
            Interpolation::Linear,
            vec![(0.0, FieldValue::Int(0)), (1.0, FieldValue::Int(3))],
        );

        assert_eq!(
            fade.value_at(0.5),
            Some(FieldValue::Color([0.5, 0.25, 0.0, 1.0]))
        );
        assert_eq!(frames.value_at(0.9), Some(FieldValue::Int(0)));
        assert_eq!(track(Interpolation::Step, Vec::new()).value_at(0.0), None);
    }
}
