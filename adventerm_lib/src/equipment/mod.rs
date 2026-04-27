//! Equipment subsystem.
//!
//! The player's worn-item state. Five fixed slots (`Head`, `Torso`, `Arms`,
//! `Legs`, `Feet`); each slot holds at most one [`ItemKind`]. Effects
//! (stat bonuses, vision multiplier) come from each occupied slot's
//! [`crate::items::ItemBehavior::equip_effect`] and are folded into a
//! single [`EquipEffect`] by [`Equipment::aggregate_effect`].
//!
//! Lives as a `ComponentStore<Equipment>` on the player entity; the
//! [`crate::game::GameState`] facade exposes it through `equipment()`.

use serde::{Deserialize, Serialize};

use crate::items::{behavior_for, EquipEffect, EquipSlot, ItemKind};

/// Per-player worn-item state. Default = nothing equipped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    #[serde(default)]
    pub head: Option<ItemKind>,
    #[serde(default)]
    pub torso: Option<ItemKind>,
    #[serde(default)]
    pub arms: Option<ItemKind>,
    #[serde(default)]
    pub legs: Option<ItemKind>,
    #[serde(default)]
    pub feet: Option<ItemKind>,
}

impl Equipment {
    pub fn slot(&self, slot: EquipSlot) -> Option<ItemKind> {
        match slot {
            EquipSlot::Head => self.head,
            EquipSlot::Torso => self.torso,
            EquipSlot::Arms => self.arms,
            EquipSlot::Legs => self.legs,
            EquipSlot::Feet => self.feet,
        }
    }

    /// Equip `kind` into `slot`, returning whatever was previously equipped
    /// there (so the caller can hand it back to the inventory).
    pub fn equip(&mut self, slot: EquipSlot, kind: ItemKind) -> Option<ItemKind> {
        let cell = self.slot_mut(slot);
        cell.replace(kind)
    }

    pub fn unequip(&mut self, slot: EquipSlot) -> Option<ItemKind> {
        self.slot_mut(slot).take()
    }

    pub fn iter(&self) -> impl Iterator<Item = (EquipSlot, Option<ItemKind>)> + '_ {
        EquipSlot::ALL.iter().map(move |s| (*s, self.slot(*s)))
    }

    /// Fold each occupied slot's [`EquipEffect`] into one. Stat fields sum;
    /// `vision_multiplier` multiplies (so two doublers stack into ×4 — not
    /// reachable today but the math composes).
    pub fn aggregate_effect(&self) -> EquipEffect {
        let mut acc = EquipEffect::default();
        for (_, occupant) in self.iter() {
            let Some(kind) = occupant else { continue };
            let e = behavior_for(kind).equip_effect();
            acc.attack = acc.attack.saturating_add(e.attack);
            acc.defense = acc.defense.saturating_add(e.defense);
            acc.speed = acc.speed.saturating_add(e.speed);
            acc.vision_multiplier = acc.vision_multiplier.saturating_mul(e.vision_multiplier);
        }
        acc
    }

    fn slot_mut(&mut self, slot: EquipSlot) -> &mut Option<ItemKind> {
        match slot {
            EquipSlot::Head => &mut self.head,
            EquipSlot::Torso => &mut self.torso,
            EquipSlot::Arms => &mut self.arms,
            EquipSlot::Legs => &mut self.legs,
            EquipSlot::Feet => &mut self.feet,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_equipment_is_empty() {
        let eq = Equipment::default();
        for slot in EquipSlot::ALL {
            assert_eq!(eq.slot(slot), None);
        }
    }

    #[test]
    fn equip_returns_previous_occupant() {
        let mut eq = Equipment::default();
        assert_eq!(eq.equip(EquipSlot::Arms, ItemKind::Gauntlets), None);
        assert_eq!(
            eq.equip(EquipSlot::Arms, ItemKind::Gauntlets),
            Some(ItemKind::Gauntlets)
        );
    }

    #[test]
    fn unequip_clears_slot_and_returns_kind() {
        let mut eq = Equipment::default();
        eq.equip(EquipSlot::Feet, ItemKind::Boots);
        assert_eq!(eq.unequip(EquipSlot::Feet), Some(ItemKind::Boots));
        assert_eq!(eq.slot(EquipSlot::Feet), None);
        assert_eq!(eq.unequip(EquipSlot::Feet), None);
    }

    #[test]
    fn aggregate_effect_sums_stat_bonuses() {
        let mut eq = Equipment::default();
        eq.equip(EquipSlot::Arms, ItemKind::Gauntlets);
        eq.equip(EquipSlot::Torso, ItemKind::Shirt);
        eq.equip(EquipSlot::Legs, ItemKind::Trousers);
        eq.equip(EquipSlot::Feet, ItemKind::Boots);
        let agg = eq.aggregate_effect();
        assert_eq!(agg.attack, 1);
        assert_eq!(agg.defense, 2);
        assert_eq!(agg.speed, 1);
        assert_eq!(agg.vision_multiplier, 1);
    }

    #[test]
    fn goggles_double_vision_multiplier() {
        let mut eq = Equipment::default();
        eq.equip(EquipSlot::Head, ItemKind::Goggles);
        assert_eq!(eq.aggregate_effect().vision_multiplier, 2);
    }
}
