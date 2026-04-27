use crate::items::behavior::{EquipEffect, ItemBehavior};
use crate::items::category::{EquipSlot, ItemCategory};

/// Worn on the arms; +1 attack.
pub struct GauntletsBehavior;

impl ItemBehavior for GauntletsBehavior {
    fn category(&self) -> ItemCategory {
        ItemCategory::Equipment(EquipSlot::Arms)
    }

    fn equip_effect(&self) -> EquipEffect {
        EquipEffect {
            attack: 1,
            ..EquipEffect::default()
        }
    }
}
