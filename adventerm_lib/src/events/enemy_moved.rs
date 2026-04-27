use crate::ecs::EntityId;
use crate::event::Event;
use crate::room::RoomId;

/// An enemy entity stepped within its room. Emitted by the enemy-tick
/// handler in response to a `PlayerMoved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyMoved {
    pub entity: EntityId,
    pub room: RoomId,
    pub from: (usize, usize),
    pub to: (usize, usize),
}

impl Event for EnemyMoved {}
