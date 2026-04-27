use crate::event::Event;

/// The player entity stepped from `from` to `to`. Subscribers include
/// visibility (recompute LOS / lit) and enemy AI (advance one tick).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerMoved {
    pub from: (usize, usize),
    pub to: (usize, usize),
}

impl Event for PlayerMoved {}
