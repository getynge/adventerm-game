use crate::action::Action;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::ItemConsumed;
use crate::game::GameState;
use crate::items::{behavior_for, ConsumeCtx, ConsumeOutcome, ConsumeTarget};

/// "Consume the inventory item at `inventory_slot` with the supplied
/// targeting payload." The behavior decides whether `target` is the right
/// shape — on mismatch it returns `None` and the inventory is left intact.
#[derive(Debug, Clone, Copy)]
pub struct ConsumeItemAction {
    pub inventory_slot: usize,
    pub target: ConsumeTarget,
}

impl Action for ConsumeItemAction {
    type Outcome = Option<ConsumeOutcome>;

    fn perform(
        self,
        game: &mut GameState,
        _actor: EntityId,
        bus: &mut EventBus,
    ) -> Option<ConsumeOutcome> {
        let kind = game.player.inventory_get(self.inventory_slot)?;
        let entity = game.player.entity();
        // Borrow the player's component stores split across method calls
        // because `ConsumeCtx` needs simultaneous &mut on `abilities` and
        // `cur_health` (different stores) plus a shared &Stats.
        let stats = *game.player.stats.get(entity).expect("stats");
        let outcome = {
            let abilities = game.player.abilities.get_mut(entity).expect("abilities");
            let cur_health = &mut game
                .player
                .cur_health
                .get_mut(entity)
                .expect("cur_health")
                .0;
            let mut ctx = ConsumeCtx {
                abilities,
                cur_health,
                stats: &stats,
                target: self.target,
            };
            behavior_for(kind).on_consume(&mut ctx)?
        };
        game.player.inventory_remove(self.inventory_slot);
        bus.emit(ItemConsumed { kind, outcome });
        Some(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilityKind;
    use crate::action::dispatch;
    use crate::game::GameState;
    use crate::items::{ConsumeOutcome, ItemKind};

    fn fresh_game_with(kind: ItemKind) -> (GameState, EntityId) {
        let mut state = GameState::new_seeded(7);
        state.player.inventory_push(kind);
        let player = state.player.entity();
        (state, player)
    }

    #[test]
    fn consuming_scroll_overwrites_target_slot_and_learns_ability() {
        let (mut state, player) = fresh_game_with(ItemKind::ScrollOfFire);
        let outcome = dispatch(
            &mut state,
            player,
            ConsumeItemAction {
                inventory_slot: 0,
                target: ConsumeTarget::AbilitySlot(0),
            },
        );
        assert_eq!(
            outcome,
            Some(ConsumeOutcome::LearnedAbility {
                kind: AbilityKind::Fireball,
                slot: 0,
            })
        );
        assert!(state.inventory().is_empty());
        assert_eq!(state.abilities().active_slots[0], Some(AbilityKind::Fireball));
        assert!(state
            .abilities()
            .learned_active
            .contains(&AbilityKind::Fireball));
    }

    #[test]
    fn consuming_with_wrong_target_is_a_noop() {
        let (mut state, player) = fresh_game_with(ItemKind::ScrollOfFire);
        let outcome = dispatch(
            &mut state,
            player,
            ConsumeItemAction {
                inventory_slot: 0,
                target: ConsumeTarget::None,
            },
        );
        assert_eq!(outcome, None);
        assert_eq!(state.inventory(), &[ItemKind::ScrollOfFire]);
    }
}
