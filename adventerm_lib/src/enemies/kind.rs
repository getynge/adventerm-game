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

    /// Reverse of `name()`: case-insensitive lookup against display names.
    pub fn from_display_name(name: &str) -> Option<Self> {
        const ALL: &[EnemyKind] = &[EnemyKind::Slime];
        ALL.iter()
            .copied()
            .find(|k| k.name().eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_round_trip() {
        for kind in [EnemyKind::Slime] {
            assert_eq!(EnemyKind::from_display_name(kind.name()), Some(kind));
            assert_eq!(
                EnemyKind::from_display_name(&kind.name().to_lowercase()),
                Some(kind)
            );
            assert_eq!(
                EnemyKind::from_display_name(&kind.name().to_uppercase()),
                Some(kind)
            );
        }
        assert_eq!(EnemyKind::from_display_name("nope"), None);
    }
}
