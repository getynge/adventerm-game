//! M7 — battle subsystem integration tests.
//!
//! The starting room of a seeded game holds no enemy by design (see
//! `dungeon.rs::place_room_enemy`'s `i == 0` skip), so each test here builds a
//! `GameState` directly via `adventerm_lib`, spawns a slime adjacent to the
//! player, then ferries the modified state across the FFI boundary by
//! serializing through `Save::to_bytes` and reloading via `save_from_bytes` /
//! `save_to_game`. That gives us a `GameHandle` with a known enemy entity id
//! we can pass to `battle_start` — no need to drive movement actions to
//! generate an encounter.

use std::ptr;

use adventerm_ffi::{
    battle_apply_enemy_turn, battle_apply_player_ability, battle_combatants, battle_enemy_cur_hp,
    battle_free, battle_hp_snapshot, battle_is_resolved, battle_log_line_copy,
    battle_log_line_count, battle_player_cur_hp, battle_result, battle_start, battle_turn,
    game_free, game_new_seeded, save_from_bytes, save_to_game, BattleHandle, CBattleResult,
    CBattleTurn, CCombatants, CHpSnapshot, FfiError, GameHandle, SaveHandle,
};
use adventerm_lib::enemies::EnemyKind;
use adventerm_lib::room::TileKind;
use adventerm_lib::save::Save;
use adventerm_lib::{EntityId, GameState};

const SEED: u64 = 42;
/// Cap on the number of full turn cycles in a scripted battle. Slime starts
/// at a low HP value and Impact does at least one damage per hit, so a runaway
/// loop here means something is wrong with the engine wiring.
const TURN_CAP: usize = 100;

/// Build a [`GameState`], spawn one slime on a free floor tile next to the
/// player, and return the resulting `(GameHandle, slime_entity_id)` via the
/// FFI's save/load round-trip.
fn make_game_with_adjacent_slime() -> (*mut GameHandle, u32) {
    let mut state = GameState::new_seeded(SEED);
    let player = state.player_pos();
    let room = state.dungeon.room_mut(state.current_room);

    let mut spawn_pos = None;
    'outer: for y in 0..room.height {
        for x in 0..room.width {
            if (x, y) == player {
                continue;
            }
            if matches!(room.kind_at(x, y), Some(TileKind::Floor)) {
                spawn_pos = Some((x, y));
                break 'outer;
            }
        }
    }
    let spawn_pos = spawn_pos.expect("seed must have a second floor tile");
    let enemy_entity = room
        .enemies
        .spawn_at(&mut room.world, spawn_pos, EnemyKind::Slime);

    let bytes = Save::new("battle_flow".to_string(), state).to_bytes();

    let mut save: *mut SaveHandle = ptr::null_mut();
    let rc = save_from_bytes(bytes.as_ptr(), bytes.len(), &mut save);
    assert_eq!(rc, FfiError::Ok as i32);

    let mut game: *mut GameHandle = ptr::null_mut();
    let rc = save_to_game(save, &mut game);
    assert_eq!(rc, FfiError::Ok as i32);

    adventerm_ffi::save_free(save);
    (game, enemy_entity.raw())
}

#[test]
fn scripted_battle_resolves_to_known_outcome() {
    let (game, enemy) = make_game_with_adjacent_slime();

    let mut battle: *mut BattleHandle = ptr::null_mut();
    let mut started = false;
    assert_eq!(
        battle_start(game, enemy, &mut battle, &mut started),
        FfiError::Ok as i32
    );
    assert!(started);
    assert!(!battle.is_null());

    // The opening line is appended at construction.
    let mut count = 0usize;
    assert_eq!(
        battle_log_line_count(battle, &mut count),
        FfiError::Ok as i32
    );
    assert!(count > 0);

    // Snapshot HP before any turns so we can assert that abilities tick HP.
    let mut start_snapshot = CHpSnapshot::default();
    assert_eq!(
        battle_hp_snapshot(battle, &mut start_snapshot),
        FfiError::Ok as i32
    );
    assert!(start_snapshot.player > 0);
    assert!(start_snapshot.enemy > 0);

    // Drive the loop until a terminal state is reached.
    let mut resolved_via_loop = false;
    for _ in 0..TURN_CAP {
        let mut turn = CBattleTurn::default();
        assert_eq!(battle_turn(battle, &mut turn), FfiError::Ok as i32);
        match turn.tag {
            0 => assert_eq!(
                battle_apply_player_ability(game, battle, 0),
                FfiError::Ok as i32
            ),
            1 => assert_eq!(
                battle_apply_enemy_turn(game, battle),
                FfiError::Ok as i32
            ),
            2 => {
                resolved_via_loop = true;
                break;
            }
            other => panic!("unknown CBattleTurn tag {other}"),
        }
    }
    assert!(resolved_via_loop, "battle did not resolve in {TURN_CAP} turns");

    let mut resolved = false;
    assert_eq!(battle_is_resolved(battle, &mut resolved), FfiError::Ok as i32);
    assert!(resolved);

    let mut result_byte = 0u8;
    let mut has_result = false;
    assert_eq!(
        battle_result(battle, &mut result_byte, &mut has_result),
        FfiError::Ok as i32
    );
    assert!(has_result);
    // Slime is feeble; player on slot-0 Impact wins this matchup.
    assert_eq!(result_byte, CBattleResult::Victory as u8);

    let mut combatants = CCombatants::default();
    assert_eq!(
        battle_combatants(battle, &mut combatants),
        FfiError::Ok as i32
    );
    assert_eq!(combatants.enemy_entity, enemy);

    let mut enemy_hp = 1u8;
    assert_eq!(battle_enemy_cur_hp(battle, &mut enemy_hp), FfiError::Ok as i32);
    assert_eq!(enemy_hp, 0);

    let mut player_hp = 0u8;
    assert_eq!(
        battle_player_cur_hp(battle, &mut player_hp),
        FfiError::Ok as i32
    );
    assert!(player_hp > 0);

    // Log accumulates the opening line plus per-turn entries.
    let mut count_after = 0usize;
    assert_eq!(
        battle_log_line_count(battle, &mut count_after),
        FfiError::Ok as i32
    );
    assert!(count_after >= count);

    battle_free(battle);
    game_free(game);
}

#[test]
fn battle_start_returns_false_for_invalid_entity() {
    let game = game_new_seeded(SEED);
    let mut battle: *mut BattleHandle = ptr::null_mut();
    // Pre-fill `started=true` to verify the success path overwrites it.
    let mut started = true;
    // Entity id `0` is the player in seed 42's allocator order, never a
    // current-room enemy — `start_battle` returns `None`.
    let invalid_entity = EntityId::from_raw(99_999).raw();
    assert_eq!(
        battle_start(game, invalid_entity, &mut battle, &mut started),
        FfiError::Ok as i32
    );
    assert!(!started);
    assert!(battle.is_null());
    game_free(game);
}

#[test]
fn battle_apply_enemy_turn_on_player_turn_is_no_op() {
    // The lib's `apply_enemy_turn` is silent on non-enemy turns (returns
    // without error), but the FFI wrapper still rejects null pointers and
    // returns Ok. Confirm an enemy-turn call during the player turn doesn't
    // mutate HP.
    let (game, enemy) = make_game_with_adjacent_slime();

    let mut battle: *mut BattleHandle = ptr::null_mut();
    let mut started = false;
    assert_eq!(
        battle_start(game, enemy, &mut battle, &mut started),
        FfiError::Ok as i32
    );
    assert!(started);

    let mut before = CHpSnapshot::default();
    battle_hp_snapshot(battle, &mut before);
    assert_eq!(
        battle_apply_enemy_turn(game, battle),
        FfiError::Ok as i32
    );
    let mut after = CHpSnapshot::default();
    battle_hp_snapshot(battle, &mut after);
    assert_eq!(before, after);

    battle_free(battle);
    game_free(game);
}

#[test]
fn battle_apply_player_ability_empty_slot_returns_empty_slot() {
    let (game, enemy) = make_game_with_adjacent_slime();

    let mut battle: *mut BattleHandle = ptr::null_mut();
    let mut started = false;
    assert_eq!(
        battle_start(game, enemy, &mut battle, &mut started),
        FfiError::Ok as i32
    );

    // Slot 1 is empty for a fresh game (default abilities only fill slot 0).
    assert_eq!(
        battle_apply_player_ability(game, battle, 1),
        FfiError::EmptySlot as i32
    );

    battle_free(battle);
    game_free(game);
}

#[test]
fn battle_apply_player_ability_wrong_turn_returns_not_player_turn() {
    let (game, enemy) = make_game_with_adjacent_slime();

    let mut battle: *mut BattleHandle = ptr::null_mut();
    let mut started = false;
    assert_eq!(
        battle_start(game, enemy, &mut battle, &mut started),
        FfiError::Ok as i32
    );

    // First player attack succeeds and flips the turn to Enemy (assuming the
    // slime survives — its base HP is well above Impact's damage).
    assert_eq!(
        battle_apply_player_ability(game, battle, 0),
        FfiError::Ok as i32
    );
    let mut turn = CBattleTurn::default();
    battle_turn(battle, &mut turn);
    if turn.tag == 1 {
        assert_eq!(
            battle_apply_player_ability(game, battle, 0),
            FfiError::NotPlayerTurn as i32
        );
    }

    battle_free(battle);
    game_free(game);
}

#[test]
fn battle_log_line_copy_two_call_discovery() {
    let (game, enemy) = make_game_with_adjacent_slime();

    let mut battle: *mut BattleHandle = ptr::null_mut();
    let mut started = false;
    assert_eq!(
        battle_start(game, enemy, &mut battle, &mut started),
        FfiError::Ok as i32
    );

    // Discovery call: `cap=0` is always too small.
    let mut needed = 0usize;
    let rc = battle_log_line_copy(battle, 0, ptr::null_mut(), 0, &mut needed);
    assert_eq!(rc, FfiError::BufferTooSmall as i32);
    assert!(needed > 1);

    let mut buf = vec![0u8; needed];
    let rc = battle_log_line_copy(battle, 0, buf.as_mut_ptr(), buf.len(), &mut needed);
    assert_eq!(rc, FfiError::Ok as i32);
    assert_eq!(buf[buf.len() - 1], 0);

    let line = std::str::from_utf8(&buf[..buf.len() - 1]).unwrap();
    assert!(line.contains("appears"));

    // Out-of-range index yields OutOfRange.
    let mut count = 0usize;
    battle_log_line_count(battle, &mut count);
    let rc = battle_log_line_copy(battle, count, ptr::null_mut(), 0, &mut needed);
    assert_eq!(rc, FfiError::OutOfRange as i32);

    battle_free(battle);
    game_free(game);
}

#[test]
fn battle_free_null_is_safe() {
    battle_free(ptr::null_mut());
}

#[test]
fn null_handle_returns_null_argument() {
    let mut count = 0usize;
    assert_eq!(
        battle_log_line_count(ptr::null(), &mut count),
        FfiError::NullArgument as i32
    );

    let mut started = false;
    let mut battle: *mut BattleHandle = ptr::null_mut();
    assert_eq!(
        battle_start(ptr::null(), 0, &mut battle, &mut started),
        FfiError::NullArgument as i32
    );
}
