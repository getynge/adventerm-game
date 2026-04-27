use crate::items::behavior::{EquipEffect, ItemBehavior};
use crate::items::category::{EquipSlot, ItemCategory};

/// Worn on the head; doubles the player's line-of-sight radius. Light
/// sources are unaffected — the multiplier is applied at the LOS call site,
/// not on the shared lighting constant.
pub struct GogglesBehavior;

impl ItemBehavior for GogglesBehavior {
    fn category(&self) -> ItemCategory {
        ItemCategory::Equipment(EquipSlot::Head)
    }

    fn equip_effect(&self) -> EquipEffect {
        EquipEffect {
            vision_multiplier: 2,
            ..EquipEffect::default()
        }
    }
}
