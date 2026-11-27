//! How a sound played somewhere is heard from somewhere else (decision
//! 87): quieter the farther it is, and from the side it is on.

/// How far a sound is heard, in world units: as loud as it is within
/// `near`, silent beyond `far`, and fading between them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// Within it, the sound is as loud as it plays.
    pub near: f32,
    /// Beyond it, the sound is not heard.
    pub far: f32,
}

/// Where a sound is heard from: the camera, as games have it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Listener {
    /// Where it is.
    pub position: [f32; 3],
    /// Its right, of length 1: what is that way is heard on the right.
    pub right: [f32; 3],
}

/// How a sound at `source` is heard by `listener`, reaching as far as
/// `reach` says: its volume, from 0 to 1, and its pan, from -1 (the left)
/// to 1 (the right).
#[must_use]
pub fn hear(listener: &Listener, source: [f32; 3], reach: Reach) -> (f32, f32) {
    let offset: [f32; 3] = std::array::from_fn(|axis| source[axis] - listener.position[axis]);
    let distance = offset.iter().map(|part| part * part).sum::<f32>().sqrt();
    let near = reach.near.max(0.0);
    let far = reach.far.max(near);
    let volume = if distance <= near {
        1.0
    } else if distance >= far {
        0.0
    } else {
        1.0 - (distance - near) / (far - near)
    };
    let across: f32 = offset.iter().zip(listener.right).map(|(a, b)| a * b).sum();
    let pan = if far > 0.0 {
        (across / far).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    (volume, pan)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTENER: Listener = Listener {
        position: [0.0; 3],
        right: [1.0, 0.0, 0.0],
    };
    const REACH: Reach = Reach {
        near: 2.0,
        far: 10.0,
    };

    #[test]
    fn sounds_fade_between_near_and_far() {
        let at = |x: f32| hear(&LISTENER, [0.0, x, 0.0], REACH).0;

        assert!((at(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((at(6.0) - 0.5).abs() < 1e-6);
        assert!(at(12.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sounds_are_heard_from_their_side() {
        let (_, left) = hear(&LISTENER, [-5.0, 0.0, 0.0], REACH);
        let (_, right) = hear(&LISTENER, [20.0, 0.0, 0.0], REACH);
        let (_, ahead) = hear(&LISTENER, [0.0, 0.0, -5.0], REACH);

        assert!((left + 0.5).abs() < 1e-6);
        assert!((right - 1.0).abs() < f32::EPSILON);
        assert!(ahead.abs() < f32::EPSILON);
    }
}
