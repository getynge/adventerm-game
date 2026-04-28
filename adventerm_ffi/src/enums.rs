//! Unit-only `#[repr(u8)]` mirrors of the lib's plain enums.
//!
//! Each `Cxxx` here is wire-stable: discriminants are explicit (`= N`) and
//! must only ever be appended. `From<lib>` is total; `TryFrom<u8>` is the
//! reverse for FFI inputs and rejects unknown discriminants with
//! [`crate::FfiError::OutOfRange`].

use adventerm_lib::abilities::{AbilityKind, PassiveKind};
use adventerm_lib::battle::BattleResult;
use adventerm_lib::dungeon::DoorState;
use adventerm_lib::enemies::EnemyKind;
use adventerm_lib::game::PlaceOutcome;
use adventerm_lib::items::{ConsumeIntent, EquipSlot};
use adventerm_lib::registry::ActorKind;
use adventerm_lib::stats::Attribute;
use adventerm_lib::world::{Direction, Tile};
use adventerm_lib::ItemKind;

use crate::error::FfiError;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CDirection {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

impl From<Direction> for CDirection {
    fn from(d: Direction) -> Self {
        match d {
            Direction::Up => CDirection::Up,
            Direction::Down => CDirection::Down,
            Direction::Left => CDirection::Left,
            Direction::Right => CDirection::Right,
        }
    }
}

impl From<CDirection> for Direction {
    fn from(d: CDirection) -> Self {
        match d {
            CDirection::Up => Direction::Up,
            CDirection::Down => Direction::Down,
            CDirection::Left => Direction::Left,
            CDirection::Right => Direction::Right,
        }
    }
}

impl TryFrom<u8> for CDirection {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CDirection::Up),
            1 => Ok(CDirection::Down),
            2 => Ok(CDirection::Left),
            3 => Ok(CDirection::Right),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CTile {
    Wall = 0,
    Floor = 1,
    Door = 2,
    Player = 3,
}

impl From<Tile> for CTile {
    fn from(t: Tile) -> Self {
        match t {
            Tile::Wall => CTile::Wall,
            Tile::Floor => CTile::Floor,
            Tile::Door => CTile::Door,
            Tile::Player => CTile::Player,
        }
    }
}

impl TryFrom<u8> for CTile {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CTile::Wall),
            1 => Ok(CTile::Floor),
            2 => Ok(CTile::Door),
            3 => Ok(CTile::Player),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CAttribute {
    Fire = 0,
    Water = 1,
    Earth = 2,
    Light = 3,
    Dark = 4,
}

impl From<Attribute> for CAttribute {
    fn from(a: Attribute) -> Self {
        match a {
            Attribute::Fire => CAttribute::Fire,
            Attribute::Water => CAttribute::Water,
            Attribute::Earth => CAttribute::Earth,
            Attribute::Light => CAttribute::Light,
            Attribute::Dark => CAttribute::Dark,
        }
    }
}

impl From<CAttribute> for Attribute {
    fn from(a: CAttribute) -> Self {
        match a {
            CAttribute::Fire => Attribute::Fire,
            CAttribute::Water => Attribute::Water,
            CAttribute::Earth => Attribute::Earth,
            CAttribute::Light => Attribute::Light,
            CAttribute::Dark => Attribute::Dark,
        }
    }
}

impl TryFrom<u8> for CAttribute {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CAttribute::Fire),
            1 => Ok(CAttribute::Water),
            2 => Ok(CAttribute::Earth),
            3 => Ok(CAttribute::Light),
            4 => Ok(CAttribute::Dark),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CEquipSlot {
    Head = 0,
    Torso = 1,
    Arms = 2,
    Legs = 3,
    Feet = 4,
}

impl From<EquipSlot> for CEquipSlot {
    fn from(s: EquipSlot) -> Self {
        match s {
            EquipSlot::Head => CEquipSlot::Head,
            EquipSlot::Torso => CEquipSlot::Torso,
            EquipSlot::Arms => CEquipSlot::Arms,
            EquipSlot::Legs => CEquipSlot::Legs,
            EquipSlot::Feet => CEquipSlot::Feet,
        }
    }
}

impl From<CEquipSlot> for EquipSlot {
    fn from(s: CEquipSlot) -> Self {
        match s {
            CEquipSlot::Head => EquipSlot::Head,
            CEquipSlot::Torso => EquipSlot::Torso,
            CEquipSlot::Arms => EquipSlot::Arms,
            CEquipSlot::Legs => EquipSlot::Legs,
            CEquipSlot::Feet => EquipSlot::Feet,
        }
    }
}

impl TryFrom<u8> for CEquipSlot {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CEquipSlot::Head),
            1 => Ok(CEquipSlot::Torso),
            2 => Ok(CEquipSlot::Arms),
            3 => Ok(CEquipSlot::Legs),
            4 => Ok(CEquipSlot::Feet),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CItemKind {
    Torch = 0,
    Flare = 1,
    Goggles = 2,
    Shirt = 3,
    Gauntlets = 4,
    Trousers = 5,
    Boots = 6,
    ScrollOfFire = 7,
}

impl From<ItemKind> for CItemKind {
    fn from(k: ItemKind) -> Self {
        match k {
            ItemKind::Torch => CItemKind::Torch,
            ItemKind::Flare => CItemKind::Flare,
            ItemKind::Goggles => CItemKind::Goggles,
            ItemKind::Shirt => CItemKind::Shirt,
            ItemKind::Gauntlets => CItemKind::Gauntlets,
            ItemKind::Trousers => CItemKind::Trousers,
            ItemKind::Boots => CItemKind::Boots,
            ItemKind::ScrollOfFire => CItemKind::ScrollOfFire,
        }
    }
}

impl From<CItemKind> for ItemKind {
    fn from(k: CItemKind) -> Self {
        match k {
            CItemKind::Torch => ItemKind::Torch,
            CItemKind::Flare => ItemKind::Flare,
            CItemKind::Goggles => ItemKind::Goggles,
            CItemKind::Shirt => ItemKind::Shirt,
            CItemKind::Gauntlets => ItemKind::Gauntlets,
            CItemKind::Trousers => ItemKind::Trousers,
            CItemKind::Boots => ItemKind::Boots,
            CItemKind::ScrollOfFire => ItemKind::ScrollOfFire,
        }
    }
}

impl TryFrom<u8> for CItemKind {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CItemKind::Torch),
            1 => Ok(CItemKind::Flare),
            2 => Ok(CItemKind::Goggles),
            3 => Ok(CItemKind::Shirt),
            4 => Ok(CItemKind::Gauntlets),
            5 => Ok(CItemKind::Trousers),
            6 => Ok(CItemKind::Boots),
            7 => Ok(CItemKind::ScrollOfFire),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CEnemyKind {
    Slime = 0,
}

impl From<EnemyKind> for CEnemyKind {
    fn from(k: EnemyKind) -> Self {
        match k {
            EnemyKind::Slime => CEnemyKind::Slime,
        }
    }
}

impl From<CEnemyKind> for EnemyKind {
    fn from(k: CEnemyKind) -> Self {
        match k {
            CEnemyKind::Slime => EnemyKind::Slime,
        }
    }
}

impl TryFrom<u8> for CEnemyKind {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CEnemyKind::Slime),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CAbilityKind {
    Impact = 0,
    Fireball = 1,
}

impl From<AbilityKind> for CAbilityKind {
    fn from(k: AbilityKind) -> Self {
        match k {
            AbilityKind::Impact => CAbilityKind::Impact,
            AbilityKind::Fireball => CAbilityKind::Fireball,
        }
    }
}

impl From<CAbilityKind> for AbilityKind {
    fn from(k: CAbilityKind) -> Self {
        match k {
            CAbilityKind::Impact => AbilityKind::Impact,
            CAbilityKind::Fireball => AbilityKind::Fireball,
        }
    }
}

impl TryFrom<u8> for CAbilityKind {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CAbilityKind::Impact),
            1 => Ok(CAbilityKind::Fireball),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

/// Empty mirror: lib's [`PassiveKind`] is uninhabited until the first passive
/// ships. The mirror has no variants either, so it cannot be `#[repr(u8)]`
/// (Rust forbids that on zero-variant enums); discriminants are reserved
/// implicitly by appending here when the first passive lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CPassiveKind {}

impl From<PassiveKind> for CPassiveKind {
    fn from(k: PassiveKind) -> Self {
        match k {}
    }
}

impl TryFrom<u8> for CPassiveKind {
    type Error = FfiError;
    fn try_from(_: u8) -> Result<Self, Self::Error> {
        Err(FfiError::OutOfRange)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CBattleResult {
    Victory = 0,
    Defeat = 1,
    Fled = 2,
}

impl From<BattleResult> for CBattleResult {
    fn from(r: BattleResult) -> Self {
        match r {
            BattleResult::Victory => CBattleResult::Victory,
            BattleResult::Defeat => CBattleResult::Defeat,
            BattleResult::Fled => CBattleResult::Fled,
        }
    }
}

impl From<CBattleResult> for BattleResult {
    fn from(r: CBattleResult) -> Self {
        match r {
            CBattleResult::Victory => BattleResult::Victory,
            CBattleResult::Defeat => BattleResult::Defeat,
            CBattleResult::Fled => BattleResult::Fled,
        }
    }
}

impl TryFrom<u8> for CBattleResult {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CBattleResult::Victory),
            1 => Ok(CBattleResult::Defeat),
            2 => Ok(CBattleResult::Fled),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CPlaceOutcome {
    TorchPlaced = 0,
    FlarePlaced = 1,
}

impl From<PlaceOutcome> for CPlaceOutcome {
    fn from(o: PlaceOutcome) -> Self {
        match o {
            PlaceOutcome::TorchPlaced => CPlaceOutcome::TorchPlaced,
            PlaceOutcome::FlarePlaced => CPlaceOutcome::FlarePlaced,
        }
    }
}

impl From<CPlaceOutcome> for PlaceOutcome {
    fn from(o: CPlaceOutcome) -> Self {
        match o {
            CPlaceOutcome::TorchPlaced => PlaceOutcome::TorchPlaced,
            CPlaceOutcome::FlarePlaced => PlaceOutcome::FlarePlaced,
        }
    }
}

impl TryFrom<u8> for CPlaceOutcome {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CPlaceOutcome::TorchPlaced),
            1 => Ok(CPlaceOutcome::FlarePlaced),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CConsumeIntent {
    Immediate = 0,
    PickAbilitySlot = 1,
}

impl From<ConsumeIntent> for CConsumeIntent {
    fn from(i: ConsumeIntent) -> Self {
        // Lib enum is `#[non_exhaustive]`. Future variants must round-trip
        // through this match — adding a variant without a new FFI arm is a
        // compile-time miss only on the lib side, so we fall back to
        // `Immediate` as the conservative default.
        match i {
            ConsumeIntent::Immediate => CConsumeIntent::Immediate,
            ConsumeIntent::PickAbilitySlot => CConsumeIntent::PickAbilitySlot,
            _ => CConsumeIntent::Immediate,
        }
    }
}

impl From<CConsumeIntent> for ConsumeIntent {
    fn from(i: CConsumeIntent) -> Self {
        match i {
            CConsumeIntent::Immediate => ConsumeIntent::Immediate,
            CConsumeIntent::PickAbilitySlot => ConsumeIntent::PickAbilitySlot,
        }
    }
}

impl TryFrom<u8> for CConsumeIntent {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CConsumeIntent::Immediate),
            1 => Ok(CConsumeIntent::PickAbilitySlot),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

/// Lib's `DoorState` is a struct (`open: bool`, `locked: bool`). The FFI
/// collapses it into a tri-state enum: locked dominates, then closed, then
/// open. No live door is locked today, so round-tripping `Open`/`Closed`
/// preserves the `open` flag and leaves `locked` false.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CDoorState {
    Open = 0,
    Closed = 1,
    Locked = 2,
}

impl From<DoorState> for CDoorState {
    fn from(s: DoorState) -> Self {
        if s.locked {
            CDoorState::Locked
        } else if s.open {
            CDoorState::Open
        } else {
            CDoorState::Closed
        }
    }
}

impl TryFrom<u8> for CDoorState {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CDoorState::Open),
            1 => Ok(CDoorState::Closed),
            2 => Ok(CDoorState::Locked),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CActorKind {
    Player = 0,
    Enemy = 1,
}

impl From<ActorKind> for CActorKind {
    fn from(a: ActorKind) -> Self {
        match a {
            ActorKind::Player => CActorKind::Player,
            ActorKind::Enemy => CActorKind::Enemy,
        }
    }
}

impl From<CActorKind> for ActorKind {
    fn from(a: CActorKind) -> Self {
        match a {
            CActorKind::Player => ActorKind::Player,
            CActorKind::Enemy => ActorKind::Enemy,
        }
    }
}

impl TryFrom<u8> for CActorKind {
    type Error = FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CActorKind::Player),
            1 => Ok(CActorKind::Enemy),
            _ => Err(FfiError::OutOfRange),
        }
    }
}
