//! Simulated time, in fixed steps.

/// Seconds simulated by each step. Steps always have the same length, so
/// the simulation behaves the same whatever the frame rate.
pub const FIXED_STEP: f32 = 1.0 / 60.0;

/// Most steps one `advance` of a [`Physics2d`](crate::Physics2d) or a
/// [`Physics3d`](crate::Physics3d) runs. A frame that took longer (the
/// window was dragged, the game paused in a debugger) slows the simulation
/// down instead of making it run ever more steps to catch up.
pub const MAX_STEPS_PER_ADVANCE: u32 = 8;

/// The time waiting to be simulated, less than a step after each advance.
#[derive(Debug, Default)]
pub(crate) struct StepClock {
    pending_time: f32,
}

impl StepClock {
    /// Adds `elapsed` seconds and runs `step` once per [`FIXED_STEP`] they
    /// make, at most [`MAX_STEPS_PER_ADVANCE`] times (the rest is
    /// dropped); gives how many steps ran. Time shorter than a step waits
    /// for the next call.
    pub(crate) fn advance(&mut self, elapsed: f32, mut step: impl FnMut()) -> u32 {
        if elapsed.is_finite() && elapsed > 0.0 {
            self.pending_time += elapsed;
        }
        let mut steps = 0;
        while self.pending_time >= FIXED_STEP && steps < MAX_STEPS_PER_ADVANCE {
            step();
            self.pending_time -= FIXED_STEP;
            steps += 1;
        }
        if steps == MAX_STEPS_PER_ADVANCE {
            self.pending_time = self.pending_time.min(FIXED_STEP);
        }
        steps
    }
}
