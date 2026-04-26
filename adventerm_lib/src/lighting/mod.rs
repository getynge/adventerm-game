use serde::{Deserialize, Serialize};

use crate::ecs::{ComponentStore, EntityId, World};
use crate::los::LIGHT_RANGE;

/// Persistent light source attached to an entity. The entity's `Position`
/// (managed by `World`) is the tile the light shines from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightSource {
    pub radius: u8,
}

/// Marker component for an active flare. While at least one flare exists in a
/// room, the visibility system treats every tile as lit. On room exit the
/// flare is replaced by a `LightSource` at the same entity (same tile).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlareSource;

/// Subsystem owning all lighting-related component storage for a room.
/// `World` knows nothing about lights or flares — they live here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lighting {
    sources: ComponentStore<LightSource>,
    flares: ComponentStore<FlareSource>,
}

impl Lighting {
    /// Spawn a persistent torch (light source) at `pos`. Idempotent: if a
    /// `LightSource` already exists at this position the call is a no-op.
    pub fn add_torch(&mut self, world: &mut World, pos: (usize, usize)) -> EntityId {
        if let Some(e) = self.find_source_at(world, pos) {
            return e;
        }
        let e = world.spawn();
        world.set_position(e, pos);
        self.sources.insert(
            e,
            LightSource {
                radius: LIGHT_RANGE as u8,
            },
        );
        e
    }

    /// Spawn an active flare at `pos`. Idempotent on position.
    pub fn add_flare(&mut self, world: &mut World, pos: (usize, usize)) -> EntityId {
        if let Some(e) = self.find_flare_at(world, pos) {
            return e;
        }
        let e = world.spawn();
        world.set_position(e, pos);
        self.flares.insert(e, FlareSource);
        e
    }

    /// Convert every active flare into a regular `LightSource` at the same
    /// entity (same `Position`). Called when the player leaves a room.
    pub fn burn_out_flares(&mut self) {
        let to_convert: Vec<EntityId> = self.flares.entities().collect();
        for e in to_convert {
            self.flares.remove(e);
            self.sources.insert(
                e,
                LightSource {
                    radius: LIGHT_RANGE as u8,
                },
            );
        }
    }

    pub fn any_flare_active(&self) -> bool {
        !self.flares.is_empty()
    }

    /// Yield `(position, &LightSource)` for every persistent torch in the room.
    pub fn iter_sources<'a>(
        &'a self,
        world: &'a World,
    ) -> impl Iterator<Item = ((usize, usize), &'a LightSource)> + 'a {
        self.sources.iter().filter_map(move |(e, s)| {
            world.position_of(e).map(|p| (p, s))
        })
    }

    fn find_source_at(&self, world: &World, pos: (usize, usize)) -> Option<EntityId> {
        self.sources
            .entities()
            .find(|e| world.position_of(*e) == Some(pos))
    }

    fn find_flare_at(&self, world: &World, pos: (usize, usize)) -> Option<EntityId> {
        self.flares
            .entities()
            .find(|e| world.position_of(*e) == Some(pos))
    }
}
