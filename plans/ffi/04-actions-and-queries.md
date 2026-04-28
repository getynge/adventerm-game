# 04 — Action dispatch and query expansion

Covers **M4** (action dispatch shim). Read [03-enums-and-structs.md](03-enums-and-structs.md) first — this file depends on the `Cxxx` types defined there.

## Decision: per-action FFI entries (not a tagged-action union)

Nine `extern "C"` functions, one per `Action` impl in `adventerm_lib::actions`. Each:
1. Null-checks the handle and any out-pointers.
2. Validates input (e.g., direction discriminant, slot bounds).
3. Constructs the lib-side `Action` struct.
4. Calls `adventerm_lib::action::dispatch(...)` with the player entity.
5. Converts the outcome to its `Cxxx` shape and writes to the out-pointer(s).

Why per-action and not a single dispatcher with a tagged action input + tagged outcome:
- Outcome shape varies per action; a single dispatcher would have to write a discriminated outcome union, forcing the caller to switch on action-tag *and* outcome-tag.
- Swift gets nine clean methods on its `Game` wrapper class with precise signatures. No runtime tag-mismatch checks.
- Per-action entries cost ~9 functions vs. 1 — negligible header bloat.

## Function signatures

`adventerm_ffi/src/action.rs`:

```rust
use crate::error::{FfiError, set_last_error};
use crate::handle::GameHandle;
use crate::structs::*;
use crate::enums::*;
use adventerm_lib::action::dispatch;
use adventerm_lib::actions::*;

#[no_mangle]
pub extern "C" fn game_action_move(
    handle: *mut GameHandle,
    direction: u8,                 // CDirection
    out_outcome: *mut CMoveOutcome,
) -> i32;

#[no_mangle]
pub extern "C" fn game_action_quick_move(
    handle: *mut GameHandle,
    direction: u8,
    out_outcome: *mut CMoveOutcome,
) -> i32;

#[no_mangle]
pub extern "C" fn game_action_interact(
    handle: *mut GameHandle,
    out_door_event: *mut CDoorEvent,
    out_has_event: *mut bool,      // true iff Some(DoorEvent)
) -> i32;

#[no_mangle]
pub extern "C" fn game_action_pickup(
    handle: *mut GameHandle,
    out_picked_kind: *mut u8,      // CItemKind, valid iff out_has_pickup is true
    out_has_pickup: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_action_place(
    handle: *mut GameHandle,
    inventory_slot: usize,
    out_outcome: *mut u8,          // CPlaceOutcome, valid iff out_has_outcome is true
    out_has_outcome: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_action_equip(
    handle: *mut GameHandle,
    inventory_slot: usize,
    out_unequipped_kind: *mut u8,  // CItemKind of previously equipped item, valid iff out_has_unequipped is true
    out_has_unequipped: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_action_unequip(
    handle: *mut GameHandle,
    equip_slot: u8,                // CEquipSlot
    out_success: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_action_consume(
    handle: *mut GameHandle,
    inventory_slot: usize,
    target: CConsumeTarget,
    out_outcome: *mut CConsumeOutcome,
    out_has_outcome: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_action_defeat_enemy(
    handle: *mut GameHandle,
    room: u32,
    entity: u32,
) -> i32;
```

## Worked implementation example

```rust
#[no_mangle]
pub extern "C" fn game_action_move(
    handle: *mut GameHandle,
    direction: u8,
    out_outcome: *mut CMoveOutcome,
) -> i32 {
    crate::ffi_try!({
        let h = match unsafe { handle.as_mut() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        };
        if out_outcome.is_null() {
            return FfiError::NullArgument as i32;
        }
        let dir = match CDirection::try_from(direction) {
            Ok(d) => d,
            Err(e) => return e as i32,
        };
        let actor = h.inner.player.entity();
        let action = MoveAction { direction: dir.into() };
        // Need an EventBus for dispatch — for now, allocate one transient bus per call.
        // Events emitted during a single FFI call are dropped after the dispatch returns.
        // Future: optional event-recording handle; out of scope for M4.
        let mut bus = adventerm_lib::EventBus::default();
        let outcome = dispatch(&mut h.inner, actor, action);
        unsafe { *out_outcome = CMoveOutcome::from(outcome); }
        FfiError::Ok as i32
    })
}
```

**Important detail — `EventBus`:** `dispatch` requires `&mut EventBus`. The TUI binary keeps a long-lived bus and drains it after each player action to update screens. The FFI's first-cut behavior is to allocate a transient `EventBus`, dispatch, then drop it (events are not surfaced to the caller).

This is the right starting point because:
- The 11 event types currently emitted are all consumable as observable state changes (PlayerMoved → query player_pos; ItemPickedUp → query inventory; EnemyDefeated → entity is despawned).
- Event surfacing requires an opaque `EventBusHandle` and per-event `Cxxx` shims for all 11 event types, doubling the FFI surface for marginal gain.
- A follow-up milestone can add event surfacing if iOS UX needs it (e.g., to play sounds on specific events). Tracked as out-of-scope per `00-overview.md`.

## Outcome conversion table

| Action | Lib outcome | FFI outcome | How to convert |
|--------|-------------|-------------|----------------|
| `MoveAction` | `MoveOutcome` | `CMoveOutcome` (tagged) | `From` impl from 03 |
| `QuickMoveAction` | `MoveOutcome` | `CMoveOutcome` (tagged) | same as above |
| `InteractAction` | `Option<DoorEvent>` | `CDoorEvent` + `out_has_event: bool` | match Some/None |
| `PickUpAction` | `Option<ItemKind>` | `u8` (CItemKind) + `out_has_pickup: bool` | match Some/None |
| `PlaceItemAction` | `Option<PlaceOutcome>` | `u8` (CPlaceOutcome) + `out_has_outcome: bool` | match Some/None |
| `EquipItemAction` | `Option<ItemKind>` (previously equipped) | `u8` + `out_has_unequipped: bool` | match Some/None |
| `UnequipItemAction` | `bool` | `out_success: bool` | direct |
| `ConsumeItemAction` | `Option<ConsumeOutcome>` | `CConsumeOutcome` (tagged) + `out_has_outcome: bool` | match Some/None |
| `DefeatEnemyAction` | `()` | (no out param) | unit return; `Ok` only |

## Read-only query exports (already covered in M3)

The query functions land in M3 (see [03-enums-and-structs.md](03-enums-and-structs.md) "Scalar query exports"). Reproduced here as a checklist for the M3 PR:

- `game_player_pos`
- `game_cur_health` / `game_set_cur_health`
- `game_vision_radius`
- `game_refresh_visibility`
- `game_tile_at` / `game_terrain_at`
- `game_is_visible` / `game_is_explored`
- `game_player_on_door`
- `game_items_here` / `game_peek_item_here`
- `game_effective_stats`
- `game_set_fullbright` / `game_fullbright`
- `game_pending_encounter` / `game_take_pending_encounter` / `game_set_pending_encounter`
- Static name lookups: `item_kind_name`, `item_kind_glyph`, `enemy_kind_name`, `enemy_kind_glyph`, `enemy_kind_base_stats`, `ability_kind_name`, `attribute_name`, `equip_slot_name`

## Test plan (M4)

`adventerm_ffi/tests/action_dispatch.rs` — parity tests against direct Rust `dispatch`:

```rust
#[test]
fn move_action_matches_direct_dispatch() {
    let h = adventerm_ffi::game_new_seeded(42);
    let mut outcome = adventerm_ffi::CMoveOutcome::default();
    let rc = adventerm_ffi::game_action_move(
        h,
        adventerm_ffi::CDirection::Up as u8,
        &mut outcome,
    );
    assert_eq!(rc, 0);

    // Direct comparison
    let mut game = adventerm_lib::GameState::new_seeded(42);
    let actor = game.player.entity();
    let mut bus = adventerm_lib::EventBus::default();
    let direct = adventerm_lib::action::dispatch(
        &mut game,
        actor,
        adventerm_lib::actions::MoveAction { direction: adventerm_lib::Direction::Up },
    );
    let expected = adventerm_ffi::CMoveOutcome::from(direct);
    assert_eq!(outcome.tag, expected.tag);
    assert_eq!(outcome.encounter_entity, expected.encounter_entity);

    adventerm_ffi::game_free(h);
}

// One test per action:
// quick_move_action_matches_direct_dispatch
// interact_action_matches_direct_dispatch
// pickup_action_matches_direct_dispatch
// place_action_matches_direct_dispatch
// equip_action_matches_direct_dispatch
// unequip_action_matches_direct_dispatch
// consume_action_matches_direct_dispatch
// defeat_enemy_action_matches_direct_dispatch

#[test]
fn move_action_invalid_direction_returns_out_of_range() {
    let h = adventerm_ffi::game_new_seeded(42);
    let mut outcome = adventerm_ffi::CMoveOutcome::default();
    let rc = adventerm_ffi::game_action_move(h, 99, &mut outcome);
    assert_eq!(rc, adventerm_ffi::FfiError::OutOfRange as i32);
    adventerm_ffi::game_free(h);
}

#[test]
fn move_action_null_handle_returns_null_argument() {
    let mut outcome = adventerm_ffi::CMoveOutcome::default();
    let rc = adventerm_ffi::game_action_move(
        std::ptr::null_mut(),
        adventerm_ffi::CDirection::Up as u8,
        &mut outcome,
    );
    assert_eq!(rc, adventerm_ffi::FfiError::NullArgument as i32);
}
```

## Validation

The parity test is the strongest contract: if a `From` impl from `03-enums-and-structs.md` is wrong (missing variant, mistransposed fields), this test catches it for every action's outcome. Add a parity test for each action; nine tests total.
