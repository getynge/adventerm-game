use crate::items::behavior::{EquipEffect, ItemBehavior};
use crate::items::category::{EquipSlot, ItemCategory};

/// Worn on the torso; +1 defense.
pub struct ShirtBehavior;

impl ItemBehavior for ShirtBehavior {
    fn category(&self) -> ItemCategory {
        ItemCategory::Equipment(EquipSlot::Torso)
    }

    fn equip_effect(&self) -> EquipEffect {
        EquipEffect {
            defense: 1,
            ..EquipEffect::default()
        }
    }
}
