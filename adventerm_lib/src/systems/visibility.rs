use crate::event::EventBus;
use crate::events::{DoorTraversed, ItemPlaced, PlayerMoved};
use crate::game::GameState;
use crate::registry::{EventHandler, Registry};
use crate::visibility;

/// Subscribes to events that change what the player can see and refreshes
/// the LOS / lit bitmaps after each one. The handler is a zero-sized
/// type — registration order is the only configuration it needs.
#[derive(Debug, Clone, Copy, Default)]
pub struct VisibilityHandler;

impl EventHandler<PlayerMoved> for VisibilityHandler {
    fn handle(&self, game: &mut GameState, _event: &PlayerMoved, _bus: &mut EventBus) {
        refresh_visibility(game);
    }
}

impl EventHandler<DoorTraversed> for VisibilityHandler {
    fn handle(&self, game: &mut GameState, _event: &DoorTraversed, _bus: &mut EventBus) {
        refresh_visibility(game);
    }
}

impl EventHandler<ItemPlaced> for VisibilityHandler {
    fn handle(&self, game: &mut GameState, _event: &ItemPlaced, _bus: &mut EventBus) {
        refresh_visibility(game);
    }
}

/// Visibility module's registration entry. Subscribes the
/// [`VisibilityHandler`] ZST to every event whose effect changes what the
/// player can see (movement, room swap, light placement). Adding a new
/// "this changes the lighting" event needs nothing here besides one more
/// `subscribe::<NewEvent, _>(VisibilityHandler)` line.
pub fn register(reg: &mut Registry) {
    reg.subscribe::<PlayerMoved, _>(VisibilityHandler);
    reg.subscribe::<DoorTraversed, _>(VisibilityHandler);
    reg.subscribe::<ItemPlaced, _>(VisibilityHandler);
}

/// Recompute the player's `visible`/`lit` bitmaps for the current room and
/// merge them into the persistent explored memory. Used by the handler
/// above and called directly during `new_seeded` and post-deserialize
/// rehydration, where dispatch is not running.
pub fn refresh_visibility(game: &mut GameState) {
    let room_id = game.current_room;
    let player_pos = game.player.position();
    let room = game.dungeon.room(room_id);
    let len = room.width * room.height;

    let cache = game.player.visibility_mut();
    visibility::compute_room_lighting(room, player_pos, &mut cache.visible, &mut cache.lit);

    let visible = cache.visible.clone();
    let lit = cache.lit.clone();
    game.explored.merge_room(room_id, len, &visible, &lit);
}
