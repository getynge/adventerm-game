use serde::{Deserialize, Serialize};

/// Where an equipment item is worn. Five fixed body slots; an item declares
/// its slot via [`super::ItemBehavior::category`] returning
/// [`ItemCategory::Equipment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Head,
    Torso,
    Arms,
    Legs,
    Feet,
}

impl EquipSlot {
    pub const ALL: [EquipSlot; 5] = [
        EquipSlot::Head,
        EquipSlot::Torso,
        EquipSlot::Arms,
        EquipSlot::Legs,
        EquipSlot::Feet,
    ];

    pub fn name(self) -> &'static str {
        match self {
            EquipSlot::Head => "Head",
            EquipSlot::Torso => "Torso",
            EquipSlot::Arms => "Arms",
            EquipSlot::Legs => "Legs",
            EquipSlot::Feet => "Feet",
        }
    }
}

/// What a player can do with an item, dispatched on at the action layer to
/// route Confirm to the right action (`PlaceItemAction`, `EquipItemAction`,
/// `ConsumeItemAction`). Each `ItemKind` declares its category through
/// [`super::ItemBehavior::category`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemCategory {
    Placeable,
    Equipment(EquipSlot),
    Consumable,
}
