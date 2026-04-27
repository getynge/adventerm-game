use crate::ecs::EntityId;
use crate::event::Event;
use crate::room::RoomId;

/// An enemy is adjacent to the player and has chosen to engage. The
/// enemy-tick handler emits this so the action layer can promote a normal
/// move outcome into an encounter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyEngaged {
    pub entity: EntityId,
    pub room: RoomId,
    pub pos: (usize, usize),
}

impl Event for EnemyEngaged {}
