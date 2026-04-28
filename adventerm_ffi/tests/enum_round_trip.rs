//! Round-trip every unit-only enum mirror through `From<lib>` and
//! `TryFrom<u8>` so a missing variant or shifted discriminant trips a test.

use adventerm_ffi::{
    CAbilityKind, CActorKind, CAttribute, CBattleResult, CConsumeIntent, CDirection, CDoorState,
    CEnemyKind, CEquipSlot, CItemKind, CPlaceOutcome, CTile,
};

#[test]
fn directions_round_trip() {
    use adventerm_lib::Direction;
    for d in [Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
        let c: CDirection = d.into();
        assert_eq!(CDirection::try_from(c as u8).unwrap(), c);
        let back: Direction = c.into();
        assert_eq!(back, d);
    }
}

#[test]
fn tiles_round_trip() {
    use adventerm_lib::world::Tile;
    for t in [Tile::Wall, Tile::Floor, Tile::Door, Tile::Player] {
        let c: CTile = t.into();
        assert_eq!(CTile::try_from(c as u8).unwrap(), c);
    }
}

#[test]
fn attributes_round_trip() {
    use adventerm_lib::stats::Attribute;
    for a in [
        Attribute::Fire,
        Attribute::Water,
        Attribute::Earth,
        Attribute::Light,
        Attribute::Dark,
    ] {
        let c: CAttribute = a.into();
        assert_eq!(CAttribute::try_from(c as u8).unwrap(), c);
        let back: Attribute = c.into();
        assert_eq!(back, a);
    }
}

#[test]
fn equip_slots_round_trip() {
    use adventerm_lib::items::EquipSlot;
    for s in EquipSlot::ALL {
        let c: CEquipSlot = s.into();
        assert_eq!(CEquipSlot::try_from(c as u8).unwrap(), c);
        let back: EquipSlot = c.into();
        assert_eq!(back, s);
    }
}

#[test]
fn item_kinds_round_trip() {
    use adventerm_lib::ItemKind;
    for &k in ItemKind::ALL {
        let c: CItemKind = k.into();
        assert_eq!(CItemKind::try_from(c as u8).unwrap(), c);
        let back: ItemKind = c.into();
        assert_eq!(back, k);
    }
}

#[test]
fn enemy_kinds_round_trip() {
    use adventerm_lib::enemies::EnemyKind;
    for k in [EnemyKind::Slime] {
        let c: CEnemyKind = k.into();
        assert_eq!(CEnemyKind::try_from(c as u8).unwrap(), c);
        let back: EnemyKind = c.into();
        assert_eq!(back, k);
    }
}

#[test]
fn ability_kinds_round_trip() {
    use adventerm_lib::abilities::AbilityKind;
    for k in [AbilityKind::Impact, AbilityKind::Fireball] {
        let c: CAbilityKind = k.into();
        assert_eq!(CAbilityKind::try_from(c as u8).unwrap(), c);
        let back: AbilityKind = c.into();
        assert_eq!(back, k);
    }
}

#[test]
fn battle_results_round_trip() {
    use adventerm_lib::BattleResult;
    for r in [BattleResult::Victory, BattleResult::Defeat, BattleResult::Fled] {
        let c: CBattleResult = r.into();
        assert_eq!(CBattleResult::try_from(c as u8).unwrap(), c);
        let back: BattleResult = c.into();
        assert_eq!(back, r);
    }
}

#[test]
fn place_outcomes_round_trip() {
    use adventerm_lib::PlaceOutcome;
    for o in [PlaceOutcome::TorchPlaced, PlaceOutcome::FlarePlaced] {
        let c: CPlaceOutcome = o.into();
        assert_eq!(CPlaceOutcome::try_from(c as u8).unwrap(), c);
        let back: PlaceOutcome = c.into();
        assert_eq!(back, o);
    }
}

#[test]
fn consume_intents_round_trip() {
    use adventerm_lib::items::ConsumeIntent;
    for i in [ConsumeIntent::Immediate, ConsumeIntent::PickAbilitySlot] {
        let c: CConsumeIntent = i.into();
        assert_eq!(CConsumeIntent::try_from(c as u8).unwrap(), c);
        let back: ConsumeIntent = c.into();
        assert_eq!(back, i);
    }
}

#[test]
fn door_states_collapse_struct_to_enum() {
    use adventerm_lib::dungeon::DoorState;
    let open = DoorState { open: true, locked: false };
    let closed = DoorState { open: false, locked: false };
    let locked = DoorState { open: true, locked: true };
    assert_eq!(CDoorState::from(open), CDoorState::Open);
    assert_eq!(CDoorState::from(closed), CDoorState::Closed);
    assert_eq!(CDoorState::from(locked), CDoorState::Locked);
    for c in [CDoorState::Open, CDoorState::Closed, CDoorState::Locked] {
        assert_eq!(CDoorState::try_from(c as u8).unwrap(), c);
    }
}

#[test]
fn actor_kinds_round_trip() {
    use adventerm_lib::registry::ActorKind;
    for a in [ActorKind::Player, ActorKind::Enemy] {
        let c: CActorKind = a.into();
        assert_eq!(CActorKind::try_from(c as u8).unwrap(), c);
        let back: ActorKind = c.into();
        assert_eq!(back, a);
    }
}

#[test]
fn unknown_discriminants_return_out_of_range() {
    assert!(CDirection::try_from(99).is_err());
    assert!(CTile::try_from(99).is_err());
    assert!(CAttribute::try_from(99).is_err());
    assert!(CEquipSlot::try_from(99).is_err());
    assert!(CItemKind::try_from(99).is_err());
    assert!(CEnemyKind::try_from(99).is_err());
    assert!(CAbilityKind::try_from(99).is_err());
    assert!(CBattleResult::try_from(99).is_err());
    assert!(CPlaceOutcome::try_from(99).is_err());
    assert!(CConsumeIntent::try_from(99).is_err());
    assert!(CDoorState::try_from(99).is_err());
    assert!(CActorKind::try_from(99).is_err());
}
