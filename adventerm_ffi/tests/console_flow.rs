//! M8 — developer console FFI integration tests.
//!
//! Exercises the round-trip: build input via `console_input_set` /
//! `console_insert_char`, dispatch via `console_submit`, and read back
//! state through the history/completion accessors. Covers both happy-path
//! (`fullbright` mutates `game_fullbright`) and error-path (unknown command
//! sets `was_error`, registry introspection).

use std::ffi::CString;
use std::ptr;

use adventerm_ffi::{
    console_clear, console_command_count, console_command_help, console_command_name,
    console_complete, console_completion_at, console_completion_count, console_cursor,
    console_delete_back, console_free, console_history_count, console_history_line_copy,
    console_input_get, console_input_set, console_insert_char, console_new, console_set_cursor,
    console_submit, game_free, game_fullbright, game_new_seeded, ConsoleHandle, FfiError,
};

const SEED: u64 = 42;
const RESPONSE_CAP: usize = 256;
const NAME_CAP: usize = 64;

/// Construct a console handle, asserting the export wrote a non-null pointer.
fn make_console() -> *mut ConsoleHandle {
    let mut console: *mut ConsoleHandle = ptr::null_mut();
    let rc = console_new(&mut console);
    assert_eq!(rc, FfiError::Ok as i32);
    assert!(!console.is_null());
    console
}

#[test]
fn submit_fullbright_flips_game_state() {
    let game = game_new_seeded(SEED);
    let console = make_console();

    let cmd = CString::new("fullbright").unwrap();
    assert_eq!(console_input_set(console, cmd.as_ptr()), FfiError::Ok as i32);

    let mut response = vec![0u8; RESPONSE_CAP];
    let mut needed = 0usize;
    let mut was_error = true;
    let rc = console_submit(
        console,
        game,
        response.as_mut_ptr(),
        response.len(),
        &mut needed,
        &mut was_error,
    );
    assert_eq!(rc, FfiError::Ok as i32);
    assert!(!was_error, "fullbright should not produce an error");

    let mut on = false;
    assert_eq!(game_fullbright(game, &mut on), FfiError::Ok as i32);
    assert!(on, "fullbright command should toggle the override on");

    console_free(console);
    game_free(game);
}

#[test]
fn submit_unknown_command_sets_was_error() {
    let game = game_new_seeded(SEED);
    let console = make_console();

    let cmd = CString::new("asdf").unwrap();
    console_input_set(console, cmd.as_ptr());

    let mut response = vec![0u8; RESPONSE_CAP];
    let mut needed = 0usize;
    let mut was_error = false;
    let rc = console_submit(
        console,
        game,
        response.as_mut_ptr(),
        response.len(),
        &mut needed,
        &mut was_error,
    );
    assert_eq!(rc, FfiError::Ok as i32);
    assert!(was_error, "unknown commands set was_error=true");

    let nul = response.iter().position(|&b| b == 0).expect("NUL terminator");
    let response = std::str::from_utf8(&response[..nul]).unwrap();
    assert!(
        response.contains("unknown command"),
        "response should describe the failure, got {response:?}"
    );

    console_free(console);
    game_free(game);
}

#[test]
fn submit_empty_input_is_noop() {
    let game = game_new_seeded(SEED);
    let console = make_console();

    let mut response = vec![0u8; RESPONSE_CAP];
    let mut needed = 0usize;
    let mut was_error = true;
    let rc = console_submit(
        console,
        game,
        response.as_mut_ptr(),
        response.len(),
        &mut needed,
        &mut was_error,
    );
    assert_eq!(rc, FfiError::Ok as i32);
    assert!(!was_error);

    let mut history = 0usize;
    console_history_count(console, &mut history);
    assert_eq!(history, 0, "empty submit must not record history");

    console_free(console);
    game_free(game);
}

#[test]
fn complete_lists_known_commands() {
    let console = make_console();

    let cmd = CString::new("ful").unwrap();
    console_input_set(console, cmd.as_ptr());

    assert_eq!(console_complete(console, ptr::null()), FfiError::Ok as i32);

    let mut count = 0usize;
    console_completion_count(console, &mut count);
    assert!(count >= 1, "expected at least one candidate, got {count}");

    let mut needed = 0usize;
    console_completion_at(console, 0, ptr::null_mut(), 0, &mut needed);
    let mut buf = vec![0u8; needed];
    console_completion_at(console, 0, buf.as_mut_ptr(), buf.len(), &mut needed);
    let nul = buf.iter().position(|&b| b == 0).unwrap();
    let candidate = std::str::from_utf8(&buf[..nul]).unwrap();
    assert_eq!(candidate, "fullbright");

    console_free(console);
}

#[test]
fn command_registry_introspection() {
    let mut count = 0usize;
    assert_eq!(console_command_count(&mut count), FfiError::Ok as i32);
    assert!(count >= 3, "registry must expose fullbright, spawn, give");

    let mut name_buf = vec![0u8; NAME_CAP];
    let mut needed = 0usize;
    assert_eq!(
        console_command_name(0, name_buf.as_mut_ptr(), name_buf.len(), &mut needed),
        FfiError::Ok as i32
    );
    let nul = name_buf.iter().position(|&b| b == 0).unwrap();
    let name = std::str::from_utf8(&name_buf[..nul]).unwrap();
    assert!(matches!(name, "fullbright" | "spawn" | "give"));

    let mut help_buf = vec![0u8; 256];
    assert_eq!(
        console_command_help(0, help_buf.as_mut_ptr(), help_buf.len(), &mut needed),
        FfiError::Ok as i32
    );
}

#[test]
fn out_of_range_index_reports_error() {
    let console = make_console();

    let mut buf = vec![0u8; 32];
    let mut needed = 0usize;
    let rc = console_history_line_copy(console, 99, buf.as_mut_ptr(), buf.len(), &mut needed);
    assert_eq!(rc, FfiError::OutOfRange as i32);

    let rc = console_completion_at(console, 99, buf.as_mut_ptr(), buf.len(), &mut needed);
    assert_eq!(rc, FfiError::OutOfRange as i32);

    let rc = console_command_name(99, buf.as_mut_ptr(), buf.len(), &mut needed);
    assert_eq!(rc, FfiError::OutOfRange as i32);

    console_free(console);
}

#[test]
fn input_editing_round_trips() {
    let console = make_console();

    // Insert "ful" via per-character API.
    for c in "ful".chars() {
        assert_eq!(
            console_insert_char(console, c as u32),
            FfiError::Ok as i32
        );
    }

    // Read back the buffer using two-call discovery.
    let mut needed = 0usize;
    console_input_get(console, ptr::null_mut(), 0, &mut needed);
    let mut buf = vec![0u8; needed];
    console_input_get(console, buf.as_mut_ptr(), buf.len(), &mut needed);
    let nul = buf.iter().position(|&b| b == 0).unwrap();
    assert_eq!(std::str::from_utf8(&buf[..nul]).unwrap(), "ful");

    let mut cursor = 0usize;
    console_cursor(console, &mut cursor);
    assert_eq!(cursor, 3);

    // Delete one char, then move cursor to start.
    assert_eq!(console_delete_back(console), FfiError::Ok as i32);
    assert_eq!(console_set_cursor(console, 0), FfiError::Ok as i32);

    // Surrogate codepoints are rejected.
    let rc = console_insert_char(console, 0xD800);
    assert_eq!(rc, FfiError::OutOfRange as i32);

    // Clear wipes the buffer back to empty.
    assert_eq!(console_clear(console), FfiError::Ok as i32);
    console_input_get(console, ptr::null_mut(), 0, &mut needed);
    assert_eq!(needed, 1, "cleared buffer should require only the NUL byte");

    console_free(console);
}

#[test]
fn submit_records_history() {
    let game = game_new_seeded(SEED);
    let console = make_console();

    let cmd = CString::new("fullbright").unwrap();
    console_input_set(console, cmd.as_ptr());

    let mut response = vec![0u8; RESPONSE_CAP];
    let mut needed = 0usize;
    let mut was_error = true;
    console_submit(
        console,
        game,
        response.as_mut_ptr(),
        response.len(),
        &mut needed,
        &mut was_error,
    );

    let mut history = 0usize;
    console_history_count(console, &mut history);
    assert_eq!(history, 1);

    let mut buf = vec![0u8; 64];
    let rc = console_history_line_copy(console, 0, buf.as_mut_ptr(), buf.len(), &mut needed);
    assert_eq!(rc, FfiError::Ok as i32);
    let nul = buf.iter().position(|&b| b == 0).unwrap();
    assert_eq!(std::str::from_utf8(&buf[..nul]).unwrap(), "fullbright");

    console_free(console);
    game_free(game);
}

#[test]
fn console_free_null_is_safe() {
    console_free(ptr::null_mut());
}
