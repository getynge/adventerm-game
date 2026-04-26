# adventerm_lib

Pure gameplay logic. No `ratatui`/`crossterm` imports. Serde for serialization, otherwise stdlib only.

## Public surface

Re-exported from [lib.rs](../adventerm_lib/src/lib.rs): `Dungeon`, `Door`, `GameState`, `DoorEvent`, `MoveOutcome`, `Room`, `RoomId`, `DoorId`, `TileKind`, `Direction`, `Tile`, `Save`, `SaveError`, `SaveSlot`, `SAVE_VERSION`, `LOS_RANGE`.

## Modules

### [game.rs](../adventerm_lib/src/game.rs) — gameplay state

- `GameState` — owns `Dungeon`, current `RoomId`, player `(x, y)`, `explored: HashMap<RoomId, Vec<bool>>` (per-room memory of seen tiles), and a transient `visible` bitmap (`#[serde(skip)]`, rehydrated by `Save::from_bytes`). Constructed via `GameState::new_seeded(seed)`.
- `MoveOutcome::{Blocked, Moved}` — return type of movement attempts.
- `DoorEvent { from, to, room }` — door transition record.

Key methods:

- `current_room() -> &Room`
- `tile_at(x, y) -> Tile` — for renderers; returns `Wall | Floor | Door | Player` (with player overlay).
- `terrain_at(x, y) -> Tile` — same as `tile_at` minus the player overlay; used to render remembered (out-of-LOS) tiles.
- `is_visible(x, y) -> bool` — currently in LOS (Euclidean radius `LOS_RANGE`, walls block).
- `is_explored(x, y) -> bool` — the player has seen this tile in the current room at least once.
- `player_on_door() -> Option<DoorId>` — UI uses this to show "Press Enter to open door"
- `move_player(direction) -> MoveOutcome` — single step with collision; on a successful move, recomputes visibility.
- `quick_move(direction) -> MoveOutcome` — slide until wall/door/boundary; **stops one tile before doors** (no accidental traversal); refreshes visibility on a successful slide.
- `interact() -> Option<DoorEvent>` — only way to traverse a door; refreshes visibility for the destination room.
- `refresh_visibility()` — public so that `Save::from_bytes` can rehydrate the transient visible bitmap; otherwise called internally after every state change.

Invariants: walls and out-of-bounds block. Player may stand on a door tile but only `interact()` changes rooms. The player's tile is always visible (and therefore always explored).

### [dungeon.rs](../adventerm_lib/src/dungeon.rs) — generation and graph

- `Dungeon { seed, rooms, doors }`, `Door { id, room, pos, leads_to }`.
- `Dungeon::generate(seed)` — deterministic; pipeline: per-room `generate_room` → `build_edges` (linear chain + optional extras) → `place_door` per edge → `carve_to_nearest_floor` to guarantee connectivity.
- `Dungeon::generate_with_room_count(seed, count)` — same pipeline but skips the random room-count draw. Used by the TUI binary's `--dump-rooms` flag; do not use for gameplay (RNG sequence diverges from `generate`).
- Helpers: `room`, `room_mut`, `door`, `step_inward(door_pos, room)` (used by `GameState::interact` for the spawn tile after traversal).

Generation constants (load-bearing — see CLAUDE.md style rule #2):

| Constant | Value |
| --- | --- |
| Rooms per dungeon | 6–10 |
| Room width | 42–65 |
| Room height | 20–31 |
| Subrooms per room | 3–5 |
| Subroom min size | 8 × 6 |
| Extra edges beyond chain | 1–2 (only when ≥4 rooms) |

Invariants: bidirectional door pairs (`leads_to` always reciprocates), every door has a floor neighbor, all rooms reachable from room 0 via the door graph.

### [room.rs](../adventerm_lib/src/room.rs) — single-room grid

- `Room { id, width, height, tiles }` — flat row-major `Vec<TileKind>`, indexed `y * width + x`.
- `RoomId(u32)`, `DoorId(u32)` — newtypes.
- `TileKind::{Wall, Floor, Door(DoorId)}` — `Door` carries the door reference inline.

Methods: `new_filled`, `kind_at`, `set`, `is_walkable` (floor and doors), `in_bounds` (signed), `doors() -> impl Iterator`, `find_door(DoorId)`, `first_floor()` (used as spawn).

### [world.rs](../adventerm_lib/src/world.rs) — frontend-facing primitives

- `Direction::{Up, Down, Left, Right}` (serde) — sole movement input type.
- `Tile::{Wall, Floor, Door, Player}` — what a renderer sees per cell. Distinct from `TileKind`: this enum is collapsed for display and includes the synthetic `Player` variant.

These two enums are the only types exposed for input/rendering — keep them frontend-agnostic (no key codes, no colors).

### [save.rs](../adventerm_lib/src/save.rs) — persistence

- `Save { version, name, state }` — JSON envelope around `GameState`.
- `SaveSlot { path, name, modified }` — directory listing entry.
- `SaveError::{Format(serde_json::Error), UnsupportedVersion { found, expected }}`.
- `SAVE_VERSION = 3` — bump when changing the wire format.

Functions: `Save::new`, `Save::to_bytes`, `Save::from_bytes` (validates version, then calls `state.refresh_visibility()` to rehydrate the transient FOV bitmap), `slugify(name)` (filesystem-safe slug), `slot_path(dir, name)`, `list_saves(dir)` (sorted newest first; tolerates missing dir / corrupt files / version mismatches), `delete_save(path)`.

Always go through these helpers — never roll your own filename or scan directly.

### [los.rs](../adventerm_lib/src/los.rs) — line-of-sight

- `LOS_RANGE: usize = 6` — horizontal radius of the player's vision disc.
- `CELL_ASPECT_Y_OVER_X: f32 = 2.0` — terminal cells are about twice as tall as wide; `dy` is multiplied by this in the distance check so the disc reads as round on screen instead of a vertical ellipse. Vertical reach is therefore ~`LOS_RANGE / 2` tiles.
- `compute_visible(room, origin, out)` — fills `out` with one `bool` per tile (row-major). Origin is always visible; for each tile with `dx² + (dy * aspect)² ≤ LOS_RANGE²`, a Bresenham line from origin determines whether an intermediate wall blocks the endpoint. Endpoint walls are visible (you see the wall, not past it).

Vision is omnidirectional. The asymmetry inherent in per-tile Bresenham is acceptable at this scale; switch to symmetric shadowcasting later if artifacts become noticeable.

### [rng.rs](../adventerm_lib/src/rng.rs) — seeded PRNG

- `Rng { state: u64 }` — xorshift (13 / 7 / 17). Seed 0 substitutes a constant.
- Methods: `new`, `next_u64`, `next_u32`, `range(low, high_exclusive)`, `chance(num, den)`.

Used only by [dungeon.rs](../adventerm_lib/src/dungeon.rs); not for any UI randomness.

## Tests

Each module has unit tests in-file. Coverage hot spots:

- [game.rs](../adventerm_lib/src/game.rs) — movement blocking, door interact, quick-move stop-before-door, room transitions
- [dungeon.rs](../adventerm_lib/src/dungeon.rs) — determinism, door bidirectionality, reachability, floor connectivity
- [save.rs](../adventerm_lib/src/save.rs) — round-trip, version rejection, slugify, listing
- [rng.rs](../adventerm_lib/src/rng.rs) — determinism, seed divergence, range bounds

Run scope-targeted: `cargo test -p adventerm_lib`.
