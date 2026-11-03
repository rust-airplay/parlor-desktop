// SPDX-License-Identifier: GPL-3.0-or-later
//! The log the application installs, and the window's view of it.
//!
//! The library records instead of printing, and a desktop application has no console to
//! print to. This logger keeps the last records in memory for the diagnostics page and
//! appends every record to one file, so a report about a session can be read after the
//! window has been closed.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Local};

use crate::settings::store::Paths;

/// How many records are kept in memory.
const CAPACITY: usize = 2_000;

/// The largest log file that is appended to before it is started over.
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// One recorded line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The wall clock at the moment the line was recorded.
    pub at: DateTime<Local>,
    /// How loud it is.
    pub level: log::Level,
    /// The module that recorded it.
    pub target: String,
    /// The message.
    pub message: String,
}

impl std::fmt::Display for Line {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {:>5} {}: {}",
            self.at.format("%H:%M:%S"),
            self.level,
            self.target,
            self.message
        )
    }
}

/// The log a window reads, and the file it is written to.
#[derive(Clone)]
pub struct LogHandle {
    lines: Arc<Mutex<VecDeque<Line>>>,
    level: Arc<AtomicUsize>,
    file: Arc<Mutex<Option<std::fs::File>>>,
}

impl LogHandle {
    /// Installs the logger, once, and opens the log file.
    ///
    /// A file that cannot be opened is not a reason to refuse to start: the records are
    /// still in memory for the window to show.
    ///
    /// # Errors
    ///
    /// Returns the logger that is already installed as an error string, because a second
    /// installation would silently lose one of the two.
    pub fn install(paths: &Paths, level: log::LevelFilter) -> Result<Self, String> {
        let file_path = paths.logs().join("parlor.log");
        let file = open_log_file(&file_path);
        let handle = Self {
            lines: Arc::new(Mutex::new(VecDeque::with_capacity(CAPACITY))),
            level: Arc::new(AtomicUsize::new(level as usize)),
            file: Arc::new(Mutex::new(file)),
        };
        let logger = Logger { handle: handle.clone() };
        log::set_boxed_logger(Box::new(logger)).map_err(|error| error.to_string())?;
        log::set_max_level(level);
        Ok(handle)
    }

    /// Changes how much is kept, now.
    pub fn set_level(&self, level: log::LevelFilter) {
        self.level.store(level as usize, Ordering::Relaxed);
        log::set_max_level(level);
    }

    /// The records that are still in memory, oldest first.
    pub fn lines(&self) -> Vec<Line> {
        self.lines.lock().map(|lines| lines.iter().cloned().collect()).unwrap_or_default()
    }

    /// Whether a record at this level is kept.
    fn accepts(&self, level: log::Level) -> bool {
        level as usize <= self.level.load(Ordering::Relaxed)
    }

    /// Records one line in memory and on disk.
    fn record(&self, line: Line) {
        {
            let mut lines = match self.lines.lock() {
                Ok(lines) => lines,
                Err(poisoned) => poisoned.into_inner(),
            };
            if lines.len() == CAPACITY {
                lines.pop_front();
            }
            lines.push_back(line.clone());
        }
        if let Ok(mut file) = self.file.lock()
            && let Some(file) = file.as_mut()
        {
            let _ = writeln!(file, "{line}");
        }
    }
}

/// The logger installed with the process.
struct Logger {
    handle: LogHandle,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        self.handle.accepts(metadata.level())
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        self.handle.record(Line {
            at: Local::now(),
            level: record.level(),
            target: record.target().to_owned(),
            message: record.args().to_string(),
        });
    }

    fn flush(&self) {
        if let Ok(mut file) = self.handle.file.lock()
            && let Some(file) = file.as_mut()
        {
            let _ = file.flush();
        }
    }
}

/// Opens the log file, starting it over when it has grown past its bound.
fn open_log_file(path: &std::path::Path) -> Option<std::fs::File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    let too_large = std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_FILE_BYTES);
    std::fs::OpenOptions::new().create(true).truncate(too_large).append(true).open(path).ok()
}

#[cfg(test)]
mod tests {
    use super::{CAPACITY, Line, LogHandle};
    use chrono::Local;
    use std::sync::atomic::Ordering;

    fn handle() -> LogHandle {
        LogHandle {
            lines: std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new())),
            level: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(
                log::LevelFilter::Info as usize,
            )),
            file: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    fn line(level: log::Level) -> Line {
        Line { at: Local::now(), level, target: "test".to_owned(), message: "x".to_owned() }
    }

    #[test]
    fn a_quiet_level_is_not_kept() {
        let handle = handle();
        assert!(handle.accepts(log::Level::Warn));
        assert!(!handle.accepts(log::Level::Debug));
    }

    #[test]
    fn the_buffer_keeps_its_newest_lines() {
        let handle = handle();
        for _ in 0..CAPACITY + 5 {
            handle.record(line(log::Level::Info));
        }
        assert_eq!(handle.lines().len(), CAPACITY);
        handle.level.store(log::LevelFilter::Off as usize, Ordering::Relaxed);
        assert!(!handle.accepts(log::Level::Error));
    }
}
