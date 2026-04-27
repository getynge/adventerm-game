use crate::enemies::{self, EnemyTickOutcome};
use crate::event::EventBus;
use crate::events::PlayerMoved;
use crate::game::{GameState, ENEMY_RNG_SALT};
use crate::registry::{EventHandler, Registry};

/// Subscribes to [`PlayerMoved`]. Each player step kicks one enemy AI tick
/// in the room the player is currently in; if any enemy ends adjacent to
/// the player, the engaging entity is stashed on `GameState` so the
/// binary's encounter check picks it up after dispatch returns.
#[derive(Debug, Clone, Copy, Default)]
pub struct EnemyTickHandler;

impl EventHandler<PlayerMoved> for EnemyTickHandler {
    fn handle(&self, game: &mut GameState, _event: &PlayerMoved, bus: &mut EventBus) {
        if let Some(entity) = tick_current_room(game, bus) {
            game.set_pending_encounter(entity);
        }
    }
}

pub fn register(reg: &mut Registry) {
    reg.subscribe::<PlayerMoved, _>(EnemyTickHandler);
}

/// Run one tick of the enemy AI in the player's current room. Returns the
/// engaging enemy entity if the tick produced an encounter. Per-enemy
/// position changes and engagements are emitted into `bus` as typed
/// [`crate::events::EnemyMoved`] / [`crate::events::EnemyEngaged`] events.
pub fn tick_current_room(
    game: &mut GameState,
    bus: &mut EventBus,
) -> Option<crate::ecs::EntityId> {
    let player_pos = game.player.position();
    let room_id = game.current_room;
    let seed = game.dungeon.seed;
    // Borrow split: rng on `player`, room on `dungeon.rooms` — disjoint.
    let rng = game.player.enemy_rng_mut(seed, ENEMY_RNG_SALT);
    let room = &mut game.dungeon.rooms[room_id.0 as usize];
    match enemies::tick_enemies(room, room_id, player_pos, rng, bus) {
        EnemyTickOutcome::EncounterTriggered(e) => Some(e),
        EnemyTickOutcome::Quiet => None,
    }
}
