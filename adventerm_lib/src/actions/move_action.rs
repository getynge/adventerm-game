use crate::action::Action;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::PlayerMoved;
use crate::game::{GameState, MoveOutcome};
use crate::world::Direction;

/// "Move the actor one tile in `direction` if the destination is walkable
/// and unoccupied." Emits [`PlayerMoved`] on a successful step. The bus
/// drain triggers visibility refresh and the enemy AI tick; if that tick
/// produces an encounter the engaging entity is stashed via
/// [`GameState::set_pending_encounter`] for the binary to pick up.
#[derive(Debug, Clone, Copy)]
pub struct MoveAction {
    pub direction: Direction,
}

impl Action for MoveAction {
    type Outcome = MoveOutcome;

    fn perform(self, game: &mut GameState, _actor: EntityId, bus: &mut EventBus) -> MoveOutcome {
        let (x, y) = game.player.position();
        let (nx, ny) = step(self.direction, x, y);
        let room = game.current_room();
        if !room.in_bounds(nx, ny) {
            return MoveOutcome::Blocked;
        }
        let (nx, ny) = (nx as usize, ny as usize);
        if !room.is_walkable(nx, ny) {
            return MoveOutcome::Blocked;
        }
        if let Some(entity) = room.enemies.entity_at(&room.world, (nx, ny)) {
            // Walking *into* an enemy doesn't move; surfaced as a direct
            // encounter without going through the bus.
            return MoveOutcome::Encounter(entity);
        }
        game.player.set_position((nx, ny));
        bus.emit(PlayerMoved {
            from: (x, y),
            to: (nx, ny),
        });
        MoveOutcome::Moved
    }
}

pub(crate) fn step(direction: Direction, x: usize, y: usize) -> (isize, isize) {
    match direction {
        Direction::Up => (x as isize, y as isize - 1),
        Direction::Down => (x as isize, y as isize + 1),
        Direction::Left => (x as isize - 1, y as isize),
        Direction::Right => (x as isize + 1, y as isize),
    }
}
