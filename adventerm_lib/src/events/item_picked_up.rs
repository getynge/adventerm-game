use crate::event::Event;
use crate::items::ItemKind;

/// The player picked up an item from the floor. No default subscribers
/// today — surfaced for future logging / achievements / quests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPickedUp {
    pub kind: ItemKind,
    pub pos: (usize, usize),
}

impl Event for ItemPickedUp {}
