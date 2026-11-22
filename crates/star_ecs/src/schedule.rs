//! Systems and the stages of a frame they run in.

use star_core::EngineResult;

use crate::World;

/// A part of a frame. Every frame runs the stages in this order, and the
/// systems of each stage in the order they were added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    /// Before the game logic: preparing what the frame needs.
    First,
    /// The game logic.
    Update,
    /// Physics, after the logic has moved things (filled in Phase 8).
    Physics,
    /// After everything moved: reacting to the final state of the frame.
    Last,
}

impl Stage {
    /// Every stage, in the order they run.
    pub const ALL: [Self; 4] = [Self::First, Self::Update, Self::Physics, Self::Last];

    const fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Update => 1,
            Self::Physics => 2,
            Self::Last => 3,
        }
    }
}

/// Logic that runs every frame on the world: a function or a closure (which
/// can keep its own state between frames).
pub type System = Box<dyn FnMut(&mut World) -> EngineResult<()>>;

/// The systems of a game, sorted by stage.
#[derive(Default)]
pub struct Schedule {
    stages: [Vec<System>; Stage::ALL.len()],
}

impl Schedule {
    /// A schedule without systems.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs `system` every frame in `stage`, after the systems added before.
    pub fn add_system(
        &mut self,
        stage: Stage,
        system: impl FnMut(&mut World) -> EngineResult<()> + 'static,
    ) {
        self.stages[stage.index()].push(Box::new(system));
    }

    /// Number of systems in `stage`.
    #[must_use]
    pub fn system_count(&self, stage: Stage) -> usize {
        self.stages[stage.index()].len()
    }

    /// Runs every system once, stage by stage.
    ///
    /// # Errors
    ///
    /// Stops at the first system that fails and returns its error.
    pub fn run(&mut self, world: &mut World) -> EngineResult<()> {
        for systems in &mut self.stages {
            for system in systems {
                system(world)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use star_core::EngineError;

    use super::*;

    /// Names of the systems that ran, in order.
    #[derive(Default)]
    struct Log(Vec<&'static str>);

    fn logging(name: &'static str) -> impl FnMut(&mut World) -> EngineResult<()> {
        move |world| {
            if let Some(mut log) = world.resource_mut::<Log>() {
                log.0.push(name);
            }
            Ok(())
        }
    }

    fn world_with_log() -> World {
        let mut world = World::new();
        world.insert_resource(Log::default());
        world
    }

    fn log(world: &World) -> Vec<&'static str> {
        world
            .resource::<Log>()
            .map(|log| log.0.clone())
            .unwrap_or_default()
    }

    #[test]
    fn stages_run_in_order_whatever_the_order_systems_were_added() {
        let mut schedule = Schedule::new();
        schedule.add_system(Stage::Last, logging("last"));
        schedule.add_system(Stage::Physics, logging("physics"));
        schedule.add_system(Stage::Update, logging("update"));
        schedule.add_system(Stage::First, logging("first"));
        let mut world = world_with_log();

        schedule.run(&mut world).unwrap();

        assert_eq!(log(&world), ["first", "update", "physics", "last"]);
    }

    #[test]
    fn systems_of_a_stage_keep_their_order() {
        let mut schedule = Schedule::new();
        schedule.add_system(Stage::Update, logging("a"));
        schedule.add_system(Stage::Update, logging("b"));
        let mut world = world_with_log();

        schedule.run(&mut world).unwrap();

        assert_eq!(log(&world), ["a", "b"]);
        assert_eq!(schedule.system_count(Stage::Update), 2);
    }

    #[test]
    fn systems_can_spawn_entities_and_keep_state() {
        let mut schedule = Schedule::new();
        let mut runs = 0;
        schedule.add_system(Stage::Update, move |world| {
            runs += 1;
            if runs == 2 {
                world.spawn()?;
            }
            Ok(())
        });
        let mut world = World::new();

        schedule.run(&mut world).unwrap();
        schedule.run(&mut world).unwrap();

        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn a_failing_system_stops_the_frame() {
        let mut schedule = Schedule::new();
        schedule.add_system(Stage::Update, |_| {
            Err(EngineError::Ecs("broken".to_owned()))
        });
        schedule.add_system(Stage::Last, logging("last"));
        let mut world = world_with_log();

        assert!(schedule.run(&mut world).is_err());
        assert_eq!(log(&world), Vec::<&str>::new());
    }
}
