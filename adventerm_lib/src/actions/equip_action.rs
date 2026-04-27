use crate::action::Action;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::ItemEquipped;
use crate::game::GameState;
use crate::items::{equip_slot_of, ItemKind};

/// "Equip the inventory item at `inventory_slot`." Reads the kind's
/// equipment slot via `equip_slot_of`; if the kind is not equipment the
/// call is a no-op (the inventory UI is responsible for filtering, but
/// the action layer stays honest). Any item that was previously in the
/// target equipment slot is pushed back into inventory.
#[derive(Debug, Clone, Copy)]
pub struct EquipItemAction {
    pub inventory_slot: usize,
}

impl Action for EquipItemAction {
    type Outcome = Option<ItemKind>;

    fn perform(
        self,
        game: &mut GameState,
        _actor: EntityId,
        bus: &mut EventBus,
    ) -> Option<ItemKind> {
        let kind = game.player.inventory_get(self.inventory_slot)?;
        let slot = equip_slot_of(kind)?;
        game.player.inventory_remove(self.inventory_slot);
        let previous = game.player.equipment_mut().equip(slot, kind);
        if let Some(prev) = previous {
            game.player.inventory_push(prev);
        }
        bus.emit(ItemEquipped {
            kind,
            slot,
            previous,
        });
        Some(kind)
    }
}
