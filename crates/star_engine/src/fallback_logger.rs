//! A logger for games that did not install one.
//!
//! The engine reports problems that do not stop the game, such as a missing
//! texture, through the `log` facade. Without a logger those messages are
//! lost, and a game that is missing a sprite has no way to tell why. Before
//! the game starts, [`install`] sets this minimal logger, unless the game
//! already installed its own: then that one is kept.

use std::io::Write as _;

use log::{Level, LevelFilter, Log, Metadata, Record};

/// Only problems are shown: information messages would flood the console of
/// every game.
const SHOWN_LEVEL: LevelFilter = LevelFilter::Warn;

/// Writes warnings and errors to the standard error output.
struct FallbackLogger;

static LOGGER: FallbackLogger = FallbackLogger;

/// Installs the fallback logger if the game has no logger yet.
pub(crate) fn install() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(SHOWN_LEVEL);
    }
}

impl Log for FallbackLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= SHOWN_LEVEL
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let line = format_line(record.level(), record.args());
            // Nothing can be reported if the error output itself fails.
            let _ = writeln!(std::io::stderr(), "{line}");
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

/// How a message is shown, e.g. `warning: "hero.png" does not exist`.
fn format_line(level: Level, message: &std::fmt::Arguments<'_>) -> String {
    let label = match level {
        Level::Error => "error",
        Level::Warn => "warning",
        Level::Info => "info",
        Level::Debug => "debug",
        Level::Trace => "trace",
    };
    format!("{label}: {message}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_shown(level: Level) -> bool {
        LOGGER.enabled(&Metadata::builder().level(level).build())
    }

    #[test]
    fn only_problems_are_shown() {
        assert!(is_shown(Level::Error));
        assert!(is_shown(Level::Warn));
        assert!(!is_shown(Level::Info));
        assert!(!is_shown(Level::Debug));
    }

    #[test]
    fn lines_start_with_the_kind_of_message() {
        let line = format_line(
            Level::Warn,
            &format_args!("\"{}\" does not exist", "hero.png"),
        );

        assert_eq!(line, "warning: \"hero.png\" does not exist");
    }
}
