//! Battle subsystem FFI surface.
//!
//! [`BattleHandle`] is an independent owner: `battle_start` snapshots the
//! state it needs from a [`GameHandle`] (via the lib's `start_battle`) and
//! never borrows back. Free order between the two handles does not matter.
//!
//! Engine functions take `&GameState` — both `battle_apply_player_ability`
//! and `battle_apply_enemy_turn` therefore take `*const GameHandle`.

use adventerm_lib::battle::{apply_enemy_turn, apply_player_ability, start_battle};
use adventerm_lib::ecs::EntityId;

use crate::enums::CBattleResult;
use crate::error::{set_last_error, FfiError};
use crate::ffi_try;
use crate::handle::{BattleHandle, GameHandle};
use crate::structs::{CBattleTurn, CCombatants, CHpSnapshot};

/// Resolve `*const GameHandle` or short-circuit with `NullArgument`.
macro_rules! game_ref_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_ref() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        }
    };
}

/// Resolve `*const BattleHandle` or short-circuit with `NullArgument`.
macro_rules! battle_ref_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_ref() } {
            Some(b) => b,
            None => return FfiError::NullArgument as i32,
        }
    };
}

/// Resolve `*mut BattleHandle` or short-circuit with `NullArgument`.
macro_rules! battle_mut_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_mut() } {
            Some(b) => b,
            None => return FfiError::NullArgument as i32,
        }
    };
}

// ---- Lifecycle ------------------------------------------------------------

/// Try to construct a [`BattleHandle`] for `enemy_entity` in `game`'s current
/// room. Writes the new handle and `out_started=true` on success. When the
/// engine declines (no such enemy in the current room), writes `null` and
/// `out_started=false` and returns `Ok` — the spec models "no battle could
/// start" as a successful outcome, not an error.
#[unsafe(no_mangle)]
pub extern "C" fn battle_start(
    game: *const GameHandle,
    enemy_entity: u32,
    out_battle: *mut *mut BattleHandle,
    out_started: *mut bool,
) -> i32 {
    ffi_try!({
        if out_battle.is_null() || out_started.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_battle = std::ptr::null_mut();
            *out_started = false;
        }
        let h = game_ref_or_null!(game);
        match start_battle(&h.inner, EntityId::from_raw(enemy_entity)) {
            Some(battle) => unsafe {
                *out_battle = Box::into_raw(Box::new(BattleHandle { inner: battle }));
                *out_started = true;
            },
            None => {} // already populated null/false above
        }
        FfiError::Ok as i32
    })
}

/// Free a handle previously returned by [`battle_start`]. Null is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn battle_free(battle: *mut BattleHandle) {
    if !battle.is_null() {
        unsafe {
            drop(Box::from_raw(battle));
        }
    }
}

// ---- Turn dispatch --------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn battle_apply_player_ability(
    game: *const GameHandle,
    battle: *mut BattleHandle,
    ability_slot: usize,
) -> i32 {
    ffi_try!({
        let g = game_ref_or_null!(game);
        let b = battle_mut_or_null!(battle);
        match apply_player_ability(&g.inner, &mut b.inner, ability_slot) {
            Ok(()) => FfiError::Ok as i32,
            Err(e) => FfiError::from(e) as i32,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn battle_apply_enemy_turn(
    game: *const GameHandle,
    battle: *mut BattleHandle,
) -> i32 {
    ffi_try!({
        let g = game_ref_or_null!(game);
        let b = battle_mut_or_null!(battle);
        apply_enemy_turn(&g.inner, &mut b.inner);
        FfiError::Ok as i32
    })
}

// ---- State queries --------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn battle_turn(
    battle: *const BattleHandle,
    out_turn: *mut CBattleTurn,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        if out_turn.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_turn = CBattleTurn::from(b.inner.turn());
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn battle_combatants(
    battle: *const BattleHandle,
    out_combatants: *mut CCombatants,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        if out_combatants.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_combatants = CCombatants::from(b.inner.combatants());
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn battle_player_cur_hp(
    battle: *const BattleHandle,
    out_hp: *mut u8,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        if out_hp.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_hp = b.inner.player_cur_hp();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn battle_enemy_cur_hp(
    battle: *const BattleHandle,
    out_hp: *mut u8,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        if out_hp.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_hp = b.inner.enemy_cur_hp();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn battle_hp_snapshot(
    battle: *const BattleHandle,
    out_hp: *mut CHpSnapshot,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        if out_hp.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_hp = CHpSnapshot::from(b.inner.hp());
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn battle_is_resolved(
    battle: *const BattleHandle,
    out: *mut bool,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        if out.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out = b.inner.is_resolved();
        }
        FfiError::Ok as i32
    })
}

/// Read the terminal result. `out_has_result` is true only when the battle
/// is resolved; otherwise `out_result` is left at zero and callers should
/// ignore it.
#[unsafe(no_mangle)]
pub extern "C" fn battle_result(
    battle: *const BattleHandle,
    out_result: *mut u8,
    out_has_result: *mut bool,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        if out_result.is_null() || out_has_result.is_null() {
            return FfiError::NullArgument as i32;
        }
        match b.inner.result() {
            Some(r) => unsafe {
                *out_result = CBattleResult::from(r) as u8;
                *out_has_result = true;
            },
            None => unsafe {
                *out_result = 0;
                *out_has_result = false;
            },
        }
        FfiError::Ok as i32
    })
}

// ---- Log access (Pattern C: count + line_copy) ----------------------------

#[unsafe(no_mangle)]
pub extern "C" fn battle_log_line_count(
    battle: *const BattleHandle,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_count = b.inner.log().len();
        }
        FfiError::Ok as i32
    })
}

/// Two-call discovery copy of log line `index` into a caller-allocated
/// buffer. With `buf == null` (or `cap` smaller than the required size),
/// returns [`FfiError::BufferTooSmall`] after writing the needed byte count
/// (including the trailing NUL) into `out_required`.
#[unsafe(no_mangle)]
pub extern "C" fn battle_log_line_copy(
    battle: *const BattleHandle,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let b = battle_ref_or_null!(battle);
        let lines = b.inner.log();
        let line = match lines.get(index) {
            Some(l) => l,
            None => {
                set_last_error(format!("index {index} >= len {}", lines.len()));
                return FfiError::OutOfRange as i32;
            }
        };
        let bytes = line.as_bytes();
        let needed = bytes.len() + 1; // +1 for NUL terminator
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
    })
}
