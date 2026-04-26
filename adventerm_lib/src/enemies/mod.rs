pub mod ai;
pub mod kind;
pub mod movement;
pub mod slime;

use serde::{Deserialize, Serialize};

use crate::ecs::{ComponentStore, EntityId, World};
use crate::stats::Stats;

pub use ai::{enemy_behavior_for, AiAction, AiCtx, EnemyAi};
pub use kind::EnemyKind;
pub use movement::{tick_enemies, EnemyTickOutcome};

/// Per-room subsystem owning enemies as entities. Each enemy entity carries:
/// - a `Position` (managed by `World`)
/// - an `EnemyKind` (here, in `kinds`)
/// - a `Stats` block (here, in `stats`)
/// - a current-HP value (here, in `cur_hp`)
///
/// Despawn (e.g. on death) removes all three components and the entity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enemies {
    kinds: ComponentStore<EnemyKind>,
    stats: ComponentStore<Stats>,
    cur_hp: ComponentStore<u8>,
}

impl Enemies {
    pub fn spawn_at(
        &mut self,
        world: &mut World,
        pos: (usize, usize),
        kind: EnemyKind,
    ) -> EntityId {
        let stats = kind.base_stats();
        let e = world.spawn();
        world.set_position(e, pos);
        self.kinds.insert(e, kind);
        self.cur_hp.insert(e, stats.health);
        self.stats.insert(e, stats);
        e
    }

    pub fn despawn(&mut self, world: &mut World, entity: EntityId) {
        self.kinds.remove(entity);
        self.stats.remove(entity);
        self.cur_hp.remove(entity);
        world.despawn(entity);
    }

    pub fn kind_of(&self, entity: EntityId) -> Option<EnemyKind> {
        self.kinds.get(entity).copied()
    }

    pub fn stats_of(&self, entity: EntityId) -> Option<&Stats> {
        self.stats.get(entity)
    }

    pub fn hp_of(&self, entity: EntityId) -> Option<u8> {
        self.cur_hp.get(entity).copied()
    }

    pub fn set_hp(&mut self, entity: EntityId, value: u8) {
        if self.cur_hp.contains(entity) {
            self.cur_hp.insert(entity, value);
        }
    }

    pub fn entity_at(&self, world: &World, pos: (usize, usize)) -> Option<EntityId> {
        self.kinds
            .entities()
            .find(|e| world.position_of(*e) == Some(pos))
    }

    pub fn iter_with_pos<'a>(
        &'a self,
        world: &'a World,
    ) -> impl Iterator<Item = (EntityId, (usize, usize), EnemyKind)> + 'a {
        self.kinds.iter().filter_map(move |(e, kind)| {
            world.position_of(e).map(|pos| (e, pos, *kind))
        })
    }

    pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.kinds.entities()
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}
