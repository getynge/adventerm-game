use crate::action::Action;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::ItemPlaced;
use crate::game::GameState;
use crate::items::{self, PlaceCtx, PlaceOutcome};

/// "Place the inventory item at `slot` onto the world." Dispatches via
/// the item kind's behavior (`items::behavior_for`) — this action never
/// matches on `ItemKind`. Emits [`ItemPlaced`] on success; the bus drain
/// runs the visibility refresh.
#[derive(Debug, Clone, Copy)]
pub struct PlaceItemAction {
    pub slot: usize,
}

impl Action for PlaceItemAction {
    type Outcome = Option<PlaceOutcome>;

    fn perform(
        self,
        game: &mut GameState,
        _actor: EntityId,
        bus: &mut EventBus,
    ) -> Option<PlaceOutcome> {
        let kind = game.player.inventory_get(self.slot)?;
        let player_pos = game.player.position();
        let room = game.dungeon.room_mut(game.current_room);
        let mut ctx = PlaceCtx {
            player_pos,
            world: &mut room.world,
            lighting: &mut room.lighting,
        };
        let outcome = items::behavior_for(kind).on_place(&mut ctx);
        game.player.inventory_remove(self.slot);
        bus.emit(ItemPlaced {
            kind,
            pos: player_pos,
            outcome,
        });
        Some(outcome)
    }
}
