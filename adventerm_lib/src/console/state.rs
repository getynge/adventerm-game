//! UI-only state for the developer console screen. Captures the input
//! buffer, command history, the current completion view, and the log
//! scroll position. The console *behavior* (executing a command,
//! resolving completions) lives in `command.rs`/`complete.rs` — this
//! type holds only what survives across keystrokes within an open
//! console.

use std::collections::VecDeque;

use log::Level;

use crate::GameState;

use super::command::{find as find_command, DevCtx};
use super::complete::Completion;
use super::parse::tokenize;

const HISTORY_CAPACITY: usize = 64;

/// Console UI state. Owned by `Screen::DeveloperConsole`.
pub struct ConsoleState {
    pub input: String,
    pub cursor: usize,
    history: VecDeque<String>,
    history_cursor: Option<usize>,
    /// Cached completion view, recomputed on every input mutation.
    completion: Completion,
    /// How many times Tab has been pressed since the last input mutation.
    /// Used to cycle when the same Tab keeps firing on an ambiguous prefix.
    cycle_index: usize,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            history: VecDeque::new(),
            history_cursor: None,
            completion: Completion::default(),
            cycle_index: 0,
        }
    }

    pub fn completion(&self) -> &Completion {
        &self.completion
    }

    /// Recompute the cached completion view against `game`. Call after
    /// any input mutation.
    pub fn refresh_completion(&mut self, game: Option<&GameState>) {
        self.completion = Completion::from_input(&self.input, game);
        self.cycle_index = 0;
    }

    pub fn insert_char(&mut self, c: char, game: Option<&GameState>) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.history_cursor = None;
        self.refresh_completion(game);
    }

    pub fn backspace(&mut self, game: Option<&GameState>) {
        if self.cursor == 0 {
            return;
        }
        // remove one char before cursor
        let mut byte = self.cursor;
        while byte > 0 && !self.input.is_char_boundary(byte - 1) {
            byte -= 1;
        }
        if byte == 0 {
            return;
        }
        let start = byte - 1;
        // walk back further if multi-byte
        let mut start = start;
        while start > 0 && !self.input.is_char_boundary(start) {
            start -= 1;
        }
        self.input.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.history_cursor = None;
        self.refresh_completion(game);
    }

    /// Apply a Tab. Extends the input toward the longest common prefix
    /// (or the only candidate). Repeated Tabs cycle when no LCP extension
    /// is left.
    pub fn tab(&mut self, game: Option<&GameState>) {
        let Some(new_input) = self.completion.accept_into(&self.input, self.cycle_index) else {
            return;
        };
        if new_input == self.input {
            // Nothing changed → bump the cycle index to choose the next match.
            self.cycle_index = self.cycle_index.wrapping_add(1);
            if let Some(input) = self.completion.accept_into(&self.input, self.cycle_index) {
                self.input = input;
                self.cursor = self.input.len();
            }
        } else {
            self.input = new_input;
            self.cursor = self.input.len();
            // Re-derive completion in case the new prefix unlocks more matches.
            self.completion = Completion::from_input(&self.input, game);
        }
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.history_cursor {
            None => self.history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_cursor = Some(next);
        self.input = self.history[next].clone();
        self.cursor = self.input.len();
    }

    pub fn history_down(&mut self) {
        let Some(i) = self.history_cursor else {
            return;
        };
        if i + 1 >= self.history.len() {
            self.history_cursor = None;
            self.input.clear();
            self.cursor = 0;
        } else {
            self.history_cursor = Some(i + 1);
            self.input = self.history[i + 1].clone();
            self.cursor = self.input.len();
        }
    }

    /// Run the command currently in the input buffer (if any). Records
    /// the line into history, clears the buffer, and emits a log entry
    /// (info or error) containing the command's result.
    pub fn submit(&mut self, game: Option<&mut GameState>) {
        let line = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.history_cursor = None;
        self.completion = Completion::default();
        if line.trim().is_empty() {
            return;
        }
        log::log!(target: "dev_console", Level::Info, "> {line}");
        push_history(&mut self.history, line.clone());

        let tokens = tokenize(&line);
        let Some(first) = tokens.first() else {
            return;
        };
        let Some(cmd) = find_command(&first.text) else {
            log::error!(target: "dev_console", "unknown command: {}", first.text);
            return;
        };
        let args: Vec<String> = tokens[1..].iter().map(|t| t.text.clone()).collect();
        let mut ctx = DevCtx { game };
        match cmd.execute(&args, &mut ctx) {
            Ok(msg) => log::info!(target: "dev_console", "{msg}"),
            Err(err) => log::error!(target: "dev_console", "{err}"),
        }
    }
}

fn push_history(history: &mut VecDeque<String>, line: String) {
    if history.back().map(|s| s.as_str()) == Some(line.as_str()) {
        return;
    }
    if history.len() == HISTORY_CAPACITY {
        history.pop_front();
    }
    history.push_back(line);
}
