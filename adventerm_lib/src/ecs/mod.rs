use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Stable identifier for an entity. Entities are opaque — meaning is given by
/// the components attached to them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityId(u32);

impl EntityId {
    /// Construct an `EntityId` from a raw integer. Real game entities should
    /// always come from `World::spawn`; this is provided so tests and the
    /// occasional placeholder (e.g. an invalid sentinel) can build one
    /// directly without needing a full `World`.
    pub const fn from_raw(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Universal positional component. Lives on the `World` itself because almost
/// any entity that participates in the room grid needs one. Category-specific
/// data lives in subsystems, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position(pub (usize, usize));

/// Generic per-component sparse storage. Subsystems compose their state from
/// these so the `World` itself never enumerates every category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStore<T> {
    map: HashMap<EntityId, T>,
}

impl<T: PartialEq> PartialEq for ComponentStore<T> {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
    }
}

impl<T: Eq> Eq for ComponentStore<T> {}

impl<T> Default for ComponentStore<T> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl<T> ComponentStore<T> {
    pub fn insert(&mut self, e: EntityId, value: T) -> Option<T> {
        self.map.insert(e, value)
    }

    pub fn remove(&mut self, e: EntityId) -> Option<T> {
        self.map.remove(&e)
    }

    pub fn get(&self, e: EntityId) -> Option<&T> {
        self.map.get(&e)
    }

    pub fn get_mut(&mut self, e: EntityId) -> Option<&mut T> {
        self.map.get_mut(&e)
    }

    pub fn contains(&self, e: EntityId) -> bool {
        self.map.contains_key(&e)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> + '_ {
        self.map.iter().map(|(e, v)| (*e, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> + '_ {
        self.map.iter_mut().map(|(e, v)| (*e, v))
    }

    pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.map.keys().copied()
    }
}

/// Substrate. Owns entity allocation, lifetime tracking, and the universal
/// `Position` component. Per-category state belongs in a subsystem, never here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    next: u32,
    alive: HashSet<EntityId>,
    pub positions: ComponentStore<Position>,
}

impl World {
    pub fn spawn(&mut self) -> EntityId {
        let id = EntityId(self.next);
        self.next = self.next.wrapping_add(1);
        self.alive.insert(id);
        id
    }

    pub fn despawn(&mut self, e: EntityId) {
        self.alive.remove(&e);
        self.positions.remove(e);
    }

    pub fn is_alive(&self, e: EntityId) -> bool {
        self.alive.contains(&e)
    }

    pub fn position_of(&self, e: EntityId) -> Option<(usize, usize)> {
        self.positions.get(e).map(|p| p.0)
    }

    pub fn set_position(&mut self, e: EntityId, pos: (usize, usize)) {
        self.positions.insert(e, Position(pos));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_and_despawn_round_trip() {
        let mut w = World::default();
        let e = w.spawn();
        assert!(w.is_alive(e));
        w.set_position(e, (3, 4));
        assert_eq!(w.position_of(e), Some((3, 4)));
        w.despawn(e);
        assert!(!w.is_alive(e));
        assert_eq!(w.position_of(e), None);
    }

    #[test]
    fn entity_ids_are_unique() {
        let mut w = World::default();
        let a = w.spawn();
        let b = w.spawn();
        assert_ne!(a, b);
    }
}
