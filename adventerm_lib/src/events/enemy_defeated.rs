use crate::ecs::EntityId;
use crate::event::Event;
use crate::room::RoomId;

/// An enemy entity was despawned (post-battle, by the player). No default
/// subscribers — available for future logging / quests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyDefeated {
    pub entity: EntityId,
    pub room: RoomId,
}

impl Event for EnemyDefeated {}
