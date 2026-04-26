use crate::abilities::active::{AbilityCtx, AbilityOutcome, ActiveAbility};

/// Minimum damage dealt by Impact when defense fully cancels attack — combat
/// should never feel like a no-op when the player chose an action.
const MIN_DAMAGE: u8 = 1;

/// `Impact` — the player's starting ability. Pure damage: `attack - defense`,
/// floored at `MIN_DAMAGE`.
pub struct ImpactAbility;

impl ActiveAbility for ImpactAbility {
    fn execute(&self, ctx: &AbilityCtx<'_>) -> AbilityOutcome {
        let raw = ctx.attacker.attack.saturating_sub(ctx.defender.defense);
        AbilityOutcome {
            damage: raw.max(MIN_DAMAGE),
        }
    }
}
