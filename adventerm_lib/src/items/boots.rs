use crate::items::behavior::{EquipEffect, ItemBehavior};
use crate::items::category::{EquipSlot, ItemCategory};

/// Worn on the feet; +1 speed.
pub struct BootsBehavior;

impl ItemBehavior for BootsBehavior {
    fn category(&self) -> ItemCategory {
        ItemCategory::Equipment(EquipSlot::Feet)
    }

    fn equip_effect(&self) -> EquipEffect {
        EquipEffect {
            speed: 1,
            ..EquipEffect::default()
        }
    }
}
