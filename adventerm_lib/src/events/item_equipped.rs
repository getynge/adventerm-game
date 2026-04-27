use crate::event::Event;
use crate::items::{EquipSlot, ItemKind};

/// The player equipped an item. The previously equipped item (if any) was
/// pushed back into inventory by the action layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemEquipped {
    pub kind: ItemKind,
    pub slot: EquipSlot,
    pub previous: Option<ItemKind>,
}

impl Event for ItemEquipped {}
