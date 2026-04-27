//! Binary-side glue for the developer console. The console runtime —
//! `ConsoleState`, command registry, parser, completer — lives in
//! [`adventerm_lib::console`]. The binary only owns:
//!
//! - [`log_sink`] — the global `log::Log` buffer the renderer reads.
//!
//! Re-exports `ConsoleState` so existing call sites (`Screen::DeveloperConsole`,
//! the renderer) keep using `crate::console::ConsoleState`.

pub mod log_sink;

pub use adventerm_lib::console::ConsoleState;
