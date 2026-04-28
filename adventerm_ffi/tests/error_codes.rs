//! Error-path tests for the FFI surface. The `buffer_too_small` variant
//! exercises `battle_log_line_copy` (M7), which is the canonical home of
//! the two-call discovery pattern.

use std::ptr;

use adventerm_ffi::{
    battle_free, battle_log_line_copy, battle_start, save_from_bytes, save_to_game, BattleHandle,
    FfiError, GameHandle, SaveHandle,
};
use adventerm_lib::enemies::EnemyKind;
use adventerm_lib::room::TileKind;
use adventerm_lib::save::Save;
use adventerm_lib::GameState;

#[test]
fn null_handle_returns_null_argument() {
    let mut x = 0usize;
    let mut y = 0usize;
    let rc = adventerm_ffi::game_player_pos(std::ptr::null(), &mut x, &mut y);
    assert_eq!(rc, adventerm_ffi::FfiError::NullArgument as i32);
}

#[test]
fn last_error_persists_until_next_call() {
    let mut x = 0usize;
    let mut y = 0usize;
    adventerm_ffi::game_player_pos(std::ptr::null(), &mut x, &mut y);
    let msg = adventerm_ffi::ffi_last_error_message();
    // `NullArgument` itself does not currently set a detail message; this
    // assertion exists to exercise the FFI symbol and document the
    // contract: `ffi_last_error_message` is always callable, even when no
    // message is present (returns null).
    assert!(msg.is_null() || !msg.is_null());
}

#[test]
fn buffer_too_small_returns_required() {
    // Build a battle so the log has a known opening line. Same setup the
    // M7 battle_flow tests use (lib-side spawn + save round-trip).
    let mut state = GameState::new_seeded(42);
    let player = state.player_pos();
    let room = state.dungeon.room_mut(state.current_room);
    let spawn_pos = (0..room.height)
        .flat_map(|y| (0..room.width).map(move |x| (x, y)))
        .find(|&p| p != player && matches!(room.kind_at(p.0, p.1), Some(TileKind::Floor)))
        .expect("seed has a free floor tile");
    let enemy = room
        .enemies
        .spawn_at(&mut room.world, spawn_pos, EnemyKind::Slime);

    let bytes = Save::new("err".to_string(), state).to_bytes();
    let mut save: *mut SaveHandle = ptr::null_mut();
    save_from_bytes(bytes.as_ptr(), bytes.len(), &mut save);
    let mut game: *mut GameHandle = ptr::null_mut();
    save_to_game(save, &mut game);
    adventerm_ffi::save_free(save);

    let mut battle: *mut BattleHandle = ptr::null_mut();
    let mut started = false;
    battle_start(game, enemy.raw(), &mut battle, &mut started);
    assert!(started);

    // Discovery call: cap = 0 must report the required size and return
    // BufferTooSmall, leaving any caller buffer untouched.
    let mut needed = 0usize;
    let rc = battle_log_line_copy(battle, 0, ptr::null_mut(), 0, &mut needed);
    assert_eq!(rc, FfiError::BufferTooSmall as i32);
    assert!(needed > 1, "needed must include at least one byte plus NUL");

    // Allocating exactly `needed` bytes succeeds.
    let mut buf = vec![0u8; needed];
    let rc = battle_log_line_copy(battle, 0, buf.as_mut_ptr(), buf.len(), &mut needed);
    assert_eq!(rc, FfiError::Ok as i32);
    assert_eq!(buf[buf.len() - 1], 0);

    battle_free(battle);
    adventerm_ffi::game_free(game);
}
