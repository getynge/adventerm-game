pub mod active;
pub mod fireball;
pub mod impact;
pub mod passive;

use serde::{Deserialize, Serialize};

pub use active::{ability_behavior_for, AbilityCtx, AbilityOutcome, ActiveAbility};
pub use passive::{passive_behavior_for, PassiveAbility};

/// Number of equipped active-ability slots. The player can fill any slot with
/// any ability they have learned.
pub const ABILITY_SLOTS: usize = 4;
/// Number of equipped passive-ability slots. Passives carry no variants yet,
/// so all slots will read as empty until the first `PassiveKind` is added.
pub const PASSIVE_SLOTS: usize = 4;

/// Concrete active-ability identifier. Each variant has a sibling ZST in its
/// own module under `abilities/` plus an arm in `active::ability_behavior_for`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityKind {
    Impact,
    Fireball,
}

impl AbilityKind {
    pub fn name(self) -> &'static str {
        match self {
            AbilityKind::Impact => "Impact",
            AbilityKind::Fireball => "Fireball",
        }
    }
}

/// Concrete passive-ability identifier. Empty for now — declared so the
/// scaffolding (storage, registry, trait) compiles before any variants exist.
/// Adding a passive: add a variant, add a module with a `PassiveAbility` impl,
/// and add an arm to `passive::passive_behavior_for`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassiveKind {}

impl PassiveKind {
    pub fn name(self) -> &'static str {
        match self {}
    }
}

/// Player-global ability inventory: equipped active slots, equipped passive
/// slots, and the lists of abilities the player has learned. Lives on
/// `GameState`, not `Room` — abilities travel with the player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abilities {
    #[serde(default)]
    pub active_slots: [Option<AbilityKind>; ABILITY_SLOTS],
    #[serde(default)]
    pub passive_slots: [Option<PassiveKind>; PASSIVE_SLOTS],
    #[serde(default)]
    pub learned_active: Vec<AbilityKind>,
    #[serde(default)]
    pub learned_passive: Vec<PassiveKind>,
}

impl Default for Abilities {
    fn default() -> Self {
        let mut active_slots: [Option<AbilityKind>; ABILITY_SLOTS] = [None; ABILITY_SLOTS];
        active_slots[0] = Some(AbilityKind::Impact);
        Self {
            active_slots,
            passive_slots: [None; PASSIVE_SLOTS],
            learned_active: vec![AbilityKind::Impact],
            learned_passive: Vec::new(),
        }
    }
}

impl Abilities {
    /// Iterate over the currently equipped active slots. Yields `Option` so
    /// the renderer can show empty slots in place.
    pub fn active_iter(&self) -> impl Iterator<Item = Option<AbilityKind>> + '_ {
        self.active_slots.iter().copied()
    }

    /// Iterate over the currently equipped passive slots.
    pub fn passive_iter(&self) -> impl Iterator<Item = Option<PassiveKind>> + '_ {
        self.passive_slots.iter().copied()
    }

    /// Read an equipped active-ability slot. `None` if the slot is empty or
    /// out of range.
    pub fn slot(&self, idx: usize) -> Option<AbilityKind> {
        self.active_slots.get(idx).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_grants_impact() {
        let a = Abilities::default();
        assert_eq!(a.slot(0), Some(AbilityKind::Impact));
        assert_eq!(a.slot(1), None);
        assert!(a.learned_active.contains(&AbilityKind::Impact));
        assert!(a.learned_passive.is_empty());
    }
}
