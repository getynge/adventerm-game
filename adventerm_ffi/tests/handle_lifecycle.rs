//! Lifecycle tests for [`adventerm_ffi::GameHandle`]. Verifies the alloc/free
//! pair, null-safety on free, and a successful round-trip through
//! `game_player_pos`.

#[test]
fn game_alloc_free_no_leak() {
    let handle = adventerm_ffi::game_new_seeded(42);
    assert!(!handle.is_null());
    adventerm_ffi::game_free(handle);
}

#[test]
fn game_free_null_is_safe() {
    adventerm_ffi::game_free(std::ptr::null_mut());
}

#[test]
fn game_player_pos_round_trip() {
    let handle = adventerm_ffi::game_new_seeded(42);
    let mut x = 0usize;
    let mut y = 0usize;
    let rc = adventerm_ffi::game_player_pos(handle, &mut x, &mut y);
    assert_eq!(rc, 0);
    adventerm_ffi::game_free(handle);
}
