use crate::ecs::EntityId;
use crate::event::Event;
use crate::room::RoomId;

/// The player walked through `from` and emerged at `to`'s room. The
/// visibility system refreshes after the room swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorTraversed {
    pub from: EntityId,
    pub to: EntityId,
    pub new_room: RoomId,
}

impl Event for DoorTraversed {}
