//! Per-room "explored" memory subsystem.
//!
//! Each `RoomId` the player has set foot in gets a sentinel entity in the
//! subsystem's own `World` plus an `ExploredMap` component holding the
//! row-major bitmap of "tiles the player has ever seen" for that room.
//!
//! Rooms that have never been entered have no entry; queries against them
//! return `false`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ecs::{ComponentStore, EntityId, World};
use crate::room::RoomId;

/// Row-major "ever seen" bitmap for one room.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExploredMap(pub Vec<bool>);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExploredSubsystem {
    world: World,
    maps: ComponentStore<ExploredMap>,
    /// `RoomId` → the sentinel entity that owns that room's `ExploredMap`.
    /// Stored explicitly so we can look up by `RoomId` in O(1) without
    /// scanning components.
    by_room: HashMap<RoomId, EntityId>,
}

impl PartialEq for ExploredSubsystem {
    fn eq(&self, other: &Self) -> bool {
        // The internal entity ids may differ across instances that hold the
        // same logical maps (e.g. after deserialization). Compare by the
        // (RoomId → ExploredMap) view instead.
        let mine: HashMap<RoomId, &ExploredMap> = self
            .by_room
            .iter()
            .filter_map(|(rid, eid)| self.maps.get(*eid).map(|m| (*rid, m)))
            .collect();
        let theirs: HashMap<RoomId, &ExploredMap> = other
            .by_room
            .iter()
            .filter_map(|(rid, eid)| other.maps.get(*eid).map(|m| (*rid, m)))
            .collect();
        mine == theirs
    }
}

impl Eq for ExploredSubsystem {}

impl ExploredSubsystem {
    /// Mark `(x, y)` as explored in `room`. The map is grown / created lazily
    /// to fit `room_len`.
    pub fn mark(&mut self, room: RoomId, room_len: usize, idx: usize) {
        let map = self.ensure(room, room_len);
        if idx < map.0.len() {
            map.0[idx] = true;
        }
    }

    /// Whether `idx` is explored in `room`. Out-of-range indices read as `false`.
    pub fn is_explored(&self, room: RoomId, idx: usize) -> bool {
        let Some(eid) = self.by_room.get(&room) else {
            return false;
        };
        let Some(map) = self.maps.get(*eid) else {
            return false;
        };
        map.0.get(idx).copied().unwrap_or(false)
    }

    /// Whether the subsystem has any record for `room`. Used by tests.
    pub fn contains_room(&self, room: RoomId) -> bool {
        self.by_room.contains_key(&room)
    }

    /// Bulk-merge a row-major "currently visible" bitmap into the room's
    /// memory. `room_len` is `width * height`; both inputs must agree.
    pub fn merge_room(&mut self, room: RoomId, room_len: usize, visible: &[bool], lit: &[bool]) {
        let map = self.ensure(room, room_len);
        for ((m, v), l) in map.0.iter_mut().zip(visible.iter()).zip(lit.iter()) {
            if *v || *l {
                *m = true;
            }
        }
    }

    fn ensure(&mut self, room: RoomId, room_len: usize) -> &mut ExploredMap {
        let eid = *self.by_room.entry(room).or_insert_with(|| {
            let e = self.world.spawn();
            self.maps.insert(e, ExploredMap(vec![false; room_len]));
            e
        });
        let map = self.maps.get_mut(eid).expect("explored map");
        if map.0.len() != room_len {
            map.0.resize(room_len, false);
        }
        map
    }
}
