pub mod behavior;
pub mod boots;
pub mod category;
pub mod flare;
pub mod gauntlets;
pub mod goggles;
pub mod kind;
pub mod scroll_of_fire;
pub mod shirt;
pub mod storage;
pub mod torch;
pub mod trousers;

pub use behavior::{
    behavior_for, equip_slot_of, ConsumeCtx, ConsumeIntent, ConsumeOutcome, ConsumeTarget,
    EquipEffect, ItemBehavior, PlaceCtx, PlaceOutcome,
};
pub use category::{EquipSlot, ItemCategory};
pub use kind::ItemKind;
pub use storage::ItemSubsystem;

/// Binary-visible accessor for an item's category. The binary uses this to
/// decide which action to dispatch (Place / Equip / Consume) without
/// matching on `ItemKind` itself.
pub fn category_of(kind: ItemKind) -> ItemCategory {
    behavior_for(kind).category()
}

/// Binary-visible accessor for a consumable's targeting requirement.
pub fn consume_intent_of(kind: ItemKind) -> ConsumeIntent {
    behavior_for(kind).consume_intent()
}
