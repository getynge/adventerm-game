use serde::{Deserialize, Serialize};

use crate::dungeon::{Dungeon, step_inward};
use crate::room::{DoorId, Room, RoomId, TileKind};
use crate::world::{Direction, Tile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    Blocked,
    Moved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorEvent {
    pub from: DoorId,
    pub to: DoorId,
    pub new_room: RoomId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub dungeon: Dungeon,
    pub current_room: RoomId,
    pub player: (usize, usize),
}

impl GameState {
    pub fn new_seeded(seed: u64) -> Self {
        let dungeon = Dungeon::generate(seed);
        let start_room = RoomId(0);
        let player = dungeon
            .room(start_room)
            .first_floor()
            .unwrap_or((0, 0));
        Self {
            dungeon,
            current_room: start_room,
            player,
        }
    }

    pub fn current_room(&self) -> &Room {
        self.dungeon.room(self.current_room)
    }

    pub fn tile_at(&self, x: usize, y: usize) -> Tile {
        if (x, y) == self.player {
            return Tile::Player;
        }
        match self.current_room().kind_at(x, y) {
            Some(TileKind::Wall) | None => Tile::Wall,
            Some(TileKind::Floor) => Tile::Floor,
            Some(TileKind::Door(_)) => Tile::Door,
        }
    }

    pub fn player_on_door(&self) -> Option<DoorId> {
        match self.current_room().kind_at(self.player.0, self.player.1) {
            Some(TileKind::Door(id)) => Some(id),
            _ => None,
        }
    }

    pub fn move_player(&mut self, direction: Direction) -> MoveOutcome {
        let (x, y) = self.player;
        let (nx, ny) = match direction {
            Direction::Up => (x as isize, y as isize - 1),
            Direction::Down => (x as isize, y as isize + 1),
            Direction::Left => (x as isize - 1, y as isize),
            Direction::Right => (x as isize + 1, y as isize),
        };
        let room = self.current_room();
        if !room.in_bounds(nx, ny) {
            return MoveOutcome::Blocked;
        }
        let (nx, ny) = (nx as usize, ny as usize);
        if !room.is_walkable(nx, ny) {
            return MoveOutcome::Blocked;
        }
        self.player = (nx, ny);
        MoveOutcome::Moved
    }

    pub fn interact(&mut self) -> Option<DoorEvent> {
        let here = self.current_room().kind_at(self.player.0, self.player.1)?;
        let door_id = match here {
            TileKind::Door(id) => id,
            _ => return None,
        };
        let from = door_id;
        let to = self.dungeon.door(door_id).leads_to;
        let target_door = self.dungeon.door(to).clone();
        let target_room = target_door.owner;
        let landing = step_inward(target_door.pos, self.dungeon.room(target_room));
        self.current_room = target_room;
        self.player = landing;
        Some(DoorEvent {
            from,
            to,
            new_room: target_room,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_door_position(state: &GameState) -> (DoorId, (usize, usize)) {
        let room = state.current_room();
        for y in 0..room.height {
            for x in 0..room.width {
                if let Some(TileKind::Door(id)) = room.kind_at(x, y) {
                    return (id, (x, y));
                }
            }
        }
        panic!("no door in starting room — generation invariant broken");
    }

    #[test]
    fn walk_into_wall_blocks() {
        let mut state = GameState::new_seeded(11);
        // Walk up many times — eventually we hit a wall.
        let start = state.player;
        let mut blocked_at_least_once = false;
        for _ in 0..50 {
            if state.move_player(Direction::Up) == MoveOutcome::Blocked {
                blocked_at_least_once = true;
                break;
            }
        }
        assert!(blocked_at_least_once);
        // The player position never landed on a wall.
        let room = state.current_room();
        let (px, py) = state.player;
        assert!(room.is_walkable(px, py));
        // Check that the start position was different from a wall too.
        assert!(room.is_walkable(start.0, start.1));
    }

    #[test]
    fn interact_off_door_is_noop() {
        let mut state = GameState::new_seeded(13);
        let before_room = state.current_room;
        let before_pos = state.player;
        assert!(state.interact().is_none());
        assert_eq!(state.current_room, before_room);
        assert_eq!(state.player, before_pos);
    }

    #[test]
    fn interact_on_door_swaps_room_and_repositions() {
        let mut state = GameState::new_seeded(17);
        let (door_id, door_pos) = find_door_position(&state);
        state.player = door_pos;
        let prev_room = state.current_room;
        let event = state.interact().expect("door interact should produce event");
        assert_eq!(event.from, door_id);
        assert_ne!(state.current_room, prev_room);
        assert_eq!(state.current_room, event.new_room);
        // Player ends on a walkable tile in the new room.
        let room = state.current_room();
        assert!(room.is_walkable(state.player.0, state.player.1));
    }

    #[test]
    fn walking_onto_door_does_not_transition() {
        let mut state = GameState::new_seeded(19);
        let prev_room = state.current_room;
        let (_, door_pos) = find_door_position(&state);
        // Teleport adjacent then move onto the door.
        // Find a walkable neighbor of the door inside the same room.
        let room = state.current_room();
        let neighbors = [
            (door_pos.0 as isize - 1, door_pos.1 as isize),
            (door_pos.0 as isize + 1, door_pos.1 as isize),
            (door_pos.0 as isize, door_pos.1 as isize - 1),
            (door_pos.0 as isize, door_pos.1 as isize + 1),
        ];
        let mut placed = false;
        for (nx, ny) in neighbors {
            if room.in_bounds(nx, ny)
                && matches!(
                    room.kind_at(nx as usize, ny as usize),
                    Some(TileKind::Floor)
                )
            {
                state.player = (nx as usize, ny as usize);
                placed = true;
                break;
            }
        }
        assert!(placed, "door has no floor neighbor");
        // Step onto the door.
        let dx = door_pos.0 as isize - state.player.0 as isize;
        let dy = door_pos.1 as isize - state.player.1 as isize;
        let dir = match (dx, dy) {
            (1, 0) => Direction::Right,
            (-1, 0) => Direction::Left,
            (0, 1) => Direction::Down,
            (0, -1) => Direction::Up,
            _ => unreachable!(),
        };
        assert_eq!(state.move_player(dir), MoveOutcome::Moved);
        assert_eq!(state.player, door_pos);
        assert_eq!(state.current_room, prev_room);
    }
}
