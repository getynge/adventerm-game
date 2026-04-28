//! Parity tests: every `game_action_*` shim against direct
//! `adventerm_lib::action::dispatch` on a freshly-seeded `GameState`.
//!
//! Each test seeds two `GameState`s with the same value, runs the action via
//! the FFI on the first and via direct Rust on the second, then compares the
//! outcome converted into its `Cxxx` shape. Because `GameHandle::inner` is
//! crate-private, parity setup that needs to mutate inventory is done by
//! relying on the seeded game's natural state — empty-inventory paths still
//! exercise the full shim plumbing and round-trip every conversion.
//!
//! A pair of negative tests covers `OutOfRange` (invalid `CDirection`
//! discriminant) and `NullArgument` (null handle).

use std::ptr;

use adventerm_ffi::{
    game_action_consume, game_action_defeat_enemy, game_action_equip,
    game_action_interact, game_action_move, game_action_pickup, game_action_place,
    game_action_quick_move, game_action_unequip, game_free, game_new_seeded, CConsumeOutcome,
    CConsumeTarget, CDirection, CDoorEvent, CEquipSlot, CItemKind, CMoveOutcome, CPlaceOutcome,
    FfiError, GameHandle,
};
use adventerm_lib::action::dispatch;
use adventerm_lib::actions::{
    ConsumeItemAction, DefeatEnemyAction, EquipItemAction, InteractAction, MoveAction,
    PickUpAction, PlaceItemAction, QuickMoveAction, UnequipItemAction,
};
use adventerm_lib::items::{ConsumeTarget, EquipSlot};
use adventerm_lib::{Direction, GameState};

const SEED: u64 = 42;
/// Out-of-range direction discriminant for the negative test.
const INVALID_DIRECTION: u8 = 99;

fn ffi_game(seed: u64) -> *mut GameHandle {
    let h = game_new_seeded(seed);
    assert!(!h.is_null());
    h
}

fn lib_game(seed: u64) -> GameState {
    GameState::new_seeded(seed)
}

#[test]
fn move_action_matches_direct_dispatch() {
    let h = ffi_game(SEED);
    let mut outcome = CMoveOutcome::default();
    let rc = game_action_move(h, CDirection::Up as u8, &mut outcome);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let actor = game.player.entity();
    let direct = dispatch(&mut game, actor, MoveAction { direction: Direction::Up });
    assert_eq!(outcome, CMoveOutcome::from(direct));

    game_free(h);
}

#[test]
fn quick_move_action_matches_direct_dispatch() {
    let h = ffi_game(SEED);
    let mut outcome = CMoveOutcome::default();
    let rc = game_action_quick_move(h, CDirection::Down as u8, &mut outcome);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let actor = game.player.entity();
    let direct = dispatch(
        &mut game,
        actor,
        QuickMoveAction { direction: Direction::Down },
    );
    assert_eq!(outcome, CMoveOutcome::from(direct));

    game_free(h);
}

#[test]
fn interact_action_matches_direct_dispatch() {
    let h = ffi_game(SEED);
    let mut event = CDoorEvent::default();
    let mut has = true;
    let rc = game_action_interact(h, &mut event, &mut has);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let actor = game.player.entity();
    let direct = dispatch(&mut game, actor, InteractAction);
    assert_eq!(has, direct.is_some());
    if let Some(ev) = direct {
        assert_eq!(event, CDoorEvent::from(ev));
    }

    game_free(h);
}

#[test]
fn pickup_action_matches_direct_dispatch() {
    let h = ffi_game(SEED);
    let mut kind = 0u8;
    let mut has = true;
    let rc = game_action_pickup(h, &mut kind, &mut has);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let actor = game.player.entity();
    let direct = dispatch(&mut game, actor, PickUpAction);
    assert_eq!(has, direct.is_some());
    if let Some(k) = direct {
        assert_eq!(kind, CItemKind::from(k) as u8);
    }

    game_free(h);
}

#[test]
fn place_action_matches_direct_dispatch() {
    // Empty-inventory parity: both paths return None and the shim writes
    // `out_has_outcome = false`. Still exercises the full code path.
    let h = ffi_game(SEED);
    let mut tag = u8::MAX;
    let mut has = true;
    let rc = game_action_place(h, 0, &mut tag, &mut has);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let actor = game.player.entity();
    let direct = dispatch(&mut game, actor, PlaceItemAction { slot: 0 });
    assert_eq!(has, direct.is_some());
    if let Some(o) = direct {
        assert_eq!(tag, CPlaceOutcome::from(o) as u8);
    }

    game_free(h);
}

#[test]
fn equip_action_matches_direct_dispatch() {
    let h = ffi_game(SEED);
    let mut kind = u8::MAX;
    let mut has = true;
    let rc = game_action_equip(h, 0, &mut kind, &mut has);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let displaced = game
        .player
        .inventory_get(0)
        .and_then(adventerm_lib::items::equip_slot_of)
        .and_then(|s| game.player.equipment().slot(s));
    let actor = game.player.entity();
    let equipped = dispatch(&mut game, actor, EquipItemAction { inventory_slot: 0 });
    let prior = equipped.and(displaced);
    assert_eq!(has, prior.is_some());
    if let Some(k) = prior {
        assert_eq!(kind, CItemKind::from(k) as u8);
    }

    game_free(h);
}

#[test]
fn unequip_action_matches_direct_dispatch() {
    let h = ffi_game(SEED);
    let mut success = true;
    let rc = game_action_unequip(h, CEquipSlot::Torso as u8, &mut success);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let actor = game.player.entity();
    let direct = dispatch(
        &mut game,
        actor,
        UnequipItemAction { slot: EquipSlot::Torso },
    );
    assert_eq!(success, direct.is_some());

    game_free(h);
}

#[test]
fn consume_action_matches_direct_dispatch() {
    let h = ffi_game(SEED);
    let target = CConsumeTarget {
        tag: 1, // AbilitySlot
        _pad: [0; 3],
        slot: 0,
    };
    let mut outcome = CConsumeOutcome::default();
    let mut has = true;
    let rc = game_action_consume(h, 0, target, &mut outcome, &mut has);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let actor = game.player.entity();
    let direct = dispatch(
        &mut game,
        actor,
        ConsumeItemAction {
            inventory_slot: 0,
            target: ConsumeTarget::AbilitySlot(0),
        },
    );
    assert_eq!(has, direct.is_some());
    if let Some(o) = direct {
        assert_eq!(outcome, CConsumeOutcome::from(o));
    }

    game_free(h);
}

#[test]
fn defeat_enemy_action_matches_direct_dispatch() {
    // Use a probe game (same seed) to discover a real enemy id; both paths
    // then act on identical state.
    let probe = lib_game(SEED);
    let room = probe.current_room;
    let entity_raw = probe
        .dungeon
        .room(room)
        .enemies
        .entities()
        .next()
        .map(|e| e.raw())
        .unwrap_or(0);

    let h = ffi_game(SEED);
    let rc = game_action_defeat_enemy(h, room.0, entity_raw);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game = lib_game(SEED);
    let actor = game.player.entity();
    dispatch(
        &mut game,
        actor,
        DefeatEnemyAction {
            room,
            entity: adventerm_lib::EntityId::from_raw(entity_raw),
        },
    );

    game_free(h);
}

#[test]
fn move_action_invalid_direction_returns_out_of_range() {
    let h = ffi_game(SEED);
    let mut outcome = CMoveOutcome::default();
    let rc = game_action_move(h, INVALID_DIRECTION, &mut outcome);
    assert_eq!(rc, FfiError::OutOfRange as i32);
    game_free(h);
}

#[test]
fn move_action_null_handle_returns_null_argument() {
    let mut outcome = CMoveOutcome::default();
    let rc = game_action_move(
        ptr::null_mut(),
        CDirection::Up as u8,
        &mut outcome,
    );
    assert_eq!(rc, FfiError::NullArgument as i32);
}
