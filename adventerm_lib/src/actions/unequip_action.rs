use crate::action::Action;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::ItemUnequipped;
use crate::game::GameState;
use crate::items::{EquipSlot, ItemKind};

/// "Unequip whatever is in `slot` and return it to the inventory."
/// No-op if the slot is empty.
#[derive(Debug, Clone, Copy)]
pub struct UnequipItemAction {
    pub slot: EquipSlot,
}

impl Action for UnequipItemAction {
    type Outcome = Option<ItemKind>;

    fn perform(
        self,
        game: &mut GameState,
        _actor: EntityId,
        bus: &mut EventBus,
    ) -> Option<ItemKind> {
        let kind = game.player.equipment_mut().unequip(self.slot)?;
        game.player.inventory_push(kind);
        bus.emit(ItemUnequipped {
            kind,
            slot: self.slot,
        });
        Some(kind)
    }
}
