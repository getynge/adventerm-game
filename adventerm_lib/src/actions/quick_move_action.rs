use crate::action::Action;
use crate::actions::move_action::step;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::PlayerMoved;
use crate::game::{GameState, MoveOutcome};
use crate::room::TileKind;
use crate::world::Direction;

/// "Slide the actor as far as possible in `direction` without stepping
/// onto an interactable tile." Emits one [`PlayerMoved`] per actual step
/// taken; each step's drained handlers may produce an encounter that
/// halts the slide.
#[derive(Debug, Clone, Copy)]
pub struct QuickMoveAction {
    pub direction: Direction,
}

impl Action for QuickMoveAction {
    type Outcome = MoveOutcome;

    fn perform(self, game: &mut GameState, _actor: EntityId, bus: &mut EventBus) -> MoveOutcome {
        let mut moved = false;
        loop {
            // If a previous step's drained handlers stashed an encounter,
            // stop the slide. The pending slot is *not* cleared here —
            // the binary's post-dispatch read takes it.
            if game.peek_pending_encounter().is_some() {
                break;
            }
            let (x, y) = game.player.position();
            let (nx, ny) = step(self.direction, x, y);
            let room = game.current_room();
            if !room.in_bounds(nx, ny) {
                break;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            match room.kind_at(nx, ny) {
                Some(TileKind::Floor) => {
                    if room.enemies.entity_at(&room.world, (nx, ny)).is_some() {
                        break;
                    }
                    game.player.set_position((nx, ny));
                    bus.emit(PlayerMoved {
                        from: (x, y),
                        to: (nx, ny),
                    });
                    moved = true;
                    // Drain the bus between steps so per-step handlers
                    // (visibility, enemy tick) run before the next move
                    // is decided. Without this the slide would race past
                    // an enemy that just stepped adjacent.
                    crate::action::drain_bus(game, bus);
                }
                _ => break,
            }
        }
        if moved {
            MoveOutcome::Moved
        } else {
            MoveOutcome::Blocked
        }
    }
}
