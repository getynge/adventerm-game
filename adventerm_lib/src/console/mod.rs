//! Developer console — typed-command runtime, argument completion, and
//! command registry. The binary owns log capture and rendering; this
//! module owns the input buffer, parser, and command implementations so
//! every console feature stays inside the gameplay crate.
//!
//! Adding a command: see [`command::DevCommand`] — drop a ZST under
//! [`commands`] and add a single line to the registry.

pub mod command;
pub mod commands;
pub mod complete;
pub mod parse;
pub mod state;

pub use command::{registry, DevCommand};
pub use state::ConsoleState;
