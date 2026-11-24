//! Preparing the engine when a project opens: a tiny game with the same
//! dependencies as every game is built in the background, so the first
//! “Play” finds the engine already built and only builds the game's code.
//!
//! A “Play” started meanwhile is not lost: Cargo makes its build wait for
//! this one and then reuses what it built.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

use super::process::{self, Ended, Events};
use super::{BUILD_DIRECTORY, ENGINE_CRATES, OutputKind, build_command, engine_is_available};

/// The package built to prepare the engine. Its name cannot come from a
/// project: [`super::package_name`] gives none that starts with `_`.
const WARM_UP_PACKAGE: &str = "_star_engine_warm_up";
/// The smallest game: enough for Cargo to build all of the engine.
const WARM_UP_SOURCE: &str = "class Main extends Game {}\n";
/// Name of the thread that prepares the engine.
const THREAD_NAME: &str = "engine warm-up";

/// The engine being prepared in the background.
pub(crate) struct EngineWarmUp {
    finished: Receiver<()>,
    preparing: bool,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EngineWarmUp {
    /// Starts preparing the engine; `notify` runs when it is done (e.g. to
    /// redraw the editor). `None` without the engine's crates, which “Play”
    /// reports on its own, or if the system refused the thread.
    pub(crate) fn start(notify: impl Fn() + Send + Sync + 'static) -> Option<Self> {
        if !engine_is_available() {
            return None;
        }
        let (sender, finished) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_asked = Arc::clone(&stop);
        let spawned = thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || {
                prepare(&stop_asked);
                // A closed editor is not waiting for it, and that is fine.
                let _ = sender.send(());
                notify();
            });
        match spawned {
            Ok(thread) => Some(Self {
                finished,
                preparing: true,
                stop,
                thread: Some(thread),
            }),
            Err(error) => {
                log::warn!("The engine will be built by the first Play: {error}");
                None
            }
        }
    }

    /// Whether the engine is still being prepared.
    pub(crate) fn is_preparing(&mut self) -> bool {
        if self.preparing && self.finished.try_recv().is_ok() {
            self.preparing = false;
        }
        self.preparing
    }
}

impl Drop for EngineWarmUp {
    /// A closed project stops preparing; what Cargo finished is kept.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Builds the warm-up game, telling the log how it went.
fn prepare(stop: &AtomicBool) {
    let program = match star_script::compile_source(WARM_UP_SOURCE) {
        Ok(program) => program,
        Err(errors) => {
            log::warn!("The engine could not be prepared: {errors:?}");
            return;
        }
    };
    let output = Path::new(BUILD_DIRECTORY);
    let package = output.join(WARM_UP_PACKAGE);
    let written = super::write_package(
        WARM_UP_PACKAGE,
        &program,
        Path::new(ENGINE_CRATES),
        &package,
    );
    if let Err(error) = written {
        log::warn!("The engine could not be prepared: {error}");
        return;
    }
    log::info!("Preparing the engine for Play");
    match process::run(
        build_command(&package, output),
        OutputKind::Build,
        &silent_events(),
        stop,
    ) {
        Ok(Ended::Exited(status)) if status.success() => log::info!("The engine is ready to play"),
        Ok(Ended::Exited(status)) => {
            log::warn!("Preparing the engine failed ({status}); Play will show why");
        }
        Ok(Ended::Stopped) => {}
        Err(error) => log::warn!("The engine could not be prepared: {error}"),
    }
}

/// Events nobody reads: Cargo's output while preparing is not shown, and
/// a failure shows again, with its reasons, when the game is played.
fn silent_events() -> Events {
    let (sender, _receiver) = mpsc::channel();
    Events {
        sender,
        notify: Arc::new(|| {}),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use star_script::codegen::EntryPoint;

    use super::*;

    /// Long enough for Cargo to build the engine the first time.
    const BUILD_TIMEOUT: Duration = Duration::from_mins(30);
    /// How often the test looks at the warm-up.
    const POLL_INTERVAL: Duration = Duration::from_millis(100);

    #[test]
    fn the_warm_up_game_uses_the_engine() {
        let program = star_script::compile_source(WARM_UP_SOURCE).expect("valid game");

        assert_eq!(program.entry_point, EntryPoint::Game);
    }

    #[test]
    fn no_project_gets_the_warm_up_package_name() {
        assert_ne!(super::super::package_name(WARM_UP_PACKAGE), WARM_UP_PACKAGE);
    }

    #[test]
    fn the_engine_gets_ready() {
        let mut warm_up = EngineWarmUp::start(|| {}).expect("engine found");
        let started = Instant::now();

        while warm_up.is_preparing() {
            assert!(started.elapsed() < BUILD_TIMEOUT, "the warm-up never ended");
            thread::sleep(POLL_INTERVAL);
        }

        let binary = super::super::binary(Path::new(BUILD_DIRECTORY), WARM_UP_PACKAGE);
        assert!(binary.exists(), "{}", binary.display());
    }
}
