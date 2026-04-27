use serde::{Deserialize, Serialize};

use crate::ecs::{ComponentStore, EntityId, World};
use crate::room::{DoorId, RoomId};

/// Which room a door entity belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorOwner(pub RoomId);

/// The paired door this one leads to. Doors are spawned in mutually-linked
/// pairs by the dungeon generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorLink(pub EntityId);

/// Mutable per-door state. Reserved for growth (locked, secret, one-way…).
/// Default doors are open and unlocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorState {
    pub open: bool,
    pub locked: bool,
}

impl Default for DoorState {
    fn default() -> Self {
        Self {
            open: true,
            locked: false,
        }
    }
}

/// Read-only snapshot of a door's data. Returned from `DoorSubsystem::view`
/// so callers don't need to know about the underlying component stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorView {
    pub id: DoorId,
    pub owner: RoomId,
    pub pos: (usize, usize),
    pub leads_to: DoorId,
    pub state: DoorState,
}

/// Dungeon-scoped subsystem owning doors as entities. Each door entity carries:
/// - a `Position` (managed by the dungeon `World`)
/// - a `DoorOwner(RoomId)`
/// - a `DoorLink(EntityId)` pointing at its paired door
/// - a `DoorState` for mutable per-door flags
///
/// Mirrors the shape of `Enemies`/`Lighting` (private component stores +
/// narrow write API; world is borrowed from the caller).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorSubsystem {
    owners: ComponentStore<DoorOwner>,
    links: ComponentStore<DoorLink>,
    states: ComponentStore<DoorState>,
}

impl DoorSubsystem {
    /// Spawn a new door entity at `pos` owned by `owner`. The link must be
    /// set later via `link_pair` once both ends of the pair exist.
    pub fn spawn(
        &mut self,
        world: &mut World,
        owner: RoomId,
        pos: (usize, usize),
    ) -> DoorId {
        let e = world.spawn();
        world.set_position(e, pos);
        self.owners.insert(e, DoorOwner(owner));
        self.states.insert(e, DoorState::default());
        DoorId(e)
    }

    /// Despawn a door entity. Used by generator rollback when the paired
    /// placement fails.
    pub fn despawn(&mut self, world: &mut World, id: DoorId) {
        self.owners.remove(id.0);
        self.links.remove(id.0);
        self.states.remove(id.0);
        world.despawn(id.0);
    }

    /// Wire two freshly-spawned doors into a bidirectional pair.
    pub fn link_pair(&mut self, a: DoorId, b: DoorId) {
        self.links.insert(a.0, DoorLink(b.0));
        self.links.insert(b.0, DoorLink(a.0));
    }

    pub fn view(&self, world: &World, id: DoorId) -> Option<DoorView> {
        let owner = self.owners.get(id.0)?.0;
        let link = self.links.get(id.0)?.0;
        let state = self.states.get(id.0).copied().unwrap_or_default();
        let pos = world.position_of(id.0)?;
        Some(DoorView {
            id,
            owner,
            pos,
            leads_to: DoorId(link),
            state,
        })
    }

    pub fn owner_of(&self, id: DoorId) -> Option<RoomId> {
        self.owners.get(id.0).map(|o| o.0)
    }

    pub fn leads_to(&self, id: DoorId) -> Option<DoorId> {
        self.links.get(id.0).map(|l| DoorId(l.0))
    }

    pub fn state_of(&self, id: DoorId) -> DoorState {
        self.states.get(id.0).copied().unwrap_or_default()
    }

    pub fn ids(&self) -> impl Iterator<Item = DoorId> + '_ {
        self.owners.entities().map(DoorId)
    }
}
