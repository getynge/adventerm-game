//! Iteration-accessor tests for M5. Each pair compares the FFI iterator
//! shim against the direct `adventerm_lib` read on the same seed; the
//! buffer-too-small contract is checked separately.

use adventerm_ffi::{
    game_abilities_active_copy, game_abilities_learned_active_copy,
    game_abilities_learned_active_len, game_current_room, game_equipment_snapshot, game_free,
    game_inventory_copy, game_inventory_len, game_new_seeded, game_room_dimensions,
    game_room_kind_at, game_room_walkable, room_door_at, room_doors_count, room_enemies_count,
    room_enemy_at, room_item_at, room_items_at_count, CDoorView, CEnemyView, CEquipmentSnapshot,
    CItemKind, CTileKind, FfiError, GameHandle,
};
use adventerm_lib::abilities::ABILITY_SLOTS;
use adventerm_lib::room::{RoomId, TileKind};
use adventerm_lib::GameState;

const SEED: u64 = 42;

/// Sentinel byte for an empty ability or equipment slot. Mirrors the
/// FFI's internal `C_*_SLOT_EMPTY` constants — repeated here so callers
/// (and these tests) can assert against a named value.
const EMPTY_SLOT: u8 = u8::MAX;

fn ffi_game(seed: u64) -> *mut GameHandle {
    let h = game_new_seeded(seed);
    assert!(!h.is_null());
    h
}

#[test]
fn inventory_copy_matches_direct() {
    let h = ffi_game(SEED);
    let game = GameState::new_seeded(SEED);

    let mut len = 0usize;
    assert_eq!(game_inventory_len(h, &mut len), FfiError::Ok as i32);
    assert_eq!(len, game.inventory().len());

    let mut buf = vec![0u8; len];
    let mut required = 0usize;
    let rc = game_inventory_copy(h, buf.as_mut_ptr(), buf.len(), &mut required);
    assert_eq!(rc, FfiError::Ok as i32);
    assert_eq!(required, len);

    let expected: Vec<u8> = game
        .inventory()
        .iter()
        .map(|&k| CItemKind::from(k) as u8)
        .collect();
    assert_eq!(buf, expected);

    game_free(h);
}

#[test]
fn inventory_copy_null_buffer_reports_required() {
    // Per the spec, a null `out_buf` always returns `BufferTooSmall` even
    // when `cap >= required`. The required count is still populated so the
    // caller can size their allocation in a single round-trip.
    let h = ffi_game(SEED);
    let mut required = 999usize;
    let rc = game_inventory_copy(h, std::ptr::null_mut(), 0, &mut required);
    assert_eq!(rc, FfiError::BufferTooSmall as i32);
    assert_eq!(required, 0);
    game_free(h);
}

#[test]
fn equipment_snapshot_starts_with_all_empty_sentinels() {
    let h = ffi_game(SEED);
    let mut snap = CEquipmentSnapshot::default();
    let rc = game_equipment_snapshot(h, &mut snap);
    assert_eq!(rc, FfiError::Ok as i32);
    for byte in [snap.head, snap.torso, snap.arms, snap.legs, snap.feet] {
        assert_eq!(byte, EMPTY_SLOT);
    }
    game_free(h);
}

#[test]
fn active_abilities_default_includes_impact() {
    let h = ffi_game(SEED);
    let mut buf = [0u8; ABILITY_SLOTS];
    let mut required = 0usize;
    let rc = game_abilities_active_copy(h, buf.as_mut_ptr(), buf.len(), &mut required);
    assert_eq!(rc, FfiError::Ok as i32);
    assert_eq!(required, ABILITY_SLOTS);

    let game = GameState::new_seeded(SEED);
    let expected: Vec<u8> = game
        .abilities()
        .active_slots
        .iter()
        .map(|s| match s {
            Some(k) => adventerm_ffi::CAbilityKind::from(*k) as u8,
            None => EMPTY_SLOT,
        })
        .collect();
    assert_eq!(&buf[..], expected.as_slice());

    game_free(h);
}

#[test]
fn active_abilities_buffer_too_small_reports_required() {
    let h = ffi_game(SEED);
    let mut required = 0usize;
    let mut buf = [0u8; 1];
    let rc = game_abilities_active_copy(h, buf.as_mut_ptr(), buf.len(), &mut required);
    assert_eq!(rc, FfiError::BufferTooSmall as i32);
    assert_eq!(required, ABILITY_SLOTS);
    game_free(h);
}

#[test]
fn active_abilities_null_buffer_reports_required() {
    let h = ffi_game(SEED);
    let mut required = 0usize;
    let rc = game_abilities_active_copy(h, std::ptr::null_mut(), 0, &mut required);
    assert_eq!(rc, FfiError::BufferTooSmall as i32);
    assert_eq!(required, ABILITY_SLOTS);
    game_free(h);
}

#[test]
fn learned_active_copy_matches_direct() {
    let h = ffi_game(SEED);
    let game = GameState::new_seeded(SEED);

    let mut len = 0usize;
    assert_eq!(
        game_abilities_learned_active_len(h, &mut len),
        FfiError::Ok as i32
    );
    assert_eq!(len, game.abilities().learned_active.len());

    let mut buf = vec![0u8; len];
    let mut required = 0usize;
    let rc = game_abilities_learned_active_copy(h, buf.as_mut_ptr(), buf.len(), &mut required);
    assert_eq!(rc, FfiError::Ok as i32);
    assert_eq!(required, len);

    let expected: Vec<u8> = game
        .abilities()
        .learned_active
        .iter()
        .map(|&k| adventerm_ffi::CAbilityKind::from(k) as u8)
        .collect();
    assert_eq!(buf, expected);

    game_free(h);
}

#[test]
fn current_room_matches_initial_state() {
    let h = ffi_game(SEED);
    let mut room = u32::MAX;
    assert_eq!(game_current_room(h, &mut room), FfiError::Ok as i32);
    assert_eq!(room, 0);
    game_free(h);
}

#[test]
fn room_dimensions_match_direct_read() {
    let h = ffi_game(SEED);
    let game = GameState::new_seeded(SEED);
    let r = game.dungeon.room(RoomId(0));
    let mut w = 0usize;
    let mut hgt = 0usize;
    assert_eq!(
        game_room_dimensions(h, 0, &mut w, &mut hgt),
        FfiError::Ok as i32
    );
    assert_eq!((w, hgt), (r.width, r.height));
    game_free(h);
}

#[test]
fn room_dimensions_invalid_id_returns_out_of_range() {
    let h = ffi_game(SEED);
    let mut w = 0usize;
    let mut hgt = 0usize;
    let rc = game_room_dimensions(h, u32::MAX, &mut w, &mut hgt);
    assert_eq!(rc, FfiError::OutOfRange as i32);
    game_free(h);
}

#[test]
fn room_kind_and_walkable_match_direct() {
    let h = ffi_game(SEED);
    let game = GameState::new_seeded(SEED);
    let r = game.dungeon.room(RoomId(0));
    let (px, py) = game.player_pos();

    let mut kind = CTileKind::default();
    assert_eq!(
        game_room_kind_at(h, 0, px, py, &mut kind),
        FfiError::Ok as i32
    );
    let expected_kind: CTileKind = r.kind_at(px, py).unwrap().into();
    assert_eq!(kind, expected_kind);

    let mut walkable = false;
    assert_eq!(
        game_room_walkable(h, 0, px, py, &mut walkable),
        FfiError::Ok as i32
    );
    assert_eq!(walkable, r.is_walkable(px, py));

    game_free(h);
}

#[test]
fn room_kind_at_out_of_bounds_returns_out_of_range() {
    let h = ffi_game(SEED);
    let mut kind = CTileKind::default();
    let rc = game_room_kind_at(h, 0, usize::MAX, usize::MAX, &mut kind);
    assert_eq!(rc, FfiError::OutOfRange as i32);
    game_free(h);
}

#[test]
fn room_doors_iteration_matches_direct() {
    let h = ffi_game(SEED);
    let game = GameState::new_seeded(SEED);

    let mut count = 0usize;
    assert_eq!(room_doors_count(h, &mut count), FfiError::Ok as i32);

    let expected: Vec<CDoorView> = game
        .current_room()
        .doors()
        .filter_map(|(_, _, id)| game.dungeon.door_view(id))
        .map(CDoorView::from)
        .collect();
    assert_eq!(count, expected.len());

    for i in 0..count {
        let mut view = CDoorView::default();
        assert_eq!(room_door_at(h, i, &mut view), FfiError::Ok as i32);
        assert_eq!(view, expected[i]);
    }

    let mut view = CDoorView::default();
    let rc = room_door_at(h, count, &mut view);
    assert_eq!(rc, FfiError::OutOfRange as i32);

    game_free(h);
}

#[test]
fn room_enemies_iteration_matches_direct() {
    let h = ffi_game(SEED);
    let game = GameState::new_seeded(SEED);

    // Starting room (id 0) is intentionally empty per the lib's generation
    // invariant. Pick the first room that has any enemies.
    let target_room: u32 = game
        .dungeon
        .rooms
        .iter()
        .find(|r| !r.enemies.is_empty())
        .map(|r| r.id.0)
        .expect("seeded dungeon should have at least one enemy room");

    let mut count = 0usize;
    assert_eq!(
        room_enemies_count(h, target_room, &mut count),
        FfiError::Ok as i32
    );

    let r = game.dungeon.room(RoomId(target_room));
    // Build the expected `CEnemyView`s manually since `CEnemyView::new` is
    // crate-private. This mirrors what the FFI does internally.
    let mut expected: Vec<CEnemyView> = r
        .enemies
        .iter_with_pos(&r.world)
        .map(|(e, pos, kind)| CEnemyView {
            entity: e.raw(),
            kind: adventerm_ffi::CEnemyKind::from(kind) as u8,
            _pad: [0; 3],
            x: pos.0 as u32,
            y: pos.1 as u32,
        })
        .collect();
    // `iter_with_pos` is backed by a `HashMap`, so order is unspecified.
    expected.sort_by_key(|v| v.entity);
    assert_eq!(count, expected.len());

    let mut got: Vec<CEnemyView> = Vec::with_capacity(count);
    for i in 0..count {
        let mut view = CEnemyView::default();
        assert_eq!(
            room_enemy_at(h, target_room, i, &mut view),
            FfiError::Ok as i32
        );
        got.push(view);
    }
    got.sort_by_key(|v| v.entity);
    assert_eq!(got, expected);

    let mut view = CEnemyView::default();
    let rc = room_enemy_at(h, target_room, count, &mut view);
    assert_eq!(rc, FfiError::OutOfRange as i32);

    game_free(h);
}

#[test]
fn room_items_at_empty_tile_reports_zero() {
    let h = ffi_game(SEED);
    let mut count = usize::MAX;
    // Tile (0,0) is the wall corner of a generated room — never an item tile.
    assert_eq!(
        room_items_at_count(h, 0, 0, &mut count),
        FfiError::Ok as i32
    );
    assert_eq!(count, 0);

    let mut kind = 0u8;
    let rc = room_item_at(h, 0, 0, 0, &mut kind);
    assert_eq!(rc, FfiError::OutOfRange as i32);

    game_free(h);
}

#[test]
fn null_handle_returns_null_argument() {
    let mut len = 0usize;
    let rc = game_inventory_len(std::ptr::null(), &mut len);
    assert_eq!(rc, FfiError::NullArgument as i32);
}

#[test]
fn door_tile_parity_in_seeded_room() {
    let game = GameState::new_seeded(SEED);
    let r = game.dungeon.room(RoomId(0));
    if let Some((x, y, _)) = r.doors().next() {
        let kind = r.kind_at(x, y).unwrap();
        assert!(matches!(kind, TileKind::Door(_)));
        let c: CTileKind = kind.into();
        // Tag 2 == Door per `CTileKind`'s documented mapping.
        assert_eq!(c.tag, 2);
    }
}
