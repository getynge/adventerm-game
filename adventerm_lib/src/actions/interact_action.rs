use crate::action::Action;
use crate::dungeon::step_inward;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::{DoorTraversed, FlareBurnedOut};
use crate::game::{DoorEvent, GameState};
use crate::room::TileKind;

/// "If the actor is standing on a door, traverse to the paired door's
/// room." Burns out any active flares in the leaving room before the
/// transition, emitting a [`FlareBurnedOut`] per converted flare and a
/// single [`DoorTraversed`] for the trip itself.
#[derive(Debug, Clone, Copy)]
pub struct InteractAction;

impl Action for InteractAction {
    type Outcome = Option<DoorEvent>;

    fn perform(
        self,
        game: &mut GameState,
        _actor: EntityId,
        bus: &mut EventBus,
    ) -> Option<DoorEvent> {
        let (px, py) = game.player.position();
        let here = game.current_room().kind_at(px, py)?;
        let door_id = match here {
            TileKind::Door(id) => id,
            _ => return None,
        };
        let from = door_id;
        let to = game.dungeon.door(door_id).leads_to;
        let target_door = game.dungeon.door(to);
        let target_room = target_door.owner;
        let landing = step_inward(target_door.pos, game.dungeon.room(target_room));
        let leaving_room = game.current_room;

        let flare_positions: Vec<(usize, usize)> = {
            let room = game.dungeon.room(leaving_room);
            room.lighting
                .iter_flares(&room.world)
                .map(|(pos, _)| pos)
                .collect()
        };
        game.dungeon
            .room_mut(leaving_room)
            .lighting
            .burn_out_flares();
        for pos in flare_positions {
            bus.emit(FlareBurnedOut {
                room: leaving_room,
                pos,
            });
        }

        game.current_room = target_room;
        game.player.set_position(landing);
        bus.emit(DoorTraversed {
            from: from.0,
            to: to.0,
            new_room: target_room,
        });
        Some(DoorEvent {
            from,
            to,
            new_room: target_room,
        })
    }
}
