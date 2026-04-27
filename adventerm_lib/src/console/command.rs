//! `DevCommand` trait + the single registry that lists every console command.
//!
//! Adding a command:
//! 1. Drop a new file under `console/commands/` with a ZST that
//!    `impl DevCommand`.
//! 2. Add one `&NewCommand` line to [`registry`].
//!
//! The completer queries the same `registry` so command names and argument
//! completions stay in lockstep.

use crate::GameState;

use super::commands::{fullbright::FullbrightCommand, give::GiveCommand, spawn::SpawnCommand};

/// Mutable runtime context handed to a command's `execute`. The console
/// itself owns the input/log/state; commands only need access to the
/// in-flight `GameState`. Returning `Err` becomes a `log::error!` line.
pub struct DevCtx<'a> {
    /// Active gameplay state, if the underlying screen has one. Many
    /// commands require it; commands that don't (e.g. a future `help`)
    /// simply ignore the field.
    pub game: Option<&'a mut GameState>,
}

/// Read-only context for completion. `game` is borrowed immutably so
/// completers can inspect the world without locking out other planning.
/// Today no command consumes `game` from the completion path, but the
/// field is part of the trait surface so future commands (e.g.
/// "complete enemies in this room") can reach for it without rewriting
/// the trait.
pub struct CompletionCtx<'a> {
    #[allow(dead_code)]
    pub game: Option<&'a GameState>,
}

/// One command. Trait-objects live in `registry()`; impls live under
/// `commands/`. Methods default to "no-op" so a command only declares the
/// pieces that differ from the boilerplate.
pub trait DevCommand: Sync {
    /// Lower-case identifier the user types. Must be unique across the
    /// registry. Whitespace-free.
    fn name(&self) -> &'static str;

    /// One-line summary shown by `help` (when added) and the renderer's
    /// hint footer. Leave blank to suppress.
    #[allow(dead_code)]
    fn help(&self) -> &'static str {
        ""
    }

    /// Yield the candidate completions for the argument at index
    /// `arg_index` (zero-based, *excludes* the command name) given that
    /// the user has so far typed `prior_args` and is currently typing
    /// `partial`.
    ///
    /// Returning an empty Vec disables completion at that position.
    fn arg_completions(
        &self,
        arg_index: usize,
        prior_args: &[&str],
        partial: &str,
        ctx: &CompletionCtx<'_>,
    ) -> Vec<String> {
        let _ = (arg_index, prior_args, partial, ctx);
        Vec::new()
    }

    /// Run the command. `args` are the already-tokenized arguments
    /// (whitespace-split, quoting honored). On success returns a message
    /// to surface as an info-level log line; on failure an error message
    /// surfaces at error level. Either way the message is funneled
    /// through the `log` crate by the executor — commands themselves
    /// should not call `log::*!` for their primary output.
    fn execute(&self, args: &[String], ctx: &mut DevCtx<'_>) -> Result<String, String>;
}

/// Every console command, in display order. The completer also iterates
/// this list when offering command-name completions. Adding a new command:
/// declare another `static` ZST instance at module scope (alongside the
/// existing ones below) and add a single `&NAME` entry to `REGISTRY`.
static FULLBRIGHT: FullbrightCommand = FullbrightCommand;
static SPAWN: SpawnCommand = SpawnCommand;
static GIVE: GiveCommand = GiveCommand;

const REGISTRY: &[&'static dyn DevCommand] = &[&FULLBRIGHT, &SPAWN, &GIVE];

pub fn registry() -> &'static [&'static dyn DevCommand] {
    REGISTRY
}

/// Look up a command by exact name. `None` if unknown.
pub fn find(name: &str) -> Option<&'static dyn DevCommand> {
    registry().iter().copied().find(|c| c.name() == name)
}
