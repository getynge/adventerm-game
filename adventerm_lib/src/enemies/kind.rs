use serde::{Deserialize, Serialize};

use crate::stats::{Attribute, Stats};

/// Concrete enemy identifier. Each variant has a sibling AI module under
/// `enemies/` plus an arm in `ai::enemy_behavior_for`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnemyKind {
    Slime,
}

impl EnemyKind {
    pub fn name(self) -> &'static str {
        match self {
            EnemyKind::Slime => "Slime",
        }
    }

    pub fn glyph(self) -> char {
        match self {
            EnemyKind::Slime => 's',
        }
    }

    pub fn base_stats(self) -> Stats {
        match self {
            EnemyKind::Slime => Stats::new(12, 6, 2, 4, Attribute::Water),
        }
    }
}
