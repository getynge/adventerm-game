pub mod engine;

use serde::{Deserialize, Serialize};

use crate::ecs::EntityId;
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

/// All transient state for a single battle. Created by `start_battle` and
/// owned by `Screen::Battle`. Lives outside of `GameState` so a battle
/// in progress is naturally lost on save (matching the existing pattern
/// where the player can only save from `Playing`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleState {
    pub enemy_id: EntityId,
    pub enemy_room: RoomId,
    pub player_cur_hp: u8,
    pub enemy_cur_hp: u8,
    pub turn: BattleTurn,
    pub log: Vec<String>,
}

impl BattleState {
    pub fn is_resolved(&self) -> bool {
        matches!(self.turn, BattleTurn::Resolved(_))
    }

    pub fn result(&self) -> Option<BattleResult> {
        match self.turn {
            BattleTurn::Resolved(r) => Some(r),
            _ => None,
        }
    }

    /// Append a log line, trimming the oldest if we exceed `BATTLE_LOG_LINES`.
    pub(crate) fn push_log(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        if self.log.len() > BATTLE_LOG_LINES {
            let drop = self.log.len() - BATTLE_LOG_LINES;
            self.log.drain(0..drop);
        }
    }
}
