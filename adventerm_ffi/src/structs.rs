//! Tagged-struct and plain-Copy `#[repr(C)]` mirrors of the lib's data
//! types. Every shape here is `Copy` so values cross the FFI boundary by
//! value; pointers into handle interiors are never returned.

use adventerm_lib::battle::{BattleTurn, Combatants, HpSnapshot};
use adventerm_lib::dungeon::DoorView;
use adventerm_lib::game::{DoorEvent, MoveOutcome};
use adventerm_lib::items::{ConsumeOutcome, ConsumeTarget, EquipEffect};
use adventerm_lib::lighting::{FlareSource, LightSource};
use adventerm_lib::room::TileKind;
use adventerm_lib::stats::Stats;
use adventerm_lib::Equipment;

use crate::enums::{
    CAbilityKind, CAttribute, CBattleResult, CDoorState, CEnemyKind, CItemKind,
};
use crate::error::FfiError;

const CTILE_KIND_WALL: u8 = 0;
const CTILE_KIND_FLOOR: u8 = 1;
const CTILE_KIND_DOOR: u8 = 2;

const CMOVE_BLOCKED: u8 = 0;
const CMOVE_MOVED: u8 = 1;
const CMOVE_ENCOUNTER: u8 = 2;

const CBATTLE_TURN_PLAYER: u8 = 0;
const CBATTLE_TURN_ENEMY: u8 = 1;
const CBATTLE_TURN_RESOLVED: u8 = 2;

const CCONSUME_TARGET_NONE: u8 = 0;
const CCONSUME_TARGET_ABILITY_SLOT: u8 = 1;

const CCONSUME_OUTCOME_USED: u8 = 0;
const CCONSUME_OUTCOME_LEARNED_ABILITY: u8 = 1;

/// Sentinel value written into [`CEquipmentSnapshot`] slot fields when the
/// slot is empty. `255` is unreachable as an `ItemKind` discriminant for the
/// foreseeable future (the lib has 8 variants).
pub const C_EQUIPMENT_SLOT_EMPTY: u8 = u8::MAX;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CMoveOutcome {
    /// 0 = Blocked, 1 = Moved, 2 = Encounter.
    pub tag: u8,
    pub _pad: [u8; 3],
    /// Valid iff `tag == 2`. Otherwise `0`.
    pub encounter_entity: u32,
}

impl From<MoveOutcome> for CMoveOutcome {
    fn from(o: MoveOutcome) -> Self {
        match o {
            MoveOutcome::Blocked => CMoveOutcome {
                tag: CMOVE_BLOCKED,
                _pad: [0; 3],
                encounter_entity: 0,
            },
            MoveOutcome::Moved => CMoveOutcome {
                tag: CMOVE_MOVED,
                _pad: [0; 3],
                encounter_entity: 0,
            },
            MoveOutcome::Encounter(e) => CMoveOutcome {
                tag: CMOVE_ENCOUNTER,
                _pad: [0; 3],
                encounter_entity: e.raw(),
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CTileKind {
    /// 0 = Wall, 1 = Floor, 2 = Door.
    pub tag: u8,
    pub _pad: [u8; 3],
    /// Valid iff `tag == 2`. Otherwise `0`.
    pub door_id: u32,
}

impl From<TileKind> for CTileKind {
    fn from(k: TileKind) -> Self {
        match k {
            TileKind::Wall => CTileKind {
                tag: CTILE_KIND_WALL,
                _pad: [0; 3],
                door_id: 0,
            },
            TileKind::Floor => CTileKind {
                tag: CTILE_KIND_FLOOR,
                _pad: [0; 3],
                door_id: 0,
            },
            TileKind::Door(id) => CTileKind {
                tag: CTILE_KIND_DOOR,
                _pad: [0; 3],
                door_id: id.0.raw(),
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CBattleTurn {
    /// 0 = Player, 1 = Enemy, 2 = Resolved.
    pub tag: u8,
    /// Valid iff `tag == 2`. Otherwise `0`. Holds a [`CBattleResult`].
    pub result: u8,
    pub _pad: [u8; 2],
}

impl From<BattleTurn> for CBattleTurn {
    fn from(t: BattleTurn) -> Self {
        match t {
            BattleTurn::Player => CBattleTurn {
                tag: CBATTLE_TURN_PLAYER,
                result: 0,
                _pad: [0; 2],
            },
            BattleTurn::Enemy => CBattleTurn {
                tag: CBATTLE_TURN_ENEMY,
                result: 0,
                _pad: [0; 2],
            },
            BattleTurn::Resolved(r) => CBattleTurn {
                tag: CBATTLE_TURN_RESOLVED,
                result: CBattleResult::from(r) as u8,
                _pad: [0; 2],
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CConsumeTarget {
    /// 0 = None, 1 = AbilitySlot.
    pub tag: u8,
    pub _pad: [u8; 3],
    /// Valid iff `tag == 1`. Otherwise `0`.
    pub slot: u32,
}

impl From<ConsumeTarget> for CConsumeTarget {
    fn from(t: ConsumeTarget) -> Self {
        match t {
            ConsumeTarget::None => CConsumeTarget {
                tag: CCONSUME_TARGET_NONE,
                _pad: [0; 3],
                slot: 0,
            },
            ConsumeTarget::AbilitySlot(s) => CConsumeTarget {
                tag: CCONSUME_TARGET_ABILITY_SLOT,
                _pad: [0; 3],
                slot: s as u32,
            },
            // Lib enum is `#[non_exhaustive]`; reject any unknown variant
            // rather than silently downgrading to None.
            _ => CConsumeTarget {
                tag: u8::MAX,
                _pad: [0; 3],
                slot: 0,
            },
        }
    }
}

impl TryFrom<CConsumeTarget> for ConsumeTarget {
    type Error = FfiError;
    fn try_from(c: CConsumeTarget) -> Result<Self, Self::Error> {
        match c.tag {
            CCONSUME_TARGET_NONE => Ok(ConsumeTarget::None),
            CCONSUME_TARGET_ABILITY_SLOT => Ok(ConsumeTarget::AbilitySlot(c.slot as usize)),
            _ => Err(FfiError::OutOfRange),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CConsumeOutcome {
    /// 0 = Used (no payload), 1 = LearnedAbility (payload valid).
    pub tag: u8,
    /// [`CAbilityKind`] discriminant when `tag == 1`. Otherwise `0`.
    pub kind: u8,
    pub _pad: [u8; 2],
    /// Active-ability slot when `tag == 1`. Otherwise `0`.
    pub slot: u32,
}

impl From<ConsumeOutcome> for CConsumeOutcome {
    fn from(o: ConsumeOutcome) -> Self {
        match o {
            ConsumeOutcome::LearnedAbility { kind, slot } => CConsumeOutcome {
                tag: CCONSUME_OUTCOME_LEARNED_ABILITY,
                kind: CAbilityKind::from(kind) as u8,
                _pad: [0; 2],
                slot: slot as u32,
            },
            // Lib enum is `#[non_exhaustive]`. Tag 0 is reserved for a future
            // payload-less "Used" outcome, hence the explicit fallback.
            _ => CConsumeOutcome {
                tag: CCONSUME_OUTCOME_USED,
                kind: 0,
                _pad: [0; 2],
                slot: 0,
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CStats {
    pub health: u8,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    /// [`CAttribute`] discriminant.
    pub attribute: u8,
}

impl From<Stats> for CStats {
    fn from(s: Stats) -> Self {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CHpSnapshot {
    pub player: u8,
    pub enemy: u8,
}

impl From<HpSnapshot> for CHpSnapshot {
    fn from(s: HpSnapshot) -> Self {
        CHpSnapshot {
            player: s.player,
            enemy: s.enemy,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CCombatants {
    pub enemy_entity: u32,
    pub enemy_room: u32,
}

impl From<Combatants> for CCombatants {
    fn from(c: Combatants) -> Self {
        CCombatants {
            enemy_entity: c.enemy_entity.raw(),
            enemy_room: c.enemy_room.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CEquipEffect {
    pub attack: i8,
    pub defense: i8,
    pub speed: i8,
    pub vision_multiplier: u8,
}

impl From<EquipEffect> for CEquipEffect {
    fn from(e: EquipEffect) -> Self {
        CEquipEffect {
            attack: e.attack,
            defense: e.defense,
            speed: e.speed,
            vision_multiplier: e.vision_multiplier,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CDoorEvent {
    pub from: u32,
    pub to: u32,
    pub new_room: u32,
}

impl From<DoorEvent> for CDoorEvent {
    fn from(e: DoorEvent) -> Self {
        CDoorEvent {
            from: e.from.0.raw(),
            to: e.to.0.raw(),
            new_room: e.new_room.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CDoorView {
    pub door_id: u32,
    /// [`CDoorState`] discriminant.
    pub state: u8,
    pub _pad: [u8; 3],
    pub from_room: u32,
    pub to_room: u32,
    pub x: u32,
    pub y: u32,
}

impl From<DoorView> for CDoorView {
    fn from(v: DoorView) -> Self {
        CDoorView {
            door_id: v.id.0.raw(),
            state: CDoorState::from(v.state) as u8,
            _pad: [0; 3],
            from_room: v.owner.0,
            to_room: v.leads_to.0.raw(),
            x: v.pos.0 as u32,
            y: v.pos.1 as u32,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CEnemyView {
    pub entity: u32,
    /// [`CEnemyKind`] discriminant.
    pub kind: u8,
    pub _pad: [u8; 3],
    pub x: u32,
    pub y: u32,
}

/// Source data the FFI assembles into a [`CEnemyView`]: the enemy's entity,
/// kind, and tile coordinates.
impl CEnemyView {
    #[allow(dead_code)] // Used by M5 iteration; lands here so M3 tests can fabricate views.
    pub(crate) fn new(
        entity: adventerm_lib::EntityId,
        pos: (usize, usize),
        kind: adventerm_lib::EnemyKind,
    ) -> Self {
        CEnemyView {
            entity: entity.raw(),
            kind: CEnemyKind::from(kind) as u8,
            _pad: [0; 3],
            x: pos.0 as u32,
            y: pos.1 as u32,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CLightSource {
    pub entity: u32,
    pub x: u32,
    pub y: u32,
    pub radius: u8,
    pub _pad: [u8; 3],
}

impl CLightSource {
    #[allow(dead_code)] // Used by M5 iteration.
    pub(crate) fn new(
        entity: adventerm_lib::EntityId,
        pos: (usize, usize),
        src: &LightSource,
    ) -> Self {
        CLightSource {
            entity: entity.raw(),
            x: pos.0 as u32,
            y: pos.1 as u32,
            radius: src.radius,
            _pad: [0; 3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CFlareSource {
    pub entity: u32,
    pub x: u32,
    pub y: u32,
}

impl CFlareSource {
    #[allow(dead_code)] // Used by M5 iteration.
    pub(crate) fn new(
        entity: adventerm_lib::EntityId,
        pos: (usize, usize),
        _src: &FlareSource,
    ) -> Self {
        CFlareSource {
            entity: entity.raw(),
            x: pos.0 as u32,
            y: pos.1 as u32,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CEquipmentSnapshot {
    /// Each field is [`C_EQUIPMENT_SLOT_EMPTY`] when empty, or a
    /// [`CItemKind`] discriminant otherwise.
    pub head: u8,
    pub torso: u8,
    pub arms: u8,
    pub legs: u8,
    pub feet: u8,
}

impl Default for CEquipmentSnapshot {
    fn default() -> Self {
        CEquipmentSnapshot {
            head: C_EQUIPMENT_SLOT_EMPTY,
            torso: C_EQUIPMENT_SLOT_EMPTY,
            arms: C_EQUIPMENT_SLOT_EMPTY,
            legs: C_EQUIPMENT_SLOT_EMPTY,
            feet: C_EQUIPMENT_SLOT_EMPTY,
        }
    }
}

impl From<&Equipment> for CEquipmentSnapshot {
    fn from(eq: &Equipment) -> Self {
        let pack = |o: Option<adventerm_lib::ItemKind>| {
            o.map(|k| CItemKind::from(k) as u8)
                .unwrap_or(C_EQUIPMENT_SLOT_EMPTY)
        };
        CEquipmentSnapshot {
            head: pack(eq.head),
            torso: pack(eq.torso),
            arms: pack(eq.arms),
            legs: pack(eq.legs),
            feet: pack(eq.feet),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipment_snapshot_default_is_all_empty() {
        let snap = CEquipmentSnapshot::default();
        for byte in [snap.head, snap.torso, snap.arms, snap.legs, snap.feet] {
            assert_eq!(byte, C_EQUIPMENT_SLOT_EMPTY);
        }
    }
}
