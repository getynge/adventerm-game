pub mod engine;

use serde::{Deserialize, Serialize};

use crate::ecs::{ComponentStore, EntityId, World};
use crate::room::RoomId;

pub use engine::{apply_enemy_turn, apply_player_ability, start_battle, BattleError};

/// How many lines of battle log to keep visible. The engine appends to the
/// front and trims the back so the renderer can always read `log[0]` for the
/// most-recent line.
pub const BATTLE_LOG_LINES: usize = 8;

/// Whose turn it is in the active battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleTurn {
    Player,
    Enemy,
    Resolved(BattleResult),
}

/// Terminal outcome of a battle. The screen reads this to know which
/// transition to make next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleResult {
    Victory,
    Defeat,
    Fled,
}

/// References to the live combatants. The player is implicit (always the
/// current `GameState`); the enemy needs an `EntityId` + the room hosting
/// it because despawn happens on the room's `Enemies` subsystem after the
/// battle resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combatants {
    pub enemy_entity: EntityId,
    pub enemy_room: RoomId,
}

/// HP snapshots taken at the start of the battle. The engine works against
/// these (not the live entity) until the battle resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HpSnapshot {
    pub player: u8,
    pub enemy: u8,
}

/// Battle log lines. Newest is appended at the back; capacity caps the
/// length so a long fight can't grow unbounded.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleLog {
    lines: Vec<String>,
}

impl BattleLog {
    pub fn push(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
        if self.lines.len() > BATTLE_LOG_LINES {
            let drop = self.lines.len() - BATTLE_LOG_LINES;
            self.lines.drain(0..drop);
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

/// ECS substrate for a single in-progress battle. A `Battle` always hosts
/// exactly one battle entity carrying the `BattleTurn`, `Combatants`,
/// `HpSnapshot`, and `BattleLog` components. Living in its own world keeps
/// the type symmetric with the per-room and dungeon worlds.
///
/// The world is owned by `Battle` so callers don't need to thread one in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleSubsystem {
    pub world: World,
    pub turn: ComponentStore<BattleTurn>,
    pub combatants: ComponentStore<Combatants>,
    pub hp: ComponentStore<HpSnapshot>,
    pub log: ComponentStore<BattleLog>,
}

/// Thin handle over a `BattleSubsystem` carrying the singleton battle
/// entity. Replaces the old `BattleState` plain struct: every previous
/// field access becomes a method call routed through the component stores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Battle {
    sub: BattleSubsystem,
    entity: EntityId,
}

impl Battle {
    /// Spawn a battle entity into a fresh subsystem with the given starting
    /// state. Used internally by `start_battle`.
    pub(crate) fn spawn(
        combatants: Combatants,
        hp: HpSnapshot,
        opening_line: impl Into<String>,
    ) -> Self {
        let mut sub = BattleSubsystem::default();
        let entity = sub.world.spawn();
        sub.turn.insert(entity, BattleTurn::Player);
        sub.combatants.insert(entity, combatants);
        sub.hp.insert(entity, hp);
        let mut log = BattleLog::default();
        log.push(opening_line);
        sub.log.insert(entity, log);
        Self { sub, entity }
    }

    pub fn entity(&self) -> EntityId {
        self.entity
    }

    pub fn turn(&self) -> BattleTurn {
        self.sub
            .turn
            .get(self.entity)
            .copied()
            .expect("battle entity always carries a BattleTurn")
    }

    pub fn set_turn(&mut self, turn: BattleTurn) {
        self.sub.turn.insert(self.entity, turn);
    }

    pub fn combatants(&self) -> Combatants {
        self.sub
            .combatants
            .get(self.entity)
            .copied()
            .expect("battle entity always carries Combatants")
    }

    pub fn enemy_id(&self) -> EntityId {
        self.combatants().enemy_entity
    }

    pub fn enemy_room(&self) -> RoomId {
        self.combatants().enemy_room
    }

    pub fn hp(&self) -> HpSnapshot {
        self.sub
            .hp
            .get(self.entity)
            .copied()
            .expect("battle entity always carries HpSnapshot")
    }

    pub fn player_cur_hp(&self) -> u8 {
        self.hp().player
    }

    pub fn enemy_cur_hp(&self) -> u8 {
        self.hp().enemy
    }

    pub(crate) fn set_player_hp(&mut self, value: u8) {
        let mut hp = self.hp();
        hp.player = value;
        self.sub.hp.insert(self.entity, hp);
    }

    pub(crate) fn set_enemy_hp(&mut self, value: u8) {
        let mut hp = self.hp();
        hp.enemy = value;
        self.sub.hp.insert(self.entity, hp);
    }

    pub fn log(&self) -> &[String] {
        self.sub
            .log
            .get(self.entity)
            .map(|l| l.lines())
            .unwrap_or(&[])
    }

    pub(crate) fn push_log(&mut self, line: impl Into<String>) {
        if let Some(l) = self.sub.log.get_mut(self.entity) {
            l.push(line);
        }
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self.turn(), BattleTurn::Resolved(_))
    }

    pub fn result(&self) -> Option<BattleResult> {
        match self.turn() {
            BattleTurn::Resolved(r) => Some(r),
            _ => None,
        }
    }
}
