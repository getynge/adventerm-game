//! Per-action dispatch shims.
//!
//! Each `game_action_*` mirrors one [`Action`](adventerm_lib::action::Action)
//! impl in [`adventerm_lib::actions`]. The shape of every shim is the same:
//!
//! 1. Wrap the body in [`crate::ffi_try!`].
//! 2. Null-check the handle and any out-pointers.
//! 3. Validate input enums via the `Cxxx` `TryFrom<u8>` conversions.
//! 4. Build the lib-side action struct.
//! 5. Call [`adventerm_lib::action::dispatch`] with the player entity.
//! 6. Convert the outcome into its `Cxxx` mirror and write it out.
//!
//! `dispatch` allocates and drains its own [`adventerm_lib::EventBus`]; events
//! produced during a single FFI call are observable only through subsequent
//! state queries (per `plans/ffi/04-actions-and-queries.md`).

use adventerm_lib::action::dispatch;
use adventerm_lib::actions::{
    ConsumeItemAction, DefeatEnemyAction, EquipItemAction, InteractAction, MoveAction,
    PickUpAction, PlaceItemAction, QuickMoveAction, UnequipItemAction,
};
use adventerm_lib::ecs::EntityId;
use adventerm_lib::room::RoomId;

use crate::enums::{CDirection, CEquipSlot, CItemKind, CPlaceOutcome};
use crate::error::FfiError;
use crate::ffi_try;
use crate::handle::GameHandle;
use crate::structs::{CConsumeOutcome, CConsumeTarget, CDoorEvent, CMoveOutcome};

/// Resolve `handle` to `&mut GameHandle` or short-circuit with [`FfiError`].
///
/// Inlined into each shim via a small helper macro because the mut-borrow
/// captures `handle`'s lifetime at the call site.
macro_rules! handle_mut_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_mut() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        }
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_move(
    handle: *mut GameHandle,
    direction: u8,
    out_outcome: *mut CMoveOutcome,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        if out_outcome.is_null() {
            return FfiError::NullArgument as i32;
        }
        let dir = match CDirection::try_from(direction) {
            Ok(d) => d,
            Err(e) => return e as i32,
        };
        let actor = h.inner.player.entity();
        let outcome = dispatch(
            &mut h.inner,
            actor,
            MoveAction { direction: dir.into() },
        );
        unsafe { *out_outcome = CMoveOutcome::from(outcome); }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_quick_move(
    handle: *mut GameHandle,
    direction: u8,
    out_outcome: *mut CMoveOutcome,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        if out_outcome.is_null() {
            return FfiError::NullArgument as i32;
        }
        let dir = match CDirection::try_from(direction) {
            Ok(d) => d,
            Err(e) => return e as i32,
        };
        let actor = h.inner.player.entity();
        let outcome = dispatch(
            &mut h.inner,
            actor,
            QuickMoveAction { direction: dir.into() },
        );
        unsafe { *out_outcome = CMoveOutcome::from(outcome); }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_interact(
    handle: *mut GameHandle,
    out_door_event: *mut CDoorEvent,
    out_has_event: *mut bool,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        if out_door_event.is_null() || out_has_event.is_null() {
            return FfiError::NullArgument as i32;
        }
        let actor = h.inner.player.entity();
        let outcome = dispatch(&mut h.inner, actor, InteractAction);
        unsafe {
            match outcome {
                Some(ev) => {
                    *out_door_event = CDoorEvent::from(ev);
                    *out_has_event = true;
                }
                None => {
                    *out_door_event = CDoorEvent::default();
                    *out_has_event = false;
                }
            }
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_pickup(
    handle: *mut GameHandle,
    out_picked_kind: *mut u8,
    out_has_pickup: *mut bool,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        if out_picked_kind.is_null() || out_has_pickup.is_null() {
            return FfiError::NullArgument as i32;
        }
        let actor = h.inner.player.entity();
        let outcome = dispatch(&mut h.inner, actor, PickUpAction);
        unsafe {
            match outcome {
                Some(kind) => {
                    *out_picked_kind = CItemKind::from(kind) as u8;
                    *out_has_pickup = true;
                }
                None => {
                    *out_picked_kind = 0;
                    *out_has_pickup = false;
                }
            }
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_place(
    handle: *mut GameHandle,
    inventory_slot: usize,
    out_outcome: *mut u8,
    out_has_outcome: *mut bool,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        if out_outcome.is_null() || out_has_outcome.is_null() {
            return FfiError::NullArgument as i32;
        }
        let actor = h.inner.player.entity();
        let outcome = dispatch(
            &mut h.inner,
            actor,
            PlaceItemAction { slot: inventory_slot },
        );
        unsafe {
            match outcome {
                Some(o) => {
                    *out_outcome = CPlaceOutcome::from(o) as u8;
                    *out_has_outcome = true;
                }
                None => {
                    *out_outcome = 0;
                    *out_has_outcome = false;
                }
            }
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_equip(
    handle: *mut GameHandle,
    inventory_slot: usize,
    out_unequipped_kind: *mut u8,
    out_has_unequipped: *mut bool,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        if out_unequipped_kind.is_null() || out_has_unequipped.is_null() {
            return FfiError::NullArgument as i32;
        }
        let actor = h.inner.player.entity();
        // Snapshot the destination equipment slot before dispatch.
        // `EquipItemAction::Outcome` is the *equipped* kind; the displaced
        // kind isn't surfaced, so we read it pre-dispatch and gate it on a
        // successful equip after.
        let displaced = h
            .inner
            .player
            .inventory_get(inventory_slot)
            .and_then(adventerm_lib::items::equip_slot_of)
            .and_then(|slot| h.inner.player.equipment().slot(slot));
        let equipped = dispatch(
            &mut h.inner,
            actor,
            EquipItemAction { inventory_slot },
        );
        let prior = equipped.and(displaced);
        unsafe {
            match prior {
                Some(kind) => {
                    *out_unequipped_kind = CItemKind::from(kind) as u8;
                    *out_has_unequipped = true;
                }
                None => {
                    *out_unequipped_kind = 0;
                    *out_has_unequipped = false;
                }
            }
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_unequip(
    handle: *mut GameHandle,
    equip_slot: u8,
    out_success: *mut bool,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        if out_success.is_null() {
            return FfiError::NullArgument as i32;
        }
        let slot = match CEquipSlot::try_from(equip_slot) {
            Ok(s) => s,
            Err(e) => return e as i32,
        };
        let actor = h.inner.player.entity();
        let outcome = dispatch(
            &mut h.inner,
            actor,
            UnequipItemAction { slot: slot.into() },
        );
        unsafe { *out_success = outcome.is_some(); }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_consume(
    handle: *mut GameHandle,
    inventory_slot: usize,
    target: CConsumeTarget,
    out_outcome: *mut CConsumeOutcome,
    out_has_outcome: *mut bool,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        if out_outcome.is_null() || out_has_outcome.is_null() {
            return FfiError::NullArgument as i32;
        }
        let lib_target = match adventerm_lib::ConsumeTarget::try_from(target) {
            Ok(t) => t,
            Err(e) => return e as i32,
        };
        let actor = h.inner.player.entity();
        let outcome = dispatch(
            &mut h.inner,
            actor,
            ConsumeItemAction {
                inventory_slot,
                target: lib_target,
            },
        );
        unsafe {
            match outcome {
                Some(o) => {
                    *out_outcome = CConsumeOutcome::from(o);
                    *out_has_outcome = true;
                }
                None => {
                    *out_outcome = CConsumeOutcome::default();
                    *out_has_outcome = false;
                }
            }
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn game_action_defeat_enemy(
    handle: *mut GameHandle,
    room: u32,
    entity: u32,
) -> i32 {
    ffi_try!({
        let h = handle_mut_or_null!(handle);
        let actor = h.inner.player.entity();
        dispatch(
            &mut h.inner,
            actor,
            DefeatEnemyAction {
                room: RoomId(room),
                entity: EntityId::from_raw(entity),
            },
        );
        FfiError::Ok as i32
    })
}
