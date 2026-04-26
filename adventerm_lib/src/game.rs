use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dungeon::{step_inward, Dungeon};
use crate::items::{self, ItemKind, PlaceCtx};
use crate::room::{DoorId, Room, RoomId, TileKind};
use crate::visibility;
use crate::world::{Direction, Tile};

fn step(direction: Direction, x: usize, y: usize) -> (isize, isize) {
    match direction {
        Direction::Up => (x as isize, y as isize - 1),
        Direction::Down => (x as isize, y as isize + 1),
        Direction::Left => (x as isize - 1, y as isize),
        Direction::Right => (x as isize + 1, y as isize),
    }
}

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

pub use crate::items::PlaceOutcome;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub dungeon: Dungeon,
    pub current_room: RoomId,
    pub player: (usize, usize),
    #[serde(default)]
    pub explored: HashMap<RoomId, Vec<bool>>,
    #[serde(default)]
    pub inventory: Vec<ItemKind>,
    #[serde(skip)]
    visible: Vec<bool>,
    /// Cached "tile is within some persistent light's reach" bitmap for the
    /// current room. Recomputed in `refresh_visibility`.
    #[serde(skip)]
    lit: Vec<bool>,
}

impl PartialEq for GameState {
    fn eq(&self, other: &Self) -> bool {
        self.dungeon == other.dungeon
            && self.current_room == other.current_room
            && self.player == other.player
            && self.explored == other.explored
            && self.inventory == other.inventory
    }
}

impl GameState {
    pub fn new_seeded(seed: u64) -> Self {
        let dungeon = Dungeon::generate(seed);
        let start_room = RoomId(0);
        let player = dungeon
            .room(start_room)
            .first_floor()
            .unwrap_or((0, 0));
        let mut state = Self {
            dungeon,
            current_room: start_room,
            player,
            explored: HashMap::new(),
            inventory: Vec::new(),
            visible: Vec::new(),
            lit: Vec::new(),
        };
        state.refresh_visibility();
        state
    }

    pub fn current_room(&self) -> &Room {
        self.dungeon.room(self.current_room)
    }

    pub fn tile_at(&self, x: usize, y: usize) -> Tile {
        if (x, y) == self.player {
            return Tile::Player;
        }
        self.terrain_at(x, y)
    }

    /// Underlying terrain at `(x, y)`, ignoring the player overlay. Used by
    /// the renderer when drawing remembered (out-of-LOS) tiles.
    pub fn terrain_at(&self, x: usize, y: usize) -> Tile {
        match self.current_room().kind_at(x, y) {
            Some(TileKind::Wall) | None => Tile::Wall,
            Some(TileKind::Floor) => Tile::Floor,
            Some(TileKind::Door(_)) => Tile::Door,
        }
    }

    /// True when the tile is currently lit by either player LOS or any
    /// persistent light in the current room.
    pub fn is_visible(&self, x: usize, y: usize) -> bool {
        let room = self.current_room();
        if x >= room.width || y >= room.height {
            return false;
        }
        let i = room.idx(x, y);
        self.visible.get(i).copied().unwrap_or(false)
            || self.lit.get(i).copied().unwrap_or(false)
    }

    pub fn is_explored(&self, x: usize, y: usize) -> bool {
        let room = self.current_room();
        if x >= room.width || y >= room.height {
            return false;
        }
        let i = room.idx(x, y);
        self.explored
            .get(&self.current_room)
            .and_then(|m| m.get(i).copied())
            .unwrap_or(false)
    }

    pub fn player_on_door(&self) -> Option<DoorId> {
        match self.current_room().kind_at(self.player.0, self.player.1) {
            Some(TileKind::Door(id)) => Some(id),
            _ => None,
        }
    }

    /// True iff there is at least one item resting on the player's tile.
    pub fn items_here(&self) -> bool {
        self.current_room().has_item_at(self.player)
    }

    /// First item resting on the player's tile, if any. Used by the renderer
    /// to surface a pickup prompt.
    pub fn peek_item_here(&self) -> Option<ItemKind> {
        self.current_room().items_at(self.player).next()
    }

    /// Pick up one item at the player's tile, push it into the inventory,
    /// return it for status reporting. Returns `None` if no item is here.
    pub fn pick_up_here(&mut self) -> Option<ItemKind> {
        let pos = self.player;
        let room = self.dungeon.room_mut(self.current_room);
        let kind = room.items.take_at(&mut room.world, pos)?;
        self.inventory.push(kind);
        Some(kind)
    }

    /// Place the inventory item at `slot` onto the world by dispatching to
    /// the kind's `ItemBehavior`. `GameState` deliberately does **not** match
    /// on `ItemKind` — the registry in `items::behavior_for` owns that.
    pub fn place_item(&mut self, slot: usize) -> Option<PlaceOutcome> {
        let kind = *self.inventory.get(slot)?;
        let player_pos = self.player;
        let room = self.dungeon.room_mut(self.current_room);
        let mut ctx = PlaceCtx {
            player_pos,
            world: &mut room.world,
            lighting: &mut room.lighting,
        };
        let outcome = items::behavior_for(kind).on_place(&mut ctx);
        self.inventory.remove(slot);
        self.refresh_visibility();
        Some(outcome)
    }

    pub fn move_player(&mut self, direction: Direction) -> MoveOutcome {
        let (x, y) = self.player;
        let (nx, ny) = step(direction, x, y);
        let room = self.current_room();
        if !room.in_bounds(nx, ny) {
            return MoveOutcome::Blocked;
        }
        let (nx, ny) = (nx as usize, ny as usize);
        if !room.is_walkable(nx, ny) {
            return MoveOutcome::Blocked;
        }
        self.player = (nx, ny);
        self.refresh_visibility();
        MoveOutcome::Moved
    }

    /// Slide the player as far as possible in `direction` without stepping
    /// onto an interactable tile (currently doors). Stops when the next tile
    /// is out of bounds, a wall, or a door.
    pub fn quick_move(&mut self, direction: Direction) -> MoveOutcome {
        let mut moved = false;
        loop {
            let (x, y) = self.player;
            let (nx, ny) = step(direction, x, y);
            let room = self.current_room();
            if !room.in_bounds(nx, ny) {
                break;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            match room.kind_at(nx, ny) {
                Some(TileKind::Floor) => {
                    self.player = (nx, ny);
                    moved = true;
                }
                _ => break,
            }
        }
        if moved {
            self.refresh_visibility();
            MoveOutcome::Moved
        } else {
            MoveOutcome::Blocked
        }
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
        // Leaving the current room: any active flares burn out into regular
        // torches before we move on.
        let leaving_room = self.current_room;
        self.dungeon.room_mut(leaving_room).lighting.burn_out_flares();
        self.current_room = target_room;
        self.player = landing;
        self.refresh_visibility();
        Some(DoorEvent {
            from,
            to,
            new_room: target_room,
        })
    }

    /// Recompute `visible` and `lit` for the current room and OR them into
    /// `explored`. Call after any state change that affects what the player
    /// can see (movement, room transition, light placement, post-deserialize
    /// rehydration). The lighting computation lives in `visibility.rs`.
    pub fn refresh_visibility(&mut self) {
        let room_id = self.current_room;
        let player = self.player;
        let room = self.dungeon.room(room_id);
        let len = room.width * room.height;

        visibility::compute_room_lighting(room, player, &mut self.visible, &mut self.lit);

        let memory = self.explored.entry(room_id).or_insert_with(|| vec![false; len]);
        if memory.len() != len {
            memory.resize(len, false);
        }
        for ((m, v), l) in memory
            .iter_mut()
            .zip(self.visible.iter())
            .zip(self.lit.iter())
        {
            if *v || *l {
                *m = true;
            }
        }
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
    fn player_tile_is_always_visible() {
        let state = GameState::new_seeded(11);
        let (px, py) = state.player;
        assert!(state.is_visible(px, py));
        assert!(state.is_explored(px, py));
    }

    #[test]
    fn explored_persists_after_walking_away() {
        let mut state = GameState::new_seeded(11);
        let (px, py) = state.player;
        // Find any tile in current LOS that is not the player's tile.
        let room = state.current_room();
        let mut witness = None;
        for y in 0..room.height {
            for x in 0..room.width {
                if (x, y) != (px, py) && state.is_visible(x, y) {
                    witness = Some((x, y));
                    break;
                }
            }
            if witness.is_some() {
                break;
            }
        }
        let (wx, wy) = witness.expect("at least one tile besides the player should be visible");
        for _ in 0..crate::los::LOS_RANGE + 2 {
            for dir in [Direction::Down, Direction::Up, Direction::Left, Direction::Right] {
                if state.move_player(dir) == MoveOutcome::Moved {
                    break;
                }
            }
        }
        assert!(state.is_explored(wx, wy));
    }

    #[test]
    fn room_transition_uses_target_room_explored_map() {
        let mut state = GameState::new_seeded(17);
        let (_, door_pos) = find_door_position(&state);
        state.player = door_pos;
        state.refresh_visibility();
        let origin_room = state.current_room;
        let event = state.interact().expect("door interact should produce event");
        let dest_room = event.new_room;
        assert_ne!(origin_room, dest_room);
        let (px, py) = state.player;
        assert!(state.is_visible(px, py));
        assert!(state.is_explored(px, py));
        assert!(state.explored.contains_key(&origin_room));
        assert!(state.explored.contains_key(&dest_room));
    }

    #[test]
    fn walk_into_wall_blocks() {
        let mut state = GameState::new_seeded(11);
        let start = state.player;
        let mut blocked_at_least_once = false;
        for _ in 0..50 {
            if state.move_player(Direction::Up) == MoveOutcome::Blocked {
                blocked_at_least_once = true;
                break;
            }
        }
        assert!(blocked_at_least_once);
        let room = state.current_room();
        let (px, py) = state.player;
        assert!(room.is_walkable(px, py));
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
        let room = state.current_room();
        assert!(room.is_walkable(state.player.0, state.player.1));
    }

    #[test]
    fn quick_move_stops_before_door() {
        let mut state = GameState::new_seeded(17);
        let (_, door_pos) = find_door_position(&state);
        let room = state.current_room();
        let candidates: Vec<(Direction, (usize, usize))> = (0..room.width)
            .flat_map(|x| (0..room.height).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                matches!(room.kind_at(x, y), Some(TileKind::Floor))
                    && (x == door_pos.0 || y == door_pos.1)
                    && (x, y) != door_pos
            })
            .filter_map(|(x, y)| {
                let dir = if y == door_pos.1 && x < door_pos.0 {
                    Direction::Right
                } else if y == door_pos.1 && x > door_pos.0 {
                    Direction::Left
                } else if x == door_pos.0 && y < door_pos.1 {
                    Direction::Down
                } else if x == door_pos.0 && y > door_pos.1 {
                    Direction::Up
                } else {
                    return None;
                };
                Some((dir, (x, y)))
            })
            .collect();
        let (dir, start) = *candidates
            .first()
            .expect("door should have a floor tile in line with it");
        state.player = start;
        let outcome = state.quick_move(dir);
        let (px, py) = state.player;
        assert_ne!((px, py), door_pos);
        assert!(matches!(
            state.current_room().kind_at(px, py),
            Some(TileKind::Floor)
        ));
        let adjacent = (start.0 as isize - door_pos.0 as isize).abs()
            + (start.1 as isize - door_pos.1 as isize).abs()
            == 1;
        if adjacent {
            assert_eq!(outcome, MoveOutcome::Blocked);
            assert_eq!((px, py), start);
        } else {
            assert_eq!(outcome, MoveOutcome::Moved);
        }
    }

    #[test]
    fn quick_move_into_wall_blocks_when_no_floor_step() {
        let mut state = GameState::new_seeded(11);
        state.quick_move(Direction::Up);
        let before = state.player;
        let outcome = state.quick_move(Direction::Up);
        assert_eq!(outcome, MoveOutcome::Blocked);
        assert_eq!(state.player, before);
    }

    #[test]
    fn walking_onto_door_does_not_transition() {
        let mut state = GameState::new_seeded(19);
        let prev_room = state.current_room;
        let (_, door_pos) = find_door_position(&state);
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

    #[test]
    fn flare_lights_entire_room_then_burns_out() {
        let mut state = GameState::new_seeded(17);
        let placement = state.player;
        state.inventory.push(ItemKind::Flare);
        assert_eq!(state.place_item(0), Some(PlaceOutcome::FlarePlaced));
        let room_id = state.current_room;
        // The flare is recorded on the room and every tile reads as visible.
        let room = state.current_room();
        assert!(room.lighting.any_flare_active());
        for y in 0..room.height {
            for x in 0..room.width {
                assert!(state.is_visible(x, y));
            }
        }
        // Walk to a door and traverse: the flare burns out into a torch.
        let (_, door_pos) = find_door_position(&state);
        state.player = door_pos;
        state.refresh_visibility();
        state
            .interact()
            .expect("door interact should produce event");
        assert_ne!(state.current_room, room_id);
        let prev = state.dungeon.room(room_id);
        assert!(!prev.lighting.any_flare_active());
        assert!(prev.has_light_at(placement));
    }

    #[test]
    fn placing_torch_lights_surrounding_tiles() {
        let mut state = GameState::new_seeded(11);
        let placement = state.player;
        // Inject a torch into inventory.
        state.inventory.push(ItemKind::Torch);
        assert_eq!(state.place_item(0), Some(PlaceOutcome::TorchPlaced));
        assert!(state.inventory.is_empty());
        assert!(state.current_room().has_light_at(placement));
        // Walk away far enough that the placement tile leaves player LOS, but
        // it should still report visible because of the placed light.
        for _ in 0..crate::los::LOS_RANGE + 4 {
            for dir in [Direction::Right, Direction::Down, Direction::Left, Direction::Up] {
                if state.move_player(dir) == MoveOutcome::Moved {
                    break;
                }
            }
        }
        if state.current_room == RoomId(0) {
            // We may or may not be far enough away. If we are, the lit map
            // should still show the placement tile.
            assert!(state.is_visible(placement.0, placement.1));
        }
    }
}
