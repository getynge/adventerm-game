use crate::abilities::AbilityKind;
use crate::items::behavior::{
    ConsumeCtx, ConsumeIntent, ConsumeOutcome, ConsumeTarget, ItemBehavior,
};
use crate::items::category::ItemCategory;

/// Single-use scroll. Consuming it teaches Fireball into a player-chosen
/// active-ability slot, overwriting whatever was there. The player picks
/// the slot from the inventory UI before the action layer fires; the
/// chosen slot rides on `ConsumeCtx::target`.
pub struct ScrollOfFireBehavior;

impl ItemBehavior for ScrollOfFireBehavior {
    fn category(&self) -> ItemCategory {
        ItemCategory::Consumable
    }

    fn consume_intent(&self) -> ConsumeIntent {
        ConsumeIntent::PickAbilitySlot
    }

    fn on_consume(&self, ctx: &mut ConsumeCtx<'_>) -> Option<ConsumeOutcome> {
        let ConsumeTarget::AbilitySlot(slot) = ctx.target else {
            return None;
        };
        if slot >= ctx.abilities.active_slots.len() {
            return None;
        }
        ctx.abilities.active_slots[slot] = Some(AbilityKind::Fireball);
        if !ctx.abilities.learned_active.contains(&AbilityKind::Fireball) {
            ctx.abilities.learned_active.push(AbilityKind::Fireball);
        }
        Some(ConsumeOutcome::LearnedAbility {
            kind: AbilityKind::Fireball,
            slot,
        })
    }
}
