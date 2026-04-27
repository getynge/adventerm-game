//! Player subsystem.
//!
//! The player is a single entity in a top-level [`World`]. Player-side state
//! lives as components on that entity:
//!
//! - [`Position`] (universal, on `World`) — coordinates within the current room.
//! - [`Inventory`] — carried items.
//! - [`Stats`] — five-axis stat block (max HP, attack, defense, speed, attribute).
//! - [`CurHealth`] — current HP.
//! - [`Abilities`] — equipped/learned ability slots.
//! - [`VisibilityCache`] — transient per-room LOS / lit bitmaps. `#[serde(skip)]`.
//! - [`EnemyRngState`] — transient per-game enemy AI RNG. `#[serde(skip)]`.
//!
//! Per-room "explored" memory is owned by the [`crate::explored`] subsystem
//! rather than this one — it is keyed by `RoomId`, not by the player entity.

use serde::{Deserialize, Serialize};

use crate::abilities::Abilities;
use crate::actions::{
    DefeatEnemyAction, InteractAction, MoveAction, PickUpAction, PlaceItemAction, QuickMoveAction,
};
use crate::ecs::{ComponentStore, EntityId, World};
use crate::items::ItemKind;
use crate::registry::{ActorKind, Registry};
use crate::rng::Rng;
use crate::stats::Stats;

/// Register every action the player entity can perform. The list is
/// declarative — the registry uses it for introspection only — so adding
/// a new player action is a one-line addition here plus a new module
/// under [`crate::actions`].
pub fn register(reg: &mut Registry) {
    reg.register_action::<MoveAction>(ActorKind::Player);
    reg.register_action::<QuickMoveAction>(ActorKind::Player);
    reg.register_action::<InteractAction>(ActorKind::Player);
    reg.register_action::<PickUpAction>(ActorKind::Player);
    reg.register_action::<PlaceItemAction>(ActorKind::Player);
    reg.register_action::<DefeatEnemyAction>(ActorKind::Player);
}

/// Carried items, in pickup order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory(pub Vec<ItemKind>);

/// Current HP. `Stats::health` is the maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurHealth(pub u8);

/// Cached per-room visibility bitmaps. Recomputed by `refresh_visibility`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VisibilityCache {
    pub visible: Vec<bool>,
    pub lit: Vec<bool>,
}

/// Per-game enemy AI RNG. Lazily rehydrated from the dungeon seed on load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyRngState(pub Option<Rng>);

/// Player-side ECS state. Holds the top-level `World` (the player entity lives
/// here), the player entity handle, and component stores for player-global
/// state. Transient stores (visibility cache, enemy RNG) are skipped during
/// serialization and rehydrated on load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSubsystem {
    pub world: World,
    pub entity: EntityId,
    pub inventory: ComponentStore<Inventory>,
    pub stats: ComponentStore<Stats>,
    pub cur_health: ComponentStore<CurHealth>,
    pub abilities: ComponentStore<Abilities>,
    #[serde(skip)]
    pub visibility: ComponentStore<VisibilityCache>,
    #[serde(skip)]
    pub enemy_rng: ComponentStore<EnemyRngState>,
}

impl PartialEq for PlayerSubsystem {
    fn eq(&self, other: &Self) -> bool {
        // Transient stores intentionally excluded from equality; they are
        // skipped during serialization and rehydrated on load.
        self.world == other.world
            && self.entity == other.entity
            && self.inventory == other.inventory
            && self.stats == other.stats
            && self.cur_health == other.cur_health
            && self.abilities == other.abilities
    }
}

impl Eq for PlayerSubsystem {}

impl PlayerSubsystem {
    /// Spawn the singleton player entity at `pos` with default stats and
    /// abilities. The caller decides which room `pos` is in via the outer
    /// `current_room` tracker on `GameState`.
    pub fn new_at(pos: (usize, usize)) -> Self {
        let mut world = World::default();
        let entity = world.spawn();
        world.set_position(entity, pos);

        let stats = Stats::default();
        let mut me = Self {
            world,
            entity,
            inventory: ComponentStore::default(),
            stats: ComponentStore::default(),
            cur_health: ComponentStore::default(),
            abilities: ComponentStore::default(),
            visibility: ComponentStore::default(),
            enemy_rng: ComponentStore::default(),
        };
        me.inventory.insert(entity, Inventory::default());
        me.stats.insert(entity, stats);
        me.cur_health.insert(entity, CurHealth(stats.health));
        me.abilities.insert(entity, Abilities::default());
        me.visibility.insert(entity, VisibilityCache::default());
        me.enemy_rng.insert(entity, EnemyRngState(None));
        me
    }

    pub fn entity(&self) -> EntityId {
        self.entity
    }

    pub fn position(&self) -> (usize, usize) {
        self.world
            .position_of(self.entity)
            .expect("player entity always has a Position component")
    }

    pub fn set_position(&mut self, pos: (usize, usize)) {
        self.world.set_position(self.entity, pos);
    }

    pub fn inventory(&self) -> &[ItemKind] {
        &self.inventory.get(self.entity).expect("inventory").0
    }

    pub fn inventory_push(&mut self, kind: ItemKind) {
        self.inventory
            .get_mut(self.entity)
            .expect("inventory")
            .0
            .push(kind);
    }

    pub fn inventory_remove(&mut self, idx: usize) -> Option<ItemKind> {
        let inv = &mut self.inventory.get_mut(self.entity).expect("inventory").0;
        if idx < inv.len() {
            Some(inv.remove(idx))
        } else {
            None
        }
    }

    pub fn inventory_get(&self, idx: usize) -> Option<ItemKind> {
        self.inventory().get(idx).copied()
    }

    pub fn stats(&self) -> &Stats {
        self.stats.get(self.entity).expect("stats")
    }

    pub fn cur_health(&self) -> u8 {
        self.cur_health.get(self.entity).expect("cur_health").0
    }

    pub fn set_cur_health(&mut self, hp: u8) {
        self.cur_health.insert(self.entity, CurHealth(hp));
    }

    pub fn abilities(&self) -> &Abilities {
        self.abilities.get(self.entity).expect("abilities")
    }

    pub fn visibility(&self) -> &VisibilityCache {
        self.visibility
            .get(self.entity)
            .expect("visibility cache (call refresh_visibility after load)")
    }

    pub fn visibility_mut(&mut self) -> &mut VisibilityCache {
        // Lazily rehydrate after deserialization (the field is `serde(skip)`).
        if !self.visibility.contains(self.entity) {
            self.visibility
                .insert(self.entity, VisibilityCache::default());
        }
        self.visibility.get_mut(self.entity).unwrap()
    }

    /// Lazily seed the enemy-AI RNG on first use. The dungeon seed is mixed
    /// with `salt` so AI draws don't recapitulate the dungeon-generation
    /// sequence.
    pub fn enemy_rng_mut(&mut self, dungeon_seed: u64, salt: u64) -> &mut Rng {
        if !self.enemy_rng.contains(self.entity) {
            self.enemy_rng.insert(self.entity, EnemyRngState(None));
        }
        let cell = self.enemy_rng.get_mut(self.entity).unwrap();
        if cell.0.is_none() {
            cell.0 = Some(Rng::new(dungeon_seed ^ salt));
        }
        cell.0.as_mut().unwrap()
    }
}
