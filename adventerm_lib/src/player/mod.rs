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
    ConsumeItemAction, DefeatEnemyAction, EquipItemAction, InteractAction, MoveAction,
    PickUpAction, PlaceItemAction, QuickMoveAction, UnequipItemAction,
};
use crate::ecs::{ComponentStore, EntityId, World};
use crate::equipment::Equipment;
use crate::items::ItemKind;
use crate::los::LOS_RANGE;
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
    reg.register_action::<EquipItemAction>(ActorKind::Player);
    reg.register_action::<UnequipItemAction>(ActorKind::Player);
    reg.register_action::<ConsumeItemAction>(ActorKind::Player);
    reg.register_action::<DefeatEnemyAction>(ActorKind::Player);
}

/// Apply a signed equipment bonus to an unsigned stat without
/// over/underflow. `Stats::new` will clamp the final result.
fn apply_bonus(base: u8, bonus: i8) -> u8 {
    if bonus >= 0 {
        base.saturating_add(bonus as u8)
    } else {
        base.saturating_sub(bonus.unsigned_abs())
    }
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
    #[serde(default)]
    pub equipment: ComponentStore<Equipment>,
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
            && self.equipment == other.equipment
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
            equipment: ComponentStore::default(),
            visibility: ComponentStore::default(),
            enemy_rng: ComponentStore::default(),
        };
        me.inventory.insert(entity, Inventory::default());
        me.stats.insert(entity, stats);
        me.cur_health.insert(entity, CurHealth(stats.health));
        me.abilities.insert(entity, Abilities::default());
        me.equipment.insert(entity, Equipment::default());
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

    pub fn abilities_mut(&mut self) -> &mut Abilities {
        self.abilities.get_mut(self.entity).expect("abilities")
    }

    pub fn equipment(&self) -> &Equipment {
        // Lazily populate after deserialization of an old save where the
        // field was absent.
        self.equipment
            .get(self.entity)
            .expect("equipment (call ensure_equipment after load)")
    }

    pub fn equipment_mut(&mut self) -> &mut Equipment {
        if !self.equipment.contains(self.entity) {
            self.equipment.insert(self.entity, Equipment::default());
        }
        self.equipment.get_mut(self.entity).unwrap()
    }

    /// Apply equipped-item bonuses to the base stat block. Clamped via
    /// `Stats::new` so the result stays inside `[STAT_MIN, STAT_MAX]`.
    pub fn effective_stats(&self) -> Stats {
        let base = *self.stats();
        let bonus = self.equipment_or_default().aggregate_effect();
        Stats::new(
            base.health,
            apply_bonus(base.attack, bonus.attack),
            apply_bonus(base.defense, bonus.defense),
            apply_bonus(base.speed, bonus.speed),
            base.attribute,
        )
    }

    /// Player's effective LOS radius, after equipment multipliers. Light
    /// sources read `crate::los::LIGHT_RANGE` instead — they are
    /// intentionally unaffected by the player's vision gear.
    pub fn vision_radius(&self) -> usize {
        let multiplier = self.equipment_or_default().aggregate_effect().vision_multiplier as usize;
        LOS_RANGE.saturating_mul(multiplier.max(1))
    }

    fn equipment_or_default(&self) -> Equipment {
        self.equipment
            .get(self.entity)
            .copied()
            .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::EquipSlot;

    #[test]
    fn effective_stats_apply_equipment_bonuses() {
        let mut p = PlayerSubsystem::new_at((0, 0));
        let base = *p.stats();
        p.equipment_mut().equip(EquipSlot::Arms, ItemKind::Gauntlets);
        p.equipment_mut().equip(EquipSlot::Torso, ItemKind::Shirt);
        let eff = p.effective_stats();
        assert_eq!(eff.attack, base.attack + 1);
        assert_eq!(eff.defense, base.defense + 1);
        assert_eq!(eff.health, base.health);
    }

    #[test]
    fn vision_radius_doubles_with_goggles() {
        let mut p = PlayerSubsystem::new_at((0, 0));
        let base_radius = p.vision_radius();
        p.equipment_mut().equip(EquipSlot::Head, ItemKind::Goggles);
        assert_eq!(p.vision_radius(), base_radius * 2);
    }

    #[test]
    fn effective_stats_clamp_at_max() {
        let mut p = PlayerSubsystem::new_at((0, 0));
        // Bypass the constructor so we can plant a near-max stat directly
        // and confirm the +1 bonus is clamped instead of overflowing.
        let entity = p.entity();
        let mut s = *p.stats();
        s = Stats::new(s.health, 100, s.defense, s.speed, s.attribute);
        p.stats.insert(entity, s);
        p.equipment_mut().equip(EquipSlot::Arms, ItemKind::Gauntlets);
        assert_eq!(p.effective_stats().attack, 100);
    }
}
