use crate::action::Action;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::ItemPickedUp;
use crate::game::GameState;
use crate::items::ItemKind;

/// "Pick up one item from the actor's tile." Emits [`ItemPickedUp`] on
/// success.
#[derive(Debug, Clone, Copy)]
pub struct PickUpAction;

impl Action for PickUpAction {
    type Outcome = Option<ItemKind>;

    fn perform(
        self,
        game: &mut GameState,
        _actor: EntityId,
        bus: &mut EventBus,
    ) -> Option<ItemKind> {
        let pos = game.player.position();
        let room = game.dungeon.room_mut(game.current_room);
        let kind = room.items.take_at(&mut room.world, pos)?;
        game.player.inventory_push(kind);
        bus.emit(ItemPickedUp { kind, pos });
        Some(kind)
    }
}
