use serde::{Deserialize, Serialize};

use crate::abilities::{Abilities, AbilityKind};
use crate::ecs::World;
use crate::items::boots::BootsBehavior;
use crate::items::category::{EquipSlot, ItemCategory};
use crate::items::flare::FlareBehavior;
use crate::items::gauntlets::GauntletsBehavior;
use crate::items::goggles::GogglesBehavior;
use crate::items::kind::ItemKind;
use crate::items::scroll_of_fire::ScrollOfFireBehavior;
use crate::items::shirt::ShirtBehavior;
use crate::items::torch::TorchBehavior;
use crate::items::trousers::TrousersBehavior;
use crate::lighting::Lighting;
use crate::stats::Stats;

/// Side-effect description returned by a placement behavior so the caller
/// can surface a status string. The behavior itself has already mutated the
/// world by the time this is returned. Serialize/Deserialize are derived so
/// it can ride along inside `DungeonEvent::ItemPlaced`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaceOutcome {
    TorchPlaced,
    FlarePlaced,
}

/// Mutable view of the world handed to an `ItemBehavior` when it acts. Only
/// the subsystems a behavior is allowed to touch are exposed here — a torch
/// can change lighting but not, say, monsters. Add new subsystems here as
/// they land.
pub struct PlaceCtx<'a> {
    pub player_pos: (usize, usize),
    pub world: &'a mut World,
    pub lighting: &'a mut Lighting,
}

/// Stat / vision modifier contributed by a single piece of equipment.
/// Stat fields are signed so future debuffs slot in without a new shape.
/// Equipment-aggregating code folds these into a single `EquipEffect`
/// before applying to base stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipEffect {
    pub attack: i8,
    pub defense: i8,
    pub speed: i8,
    /// Multiplier applied to player vision radius. `1` is the no-op default;
    /// goggles set it to `2`. Aggregation multiplies across slots.
    pub vision_multiplier: u8,
}

impl Default for EquipEffect {
    fn default() -> Self {
        Self {
            attack: 0,
            defense: 0,
            speed: 0,
            vision_multiplier: 1,
        }
    }
}

/// Targeting input the inventory UI must collect before a Consumable can
/// resolve. Open enum: future consumables (potions, bombs, scrolls of
/// teleport, …) add a variant without rewriting the dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConsumeIntent {
    /// Apply immediately — no extra input needed (e.g. healing potion).
    Immediate,
    /// Player must pick an active-ability slot (e.g. Scroll of Fire).
    PickAbilitySlot,
}

/// Resolved targeting information handed to `on_consume`. The UI populates
/// the variant matching the kind's `consume_intent`. New consumable
/// categories add fields here as their first kind needs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConsumeTarget {
    None,
    AbilitySlot(usize),
}

/// Mutable view of player-side state a Consumable is allowed to touch. New
/// consumable categories (heal, teleport, buff, …) get fields added here
/// as the first kind that needs them lands — same pattern as `PlaceCtx`.
pub struct ConsumeCtx<'a> {
    pub abilities: &'a mut Abilities,
    pub cur_health: &'a mut u8,
    pub stats: &'a Stats,
    pub target: ConsumeTarget,
}

/// Result description returned by `on_consume`. New variants when new
/// consumables land. Serializable so it can ride inside
/// `events::ItemConsumed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ConsumeOutcome {
    LearnedAbility { kind: AbilityKind, slot: usize },
}

/// Trait every item kind implements. The action layer dispatches through
/// `behavior_for(kind)` and never matches on the kind itself.
///
/// Each method has a default impl that means "this category is not me", so a
/// new item kind only overrides the methods that match its `category()`.
pub trait ItemBehavior {
    /// What the item is. Drives action-layer dispatch (Place / Equip /
    /// Consume) and the inventory UI's per-row Confirm routing.
    fn category(&self) -> ItemCategory;

    /// Side-effect of placing this item on the world. Default `None` =
    /// "this isn't a Placeable item." Placeable kinds override.
    fn on_place(&self, _ctx: &mut PlaceCtx<'_>) -> Option<PlaceOutcome> {
        None
    }

    /// Stat / vision modifier contributed while equipped. Default is the
    /// no-op effect (zero bonus, vision×1). Equipment kinds override.
    fn equip_effect(&self) -> EquipEffect {
        EquipEffect::default()
    }

    /// What targeting input the UI must collect before this consumable
    /// can resolve. Only meaningful for `ItemCategory::Consumable`.
    fn consume_intent(&self) -> ConsumeIntent {
        ConsumeIntent::Immediate
    }

    /// Apply the consumable's effect. Default `None` = "this isn't a
    /// Consumable item." Consumable kinds override and read `ctx.target`
    /// for any picker-collected state.
    fn on_consume(&self, _ctx: &mut ConsumeCtx<'_>) -> Option<ConsumeOutcome> {
        None
    }
}

/// The single point in the codebase that enumerates `ItemKind`. Adding a new
/// item: add a variant, add a new module with a behavior impl, and add one
/// arm here. Exhaustiveness is compiler-enforced.
pub fn behavior_for(kind: ItemKind) -> &'static dyn ItemBehavior {
    match kind {
        ItemKind::Torch => &TorchBehavior,
        ItemKind::Flare => &FlareBehavior,
        ItemKind::Goggles => &GogglesBehavior,
        ItemKind::Shirt => &ShirtBehavior,
        ItemKind::Gauntlets => &GauntletsBehavior,
        ItemKind::Trousers => &TrousersBehavior,
        ItemKind::Boots => &BootsBehavior,
        ItemKind::ScrollOfFire => &ScrollOfFireBehavior,
    }
}

/// Convenience: the equipment slot for a kind, if it's an Equipment item.
pub fn equip_slot_of(kind: ItemKind) -> Option<EquipSlot> {
    match behavior_for(kind).category() {
        ItemCategory::Equipment(slot) => Some(slot),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_behavior_spawns_a_light_source_at_player_position() {
        let mut world = World::default();
        let mut lighting = Lighting::default();
        let mut ctx = PlaceCtx {
            player_pos: (3, 4),
            world: &mut world,
            lighting: &mut lighting,
        };
        let outcome = behavior_for(ItemKind::Torch).on_place(&mut ctx);
        assert_eq!(outcome, Some(PlaceOutcome::TorchPlaced));
        let positions: Vec<(usize, usize)> =
            lighting.iter_sources(&world).map(|(p, _)| p).collect();
        assert_eq!(positions, vec![(3, 4)]);
    }

    #[test]
    fn flare_behavior_marks_the_room_as_flare_lit() {
        let mut world = World::default();
        let mut lighting = Lighting::default();
        let mut ctx = PlaceCtx {
            player_pos: (1, 1),
            world: &mut world,
            lighting: &mut lighting,
        };
        let outcome = behavior_for(ItemKind::Flare).on_place(&mut ctx);
        assert_eq!(outcome, Some(PlaceOutcome::FlarePlaced));
        assert!(lighting.any_flare_active());
    }

    #[test]
    fn equipment_kinds_short_circuit_on_place() {
        let mut world = World::default();
        let mut lighting = Lighting::default();
        let mut ctx = PlaceCtx {
            player_pos: (0, 0),
            world: &mut world,
            lighting: &mut lighting,
        };
        for kind in [
            ItemKind::Goggles,
            ItemKind::Shirt,
            ItemKind::Gauntlets,
            ItemKind::Trousers,
            ItemKind::Boots,
            ItemKind::ScrollOfFire,
        ] {
            assert_eq!(behavior_for(kind).on_place(&mut ctx), None);
        }
    }

    #[test]
    fn each_kind_has_a_category() {
        // Compile-time enforced via `behavior_for`'s exhaustive match; this
        // just sanity-checks the values line up with what the docs claim.
        assert_eq!(behavior_for(ItemKind::Torch).category(), ItemCategory::Placeable);
        assert_eq!(behavior_for(ItemKind::Flare).category(), ItemCategory::Placeable);
        assert_eq!(
            behavior_for(ItemKind::Goggles).category(),
            ItemCategory::Equipment(EquipSlot::Head)
        );
        assert_eq!(
            behavior_for(ItemKind::Shirt).category(),
            ItemCategory::Equipment(EquipSlot::Torso)
        );
        assert_eq!(
            behavior_for(ItemKind::Gauntlets).category(),
            ItemCategory::Equipment(EquipSlot::Arms)
        );
        assert_eq!(
            behavior_for(ItemKind::Trousers).category(),
            ItemCategory::Equipment(EquipSlot::Legs)
        );
        assert_eq!(
            behavior_for(ItemKind::Boots).category(),
            ItemCategory::Equipment(EquipSlot::Feet)
        );
        assert_eq!(
            behavior_for(ItemKind::ScrollOfFire).category(),
            ItemCategory::Consumable
        );
    }
}
