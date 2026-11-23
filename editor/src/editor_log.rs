//! The editor's log: every message goes to the terminal, as `env_logger`
//! writes it, and is kept in a [`LogHistory`] that the “Console” tab shows.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use log::{Level, Log, Metadata, Record, SetLoggerError};

/// Most messages kept: a part of the editor that logs every frame never
/// fills the memory.
const MAX_ENTRIES: usize = 2000;

/// One message of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogEntry {
    pub(crate) level: Level,
    /// The module that wrote it: `star_editor::workspace`, `wgpu_core`…
    pub(crate) target: String,
    pub(crate) message: String,
    /// When it was written, since the editor started.
    pub(crate) time: Duration,
}

/// The last messages of the log, shared by the logger, which adds them
/// from any thread, and the console, which shows them.
#[derive(Clone)]
pub(crate) struct LogHistory {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    start: Instant,
}

impl LogHistory {
    pub(crate) fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::new())),
            start: Instant::now(),
        }
    }

    /// Adds a message, forgetting the oldest one if it is full.
    pub(crate) fn push(&self, level: Level, target: &str, message: String) {
        let entry = LogEntry {
            level,
            target: target.to_owned(),
            message,
            time: self.start.elapsed(),
        };
        self.with_entries(|entries| {
            if entries.len() == MAX_ENTRIES {
                entries.pop_front();
            }
            entries.push_back(entry);
        });
    }

    /// Forgets every message.
    pub(crate) fn clear(&self) {
        self.with_entries(VecDeque::clear);
    }

    /// Runs `read` on the messages, oldest first, while no message can be
    /// added: it should be quick.
    pub(crate) fn read<Output>(&self, read: impl FnOnce(&VecDeque<LogEntry>) -> Output) -> Output {
        self.with_entries(|entries| read(entries))
    }

    fn with_entries<Output>(
        &self,
        use_entries: impl FnOnce(&mut VecDeque<LogEntry>) -> Output,
    ) -> Output {
        // A thread that panicked while adding a message left a whole list:
        // it is still good to show.
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        use_entries(&mut entries)
    }
}

/// Writes to the terminal and keeps a copy of what it writes.
struct EditorLogger {
    terminal: env_logger::Logger,
    history: LogHistory,
}

impl Log for EditorLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.terminal.enabled(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        if !self.terminal.matches(record) {
            return;
        }
        self.terminal.log(record);
        self.history
            .push(record.level(), record.target(), record.args().to_string());
    }

    fn flush(&self) {
        self.terminal.flush();
    }
}

/// Makes the log write to the terminal, filtered as `RUST_LOG` says or,
/// without it, as `default_filter` does, and keep a copy in `history`.
///
/// # Errors
///
/// Fails if a logger was already installed.
pub(crate) fn install(default_filter: &str, history: LogHistory) -> Result<(), SetLoggerError> {
    let terminal =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_filter))
            .build();
    let max_level = terminal.filter();
    log::set_boxed_logger(Box::new(EditorLogger { terminal, history }))?;
    log::set_max_level(max_level);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(history: &LogHistory) -> Vec<String> {
        history.read(|entries| entries.iter().map(|entry| entry.message.clone()).collect())
    }

    #[test]
    fn messages_are_kept_in_order() {
        let history = LogHistory::new();

        history.push(Level::Info, "a", "first".to_owned());
        history.push(Level::Warn, "b", "second".to_owned());

        assert_eq!(messages(&history), ["first", "second"]);
        assert_eq!(history.read(|entries| entries[1].level), Level::Warn);
    }

    #[test]
    fn the_oldest_messages_make_room_for_new_ones() {
        let history = LogHistory::new();

        for number in 0..=MAX_ENTRIES {
            history.push(Level::Info, "test", number.to_string());
        }

        let kept = messages(&history);
        assert_eq!(kept.len(), MAX_ENTRIES);
        assert_eq!(kept.first().map(String::as_str), Some("1"));
    }

    #[test]
    fn clearing_forgets_every_message() {
        let history = LogHistory::new();
        history.push(Level::Error, "test", "gone".to_owned());

        history.clear();

        assert_eq!(messages(&history), Vec::<String>::new());
    }

    #[test]
    fn copies_share_the_messages() {
        let history = LogHistory::new();
        let logger_copy = history.clone();

        logger_copy.push(Level::Info, "test", "shared".to_owned());

        assert_eq!(messages(&history), ["shared"]);
    }
}
