//! Shortcuts to the resources the engine keeps in the world.

use std::cell::{Ref, RefMut};

use star_audio::Audio;
use star_core::Time;
use star_ecs::World;
use star_input::Input;
use star_physics::{Physics2d, Physics3d};
use star_platform::WindowSize;
use star_scene::{PhysicsNodes, PhysicsNodes3d};

/// Quick access to the engine's resources in a [`World`]: the [`Time`] and
/// the [`Input`] of the current frame, the [`WindowSize`], the [`Audio`] and
/// the 2D and 3D physics. The engine adds them before the game starts and keeps
/// them updated.
pub trait EngineWorld {
    /// The timing of the current frame (zero before the first one).
    fn time(&self) -> Time;

    /// What the player is doing. `None` only in a world the engine does not
    /// run, or while a system holds the input to change it.
    fn input(&self) -> Option<Ref<'_, Input>>;

    /// The size of the game's window, empty (zero) before it opens.
    fn window_size(&self) -> WindowSize;

    /// Where the game plays its sounds. It is silent on machines without an
    /// audio device, and in a world the engine does not run.
    fn audio(&self) -> Audio;

    /// The 2D physics, to read where bodies are and how fast they move.
    /// `None` in a world the engine does not run, or while a system holds
    /// it to change it.
    fn physics(&self) -> Option<Ref<'_, Physics2d>>;

    /// The 2D physics, to change velocities, push bodies or change gravity.
    /// `None` like [`physics`](Self::physics).
    fn physics_mut(&self) -> Option<RefMut<'_, Physics2d>>;

    /// Which node each body is, and the contacts between nodes of the last
    /// frame. `None` like [`physics`](Self::physics).
    fn physics_nodes(&self) -> Option<Ref<'_, PhysicsNodes>>;

    /// The 3D physics, to read where bodies are and how fast they move.
    /// `None` like [`physics`](Self::physics).
    fn physics_3d(&self) -> Option<Ref<'_, Physics3d>>;

    /// The 3D physics, to change velocities, push bodies or change gravity
    /// (Earth's, along -y, until the game changes it). `None` like
    /// [`physics`](Self::physics).
    fn physics_3d_mut(&self) -> Option<RefMut<'_, Physics3d>>;

    /// Which 3D node each 3D body is, and the contacts between 3D nodes of
    /// the last frame. `None` like [`physics`](Self::physics).
    fn physics_nodes_3d(&self) -> Option<Ref<'_, PhysicsNodes3d>>;
}

impl EngineWorld for World {
    fn time(&self) -> Time {
        self.resource::<Time>()
            .map(|time| *time)
            .unwrap_or_default()
    }

    fn input(&self) -> Option<Ref<'_, Input>> {
        self.resource::<Input>()
    }

    fn window_size(&self) -> WindowSize {
        self.resource::<WindowSize>()
            .map_or(WindowSize::new(0, 0), |size| *size)
    }

    fn audio(&self) -> Audio {
        self.resource::<Audio>()
            .map_or_else(Audio::silent, |audio| audio.clone())
    }

    fn physics(&self) -> Option<Ref<'_, Physics2d>> {
        self.resource::<Physics2d>()
    }

    fn physics_mut(&self) -> Option<RefMut<'_, Physics2d>> {
        self.resource_mut::<Physics2d>()
    }

    fn physics_nodes(&self) -> Option<Ref<'_, PhysicsNodes>> {
        self.resource::<PhysicsNodes>()
    }

    fn physics_3d(&self) -> Option<Ref<'_, Physics3d>> {
        self.resource::<Physics3d>()
    }

    fn physics_3d_mut(&self) -> Option<RefMut<'_, Physics3d>> {
        self.resource_mut::<Physics3d>()
    }

    fn physics_nodes_3d(&self) -> Option<Ref<'_, PhysicsNodes3d>> {
        self.resource::<PhysicsNodes3d>()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn the_engine_resources_are_read_from_the_world() {
        let mut world = World::new();
        let mut time = Time::new();
        time.advance(Duration::from_millis(16));
        world.insert_resource(time);
        world.insert_resource(Input::new());
        world.insert_resource(WindowSize::new(640, 480));

        assert_eq!(world.time().delta(), Duration::from_millis(16));
        assert!(world.input().is_some());
        assert_eq!(world.window_size(), WindowSize::new(640, 480));
    }

    #[test]
    fn worlds_without_them_have_no_time_input_or_window() {
        let world = World::new();

        assert_eq!(world.time().frame_count(), 0);
        assert!(world.input().is_none());
        assert!(world.window_size().is_empty());
        assert!(!world.audio().has_output());
        assert!(world.physics().is_none());
        assert!(world.physics_nodes().is_none());
        assert!(world.physics_3d().is_none());
        assert!(world.physics_nodes_3d().is_none());
    }
}
