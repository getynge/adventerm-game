# adventerm_lib

Pure gameplay logic. No `ratatui`/`crossterm` imports. Serde for serialization, otherwise stdlib only.

## Public surface

Re-exported from [lib.rs](../adventerm_lib/src/lib.rs): `Dungeon`, `Door`, `GameState`, `DoorEvent`, `MoveOutcome`, `PlaceOutcome`, `ItemKind`, `EntityId`, `World`, `Room`, `RoomId`, `DoorId`, `TileKind`, `Direction`, `Tile`, `Save`, `SaveError`, `SaveSlot`, `SAVE_VERSION`, `LIGHT_RANGE`, `LOS_RANGE`.

## Modules

### [game.rs](../adventerm_lib/src/game.rs) — gameplay state

- `GameState` — owns `Dungeon`, current `RoomId`, player `(x, y)`, `explored: HashMap<RoomId, Vec<bool>>` (per-room memory of seen tiles), `inventory: Vec<ItemKind>`, and transient `visible`/`lit` bitmaps (`#[serde(skip)]`, rehydrated by `Save::from_bytes`). Constructed via `GameState::new_seeded(seed)`.
- `MoveOutcome::{Blocked, Moved}` — return type of movement attempts.
- `DoorEvent { from, to, new_room }` — door transition record.
- `PlaceOutcome` — re-exported from `items::PlaceOutcome`. Returned by `place_item` so the binary can surface a status message; `GameState` does not match on `ItemKind`.

Key methods:

- `current_room() -> &Room`
- `tile_at(x, y) -> Tile` / `terrain_at(x, y) -> Tile` — for renderers (with/without the player overlay).
- `is_visible(x, y) -> bool` — currently lit by player LOS or any persistent light in the current room.
- `is_explored(x, y) -> bool` — seen at least once in the current room.
- `player_on_door() -> Option<DoorId>` — for "Press Enter to open door" prompts.
- `move_player(direction) -> MoveOutcome` / `quick_move(direction) -> MoveOutcome` — quick_move stops one tile before doors.
- `interact() -> Option<DoorEvent>` — the only way to traverse a door. Calls `room.lighting.burn_out_flares()` on the leaving room, then refreshes visibility for the destination.
- `items_here() -> bool`, `peek_item_here() -> Option<ItemKind>`, `pick_up_here() -> Option<ItemKind>` — inventory pickup pipeline (delegates to `room.items: ItemSubsystem`).
- `place_item(slot) -> Option<PlaceOutcome>` — dispatches to `items::behavior_for(kind).on_place(...)`. **No `match ItemKind` lives in `game.rs`.**
- `refresh_visibility()` — wraps `visibility::compute_room_lighting`; public so `Save::from_bytes` can rehydrate.

Invariants: walls and out-of-bounds block. Player may stand on a door tile but only `interact()` changes rooms. The player's tile is always visible (and therefore always explored).

### [ecs/mod.rs](../adventerm_lib/src/ecs/mod.rs) — entity substrate

- `EntityId(u32)` — opaque entity handle.
- `World { positions, ... }` — owns `EntityId` allocation, the live-entity set, and the universal `Position` component. `spawn`, `despawn`, `is_alive`, `position_of`, `set_position`. **Per-category state lives in subsystems, not here.**
- `Position((usize, usize))` — universal positional component.
- `ComponentStore<T>` — generic sparse storage (`HashMap<EntityId, T>`). The reusable building block subsystems compose from.

### [lighting/mod.rs](../adventerm_lib/src/lighting/mod.rs) — lighting subsystem

- `LightSource { radius: u8 }`, `FlareSource` (marker).
- `Lighting { sources: ComponentStore<LightSource>, flares: ComponentStore<FlareSource> }`.
- Methods: `add_torch`, `add_flare` (idempotent on position), `burn_out_flares` (flares → sources at the same entity), `any_flare_active`, `iter_sources` (yields `(pos, &LightSource)` joined with `World::positions`).

### [items/](../adventerm_lib/src/items/) — items as entities + behaviors

- `kind.rs`: `ItemKind::{Torch, Flare}` with `name()` / `glyph()`.
- `storage.rs`: `ItemSubsystem` — per-room ground-item storage (`ComponentStore<ItemKind>`). `spawn_at`, `take_at`, `iter_at`, `any_at`, `positions`.
- `behavior.rs`: `ItemBehavior` trait + `PlaceCtx { player_pos, world, lighting }` + `behavior_for(kind: ItemKind) -> &'static dyn ItemBehavior`. The single point in the codebase that enumerates kinds.
- `torch.rs`, `flare.rs`: per-kind `ItemBehavior` impls (zero-sized types).

To add an item kind: add a variant in `kind.rs`, add a new `<kind>.rs` with an impl, add one arm to `behavior_for`. Compiler-enforced exhaustiveness.

### [visibility.rs](../adventerm_lib/src/visibility.rs) — lighting computation

- `compute_room_lighting(room, player, &mut visible, &mut lit)` — runs player LOS into `visible`, then ORs each `LightSource` disc (or "all tiles" if any flare is active) into `lit`. Called by `GameState::refresh_visibility`.

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

- `Room { id, width, height, tiles, world: World, lighting: Lighting, items: ItemSubsystem }` — tiles are flat row-major `Vec<TileKind>`. `world` and the subsystems hold every entity that lives in this room.
- `RoomId(u32)`, `DoorId(u32)` — newtypes.
- `TileKind::{Wall, Floor, Door(DoorId)}` — `Door` carries the door reference inline.

Methods: `new_filled`, `kind_at`, `set`, `is_walkable`, `in_bounds`, `doors()`, `find_door(DoorId)`, `first_floor()`. Read-only facades: `items_at(pos)` (iter `ItemKind`), `has_item_at(pos)`, `has_light_at(pos)`. **Write paths go through the subsystems** (`room.items.spawn_at(...)`, `room.lighting.add_torch(...)`, etc.) rather than `Room` methods.

### [world.rs](../adventerm_lib/src/world.rs) — frontend-facing primitives

- `Direction::{Up, Down, Left, Right}` (serde) — sole movement input type.
- `Tile::{Wall, Floor, Door, Player}` — what a renderer sees per cell. Distinct from `TileKind`: this enum is collapsed for display and includes the synthetic `Player` variant.

These two enums are the only types exposed for input/rendering — keep them frontend-agnostic (no key codes, no colors).

### [save.rs](../adventerm_lib/src/save.rs) — persistence

- `Save { version, name, state }` — JSON envelope around `GameState`.
- `SaveSlot { path, name, modified }` — directory listing entry.
- `SaveError::{Format(serde_json::Error), UnsupportedVersion { found, expected }}`.
- `SAVE_VERSION = 5` — bump when changing the wire format. v4 and earlier are rejected with `SaveError::UnsupportedVersion`.

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
