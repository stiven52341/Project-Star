//! Frame timing.

use std::time::Duration;

/// Tracks how much time passes between frames and since the game started.
#[derive(Debug, Clone, Default)]
pub struct Time {
    delta: Duration,
    elapsed: Duration,
    frame_count: u64,
}

impl Time {
    /// Creates a clock at time zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the clock by one frame that lasted `delta`.
    pub fn advance(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed = self.elapsed.saturating_add(delta);
        self.frame_count = self.frame_count.saturating_add(1);
    }

    /// Duration of the last frame.
    #[must_use]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Duration of the last frame, in seconds.
    #[must_use]
    pub fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    /// Total time elapsed since the clock started.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Number of frames advanced so far.
    #[must_use]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_accumulates_elapsed_time_and_frames() {
        let mut time = Time::new();

        time.advance(Duration::from_millis(16));
        time.advance(Duration::from_millis(17));

        assert_eq!(time.delta(), Duration::from_millis(17));
        assert_eq!(time.elapsed(), Duration::from_millis(33));
        assert_eq!(time.frame_count(), 2);
    }
}
