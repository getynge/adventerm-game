//! Developer console FFI surface.
//!
//! [`ConsoleHandle`] is an independent owner: console state never borrows
//! from a [`GameHandle`]. Operations that mutate the active game (notably
//! `console_submit`) take a `*mut GameHandle` alongside the console handle.
//!
//! The lib's [`ConsoleState::submit`] logs the executed command's response
//! through the `log` crate without returning it, so the lib API doesn't
//! reach all the way to a callable surface. [`console_submit`] therefore
//! replicates the lib's `submit` flow (tokenize, find, execute) so the
//! command's `Result<String, String>` can be copied into the caller's
//! buffer with `out_was_error` set per outcome.
//!
//! History is also maintained FFI-side: `ConsoleState` stores its history
//! privately with no accessor, so [`ConsoleHandle::history`] mirrors
//! submitted prompts to back the `console_history_*` exports.

use std::ffi::c_char;

use adventerm_lib::console::command::{find as find_command, registry, DevCtx};
use adventerm_lib::console::parse::tokenize;
use adventerm_lib::console::ConsoleState;

use crate::error::{cstr_to_str, set_last_error, FfiError};
use crate::ffi_try;
use crate::handle::{ConsoleHandle, GameHandle};

/// Maximum prompts retained in FFI-side history. Matches the lib's
/// `HISTORY_CAPACITY` so behavior stays consistent across boundaries.
const HISTORY_CAPACITY: usize = 64;

/// Response surfaced when the user submits an unknown command. Matches the
/// log message the lib produces, so callers see the same wording regardless
/// of whether they consume responses via the log sink or via FFI.
const UNKNOWN_COMMAND_PREFIX: &str = "unknown command: ";

/// Resolve `*const ConsoleHandle` or short-circuit with `NullArgument`.
macro_rules! console_ref_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_ref() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        }
    };
}

/// Resolve `*mut ConsoleHandle` or short-circuit with `NullArgument`.
macro_rules! console_mut_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_mut() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        }
    };
}

/// Two-call discovery copy of a UTF-8 string into a caller buffer. Writes
/// the required size (string bytes + trailing NUL) into `out_required`
/// before checking the buffer capacity.
fn copy_str_with_nul(
    text: &str,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    let bytes = text.as_bytes();
    let needed = bytes.len() + 1;
    if !out_required.is_null() {
        unsafe {
            *out_required = needed;
        }
    }
    if buf.is_null() || cap < needed {
        return FfiError::BufferTooSmall as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
        *buf.add(bytes.len()) = 0;
    }
    FfiError::Ok as i32
}

// ---- Lifecycle ------------------------------------------------------------

/// Construct a fresh console handle. Writes the new pointer to `out_console`.
#[unsafe(no_mangle)]
pub extern "C" fn console_new(out_console: *mut *mut ConsoleHandle) -> i32 {
    ffi_try!({
        if out_console.is_null() {
            return FfiError::NullArgument as i32;
        }
        let handle = ConsoleHandle {
            inner: ConsoleState::new(),
            history: Vec::new(),
        };
        unsafe {
            *out_console = Box::into_raw(Box::new(handle));
        }
        FfiError::Ok as i32
    })
}

/// Free a handle previously returned by [`console_new`]. Null is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn console_free(console: *mut ConsoleHandle) {
    if !console.is_null() {
        unsafe {
            drop(Box::from_raw(console));
        }
    }
}

// ---- Editing --------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn console_input_get(
    console: *const ConsoleHandle,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let h = console_ref_or_null!(console);
        copy_str_with_nul(&h.inner.input, buf, cap, out_required)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn console_input_set(
    console: *mut ConsoleHandle,
    text: *const c_char,
) -> i32 {
    ffi_try!({
        let h = console_mut_or_null!(console);
        let s = match cstr_to_str(text) {
            Ok(s) => s,
            Err(e) => return e as i32,
        };
        h.inner.input.clear();
        h.inner.input.push_str(s);
        h.inner.cursor = h.inner.input.len();
        // Keep the cached completion in sync with the new buffer. No game
        // borrow available here; arg-completion candidates that depend on
        // game state still resolve via `console_complete`.
        h.inner.refresh_completion(None);
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn console_cursor(
    console: *const ConsoleHandle,
    out_pos: *mut usize,
) -> i32 {
    ffi_try!({
        let h = console_ref_or_null!(console);
        if out_pos.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_pos = h.inner.cursor;
        }
        FfiError::Ok as i32
    })
}

/// Set the caret to `pos`. Out-of-range or non-char-boundary positions
/// return [`FfiError::OutOfRange`] with detail in `LAST_ERROR`.
#[unsafe(no_mangle)]
pub extern "C" fn console_set_cursor(
    console: *mut ConsoleHandle,
    pos: usize,
) -> i32 {
    ffi_try!({
        let h = console_mut_or_null!(console);
        if pos > h.inner.input.len() {
            set_last_error(format!(
                "cursor {pos} > input.len() {}",
                h.inner.input.len()
            ));
            return FfiError::OutOfRange as i32;
        }
        if !h.inner.input.is_char_boundary(pos) {
            set_last_error(format!("cursor {pos} not on a UTF-8 char boundary"));
            return FfiError::OutOfRange as i32;
        }
        h.inner.cursor = pos;
        FfiError::Ok as i32
    })
}

/// Insert one Unicode scalar at the caret. Invalid scalar values (surrogates,
/// out-of-range) return [`FfiError::OutOfRange`].
#[unsafe(no_mangle)]
pub extern "C" fn console_insert_char(
    console: *mut ConsoleHandle,
    codepoint: u32,
) -> i32 {
    ffi_try!({
        let h = console_mut_or_null!(console);
        let Some(c) = char::from_u32(codepoint) else {
            set_last_error(format!("codepoint {codepoint:#x} is not a Unicode scalar"));
            return FfiError::OutOfRange as i32;
        };
        h.inner.insert_char(c, None);
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn console_delete_back(console: *mut ConsoleHandle) -> i32 {
    ffi_try!({
        let h = console_mut_or_null!(console);
        h.inner.backspace(None);
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn console_clear(console: *mut ConsoleHandle) -> i32 {
    ffi_try!({
        let h = console_mut_or_null!(console);
        h.inner.input.clear();
        h.inner.cursor = 0;
        h.inner.refresh_completion(None);
        FfiError::Ok as i32
    })
}

// ---- Submission -----------------------------------------------------------

/// Run the command currently in the input buffer. Mirrors the lib's
/// `ConsoleState::submit` flow but captures the executed command's
/// `Result<String, String>` into the caller's buffer instead of routing it
/// through the `log` crate. `out_was_error` is set when the user typed an
/// unknown command or the matched command returned `Err`.
#[unsafe(no_mangle)]
pub extern "C" fn console_submit(
    console: *mut ConsoleHandle,
    game: *mut GameHandle,
    out_response: *mut u8,
    cap: usize,
    out_required: *mut usize,
    out_was_error: *mut bool,
) -> i32 {
    ffi_try!({
        let h = console_mut_or_null!(console);
        if out_was_error.is_null() {
            return FfiError::NullArgument as i32;
        }

        // Drain the input the same way the lib does: clear before dispatch
        // so the user sees a fresh buffer the moment a command runs.
        let line = std::mem::take(&mut h.inner.input);
        h.inner.cursor = 0;
        h.inner.refresh_completion(None);

        let trimmed = line.trim();
        if trimmed.is_empty() {
            unsafe {
                *out_was_error = false;
            }
            return copy_str_with_nul("", out_response, cap, out_required);
        }

        push_history(&mut h.history, line.clone());

        let tokens = tokenize(&line);
        let Some(first) = tokens.first() else {
            unsafe {
                *out_was_error = false;
            }
            return copy_str_with_nul("", out_response, cap, out_required);
        };

        let Some(cmd) = find_command(&first.text) else {
            let response = format!("{UNKNOWN_COMMAND_PREFIX}{}", first.text);
            unsafe {
                *out_was_error = true;
            }
            return copy_str_with_nul(&response, out_response, cap, out_required);
        };

        let args: Vec<String> = tokens[1..].iter().map(|t| t.text.clone()).collect();
        let game_borrow = unsafe { game.as_mut() }.map(|g| &mut g.inner);
        let mut ctx = DevCtx { game: game_borrow };
        let (was_error, response) = match cmd.execute(&args, &mut ctx) {
            Ok(msg) => (false, msg),
            Err(err) => (true, err),
        };
        unsafe {
            *out_was_error = was_error;
        }
        copy_str_with_nul(&response, out_response, cap, out_required)
    })
}

/// Push `line` into the FFI-side history vec, dropping the oldest entry when
/// at capacity. Mirrors the lib's `state::push_history` (including the
/// "skip duplicate of previous line" rule).
fn push_history(history: &mut Vec<String>, line: String) {
    if history.last().map(String::as_str) == Some(line.as_str()) {
        return;
    }
    if history.len() == HISTORY_CAPACITY {
        history.remove(0);
    }
    history.push(line);
}

// ---- History (Pattern C) --------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn console_history_count(
    console: *const ConsoleHandle,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let h = console_ref_or_null!(console);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_count = h.history.len();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn console_history_line_copy(
    console: *const ConsoleHandle,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let h = console_ref_or_null!(console);
        let Some(line) = h.history.get(index) else {
            set_last_error(format!("index {index} >= len {}", h.history.len()));
            return FfiError::OutOfRange as i32;
        };
        copy_str_with_nul(line, buf, cap, out_required)
    })
}

// ---- Completions ----------------------------------------------------------

/// Recompute the cached completion view from the current input. `game` may
/// be null — completion candidates that consult game state simply receive
/// `None` for the [`CompletionCtx`], matching the lib's contract.
#[unsafe(no_mangle)]
pub extern "C" fn console_complete(
    console: *mut ConsoleHandle,
    game: *const GameHandle,
) -> i32 {
    ffi_try!({
        let h = console_mut_or_null!(console);
        let game_ref = unsafe { game.as_ref() }.map(|g| &g.inner);
        h.inner.refresh_completion(game_ref);
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn console_completion_count(
    console: *const ConsoleHandle,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let h = console_ref_or_null!(console);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_count = h.inner.completion().candidates.len();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn console_completion_at(
    console: *const ConsoleHandle,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let h = console_ref_or_null!(console);
        let candidates = &h.inner.completion().candidates;
        let Some(line) = candidates.get(index) else {
            set_last_error(format!("index {index} >= len {}", candidates.len()));
            return FfiError::OutOfRange as i32;
        };
        copy_str_with_nul(line, buf, cap, out_required)
    })
}

// ---- Static command introspection ----------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn console_command_count(out_count: *mut usize) -> i32 {
    ffi_try!({
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_count = registry().len();
        }
        FfiError::Ok as i32
    })
}

/// Resolve the `index`-th registry entry, recording an out-of-range detail.
fn command_or_out_of_range(
    index: usize,
) -> Result<&'static dyn adventerm_lib::console::DevCommand, i32> {
    let cmds = registry();
    match cmds.get(index) {
        Some(&c) => Ok(c),
        None => {
            set_last_error(format!("index {index} >= len {}", cmds.len()));
            Err(FfiError::OutOfRange as i32)
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn console_command_name(
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let cmd = match command_or_out_of_range(index) {
            Ok(c) => c,
            Err(rc) => return rc,
        };
        copy_str_with_nul(cmd.name(), buf, cap, out_required)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn console_command_help(
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let cmd = match command_or_out_of_range(index) {
            Ok(c) => c,
            Err(rc) => return rc,
        };
        copy_str_with_nul(cmd.help(), buf, cap, out_required)
    })
}
