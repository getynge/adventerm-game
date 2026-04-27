//! Process-wide log sink for the developer console.
//!
//! This module installs a `log::Log` implementation that funnels every record
//! into a bounded ring buffer. The console renderer reads from the same
//! buffer each frame. Other modules (in either crate) call into the buffer
//! transparently via the `log::{trace,debug,info,warn,error}!` macros.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use log::{Level, LevelFilter, Log, Metadata, Record};

/// Maximum number of log records retained for the console pane. Excess
/// records drop from the front (oldest first) when new ones arrive.
pub const LOG_BUFFER_CAPACITY: usize = 256;

/// One captured log record. Stored verbatim — formatting (level color,
/// target, etc.) happens in the renderer so we can render a slice without
/// re-traversing the buffer.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LogEntry {
    pub level: Level,
    pub target: String,
    pub message: String,
}

static LOG_BUFFER: OnceLock<Mutex<VecDeque<LogEntry>>> = OnceLock::new();
static LOGGER: ConsoleLogger = ConsoleLogger;

struct ConsoleLogger;

impl Log for ConsoleLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let entry = LogEntry {
            level: record.level(),
            target: record.target().to_string(),
            message: record.args().to_string(),
        };
        let buffer = LOG_BUFFER.get_or_init(default_buffer);
        if let Ok(mut guard) = buffer.lock() {
            if guard.len() == LOG_BUFFER_CAPACITY {
                guard.pop_front();
            }
            guard.push_back(entry);
        }
    }

    fn flush(&self) {}
}

fn default_buffer() -> Mutex<VecDeque<LogEntry>> {
    Mutex::new(VecDeque::with_capacity(LOG_BUFFER_CAPACITY))
}

/// Install the console logger as the global `log::Log` implementation.
/// Idempotent: a second call is a no-op (the second `set_logger` is
/// rejected by the `log` crate but that's fine — we only need the first
/// install to win).
pub fn init() {
    let _ = LOG_BUFFER.set(default_buffer());
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(LevelFilter::Trace);
}

/// Snapshot the last `take` entries (most recent last). Used by the console
/// renderer to draw the visible window without holding the lock during draw.
pub fn snapshot(take: usize) -> Vec<LogEntry> {
    let Some(buffer) = LOG_BUFFER.get() else {
        return Vec::new();
    };
    let Ok(guard) = buffer.lock() else {
        return Vec::new();
    };
    let len = guard.len();
    let start = len.saturating_sub(take);
    guard.iter().skip(start).cloned().collect()
}

/// Total number of entries currently buffered. The renderer uses this to
/// decide whether scrolling is meaningful.
#[allow(dead_code)]
pub fn len() -> usize {
    LOG_BUFFER
        .get()
        .and_then(|b| b.lock().ok().map(|g| g.len()))
        .unwrap_or(0)
}
