use crate::abilities::active::{AbilityCtx, AbilityOutcome, ActiveAbility};

/// Base fire damage. Defense halves the bite (rounded down) so a defended
/// target still feels the spell.
const BASE_DAMAGE: u8 = 8;
const MIN_DAMAGE: u8 = 1;

/// `Fireball` — the first elemental attack. Fire affinity is implicit in
/// the ability identity for now; the engine does not yet branch on
/// `Stats::attribute`.
pub struct FireballAbility;

impl ActiveAbility for FireballAbility {
    fn execute(&self, ctx: &AbilityCtx<'_>) -> AbilityOutcome {
        let mitigated = BASE_DAMAGE.saturating_sub(ctx.defender.defense / 2);
        AbilityOutcome {
            damage: mitigated.max(MIN_DAMAGE),
        }
    }
}
