use serde::{Deserialize, Serialize};

use crate::abilities::Abilities;
use crate::dungeon::Dungeon;
use crate::ecs::EntityId;
use crate::equipment::Equipment;
use crate::explored::ExploredSubsystem;
use crate::items::ItemKind;
use crate::player::PlayerSubsystem;
use crate::room::{DoorId, Room, RoomId, TileKind};
use crate::stats::Stats;
use crate::systems;
use crate::world::Tile;

/// Constant XOR'd into the dungeon seed when re-seeding the per-tick enemy
/// RNG. Keeps enemy AI reproducible from the dungeon seed alone without
/// making the AI sequence identical to dungeon generation's draws. Exposed
/// so the `systems::enemy_tick` module can use the same value.
pub const ENEMY_RNG_SALT: u64 = 0x4144_5645_4E54_524D;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    Blocked,
    Moved,
    /// The player's move ended adjacent to (or stepped into) an enemy that
    /// wants to engage. The binary opens the battle screen with this entity.
    Encounter(EntityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorEvent {
    pub from: DoorId,
    pub to: DoorId,
    pub new_room: RoomId,
}

pub use crate::items::PlaceOutcome;

/// Top-level game container. All player-side state lives in ECS components on
/// the `player` subsystem; per-room exploration memory lives in the `explored`
/// subsystem; per-room construct state (lights, items, enemies) lives on
/// `Room`. `GameState` itself holds no loose game-logic fields — it only
/// dispatches to the subsystems.
///
/// `pending_encounter` is a transient slot a handler can fill during
/// dispatch to surface "the player ended adjacent to an enemy" up to the
/// binary. It is `#[serde(skip)]` because no in-flight encounter survives
/// across save/load. The binary reads it via `take_pending_encounter`
/// after each dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub dungeon: Dungeon,
    pub current_room: RoomId,
    #[serde(default)]
    pub player: PlayerSubsystem,
    #[serde(default)]
    pub explored: ExploredSubsystem,
    #[serde(skip)]
    pending_encounter: Option<EntityId>,
    /// Dev-console override: when true, every tile of the current room is
    /// treated as visible and lit. Transient — cleared on load.
    #[serde(skip)]
    fullbright: bool,
}

impl PartialEq for GameState {
    fn eq(&self, other: &Self) -> bool {
        // Transient `pending_encounter` and `fullbright` are intentionally
        // excluded from equality so a freshly-loaded state and the pre-save
        // state compare equal during round-trip tests.
        self.dungeon == other.dungeon
            && self.current_room == other.current_room
            && self.player == other.player
            && self.explored == other.explored
    }
}

impl Eq for GameState {}

// `Default` for `PlayerSubsystem` is needed for `#[serde(default)]` to fall
// back gracefully on an old save shape. It spawns a player at the origin;
// `new_seeded` immediately overwrites the position with a real floor tile.
impl Default for PlayerSubsystem {
    fn default() -> Self {
        PlayerSubsystem::new_at((0, 0))
    }
}

impl GameState {
    pub fn new_seeded(seed: u64) -> Self {
        let dungeon = Dungeon::generate(seed);
        let start_room = RoomId(0);
        let player_pos = dungeon.room(start_room).first_floor().unwrap_or((0, 0));
        let player = PlayerSubsystem::new_at(player_pos);
        let mut state = Self {
            dungeon,
            current_room: start_room,
            player,
            explored: ExploredSubsystem::default(),
            pending_encounter: None,
            fullbright: false,
        };
        // Seed the per-game enemy RNG up front so behavior matches the
        // pre-ECS layout exactly. (Lazy rehydration covers the load path.)
        let _ = state.player.enemy_rng_mut(state.dungeon.seed, ENEMY_RNG_SALT);
        state.refresh_visibility();
        state
    }

    // ---- read-only facade -------------------------------------------------

    pub fn current_room(&self) -> &Room {
        self.dungeon.room(self.current_room)
    }

    pub fn player_pos(&self) -> (usize, usize) {
        self.player.position()
    }

    pub fn stats(&self) -> &Stats {
        self.player.stats()
    }

    /// Player stats with equipment bonuses applied. Combat reads this; the
    /// inventory UI shows both the base and effective values.
    pub fn effective_stats(&self) -> Stats {
        self.player.effective_stats()
    }

    pub fn equipment(&self) -> &Equipment {
        self.player.equipment()
    }

    /// Effective LOS radius after equipment multipliers (e.g. goggles).
    /// Light sources use the unrelated `crate::los::LIGHT_RANGE` constant.
    pub fn vision_radius(&self) -> usize {
        self.player.vision_radius()
    }

    pub fn cur_health(&self) -> u8 {
        self.player.cur_health()
    }

    pub fn set_cur_health(&mut self, hp: u8) {
        self.player.set_cur_health(hp);
    }

    pub fn inventory(&self) -> &[ItemKind] {
        self.player.inventory()
    }

    pub fn abilities(&self) -> &Abilities {
        self.player.abilities()
    }

    pub fn tile_at(&self, x: usize, y: usize) -> Tile {
        if (x, y) == self.player.position() {
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
        let cache = self.player.visibility();
        cache.visible.get(i).copied().unwrap_or(false)
            || cache.lit.get(i).copied().unwrap_or(false)
    }

    pub fn is_explored(&self, x: usize, y: usize) -> bool {
        let room = self.current_room();
        if x >= room.width || y >= room.height {
            return false;
        }
        self.explored.is_explored(self.current_room, room.idx(x, y))
    }

    pub fn player_on_door(&self) -> Option<DoorId> {
        let (px, py) = self.player.position();
        match self.current_room().kind_at(px, py) {
            Some(TileKind::Door(id)) => Some(id),
            _ => None,
        }
    }

    /// True iff there is at least one item resting on the player's tile.
    pub fn items_here(&self) -> bool {
        self.current_room().has_item_at(self.player.position())
    }

    /// First item resting on the player's tile, if any. Used by the renderer
    /// to surface a pickup prompt.
    pub fn peek_item_here(&self) -> Option<ItemKind> {
        self.current_room().items_at(self.player.position()).next()
    }

    // ---- transient dispatch slot ----------------------------------------

    /// Set during a dispatch when a handler determines the player is in an
    /// encounter (e.g. an enemy stepped adjacent during the enemy-tick
    /// reaction to a `PlayerMoved`). Cleared by `take_pending_encounter`.
    pub fn set_pending_encounter(&mut self, entity: EntityId) {
        self.pending_encounter = Some(entity);
    }

    /// Read and clear the pending-encounter slot. The binary calls this
    /// after each `dispatch` to decide whether to open the battle screen.
    pub fn take_pending_encounter(&mut self) -> Option<EntityId> {
        self.pending_encounter.take()
    }

    /// Non-clearing read. Used by multi-step actions (slide / quick-move)
    /// that need to abort early on the first encounter without consuming
    /// the slot before the binary can see it.
    pub fn peek_pending_encounter(&self) -> Option<EntityId> {
        self.pending_encounter
    }

    /// Dev-console "fullbright" override: when on, every tile of the current
    /// room renders visible and lit. Transient — never serialized.
    pub fn fullbright(&self) -> bool {
        self.fullbright
    }

    pub fn set_fullbright(&mut self, on: bool) {
        self.fullbright = on;
    }

    /// Recompute the player's visibility / lit cache for the current room
    /// and merge the result into `explored`. Called during `new_seeded`
    /// and save-load — both situations where dispatch is not running so
    /// the [`crate::events::PlayerMoved`] handler chain cannot be relied
    /// on. Gameplay-time refreshes happen automatically via the
    /// [`crate::systems::visibility::VisibilityHandler`] subscription.
    pub fn refresh_visibility(&mut self) {
        systems::refresh_visibility(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::action::dispatch;
    use crate::actions::{
        DefeatEnemyAction, InteractAction, MoveAction, PlaceItemAction, QuickMoveAction,
    };
    use crate::world::Direction;

    fn step_dir(state: &mut GameState, dir: Direction) -> MoveOutcome {
        let player = state.player.entity();
        dispatch(state, player, MoveAction { direction: dir })
    }

    fn quick_dir(state: &mut GameState, dir: Direction) -> MoveOutcome {
        let player = state.player.entity();
        dispatch(state, player, QuickMoveAction { direction: dir })
    }

    fn interact(state: &mut GameState) -> Option<DoorEvent> {
        let player = state.player.entity();
        dispatch(state, player, InteractAction)
    }

    fn place(state: &mut GameState, slot: usize) -> Option<PlaceOutcome> {
        let player = state.player.entity();
        dispatch(state, player, PlaceItemAction { slot })
    }

    fn defeat(state: &mut GameState, room: RoomId, entity: EntityId) {
        let player = state.player.entity();
        dispatch(
            state,
            player,
            DefeatEnemyAction {
                room,
                entity,
            },
        );
    }

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
        let (px, py) = state.player_pos();
        assert!(state.is_visible(px, py));
        assert!(state.is_explored(px, py));
    }

    #[test]
    fn explored_persists_after_walking_away() {
        let mut state = GameState::new_seeded(11);
        let (px, py) = state.player_pos();
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
            for dir in [
                Direction::Down,
                Direction::Up,
                Direction::Left,
                Direction::Right,
            ] {
                if step_dir(&mut state, dir) == MoveOutcome::Moved {
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
        state.player.set_position(door_pos);
        state.refresh_visibility();
        let origin_room = state.current_room;
        let event = interact(&mut state).expect("door interact should produce event");
        let dest_room = event.new_room;
        assert_ne!(origin_room, dest_room);
        let (px, py) = state.player_pos();
        assert!(state.is_visible(px, py));
        assert!(state.is_explored(px, py));
        assert!(state.explored.contains_room(origin_room));
        assert!(state.explored.contains_room(dest_room));
    }

    #[test]
    fn walk_into_wall_blocks() {
        let mut state = GameState::new_seeded(11);
        let start = state.player_pos();
        let mut blocked_at_least_once = false;
        for _ in 0..50 {
            if step_dir(&mut state, Direction::Up) == MoveOutcome::Blocked {
                blocked_at_least_once = true;
                break;
            }
        }
        assert!(blocked_at_least_once);
        let room = state.current_room();
        let (px, py) = state.player_pos();
        assert!(room.is_walkable(px, py));
        assert!(room.is_walkable(start.0, start.1));
    }

    #[test]
    fn interact_off_door_is_noop() {
        let mut state = GameState::new_seeded(13);
        let before_room = state.current_room;
        let before_pos = state.player_pos();
        assert!(interact(&mut state).is_none());
        assert_eq!(state.current_room, before_room);
        assert_eq!(state.player_pos(), before_pos);
    }

    #[test]
    fn interact_on_door_swaps_room_and_repositions() {
        let mut state = GameState::new_seeded(17);
        let (door_id, door_pos) = find_door_position(&state);
        state.player.set_position(door_pos);
        let prev_room = state.current_room;
        let event = interact(&mut state).expect("door interact should produce event");
        assert_eq!(event.from, door_id);
        assert_ne!(state.current_room, prev_room);
        assert_eq!(state.current_room, event.new_room);
        let room = state.current_room();
        let (px, py) = state.player_pos();
        assert!(room.is_walkable(px, py));
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
        state.player.set_position(start);
        let outcome = quick_dir(&mut state, dir);
        let (px, py) = state.player_pos();
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
        quick_dir(&mut state, Direction::Up);
        let before = state.player_pos();
        let outcome = quick_dir(&mut state, Direction::Up);
        assert_eq!(outcome, MoveOutcome::Blocked);
        assert_eq!(state.player_pos(), before);
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
                state.player.set_position((nx as usize, ny as usize));
                placed = true;
                break;
            }
        }
        assert!(placed, "door has no floor neighbor");
        let (sx, sy) = state.player_pos();
        let dx = door_pos.0 as isize - sx as isize;
        let dy = door_pos.1 as isize - sy as isize;
        let dir = match (dx, dy) {
            (1, 0) => Direction::Right,
            (-1, 0) => Direction::Left,
            (0, 1) => Direction::Down,
            (0, -1) => Direction::Up,
            _ => unreachable!(),
        };
        assert_eq!(step_dir(&mut state, dir), MoveOutcome::Moved);
        assert_eq!(state.player_pos(), door_pos);
        assert_eq!(state.current_room, prev_room);
    }

    #[test]
    fn flare_lights_entire_room_then_burns_out() {
        let mut state = GameState::new_seeded(17);
        let placement = state.player_pos();
        state.player.inventory_push(ItemKind::Flare);
        assert_eq!(place(&mut state, 0), Some(PlaceOutcome::FlarePlaced));
        let room_id = state.current_room;
        let room = state.current_room();
        assert!(room.lighting.any_flare_active());
        for y in 0..room.height {
            for x in 0..room.width {
                assert!(state.is_visible(x, y));
            }
        }
        let (_, door_pos) = find_door_position(&state);
        state.player.set_position(door_pos);
        state.refresh_visibility();
        interact(&mut state).expect("door interact should produce event");
        assert_ne!(state.current_room, room_id);
        let prev = state.dungeon.room(room_id);
        assert!(!prev.lighting.any_flare_active());
        assert!(prev.has_light_at(placement));
    }

    #[test]
    fn defeat_enemy_removes_it_from_room() {
        let mut state = GameState::new_seeded(11);
        let target_room = state
            .dungeon
            .rooms
            .iter()
            .find(|r| !r.enemies.is_empty())
            .map(|r| r.id);
        let Some(target_room) = target_room else {
            return;
        };
        let entity = state
            .dungeon
            .room(target_room)
            .enemies
            .entities()
            .next()
            .unwrap();
        defeat(&mut state, target_room, entity);
        let room = state.dungeon.room(target_room);
        assert!(!room.enemies.entities().any(|e| e == entity));
    }

    #[test]
    fn save_round_trip_preserves_combat_fields() {
        use crate::save::Save;
        let mut state = GameState::new_seeded(31);
        state.set_cur_health(state.stats().health - 3);
        let save = Save::new("Combat Run".into(), state.clone());
        let bytes = save.to_bytes();
        let recovered = Save::from_bytes(&bytes).expect("decode");
        assert_eq!(recovered.state.stats(), state.stats());
        assert_eq!(recovered.state.cur_health(), state.cur_health());
        assert_eq!(recovered.state.abilities(), state.abilities());
    }

    #[test]
    fn placing_torch_lights_surrounding_tiles() {
        let mut state = GameState::new_seeded(11);
        let placement = state.player_pos();
        state.player.inventory_push(ItemKind::Torch);
        assert_eq!(place(&mut state, 0), Some(PlaceOutcome::TorchPlaced));
        assert!(state.inventory().is_empty());
        assert!(state.current_room().has_light_at(placement));
        for _ in 0..crate::los::LOS_RANGE + 4 {
            for dir in [
                Direction::Right,
                Direction::Down,
                Direction::Left,
                Direction::Up,
            ] {
                if step_dir(&mut state, dir) == MoveOutcome::Moved {
                    break;
                }
            }
        }
        if state.current_room == RoomId(0) {
            assert!(state.is_visible(placement.0, placement.1));
        }
    }
}
