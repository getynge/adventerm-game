//! Iteration accessors for collection-shaped state on a [`GameHandle`].
//!
//! Three patterns, picked per accessor by element shape:
//!
//! * **Pattern A — copy-element slice:** single-call copy-into-buffer for
//!   slices of `Copy` discriminants ([`game_inventory_copy`], the ability
//!   slots).
//! * **Pattern B — count + index:** `_count` plus `_at(idx, ...)` for
//!   heterogeneous compounds ([`room_door_at`], [`room_enemy_at`], ...).
//! * **Room introspection:** scalar reads on the current or named room
//!   ([`game_current_room`], [`game_room_dimensions`], ...).
//!
//! Iterators never cross the FFI boundary. Each `_count` call snapshots the
//! current shape; indices are valid until the next mutation on the handle.

use adventerm_lib::abilities::{ABILITY_SLOTS, PASSIVE_SLOTS};
use adventerm_lib::dungeon::DoorView;
use adventerm_lib::room::RoomId;

use crate::enums::{CAbilityKind, CItemKind};
use crate::error::{set_last_error, FfiError};
use crate::ffi_try;
use crate::handle::GameHandle;
use crate::structs::{
    CDoorView, CEnemyView, CEquipmentSnapshot, CFlareSource, CLightSource, CTileKind,
};

/// Sentinel byte written into ability-slot copy buffers when a slot is empty.
/// Matches the equipment empty sentinel and is unreachable as an `AbilityKind`
/// discriminant.
const C_ABILITY_SLOT_EMPTY: u8 = u8::MAX;

/// Resolve `handle` to a shared reference or short-circuit with `NullArgument`.
macro_rules! handle_ref_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_ref() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        }
    };
}

/// Copy a slice of `Copy` discriminants into `out_buf` using the
/// "two-call discovery" pattern: always populate `out_required` (so callers
/// can re-allocate) before checking the buffer.
fn copy_discriminants(
    src: &[u8],
    out_buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    if !out_required.is_null() {
        unsafe {
            *out_required = src.len();
        }
    }
    if out_buf.is_null() || cap < src.len() {
        return FfiError::BufferTooSmall as i32;
    }
    for (i, &byte) in src.iter().enumerate() {
        unsafe {
            *out_buf.add(i) = byte;
        }
    }
    FfiError::Ok as i32
}

/// Resolve a `room: u32` parameter to a `&Room`, or return [`OutOfRange`]
/// after recording the bad index.
fn room_or_out_of_range<'a>(
    h: &'a GameHandle,
    room: u32,
) -> Result<&'a adventerm_lib::Room, i32> {
    let count = h.inner.dungeon.rooms.len();
    let idx = room as usize;
    if idx >= count {
        set_last_error(format!("room id {room} >= rooms.len() {count}"));
        return Err(FfiError::OutOfRange as i32);
    }
    Ok(h.inner.dungeon.room(RoomId(room)))
}

/// Bounds-check an index for a Pattern B `_at(idx, ...)` accessor; record a
/// detail message and return [`OutOfRange`] when out of range.
fn check_index(idx: usize, len: usize) -> Result<(), i32> {
    if idx >= len {
        set_last_error(format!("index {idx} >= len {len}"));
        Err(FfiError::OutOfRange as i32)
    } else {
        Ok(())
    }
}

// ---- Pattern A — copy-element slices --------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn game_inventory_len(
    handle: *const GameHandle,
    out_len: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_len.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_len = h.inner.inventory().len();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_inventory_copy(
    handle: *const GameHandle,
    out_buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        let inv = h.inner.inventory();
        let bytes: Vec<u8> = inv.iter().map(|&k| CItemKind::from(k) as u8).collect();
        copy_discriminants(&bytes, out_buf, cap, out_required)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_abilities_active_copy(
    handle: *const GameHandle,
    out_buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        let bytes: [u8; ABILITY_SLOTS] = {
            let mut buf = [C_ABILITY_SLOT_EMPTY; ABILITY_SLOTS];
            for (i, slot) in h.inner.abilities().active_slots.iter().enumerate() {
                if let Some(kind) = slot {
                    buf[i] = CAbilityKind::from(*kind) as u8;
                }
            }
            buf
        };
        copy_discriminants(&bytes, out_buf, cap, out_required)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_abilities_passive_copy(
    handle: *const GameHandle,
    out_buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        // `PassiveKind` is uninhabited today, so every slot reads as empty.
        // The byte layout is still produced so callers can size buffers from
        // the `PASSIVE_SLOTS` constant without special-casing.
        let mut buf = [C_ABILITY_SLOT_EMPTY; PASSIVE_SLOTS];
        for (i, slot) in h.inner.abilities().passive_slots.iter().enumerate() {
            if slot.is_some() {
                buf[i] = 0;
            }
        }
        copy_discriminants(&buf, out_buf, cap, out_required)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_abilities_learned_active_len(
    handle: *const GameHandle,
    out_len: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_len.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_len = h.inner.abilities().learned_active.len();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_abilities_learned_active_copy(
    handle: *const GameHandle,
    out_buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        let bytes: Vec<u8> = h
            .inner
            .abilities()
            .learned_active
            .iter()
            .map(|&k| CAbilityKind::from(k) as u8)
            .collect();
        copy_discriminants(&bytes, out_buf, cap, out_required)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_abilities_learned_passive_len(
    handle: *const GameHandle,
    out_len: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_len.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_len = h.inner.abilities().learned_passive.len();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_abilities_learned_passive_copy(
    handle: *const GameHandle,
    out_buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        // `learned_passive` is `Vec<PassiveKind>` and `PassiveKind` is
        // uninhabited; the resulting byte slice is always empty.
        let len = h.inner.abilities().learned_passive.len();
        copy_discriminants(&vec![0u8; len], out_buf, cap, out_required)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_equipment_snapshot(
    handle: *const GameHandle,
    out: *mut CEquipmentSnapshot,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out = CEquipmentSnapshot::from(h.inner.equipment());
        }
        FfiError::Ok as i32
    })
}

// ---- Pattern B — count + index over the current room ----------------------

/// Snapshot the current room's doors as `DoorView`s. Performed in one pass
/// so the index-based accessor below pairs with the count from `_count`.
fn current_room_doors(h: &GameHandle) -> Vec<DoorView> {
    let room = h.inner.current_room();
    room.doors()
        .filter_map(|(_, _, id)| h.inner.dungeon.door_view(id))
        .collect()
}

#[unsafe(no_mangle)]
pub extern "C" fn room_doors_count(
    handle: *const GameHandle,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_count = current_room_doors(h).len();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_door_at(
    handle: *const GameHandle,
    index: usize,
    out_view: *mut CDoorView,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_view.is_null() {
            return FfiError::NullArgument as i32;
        }
        let doors = current_room_doors(h);
        if let Err(code) = check_index(index, doors.len()) {
            return code;
        }
        unsafe {
            *out_view = CDoorView::from(doors[index]);
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_enemies_count(
    handle: *const GameHandle,
    room: u32,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        let r = match room_or_out_of_range(h, room) {
            Ok(r) => r,
            Err(code) => return code,
        };
        unsafe {
            *out_count = r.enemies.iter_with_pos(&r.world).count();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_enemy_at(
    handle: *const GameHandle,
    room: u32,
    index: usize,
    out_view: *mut CEnemyView,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_view.is_null() {
            return FfiError::NullArgument as i32;
        }
        let r = match room_or_out_of_range(h, room) {
            Ok(r) => r,
            Err(code) => return code,
        };
        let snapshot: Vec<CEnemyView> = r
            .enemies
            .iter_with_pos(&r.world)
            .map(|(e, pos, kind)| CEnemyView::new(e, pos, kind))
            .collect();
        if let Err(code) = check_index(index, snapshot.len()) {
            return code;
        }
        unsafe {
            *out_view = snapshot[index];
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_lights_count(
    handle: *const GameHandle,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        let room = h.inner.current_room();
        unsafe {
            *out_count = room.lighting.iter_sources(&room.world).count();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_light_at(
    handle: *const GameHandle,
    index: usize,
    out_view: *mut CLightSource,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_view.is_null() {
            return FfiError::NullArgument as i32;
        }
        let room = h.inner.current_room();
        // Lib's `iter_sources` does not yield entity ids; entity field is 0.
        let snapshot: Vec<CLightSource> = room
            .lighting
            .iter_sources(&room.world)
            .map(|(pos, src)| CLightSource::new(adventerm_lib::EntityId::from_raw(0), pos, src))
            .collect();
        if let Err(code) = check_index(index, snapshot.len()) {
            return code;
        }
        unsafe {
            *out_view = snapshot[index];
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_flares_count(
    handle: *const GameHandle,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        let room = h.inner.current_room();
        unsafe {
            *out_count = room.lighting.iter_flares(&room.world).count();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_flare_at(
    handle: *const GameHandle,
    index: usize,
    out_view: *mut CFlareSource,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_view.is_null() {
            return FfiError::NullArgument as i32;
        }
        let room = h.inner.current_room();
        // Lib's `iter_flares` does not yield entity ids; entity field is 0.
        let snapshot: Vec<CFlareSource> = room
            .lighting
            .iter_flares(&room.world)
            .map(|(pos, src)| CFlareSource::new(adventerm_lib::EntityId::from_raw(0), pos, src))
            .collect();
        if let Err(code) = check_index(index, snapshot.len()) {
            return code;
        }
        unsafe {
            *out_view = snapshot[index];
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_items_at_count(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        let room = h.inner.current_room();
        unsafe {
            *out_count = room.items_at((x, y)).count();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn room_item_at(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    index: usize,
    out_kind: *mut u8,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_kind.is_null() {
            return FfiError::NullArgument as i32;
        }
        let room = h.inner.current_room();
        let snapshot: Vec<u8> = room
            .items_at((x, y))
            .map(|k| CItemKind::from(k) as u8)
            .collect();
        if let Err(code) = check_index(index, snapshot.len()) {
            return code;
        }
        unsafe {
            *out_kind = snapshot[index];
        }
        FfiError::Ok as i32
    })
}

// ---- Room introspection ---------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn game_current_room(
    handle: *const GameHandle,
    out_room: *mut u32,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_room.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_room = h.inner.current_room.0;
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_room_dimensions(
    handle: *const GameHandle,
    room: u32,
    out_width: *mut usize,
    out_height: *mut usize,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_width.is_null() || out_height.is_null() {
            return FfiError::NullArgument as i32;
        }
        let r = match room_or_out_of_range(h, room) {
            Ok(r) => r,
            Err(code) => return code,
        };
        unsafe {
            *out_width = r.width;
            *out_height = r.height;
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_room_kind_at(
    handle: *const GameHandle,
    room: u32,
    x: usize,
    y: usize,
    out_kind: *mut CTileKind,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out_kind.is_null() {
            return FfiError::NullArgument as i32;
        }
        let r = match room_or_out_of_range(h, room) {
            Ok(r) => r,
            Err(code) => return code,
        };
        let kind = match r.kind_at(x, y) {
            Some(k) => k,
            None => {
                set_last_error(format!(
                    "tile ({x}, {y}) out of bounds for room of size {}x{}",
                    r.width, r.height
                ));
                return FfiError::OutOfRange as i32;
            }
        };
        unsafe {
            *out_kind = CTileKind::from(kind);
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_room_walkable(
    handle: *const GameHandle,
    room: u32,
    x: usize,
    y: usize,
    out: *mut bool,
) -> i32 {
    ffi_try!({
        let h = handle_ref_or_null!(handle);
        if out.is_null() {
            return FfiError::NullArgument as i32;
        }
        let r = match room_or_out_of_range(h, room) {
            Ok(r) => r,
            Err(code) => return code,
        };
        unsafe {
            *out = r.is_walkable(x, y);
        }
        FfiError::Ok as i32
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structs::C_EQUIPMENT_SLOT_EMPTY;

    #[test]
    fn ability_slot_empty_sentinel_matches_equipment_sentinel() {
        // Both share the same byte so iOS callers only need one constant.
        assert_eq!(C_ABILITY_SLOT_EMPTY, C_EQUIPMENT_SLOT_EMPTY);
    }
}
