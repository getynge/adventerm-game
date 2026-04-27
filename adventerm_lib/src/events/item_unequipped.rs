use crate::event::Event;
use crate::items::{EquipSlot, ItemKind};

/// The player unequipped an item; the action layer pushed it back into
/// inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemUnequipped {
    pub kind: ItemKind,
    pub slot: EquipSlot,
}

impl Event for ItemUnequipped {}
