use crate::event::Event;
use crate::room::RoomId;

/// A flare in `room` at `pos` finished burning and was downgraded to a
/// permanent light source. Emitted once per converted flare during a door
/// traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlareBurnedOut {
    pub room: RoomId,
    pub pos: (usize, usize),
}

impl Event for FlareBurnedOut {}
