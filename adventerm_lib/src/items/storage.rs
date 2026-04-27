use serde::{Deserialize, Serialize};

use crate::ecs::{ComponentStore, EntityId, World};
use crate::items::kind::ItemKind;

/// Per-room subsystem owning ground items as entities. Each entity has a
/// `Position` (on `World`) and an `ItemKind` (here). Pickup removes the
/// entity entirely and returns the kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemSubsystem {
    kinds: ComponentStore<ItemKind>,
}

impl ItemSubsystem {
    pub fn spawn_at(
        &mut self,
        world: &mut World,
        pos: (usize, usize),
        kind: ItemKind,
    ) -> EntityId {
        let e = world.spawn();
        world.set_position(e, pos);
        self.kinds.insert(e, kind);
        e
    }

    pub fn take_at(&mut self, world: &mut World, pos: (usize, usize)) -> Option<ItemKind> {
        let e = self
            .kinds
            .entities()
            .find(|e| world.position_of(*e) == Some(pos))?;
        let kind = self.kinds.remove(e)?;
        world.despawn(e);
        Some(kind)
    }

    pub fn iter_at<'a>(
        &'a self,
        world: &'a World,
        pos: (usize, usize),
    ) -> impl Iterator<Item = ItemKind> + 'a {
        self.kinds.iter().filter_map(move |(e, k)| {
            if world.position_of(e) == Some(pos) {
                Some(*k)
            } else {
                None
            }
        })
    }

    pub fn any_at(&self, world: &World, pos: (usize, usize)) -> bool {
        self.iter_at(world, pos).next().is_some()
    }

    /// All ground-item positions in this room. Used by tests and generation
    /// invariants to verify items landed on legal tiles.
    pub fn positions<'a>(&'a self, world: &'a World) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.kinds
            .entities()
            .filter_map(move |e| world.position_of(e))
    }

    /// Yield every ground-item kind in this room, regardless of position.
    /// Used by determinism / spawn-distribution tests.
    pub fn iter_at_any<'a>(&'a self, _world: &'a World) -> impl Iterator<Item = ItemKind> + 'a {
        self.kinds.iter().map(|(_, k)| *k)
    }
}
