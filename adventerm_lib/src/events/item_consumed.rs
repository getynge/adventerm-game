use crate::event::Event;
use crate::items::{ConsumeOutcome, ItemKind};

/// The player consumed a single-use item. `outcome` describes the side
/// effect (e.g. `LearnedAbility`) so subscribers can branch without
/// knowing about the specific kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemConsumed {
    pub kind: ItemKind,
    pub outcome: ConsumeOutcome,
}

impl Event for ItemConsumed {}
