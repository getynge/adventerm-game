use crate::abilities::impact::ImpactAbility;
use crate::abilities::AbilityKind;
use crate::stats::Stats;

/// Damage and side-effect description returned by an active ability. The
/// caller (battle engine) is responsible for applying the damage to the
/// defender — this keeps `ActiveAbility::execute` independent of HP storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityOutcome {
    pub damage: u8,
}

/// Mutable view of the combat state handed to an `ActiveAbility`. Mirrors the
/// `PlaceCtx` pattern from the items system: only the data an ability is
/// allowed to read or change is exposed here. New surfaces (status effects,
/// random rolls) get added here when their first ability needs them.
pub struct AbilityCtx<'a> {
    pub attacker: &'a Stats,
    pub defender: &'a Stats,
}

/// Trait every ability kind implements to declare what happens when the
/// player (or, later, an enemy) uses it during battle. The battle engine
/// dispatches through `ability_behavior_for(kind)` and never matches on the
/// kind itself.
pub trait ActiveAbility {
    fn execute(&self, ctx: &AbilityCtx<'_>) -> AbilityOutcome;
}

/// The single point in the codebase that enumerates `AbilityKind`. Adding a
/// new ability: add a variant, add a new module with a behavior impl, and add
/// one arm here. Exhaustiveness is compiler-enforced.
pub fn ability_behavior_for(kind: AbilityKind) -> &'static dyn ActiveAbility {
    match kind {
        AbilityKind::Impact => &ImpactAbility,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{Attribute, Stats};

    #[test]
    fn impact_dispatch_returns_positive_damage() {
        let attacker = Stats::new(20, 10, 0, 5, Attribute::Fire);
        let defender = Stats::new(20, 0, 3, 5, Attribute::Water);
        let ctx = AbilityCtx {
            attacker: &attacker,
            defender: &defender,
        };
        let out = ability_behavior_for(AbilityKind::Impact).execute(&ctx);
        assert_eq!(out.damage, 7);
    }
}
