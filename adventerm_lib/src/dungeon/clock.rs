use serde::{Deserialize, Serialize};

use crate::ecs::{ComponentStore, EntityId, World};

/// Singleton "clock" entity for the dungeon. Owns a monotonic turn
/// counter and lives in the dungeon-scoped `World` so future
/// dungeon-level systems (achievements, scheduling, replay) have a
/// stable host entity to hang components off.
///
/// Per-step game events flow through the open registry's
/// [`crate::event::EventBus`], not through this clock — handlers receive
/// typed events directly. The clock's only responsibility is timekeeping.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DungeonClock {
    entity: Option<EntityId>,
    turn: ComponentStore<TurnCounter>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnCounter(pub u64);

impl DungeonClock {
    /// Spawn the clock entity into `world` and attach an empty
    /// [`TurnCounter`]. Idempotent — repeated calls return the same entity.
    pub fn install(&mut self, world: &mut World) -> EntityId {
        if let Some(e) = self.entity {
            return e;
        }
        let e = world.spawn();
        self.turn.insert(e, TurnCounter::default());
        self.entity = Some(e);
        e
    }

    pub fn entity(&mut self, world: &mut World) -> EntityId {
        self.install(world)
    }

    pub fn turn(&self) -> u64 {
        self.entity
            .and_then(|e| self.turn.get(e))
            .map(|t| t.0)
            .unwrap_or(0)
    }

    pub fn advance(&mut self, world: &mut World) -> u64 {
        let e = self.install(world);
        let next = self.turn.get(e).map(|t| t.0 + 1).unwrap_or(1);
        self.turn.insert(e, TurnCounter(next));
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_is_idempotent() {
        let mut world = World::default();
        let mut clock = DungeonClock::default();
        let a = clock.install(&mut world);
        let b = clock.install(&mut world);
        assert_eq!(a, b);
    }

    #[test]
    fn advance_increments_turn() {
        let mut world = World::default();
        let mut clock = DungeonClock::default();
        assert_eq!(clock.turn(), 0);
        clock.advance(&mut world);
        clock.advance(&mut world);
        assert_eq!(clock.turn(), 2);
    }
}
