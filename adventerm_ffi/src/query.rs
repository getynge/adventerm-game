//! Scalar read-only queries plus the static name-lookup table.
//!
//! Every entry that touches a handle uses the `i32` + out-pointer
//! convention; the static lookups return `*const c_char` (or write through
//! an out-pointer for the glyph helpers) and never fail.

use std::collections::HashMap;
use std::ffi::{c_char, CString};
use std::sync::OnceLock;

use adventerm_lib::abilities::AbilityKind;
use adventerm_lib::enemies::EnemyKind;
use adventerm_lib::ecs::EntityId;
use adventerm_lib::items::EquipSlot;
use adventerm_lib::stats::Attribute;
use adventerm_lib::ItemKind;

use crate::enums::{
    CAbilityKind, CAttribute, CEnemyKind, CEquipSlot, CItemKind, CTile,
};
use crate::error::FfiError;
use crate::ffi_try;
use crate::handle::GameHandle;
use crate::structs::CStats;

const ATTRIBUTE_KINDS: [Attribute; 5] = [
    Attribute::Fire,
    Attribute::Water,
    Attribute::Earth,
    Attribute::Light,
    Attribute::Dark,
];

const ENEMY_KINDS: [EnemyKind; 1] = [EnemyKind::Slime];

const ABILITY_KINDS: [AbilityKind; 2] = [AbilityKind::Impact, AbilityKind::Fireball];

#[unsafe(no_mangle)]
pub extern "C" fn game_tile_at(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out_tile: *mut u8,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_tile.is_null() {
            return FfiError::NullArgument as i32;
        }
        let tile = h.inner.tile_at(x, y);
        unsafe {
            *out_tile = CTile::from(tile) as u8;
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_terrain_at(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out_tile: *mut u8,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_tile.is_null() {
            return FfiError::NullArgument as i32;
        }
        let tile = h.inner.terrain_at(x, y);
        unsafe {
            *out_tile = CTile::from(tile) as u8;
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_is_visible(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out: *mut bool,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out = h.inner.is_visible(x, y);
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_is_explored(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out: *mut bool,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out = h.inner.is_explored(x, y);
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_player_on_door(
    handle: *const GameHandle,
    out_door_id: *mut u32,
    out_has_door: *mut bool,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_door_id.is_null() || out_has_door.is_null() {
            return FfiError::NullArgument as i32;
        }
        match h.inner.player_on_door() {
            Some(id) => unsafe {
                *out_door_id = id.0.raw();
                *out_has_door = true;
            },
            None => unsafe {
                *out_door_id = 0;
                *out_has_door = false;
            },
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_items_here(handle: *const GameHandle, out: *mut bool) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out = h.inner.items_here();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_peek_item_here(
    handle: *const GameHandle,
    out_kind: *mut u8,
    out_has_item: *mut bool,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_kind.is_null() || out_has_item.is_null() {
            return FfiError::NullArgument as i32;
        }
        match h.inner.peek_item_here() {
            Some(kind) => unsafe {
                *out_kind = CItemKind::from(kind) as u8;
                *out_has_item = true;
            },
            None => unsafe {
                *out_kind = 0;
                *out_has_item = false;
            },
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_effective_stats(
    handle: *const GameHandle,
    out_stats: *mut CStats,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_stats.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out_stats = CStats::from(h.inner.effective_stats());
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_set_fullbright(handle: *mut GameHandle, on: bool) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_mut() }) else {
            return FfiError::NullArgument as i32;
        };
        h.inner.set_fullbright(on);
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_fullbright(handle: *const GameHandle, out: *mut bool) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe {
            *out = h.inner.fullbright();
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_pending_encounter(
    handle: *const GameHandle,
    out_entity: *mut u32,
    out_has_encounter: *mut bool,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_entity.is_null() || out_has_encounter.is_null() {
            return FfiError::NullArgument as i32;
        }
        match h.inner.peek_pending_encounter() {
            Some(e) => unsafe {
                *out_entity = e.raw();
                *out_has_encounter = true;
            },
            None => unsafe {
                *out_entity = 0;
                *out_has_encounter = false;
            },
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_take_pending_encounter(
    handle: *mut GameHandle,
    out_entity: *mut u32,
    out_has_encounter: *mut bool,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_mut() }) else {
            return FfiError::NullArgument as i32;
        };
        if out_entity.is_null() || out_has_encounter.is_null() {
            return FfiError::NullArgument as i32;
        }
        match h.inner.take_pending_encounter() {
            Some(e) => unsafe {
                *out_entity = e.raw();
                *out_has_encounter = true;
            },
            None => unsafe {
                *out_entity = 0;
                *out_has_encounter = false;
            },
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_set_pending_encounter(
    handle: *mut GameHandle,
    entity: u32,
) -> i32 {
    ffi_try!({
        let Some(h) = (unsafe { handle.as_mut() }) else {
            return FfiError::NullArgument as i32;
        };
        h.inner.set_pending_encounter(EntityId::from_raw(entity));
        FfiError::Ok as i32
    })
}

// ---- static name lookups (interned `OnceLock<HashMap<u8, CString>>`) ----

fn item_name_table() -> &'static HashMap<u8, CString> {
    static TABLE: OnceLock<HashMap<u8, CString>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ItemKind::ALL
            .iter()
            .map(|&k| (CItemKind::from(k) as u8, CString::new(k.name()).unwrap()))
            .collect()
    })
}

fn enemy_name_table() -> &'static HashMap<u8, CString> {
    static TABLE: OnceLock<HashMap<u8, CString>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ENEMY_KINDS
            .iter()
            .map(|&k| (CEnemyKind::from(k) as u8, CString::new(k.name()).unwrap()))
            .collect()
    })
}

fn ability_name_table() -> &'static HashMap<u8, CString> {
    static TABLE: OnceLock<HashMap<u8, CString>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ABILITY_KINDS
            .iter()
            .map(|&k| (CAbilityKind::from(k) as u8, CString::new(k.name()).unwrap()))
            .collect()
    })
}

fn attribute_name_table() -> &'static HashMap<u8, CString> {
    static TABLE: OnceLock<HashMap<u8, CString>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ATTRIBUTE_KINDS
            .iter()
            .map(|&a| (CAttribute::from(a) as u8, CString::new(a.name()).unwrap()))
            .collect()
    })
}

fn equip_slot_name_table() -> &'static HashMap<u8, CString> {
    static TABLE: OnceLock<HashMap<u8, CString>> = OnceLock::new();
    TABLE.get_or_init(|| {
        EquipSlot::ALL
            .iter()
            .map(|&s| (CEquipSlot::from(s) as u8, CString::new(s.name()).unwrap()))
            .collect()
    })
}

fn lookup_name(table: &'static HashMap<u8, CString>, key: u8) -> *const c_char {
    table
        .get(&key)
        .map(|s| s.as_ptr())
        .unwrap_or(std::ptr::null())
}

#[unsafe(no_mangle)]
pub extern "C" fn item_kind_name(kind: u8) -> *const c_char {
    lookup_name(item_name_table(), kind)
}

#[unsafe(no_mangle)]
pub extern "C" fn item_kind_glyph(kind: u8, out_glyph: *mut u32) -> i32 {
    ffi_try!({
        if out_glyph.is_null() {
            return FfiError::NullArgument as i32;
        }
        let lib_kind: ItemKind = match CItemKind::try_from(kind) {
            Ok(k) => k.into(),
            Err(e) => return e as i32,
        };
        unsafe {
            *out_glyph = lib_kind.glyph() as u32;
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn enemy_kind_name(kind: u8) -> *const c_char {
    lookup_name(enemy_name_table(), kind)
}

#[unsafe(no_mangle)]
pub extern "C" fn enemy_kind_glyph(kind: u8, out_glyph: *mut u32) -> i32 {
    ffi_try!({
        if out_glyph.is_null() {
            return FfiError::NullArgument as i32;
        }
        let lib_kind: EnemyKind = match CEnemyKind::try_from(kind) {
            Ok(k) => k.into(),
            Err(e) => return e as i32,
        };
        unsafe {
            *out_glyph = lib_kind.glyph() as u32;
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn enemy_kind_base_stats(kind: u8, out_stats: *mut CStats) -> i32 {
    ffi_try!({
        if out_stats.is_null() {
            return FfiError::NullArgument as i32;
        }
        let lib_kind: EnemyKind = match CEnemyKind::try_from(kind) {
            Ok(k) => k.into(),
            Err(e) => return e as i32,
        };
        unsafe {
            *out_stats = CStats::from(lib_kind.base_stats());
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ability_kind_name(kind: u8) -> *const c_char {
    lookup_name(ability_name_table(), kind)
}

#[unsafe(no_mangle)]
pub extern "C" fn attribute_name(attr: u8) -> *const c_char {
    lookup_name(attribute_name_table(), attr)
}

#[unsafe(no_mangle)]
pub extern "C" fn equip_slot_name(slot: u8) -> *const c_char {
    lookup_name(equip_slot_name_table(), slot)
}
