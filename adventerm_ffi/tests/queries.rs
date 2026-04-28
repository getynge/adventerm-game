//! Compare each scalar FFI query against the direct Rust read on the same
//! seed. Also exercises null-handling and the static name interning.

use std::ptr;

use adventerm_ffi::{
    ability_kind_name, attribute_name, enemy_kind_base_stats, enemy_kind_glyph, enemy_kind_name,
    equip_slot_name, ffi_last_error_message, game_effective_stats, game_free, game_fullbright,
    game_is_explored, game_is_visible, game_items_here, game_new_seeded, game_peek_item_here,
    game_pending_encounter, game_player_on_door, game_player_pos, game_set_fullbright,
    game_set_pending_encounter, game_take_pending_encounter, game_terrain_at, game_tile_at,
    item_kind_glyph, item_kind_name, CAbilityKind, CAttribute, CEnemyKind, CEquipSlot, CItemKind,
    CStats, CTile, FfiError,
};

const SEED: u64 = 42;

fn ffi_game() -> *mut adventerm_ffi::GameHandle {
    let h = game_new_seeded(SEED);
    assert!(!h.is_null());
    h
}

#[test]
fn tile_and_terrain_match_direct_read() {
    let h = ffi_game();
    let game = adventerm_lib::GameState::new_seeded(SEED);
    let (px, py) = game.player_pos();

    for (x, y) in [(px, py), (0, 0), (px.saturating_add(1), py)] {
        let mut tile = 0u8;
        let mut terrain = 0u8;
        assert_eq!(game_tile_at(h, x, y, &mut tile), 0);
        assert_eq!(game_terrain_at(h, x, y, &mut terrain), 0);
        assert_eq!(tile, CTile::from(game.tile_at(x, y)) as u8);
        assert_eq!(terrain, CTile::from(game.terrain_at(x, y)) as u8);
    }

    game_free(h);
}

#[test]
fn visibility_and_explored_match_direct_read() {
    let h = ffi_game();
    let game = adventerm_lib::GameState::new_seeded(SEED);
    let (px, py) = game.player_pos();

    let mut vis = false;
    let mut exp = false;
    assert_eq!(game_is_visible(h, px, py, &mut vis), 0);
    assert_eq!(game_is_explored(h, px, py, &mut exp), 0);
    assert_eq!(vis, game.is_visible(px, py));
    assert_eq!(exp, game.is_explored(px, py));

    game_free(h);
}

#[test]
fn player_on_door_reports_no_door_initially() {
    let h = ffi_game();
    let mut id = 0u32;
    let mut has = true;
    assert_eq!(game_player_on_door(h, &mut id, &mut has), 0);
    assert!(!has);
    assert_eq!(id, 0);
    game_free(h);
}

#[test]
fn items_here_matches_direct_read() {
    let h = ffi_game();
    let game = adventerm_lib::GameState::new_seeded(SEED);

    let mut here = false;
    assert_eq!(game_items_here(h, &mut here), 0);
    assert_eq!(here, game.items_here());

    let mut kind = 0u8;
    let mut has = false;
    assert_eq!(game_peek_item_here(h, &mut kind, &mut has), 0);
    assert_eq!(has, game.peek_item_here().is_some());

    game_free(h);
}

#[test]
fn effective_stats_match_direct_read() {
    let h = ffi_game();
    let game = adventerm_lib::GameState::new_seeded(SEED);

    let mut stats = CStats::default();
    assert_eq!(game_effective_stats(h, &mut stats), 0);
    assert_eq!(stats, CStats::from(game.effective_stats()));

    game_free(h);
}

#[test]
fn fullbright_round_trips() {
    let h = ffi_game();

    let mut on = true;
    assert_eq!(game_fullbright(h, &mut on), 0);
    assert!(!on);

    assert_eq!(game_set_fullbright(h, true), 0);
    let mut on2 = false;
    assert_eq!(game_fullbright(h, &mut on2), 0);
    assert!(on2);

    game_free(h);
}

#[test]
fn pending_encounter_round_trips_through_ffi() {
    let h = ffi_game();

    let mut entity = 0u32;
    let mut has = true;
    assert_eq!(game_pending_encounter(h, &mut entity, &mut has), 0);
    assert!(!has);

    // Set, peek (still set), take (now cleared).
    assert_eq!(game_set_pending_encounter(h, 7), 0);

    let mut entity = 0u32;
    let mut has = false;
    assert_eq!(game_pending_encounter(h, &mut entity, &mut has), 0);
    assert!(has);
    assert_eq!(entity, 7);

    let mut entity = 0u32;
    let mut has = false;
    assert_eq!(game_take_pending_encounter(h, &mut entity, &mut has), 0);
    assert!(has);
    assert_eq!(entity, 7);

    let mut has = true;
    let mut entity = 0u32;
    assert_eq!(game_pending_encounter(h, &mut entity, &mut has), 0);
    assert!(!has);

    game_free(h);
}

#[test]
fn null_handle_returns_null_argument_on_query() {
    let mut tile = 0u8;
    let rc = game_tile_at(ptr::null(), 0, 0, &mut tile);
    assert_eq!(rc, FfiError::NullArgument as i32);
}

#[test]
fn null_out_pointer_returns_null_argument() {
    let h = ffi_game();
    let rc = game_tile_at(h, 0, 0, ptr::null_mut());
    assert_eq!(rc, FfiError::NullArgument as i32);
    game_free(h);
}

#[test]
fn item_name_pointer_is_stable_across_calls() {
    let p1 = item_kind_name(CItemKind::Torch as u8);
    let p2 = item_kind_name(CItemKind::Torch as u8);
    assert!(!p1.is_null());
    assert_eq!(p1, p2);
}

#[test]
fn item_glyph_matches_lib() {
    use adventerm_lib::ItemKind;
    for &k in ItemKind::ALL {
        let mut g = 0u32;
        let rc = item_kind_glyph(CItemKind::from(k) as u8, &mut g);
        assert_eq!(rc, 0);
        assert_eq!(g, k.glyph() as u32);
    }
}

#[test]
fn enemy_glyph_and_stats_match_lib() {
    use adventerm_lib::enemies::EnemyKind;
    let mut g = 0u32;
    assert_eq!(enemy_kind_glyph(CEnemyKind::Slime as u8, &mut g), 0);
    assert_eq!(g, EnemyKind::Slime.glyph() as u32);

    let mut stats = CStats::default();
    assert_eq!(enemy_kind_base_stats(CEnemyKind::Slime as u8, &mut stats), 0);
    assert_eq!(stats, CStats::from(EnemyKind::Slime.base_stats()));
}

#[test]
fn name_lookups_return_non_null_for_valid_kinds() {
    assert!(!enemy_kind_name(CEnemyKind::Slime as u8).is_null());
    assert!(!ability_kind_name(CAbilityKind::Impact as u8).is_null());
    assert!(!attribute_name(CAttribute::Fire as u8).is_null());
    assert!(!equip_slot_name(CEquipSlot::Head as u8).is_null());
}

#[test]
fn name_lookups_return_null_for_unknown_kinds() {
    assert!(item_kind_name(99).is_null());
    assert!(enemy_kind_name(99).is_null());
    assert!(ability_kind_name(99).is_null());
    assert!(attribute_name(99).is_null());
    assert!(equip_slot_name(99).is_null());
}

#[test]
fn unknown_kind_in_glyph_returns_out_of_range() {
    let mut g = 0u32;
    assert_eq!(
        item_kind_glyph(99, &mut g),
        FfiError::OutOfRange as i32
    );
    assert_eq!(
        enemy_kind_glyph(99, &mut g),
        FfiError::OutOfRange as i32
    );
}

#[test]
fn player_pos_matches_direct_read() {
    let h = ffi_game();
    let mut x = 0usize;
    let mut y = 0usize;
    assert_eq!(game_player_pos(h, &mut x, &mut y), 0);

    let game = adventerm_lib::GameState::new_seeded(SEED);
    let (rx, ry) = game.player_pos();
    assert_eq!((x, y), (rx, ry));
    game_free(h);
}

#[test]
fn last_error_callable_after_query_path() {
    // Just exercise the symbol; M2 pinned its contract.
    let _ = ffi_last_error_message();
}
