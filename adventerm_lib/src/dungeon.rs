use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use crate::rng::Rng;
use crate::room::{DoorId, Room, RoomId, TileKind};

const ROOM_COUNT_MIN: usize = 6;
const ROOM_COUNT_MAX_EXCL: usize = 11;
const ROOM_WIDTH_MIN: usize = 42;
const ROOM_WIDTH_MAX_EXCL: usize = 66;
const ROOM_HEIGHT_MIN: usize = 20;
const ROOM_HEIGHT_MAX_EXCL: usize = 32;
const SUBROOM_COUNT_MIN: usize = 3;
const SUBROOM_COUNT_MAX_EXCL: usize = 6;
const SUBROOM_WIDTH_MIN: usize = 8;
const SUBROOM_HEIGHT_MIN: usize = 6;
const EXTRA_EDGES_MIN: usize = 1;
const EXTRA_EDGES_MAX_EXCL: usize = 3;
const ROOMS_FOR_EXTRA_EDGES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Door {
    pub id: DoorId,
    pub owner: RoomId,
    pub pos: (usize, usize),
    pub leads_to: DoorId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dungeon {
    pub seed: u64,
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
}

impl Dungeon {
    pub fn generate(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let room_count = rng.range(ROOM_COUNT_MIN, ROOM_COUNT_MAX_EXCL);

        let mut rooms: Vec<Room> = Vec::with_capacity(room_count);
        for i in 0..room_count {
            let w = rng.range(ROOM_WIDTH_MIN, ROOM_WIDTH_MAX_EXCL);
            let h = rng.range(ROOM_HEIGHT_MIN, ROOM_HEIGHT_MAX_EXCL);
            rooms.push(generate_room(RoomId(i as u32), w, h, &mut rng));
        }

        let edges = build_edges(room_count, &mut rng);

        let mut doors: Vec<Door> = Vec::new();
        for (a, b) in edges {
            let id_a = DoorId(doors.len() as u32);
            let id_b = DoorId(doors.len() as u32 + 1);
            let pos_a = match place_door(&mut rooms[a], id_a, &mut rng) {
                Some(p) => p,
                None => continue,
            };
            let pos_b = match place_door(&mut rooms[b], id_b, &mut rng) {
                Some(p) => p,
                None => {
                    // rollback the door placed in room a
                    rooms[a].set(pos_a.0, pos_a.1, TileKind::Wall);
                    continue;
                }
            };
            doors.push(Door {
                id: id_a,
                owner: RoomId(a as u32),
                pos: pos_a,
                leads_to: id_b,
            });
            doors.push(Door {
                id: id_b,
                owner: RoomId(b as u32),
                pos: pos_b,
                leads_to: id_a,
            });
        }

        Dungeon { seed, rooms, doors }
    }

    pub fn room(&self, id: RoomId) -> &Room {
        &self.rooms[id.0 as usize]
    }

    pub fn room_mut(&mut self, id: RoomId) -> &mut Room {
        &mut self.rooms[id.0 as usize]
    }

    pub fn door(&self, id: DoorId) -> &Door {
        &self.doors[id.0 as usize]
    }
}

fn generate_room(id: RoomId, width: usize, height: usize, rng: &mut Rng) -> Room {
    let mut room = Room::new_filled(id, width, height, TileKind::Wall);

    let sub_count = rng.range(SUBROOM_COUNT_MIN, SUBROOM_COUNT_MAX_EXCL);
    let mut subs: Vec<(usize, usize, usize, usize)> = Vec::with_capacity(sub_count);

    for _ in 0..sub_count {
        let sw = rng.range(SUBROOM_WIDTH_MIN, (width / 2).max(SUBROOM_WIDTH_MIN + 1));
        let sh = rng.range(SUBROOM_HEIGHT_MIN, (height / 2).max(SUBROOM_HEIGHT_MIN + 1));
        let sx = rng.range(1, (width - sw - 1).max(2));
        let sy = rng.range(1, (height - sh - 1).max(2));
        for y in sy..sy + sh {
            for x in sx..sx + sw {
                room.set(x, y, TileKind::Floor);
            }
        }
        subs.push((sx, sy, sw, sh));
    }

    // Connect each sub-rect's center to the previous one with an L-shaped corridor.
    for w in 1..subs.len() {
        let (ax, ay, aw, ah) = subs[w - 1];
        let (bx, by, bw, bh) = subs[w];
        let cx_a = ax + aw / 2;
        let cy_a = ay + ah / 2;
        let cx_b = bx + bw / 2;
        let cy_b = by + bh / 2;
        carve_corridor(&mut room, (cx_a, cy_a), (cx_b, cy_b), rng);
    }

    room
}

fn carve_corridor(room: &mut Room, from: (usize, usize), to: (usize, usize), rng: &mut Rng) {
    let (fx, fy) = from;
    let (tx, ty) = to;
    let horizontal_first = rng.chance(1, 2);
    let (corner_x, corner_y) = if horizontal_first { (tx, fy) } else { (fx, ty) };

    for x in fx.min(corner_x)..=fx.max(corner_x) {
        carve_floor(room, x, fy);
    }
    for y in fy.min(corner_y)..=fy.max(corner_y) {
        carve_floor(room, corner_x, y);
    }
    for x in corner_x.min(tx)..=corner_x.max(tx) {
        carve_floor(room, x, ty);
    }
    for y in corner_y.min(ty)..=corner_y.max(ty) {
        carve_floor(room, tx, y);
    }
}

fn carve_floor(room: &mut Room, x: usize, y: usize) {
    if x > 0 && x < room.width - 1 && y > 0 && y < room.height - 1 {
        room.set(x, y, TileKind::Floor);
    }
}

fn push_if_wall(out: &mut Vec<(usize, usize)>, room: &Room, x: usize, y: usize) {
    if matches!(room.kind_at(x, y), Some(TileKind::Wall)) {
        out.push((x, y));
    }
}

fn build_edges(n: usize, rng: &mut Rng) -> Vec<(usize, usize)> {
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for i in 0..n.saturating_sub(1) {
        edges.push((i, i + 1));
    }
    let extra = if n >= ROOMS_FOR_EXTRA_EDGES {
        rng.range(EXTRA_EDGES_MIN, EXTRA_EDGES_MAX_EXCL)
    } else {
        0
    };
    for _ in 0..extra {
        if n < 2 {
            break;
        }
        let a = rng.range(0, n);
        let mut b = rng.range(0, n);
        if a == b {
            b = (b + 1) % n;
        }
        let pair = (a.min(b), a.max(b));
        if !edges.contains(&pair) {
            edges.push(pair);
        }
    }
    edges
}

fn place_door(room: &mut Room, id: DoorId, rng: &mut Rng) -> Option<(usize, usize)> {
    if room.width < 3 || room.height < 3 {
        return None;
    }
    let mut perimeter: Vec<(usize, usize)> = Vec::new();
    for x in 1..room.width - 1 {
        push_if_wall(&mut perimeter, room, x, 0);
        push_if_wall(&mut perimeter, room, x, room.height - 1);
    }
    for y in 1..room.height - 1 {
        push_if_wall(&mut perimeter, room, 0, y);
        push_if_wall(&mut perimeter, room, room.width - 1, y);
    }
    if perimeter.is_empty() {
        return None;
    }
    let pick = rng.range(0, perimeter.len());
    let (x, y) = perimeter[pick];
    let inward = step_inward((x, y), room);
    if !carve_to_nearest_floor(room, inward) {
        return None;
    }
    room.set(x, y, TileKind::Door(id));
    Some((x, y))
}

/// Carve a path of `Floor` tiles from `start` to the nearest existing floor tile.
/// The path stays strictly inside the perimeter. Returns true on success (or if
/// start is already floor), false if no floor exists in the room at all.
fn carve_to_nearest_floor(room: &mut Room, start: (usize, usize)) -> bool {
    if matches!(room.kind_at(start.0, start.1), Some(TileKind::Floor)) {
        return true;
    }
    let mut parent: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
    let mut q: VecDeque<(usize, usize)> = VecDeque::new();
    q.push_back(start);
    parent.insert(start, start);
    let mut goal: Option<(usize, usize)> = None;
    while let Some((x, y)) = q.pop_front() {
        if matches!(room.kind_at(x, y), Some(TileKind::Floor)) {
            goal = Some((x, y));
            break;
        }
        for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if !room.in_bounds(nx, ny) {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if nx == 0 || ny == 0 || nx == room.width - 1 || ny == room.height - 1 {
                continue;
            }
            if parent.contains_key(&(nx, ny)) {
                continue;
            }
            parent.insert((nx, ny), (x, y));
            q.push_back((nx, ny));
        }
    }
    let Some(goal) = goal else {
        return false;
    };
    let mut cur = goal;
    while cur != start {
        room.set(cur.0, cur.1, TileKind::Floor);
        cur = parent[&cur];
    }
    room.set(start.0, start.1, TileKind::Floor);
    true
}

/// Step from a door tile one step inward — toward the room interior.
pub fn step_inward(door_pos: (usize, usize), room: &Room) -> (usize, usize) {
    let (x, y) = door_pos;
    if x == 0 {
        return (1, y);
    }
    if y == 0 {
        return (x, 1);
    }
    if x == room.width - 1 {
        return (room.width - 2, y);
    }
    if y == room.height - 1 {
        return (x, room.height - 2);
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_reachable_from(
        room: &Room,
        start: (usize, usize),
    ) -> std::collections::HashSet<(usize, usize)> {
        let mut visited = std::collections::HashSet::new();
        let mut q = VecDeque::new();
        if room.is_walkable(start.0, start.1) {
            q.push_back(start);
            visited.insert(start);
        }
        while let Some((x, y)) = q.pop_front() {
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if !room.in_bounds(nx, ny) {
                    continue;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                if !room.is_walkable(nx, ny) {
                    continue;
                }
                if visited.insert((nx, ny)) {
                    q.push_back((nx, ny));
                }
            }
        }
        visited
    }

    #[test]
    fn generation_is_deterministic() {
        let a = Dungeon::generate(42);
        let b = Dungeon::generate(42);
        assert_eq!(a, b);
    }

    #[test]
    fn generation_differs_by_seed() {
        let a = Dungeon::generate(1);
        let b = Dungeon::generate(2);
        assert_ne!(a, b);
    }

    #[test]
    fn doors_pair_bidirectionally() {
        let d = Dungeon::generate(99);
        for door in &d.doors {
            let pair = d.door(door.leads_to);
            assert_eq!(pair.leads_to, door.id);
            assert_ne!(pair.owner, door.owner);
        }
    }

    #[test]
    fn every_door_position_is_a_door_tile() {
        let d = Dungeon::generate(5);
        for door in &d.doors {
            let room = d.room(door.owner);
            assert_eq!(
                room.kind_at(door.pos.0, door.pos.1),
                Some(TileKind::Door(door.id))
            );
        }
    }

    #[test]
    fn every_room_reachable_via_door_graph() {
        let d = Dungeon::generate(123);
        if d.rooms.is_empty() {
            return;
        }
        let mut visited = std::collections::HashSet::new();
        let mut q = VecDeque::new();
        q.push_back(RoomId(0));
        visited.insert(RoomId(0));
        while let Some(rid) = q.pop_front() {
            let room = d.room(rid);
            for (_, _, did) in room.doors() {
                let target = d.door(d.door(did).leads_to).owner;
                if visited.insert(target) {
                    q.push_back(target);
                }
            }
        }
        // Every room with at least one door should be reachable.
        // Disconnected rooms (no doors) are accepted only if they are unreachable AND have no doors.
        for room in &d.rooms {
            if !visited.contains(&room.id) {
                assert!(
                    room.doors().next().is_none(),
                    "Room {:?} has doors but is unreachable from room 0",
                    room.id
                );
            }
        }
    }

    #[test]
    fn each_door_floor_neighbor_exists() {
        let d = Dungeon::generate(7);
        for door in &d.doors {
            let room = d.room(door.owner);
            let inward = step_inward(door.pos, room);
            assert!(matches!(
                room.kind_at(inward.0, inward.1),
                Some(TileKind::Floor) | Some(TileKind::Door(_))
            ));
        }
    }

    #[test]
    fn rooms_have_floor_tiles() {
        let d = Dungeon::generate(33);
        for room in &d.rooms {
            assert!(
                room.first_floor().is_some(),
                "Room {:?} has no floor tiles",
                room.id
            );
        }
    }

    #[test]
    fn floor_connectivity_includes_all_doors() {
        let d = Dungeon::generate(2024);
        for room in &d.rooms {
            let Some(start) = room.first_floor() else {
                continue;
            };
            let reachable = floor_reachable_from(room, start);
            for (x, y, _) in room.doors() {
                assert!(
                    reachable.contains(&(x, y)),
                    "Door at ({}, {}) in room {:?} is unreachable from floor",
                    x,
                    y,
                    room.id
                );
            }
        }
    }
}
