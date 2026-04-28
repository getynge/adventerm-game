# 03 — Enum and struct shims

Covers **M3** (enum + struct shims + scalar queries). All FFI mirror types live in `adventerm_ffi/src/{enums.rs,structs.rs}`. **Zero `#[repr(C)]` is added to `adventerm_lib`.**

## Rule: mirror, don't annotate

For every Rust type that needs to cross the FFI boundary, define a parallel `Cxxx` type in the FFI crate with `#[repr(u8)]` (enums) or `#[repr(C)]` (structs), and provide `From`/`TryFrom` between the lib and FFI types. The lib type stays untouched.

## Unit-only enums (`#[repr(u8)]`)

```rust
// adventerm_ffi/src/enums.rs

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CDirection { Up = 0, Down = 1, Left = 2, Right = 3 }

impl From<adventerm_lib::Direction> for CDirection {
    fn from(d: adventerm_lib::Direction) -> Self {
        use adventerm_lib::Direction as D;
        match d {
            D::Up => CDirection::Up,
            D::Down => CDirection::Down,
            D::Left => CDirection::Left,
            D::Right => CDirection::Right,
        }
    }
}

impl TryFrom<u8> for CDirection {
    type Error = crate::FfiError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(CDirection::Up),
            1 => Ok(CDirection::Down),
            2 => Ok(CDirection::Left),
            3 => Ok(CDirection::Right),
            _ => Err(crate::FfiError::OutOfRange),
        }
    }
}

impl From<CDirection> for adventerm_lib::Direction {
    fn from(d: CDirection) -> Self {
        use adventerm_lib::Direction as D;
        match d {
            CDirection::Up => D::Up,
            CDirection::Down => D::Down,
            CDirection::Left => D::Left,
            CDirection::Right => D::Right,
        }
    }
}
```

**Full set of unit enums to mirror** (one per the lib's enums; bodies follow the same pattern):

| FFI type | Lib type | Variants | Notes |
|----------|----------|----------|-------|
| `CDirection` | `Direction` | Up=0, Down=1, Left=2, Right=3 | |
| `CTile` | `Tile` | Wall=0, Floor=1, Door=2, Player=3 | |
| `CAttribute` | `Attribute` | Fire=0, Water=1, Earth=2, Light=3, Dark=4 | |
| `CEquipSlot` | `EquipSlot` | Head=0, Torso=1, Arms=2, Legs=3, Feet=4 | |
| `CItemKind` | `ItemKind` | Torch=0, Flare=1, Goggles=2, Shirt=3, Gauntlets=4, Trousers=5, Boots=6, ScrollOfFire=7 | Add new variants by APPENDING only |
| `CEnemyKind` | `EnemyKind` | Slime=0 | |
| `CAbilityKind` | `AbilityKind` | Impact=0, Fireball=1 | |
| `CPassiveKind` | `PassiveKind` | (currently empty) | Define with no variants until a passive ships |
| `CBattleResult` | `BattleResult` | Victory=0, Defeat=1, Fled=2 | |
| `CPlaceOutcome` | `PlaceOutcome` | TorchPlaced=0, FlarePlaced=1 | |
| `CConsumeIntent` | `ConsumeIntent` | Immediate=0, PickAbilitySlot=1 | |
| `CDoorState` | `DoorState` | Open=0, Closed=1 | |
| `CActorKind` | `ActorKind` | Player=0, Enemy=1 | |

**Stability rule:** discriminant values are wire-stable. Reordering or repurposing an existing value breaks every Swift consumer compiled against the old header. Always APPEND.

## Data-carrying enums → tagged C structs

`#[repr(C) union` is forbidden — Swift bridges them awkwardly. Use tagged structs: a discriminant byte plus union'd payload fields, with the rule "fields are valid only when their discriminant says so."

```rust
// adventerm_ffi/src/structs.rs

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CMoveOutcome {
    pub tag: u8,                 // 0 = Blocked, 1 = Moved, 2 = Encounter
    pub _pad: [u8; 3],
    pub encounter_entity: u32,   // valid iff tag == 2; else 0
}

impl From<adventerm_lib::game::MoveOutcome> for CMoveOutcome {
    fn from(o: adventerm_lib::game::MoveOutcome) -> Self {
        use adventerm_lib::game::MoveOutcome as M;
        match o {
            M::Blocked => CMoveOutcome { tag: 0, _pad: [0; 3], encounter_entity: 0 },
            M::Moved => CMoveOutcome { tag: 1, _pad: [0; 3], encounter_entity: 0 },
            M::Encounter(e) => CMoveOutcome { tag: 2, _pad: [0; 3], encounter_entity: e.raw() },
        }
    }
}
```

**Full set of tagged structs:**

| FFI type | Lib type | Tag values | Payload |
|----------|----------|------------|---------|
| `CMoveOutcome` | `MoveOutcome` | 0=Blocked, 1=Moved, 2=Encounter | `encounter_entity: u32` (valid when tag=2) |
| `CTileKind` | `room::TileKind` | 0=Wall, 1=Floor, 2=Door | `door_id: u32` (valid when tag=2) |
| `CBattleTurn` | `battle::BattleTurn` | 0=Player, 1=Enemy, 2=Resolved | `result: u8` (CBattleResult, valid when tag=2) |
| `CConsumeTarget` | `items::ConsumeTarget` | 0=None, 1=AbilitySlot | `slot: u32` (valid when tag=1) |
| `CConsumeOutcome` | `items::ConsumeOutcome` | 0=Used, 1=LearnedAbility | `kind: u8` (CAbilityKind), `slot: u32` (both valid when tag=1) |

`From` impls follow the `CMoveOutcome` template. For CtoLib direction, only `CConsumeTarget` needs reverse conversion (it's an action input):

```rust
impl TryFrom<CConsumeTarget> for adventerm_lib::items::ConsumeTarget {
    type Error = crate::FfiError;
    fn try_from(c: CConsumeTarget) -> Result<Self, Self::Error> {
        match c.tag {
            0 => Ok(adventerm_lib::items::ConsumeTarget::None),
            1 => Ok(adventerm_lib::items::ConsumeTarget::AbilitySlot(c.slot as usize)),
            _ => Err(crate::FfiError::OutOfRange),
        }
    }
}
```

## Plain-Copy structs (`#[repr(C)]` mirror)

For Copy structs in the lib, define a parallel `#[repr(C)]` and translate via `From`:

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CStats {
    pub health: u8,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub attribute: u8,  // CAttribute
}

impl From<adventerm_lib::Stats> for CStats {
    fn from(s: adventerm_lib::Stats) -> Self {
        CStats {
            health: s.health,
            attack: s.attack,
            defense: s.defense,
            speed: s.speed,
            attribute: CAttribute::from(s.attribute) as u8,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CHpSnapshot {
    pub player: u8,
    pub enemy: u8,
}

impl From<adventerm_lib::battle::HpSnapshot> for CHpSnapshot {
    fn from(s: adventerm_lib::battle::HpSnapshot) -> Self {
        CHpSnapshot { player: s.player, enemy: s.enemy }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CCombatants {
    pub enemy_entity: u32,
    pub enemy_room: u32,
}

impl From<adventerm_lib::battle::Combatants> for CCombatants {
    fn from(c: adventerm_lib::battle::Combatants) -> Self {
        CCombatants {
            enemy_entity: c.enemy_entity.raw(),
            enemy_room: c.enemy_room.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CEquipEffect {
    pub attack: i8,
    pub defense: i8,
    pub speed: i8,
    pub vision_multiplier: u8,
}

impl From<adventerm_lib::items::EquipEffect> for CEquipEffect { /* trivial */ }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CDoorEvent {
    pub from: u32,    // DoorId
    pub to: u32,
    pub new_room: u32,
}

impl From<adventerm_lib::game::DoorEvent> for CDoorEvent { /* trivial */ }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CDoorView {
    pub door_id: u32,
    pub state: u8,         // CDoorState
    pub from_room: u32,
    pub to_room: u32,
    pub x: u32,
    pub y: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CEnemyView {
    pub entity: u32,
    pub kind: u8,          // CEnemyKind
    pub x: u32,
    pub y: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CLightSource {
    pub entity: u32,
    pub x: u32,
    pub y: u32,
    pub radius: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CFlareSource {
    pub entity: u32,
    pub x: u32,
    pub y: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CEquipmentSnapshot {
    /// Each element is u8::MAX (255) when empty, or a CItemKind discriminant otherwise.
    pub head: u8,
    pub torso: u8,
    pub arms: u8,
    pub legs: u8,
    pub feet: u8,
}

impl From<&adventerm_lib::Equipment> for CEquipmentSnapshot { /* trivial */ }
```

**Why use `255` as the sentinel for empty equipment slots:** Swift's `Optional<UInt8>` doesn't bridge through C; we either need a parallel `has_X: bool` field or a sentinel. Sentinel is more compact and matches the way `Equipment::iter` already produces `Option<ItemKind>` for the consumer's convenience.

## Scalar query exports (M3 deliverable)

`adventerm_ffi/src/query.rs`:

```rust
#[no_mangle]
pub extern "C" fn game_tile_at(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out_tile: *mut u8,        // CTile
) -> i32;

#[no_mangle]
pub extern "C" fn game_terrain_at(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out_tile: *mut u8,
) -> i32;

#[no_mangle]
pub extern "C" fn game_is_visible(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_is_explored(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_player_on_door(
    handle: *const GameHandle,
    out_door_id: *mut u32,
    out_has_door: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_items_here(
    handle: *const GameHandle,
    out: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_peek_item_here(
    handle: *const GameHandle,
    out_kind: *mut u8,        // CItemKind
    out_has_item: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_effective_stats(
    handle: *const GameHandle,
    out_stats: *mut CStats,
) -> i32;

#[no_mangle]
pub extern "C" fn game_set_fullbright(handle: *mut GameHandle, on: bool) -> i32;

#[no_mangle]
pub extern "C" fn game_fullbright(handle: *const GameHandle, out: *mut bool) -> i32;

#[no_mangle]
pub extern "C" fn game_pending_encounter(
    handle: *const GameHandle,
    out_entity: *mut u32,
    out_has_encounter: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_take_pending_encounter(
    handle: *mut GameHandle,
    out_entity: *mut u32,
    out_has_encounter: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn game_set_pending_encounter(
    handle: *mut GameHandle,
    entity: u32,
) -> i32;

// Static name lookups (interned, return *const c_char from OnceLock<CString>)
#[no_mangle]
pub extern "C" fn item_kind_name(kind: u8) -> *const c_char;

#[no_mangle]
pub extern "C" fn item_kind_glyph(kind: u8, out_glyph: *mut u32) -> i32;

#[no_mangle]
pub extern "C" fn enemy_kind_name(kind: u8) -> *const c_char;

#[no_mangle]
pub extern "C" fn enemy_kind_glyph(kind: u8, out_glyph: *mut u32) -> i32;

#[no_mangle]
pub extern "C" fn enemy_kind_base_stats(kind: u8, out_stats: *mut CStats) -> i32;

#[no_mangle]
pub extern "C" fn ability_kind_name(kind: u8) -> *const c_char;

#[no_mangle]
pub extern "C" fn attribute_name(attr: u8) -> *const c_char;

#[no_mangle]
pub extern "C" fn equip_slot_name(slot: u8) -> *const c_char;
```

`out_glyph` is a `u32` because `char` in Rust is a 32-bit Unicode scalar value. Swift maps `UInt32` → `Unicode.Scalar`.

## Test plan (M3)

`adventerm_ffi/tests/enum_round_trip.rs`:

```rust
fn round_trip<L, F>(variants: &[L])
where
    L: Copy + PartialEq + std::fmt::Debug,
    F: Copy + std::fmt::Debug + From<L> + TryInto<L> + Into<u8>,
    <F as TryInto<L>>::Error: std::fmt::Debug,
{
    for &v in variants {
        let f: F = v.into();
        let back: L = f.try_into().unwrap();
        assert_eq!(back, v);
    }
}

#[test]
fn directions_round_trip() {
    use adventerm_lib::Direction::*;
    for d in [Up, Down, Left, Right] {
        let c: CDirection = d.into();
        let back: adventerm_lib::Direction = c.into();
        assert_eq!(back as u8, d as u8);
        let c2: CDirection = (c as u8).try_into().unwrap();
        assert_eq!(c, c2);
    }
}

// One test per enum: directions, attributes, equip_slots, item_kinds,
// enemy_kinds, ability_kinds, battle_results, place_outcomes,
// consume_intents, door_states, actor_kinds, tiles.
```

`adventerm_ffi/tests/queries.rs`:

```rust
#[test]
fn player_pos_matches_direct_read() {
    let h = adventerm_ffi::game_new_seeded(42);
    let mut x = 0usize;
    let mut y = 0usize;
    adventerm_ffi::game_player_pos(h, &mut x, &mut y);

    // Compare against direct Rust API for the same seed
    let game = adventerm_lib::GameState::new_seeded(42);
    let (rx, ry) = game.player_pos();
    assert_eq!((x, y), (rx, ry));
    adventerm_ffi::game_free(h);
}

#[test]
fn effective_stats_match_direct_read() {
    // Same pattern for game_effective_stats vs. GameState::effective_stats
}

#[test]
fn item_name_pointer_is_stable() {
    let p1 = adventerm_ffi::item_kind_name(0);
    let p2 = adventerm_ffi::item_kind_name(0);
    assert_eq!(p1, p2);    // pointer equality — interning works
}
```

## Miri scope (M3)

```bash
cargo +nightly miri test -p adventerm_ffi --test enum_round_trip --test queries
```

Catches UB in pointer dereferences (`out_*` writes) and in the conversion code. Skip iteration tests under Miri (too slow); rely on the smoke test for those.
