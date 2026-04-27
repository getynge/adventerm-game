use crate::event::Event;
use crate::items::{ItemKind, PlaceOutcome};

/// An item was placed onto the world. Visibility refreshes because lit
/// items (torches, flares) change what the player can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPlaced {
    pub kind: ItemKind,
    pub pos: (usize, usize),
    pub outcome: PlaceOutcome,
}

impl Event for ItemPlaced {}
