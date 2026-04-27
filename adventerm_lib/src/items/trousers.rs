use crate::items::behavior::{EquipEffect, ItemBehavior};
use crate::items::category::{EquipSlot, ItemCategory};

/// Worn on the legs; +1 defense.
pub struct TrousersBehavior;

impl ItemBehavior for TrousersBehavior {
    fn category(&self) -> ItemCategory {
        ItemCategory::Equipment(EquipSlot::Legs)
    }

    fn equip_effect(&self) -> EquipEffect {
        EquipEffect {
            defense: 1,
            ..EquipEffect::default()
        }
    }
}
