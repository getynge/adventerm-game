//! Lifecycle and scalar-query exports for [`GameHandle`].
//!
//! `game_new_seeded` is the only constructor that can't fail (and so doesn't
//! follow the out-pointer + `i32` convention). Every other entry point in
//! this module is wrapped in [`crate::ffi_try!`] and returns an
//! [`FfiError`](crate::error::FfiError) code.

use adventerm_lib::GameState;

use crate::error::FfiError;
use crate::ffi_try;
use crate::handle::GameHandle;

/// Construct a fresh game from `seed`. Returns a non-null handle on success;
/// pair every successful call with exactly one [`game_free`].
#[unsafe(no_mangle)]
pub extern "C" fn game_new_seeded(seed: u64) -> *mut GameHandle {
    Box::into_raw(Box::new(GameHandle {
        inner: GameState::new_seeded(seed),
    }))
}

/// Free a handle previously returned by [`game_new_seeded`]. A null pointer
/// is a no-op (matches C's `free(NULL)` convention).
#[unsafe(no_mangle)]
pub extern "C" fn game_free(handle: *mut GameHandle) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn game_player_pos(
    handle: *const GameHandle,
    out_x: *mut usize,
    out_y: *mut usize,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_x.is_null() || out_y.is_null() {
            return FfiError::NullArgument as i32;
        }
        let (x, y) = h.inner.player_pos();
        unsafe {
            *out_x = x;
            *out_y = y;
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_cur_health(handle: *const GameHandle, out_hp: *mut u8) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_hp.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_hp = h.inner.cur_health();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_set_cur_health(handle: *mut GameHandle, hp: u8) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_mut() }) else {
            return FfiError::NullArgument as i32;
        };
        h.inner.set_cur_health(hp);
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_vision_radius(
    handle: *const GameHandle,
    out_radius: *mut usize,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_radius.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_radius = h.inner.vision_radius();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_refresh_visibility(handle: *mut GameHandle) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_mut() }) else {
            return FfiError::NullArgument as i32;
        };
        h.inner.refresh_visibility();
        FfiError::Ok as i32
    })
}
