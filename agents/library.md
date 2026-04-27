# adventerm_lib

Pure gameplay logic. No `ratatui`/`crossterm` imports. Serde for serialization, otherwise stdlib only.

## Public surface

Re-exported from [lib.rs](../adventerm_lib/src/lib.rs):

- World types: `EntityId`, `World`, `Room`, `RoomId`, `DoorId`, `TileKind`, `Tile`, `Direction`, `Stats`, `Attribute`, `AbilityKind`, `PassiveKind`, `EnemyKind`, `ItemKind`, `ItemCategory`, `EquipSlot`, `EquipEffect`, `ConsumeIntent`, `ConsumeTarget`, `ConsumeOutcome`.
- Top-level: `GameState`, `Dungeon`, `DoorView`, `DoorSubsystem`, `DungeonClock`, `Equipment`.
- Movement / interaction: `MoveOutcome`, `PlaceOutcome`, `DoorEvent`.
- Actions: `MoveAction`, `QuickMoveAction`, `InteractAction`, `PickUpAction`, `PlaceItemAction`, `EquipItemAction`, `UnequipItemAction`, `ConsumeItemAction`, `DefeatEnemyAction`.
- Battle: `Battle`, `BattleSubsystem`, `BattleLog`, `BattleTurn`, `BattleResult`, `Combatants`, `HpSnapshot`.
- Events: `DungeonEvent`, `TickLog`, `ItemEquipped`, `ItemUnequipped`, `ItemConsumed`.
- Save: `Save`, `SaveError`, `SaveSlot`, `SAVE_VERSION`.
- Constants: `LIGHT_RANGE`, `LOS_RANGE`.
- Item facades: `category_of(kind)`, `consume_intent_of(kind)` — used by the binary to dispatch Confirm by category without matching on `ItemKind`.

Per-room subsystem types (`Lighting`, `ItemSubsystem`, `Enemies`, `Abilities`) are deliberately **not** re-exported — the binary reads them through `Room`/`GameState` facades.

## Architecture (high level)

`GameState` is a four-field aggregate: `dungeon`, `current_room`, `player: PlayerSubsystem`, `explored: ExploredSubsystem`. Every piece of mutable game state lives in some `World`:

| Scope | World | Subsystems |
| --- | --- | --- |
| Per-room | `Room::world` | `Lighting`, `ItemSubsystem`, `Enemies` |
| Top-level (player) | `PlayerSubsystem::world` | inventory, stats, HP, abilities, equipment, visibility cache, enemy RNG |
| Per-room memory | `ExploredSubsystem::world` | `ExploredMap` per room |
| Dungeon | `Dungeon::world` | `DoorSubsystem`, `DungeonClock` (turn counter + `TickLog`) |
| Per-battle | `BattleSubsystem::world` | `BattleTurn`, `Combatants`, `HpSnapshot`, `BattleLog` |

`GameState` itself holds **no game-logic methods** beyond a small read-only facade. All mutators are free functions in [systems/](../adventerm_lib/src/systems/).

## Modules

### [game.rs](../adventerm_lib/src/game.rs) — gameplay container

- `GameState { dungeon, current_room, player, explored }`. Constructed via `GameState::new_seeded(seed)`.
- `MoveOutcome::{Blocked, Moved, Encounter(EntityId)}`.
- `DoorEvent { from, to, new_room }` — door transition record returned by `interact`.
- `PlaceOutcome` — re-exported from `items::PlaceOutcome`.
- Constant `ENEMY_RNG_SALT` — XOR'd into the dungeon seed by `systems::enemy_tick` so AI RNG is reproducible from the dungeon seed.

Read-only facade methods (the only methods on `GameState`):

- `current_room() -> &Room`
- `player_pos() -> (usize, usize)` (delegates to `PlayerSubsystem`)
- `stats() -> &Stats`, `effective_stats() -> Stats` (base + equipment), `cur_health() -> u8`, `set_cur_health(u8)`, `inventory() -> &[ItemKind]`, `abilities() -> &Abilities`, `equipment() -> &Equipment`, `vision_radius() -> usize`
- `tile_at(x, y)`, `terrain_at(x, y)` — for renderers (with/without the player overlay)
- `is_visible(x, y)`, `is_explored(x, y)`
- `player_on_door() -> Option<DoorId>` — for "Press Enter to open door" prompts
- `items_here() -> bool`, `peek_item_here() -> Option<ItemKind>`

Thin shim methods that forward to `systems::*` (the binary still calls these for source compatibility):

- `move_player(d)` → `systems::step_player`
- `quick_move(d)` → `systems::quick_move`
- `interact()` → `systems::interact_door`
- `pick_up_here()` → `systems::pick_up_here`
- `place_item(slot)` → `systems::place_item`
- `defeat_enemy(room, entity)` → `systems::defeat_enemy`
- `refresh_visibility()` → `systems::refresh_visibility`

Invariants: walls and out-of-bounds block. The player may stand on a door tile but only `interact()` changes rooms. The player's tile is always visible (and therefore always explored).

### [systems/](../adventerm_lib/src/systems/) — gameplay mutators

Each module exposes free functions taking `&mut GameState` (or a narrower borrow). These are the canonical mutation surface; `impl GameState` shims forward into them.

- `movement.rs` — `step_player(game, dir) -> MoveOutcome`, `quick_move(game, dir) -> MoveOutcome`. Each emits a `PlayerMoved` event into the dungeon `TickLog` per actual position change.
- `interact.rs` — `interact_door(game) -> Option<DoorEvent>`. Burns out flares in the leaving room (one `FlareBurnedOut` event per converted flare) and emits `DoorTraversed` for the trip.
- `pickup.rs` — `peek_item_here`, `pick_up_here`, `place_item`. Emit `ItemPickedUp` / `ItemPlaced`.
- `combat.rs` — `defeat_enemy(game, room, entity)`. Despawns the enemy and emits `EnemyDefeated`.
- `visibility.rs` — `refresh_visibility(game)`. Wraps `visibility::compute_room_lighting` and ORs the result into explored memory.
- `enemy_tick.rs` — `tick_current_room(game) -> Option<EntityId>`. Lazily rehydrates the enemy RNG (seed XOR `ENEMY_RNG_SALT`), runs `enemies::tick_enemies`, and threads the dungeon `TickLog` so per-enemy `EnemyMoved`/`EnemyEngaged` events are recorded.

### [events.rs](../adventerm_lib/src/events.rs) — dungeon event log

- `DungeonEvent` — sum type of every per-step gameplay event: `PlayerMoved`, `EnemyMoved`, `EnemyEngaged`, `EnemyDefeated`, `DoorTraversed`, `FlareBurnedOut`, `ItemPickedUp`, `ItemPlaced`, `ItemEquipped`, `ItemUnequipped`, `ItemConsumed`.
- `TickLog` — bounded `VecDeque<DungeonEvent>`. `TICK_LOG_CAPACITY = 128`; oldest events drop on overflow. Lives as a component on the dungeon's clock entity, marked `#[serde(skip)]` (transient — fresh loads start with an empty log).

Cross-cutting consumers (renderers, tests, achievements, replays) read the log instead of each subsystem hooking each emitter.

### [ecs/mod.rs](../adventerm_lib/src/ecs/mod.rs) — entity substrate

- `EntityId(u32)` — opaque entity handle. `EntityId::from_raw(u32)` / `.raw()` for tests, serde, and sentinels.
- `World { positions, ... }` — owns `EntityId` allocation, the live-entity set, and the universal `Position` component. `spawn`, `despawn`, `is_alive`, `position_of`, `set_position`. **Per-category state lives in subsystems, not here.**
- `Position((usize, usize))`.
- `ComponentStore<T>` — generic sparse storage (`HashMap<EntityId, T>`).

### [player/](../adventerm_lib/src/player/) — player as an ECS entity

- `PlayerSubsystem { world, entity, inventory, stats, cur_health, abilities, visibility, enemy_rng }`. The player is a singleton entity in `world`; its components are stored in the `ComponentStore<…>` fields. Visibility cache and enemy RNG are `#[serde(skip)]` and lazily rehydrated by `refresh_visibility` / `enemy_rng_mut(seed, salt)`.

### [explored/](../adventerm_lib/src/explored/) — per-room memory

- `ExploredSubsystem { world, maps, by_room }`. One sentinel entity per `RoomId` carrying an `ExploredMap(Vec<bool>)` component. `mark`, `is_explored`, `merge_room`, `contains_room`. PartialEq compares by `(RoomId → ExploredMap)` so equality survives entity-id drift across deserialization.

### [lighting/mod.rs](../adventerm_lib/src/lighting/mod.rs) — lighting subsystem

- `LightSource { radius: u8 }`, `FlareSource` (marker).
- `Lighting { sources, flares }`.
- Methods: `add_torch`, `add_flare` (idempotent on position), `burn_out_flares`, `any_flare_active`, `iter_sources`, `iter_flares`.

### [items/](../adventerm_lib/src/items/) — items as entities + behaviors

- `kind.rs`: `ItemKind::{Torch, Flare, Goggles, Shirt, Gauntlets, Trousers, Boots, ScrollOfFire}` with `name()` / `glyph()`.
- `category.rs`: `ItemCategory::{Placeable, Equipment(EquipSlot), Consumable}` and `EquipSlot::{Head, Torso, Arms, Legs, Feet}`. Drives action-layer dispatch.
- `storage.rs`: `ItemSubsystem` — per-room ground-item storage (`ComponentStore<ItemKind>`). `spawn_at`, `take_at`, `iter_at`, `iter_at_any`, `any_at`, `positions`.
- `behavior.rs`: `ItemBehavior` trait + `PlaceCtx`, `ConsumeCtx`, `EquipEffect`, `ConsumeIntent`, `ConsumeTarget`, `ConsumeOutcome` + `behavior_for(kind) -> &'static dyn ItemBehavior` and `equip_slot_of(kind) -> Option<EquipSlot>`. The single point that enumerates kinds. Trait methods (all defaulted except `category`): `category()`, `on_place()`, `equip_effect()`, `consume_intent()`, `on_consume()`. `PlaceOutcome` and `ConsumeOutcome` are `Serialize/Deserialize` so they can ride inside `events::ItemPlaced` / `events::ItemConsumed`.
- `torch.rs`, `flare.rs`, `goggles.rs`, `shirt.rs`, `gauntlets.rs`, `trousers.rs`, `boots.rs`, `scroll_of_fire.rs`: per-kind `ItemBehavior` impls (zero-sized types). Equipment modules override `equip_effect`; the scroll overrides `consume_intent` (returns `PickAbilitySlot`) and `on_consume`.

### [equipment/](../adventerm_lib/src/equipment/) — worn items

- `Equipment { head, torso, arms, legs, feet: Option<ItemKind> }` — per-player worn-item state, stored as a `ComponentStore<Equipment>` on the player entity.
- Methods: `slot(s)`, `equip(s, kind) -> Option<prev>`, `unequip(s) -> Option<kind>`, `iter()`, `aggregate_effect() -> EquipEffect` (sums attack/defense/speed across slots; multiplies vision).

### [visibility.rs](../adventerm_lib/src/visibility.rs) — lighting computation

- `compute_room_lighting(room, player, &mut visible, &mut lit)` — convenience wrapper that uses the default `LOS_RANGE`.
- `compute_room_lighting_with_radius(room, player, radius, &mut visible, &mut lit)` — runs player LOS into `visible` at the supplied `radius` (the player's effective vision after equipment), then ORs each `LightSource` disc (or "all tiles" if any flare is active) into `lit`. Light sources still use their own constants — equipment only affects the player's LOS. Called by `systems::refresh_visibility`.

### [dungeon.rs](../adventerm_lib/src/dungeon.rs) + [dungeon/](../adventerm_lib/src/dungeon/) — generation + dungeon-scoped ECS

- `Dungeon { seed, rooms, world, doors, clock }`. Rooms are still a flat `Vec<Room>`; doors and the clock live as entities in `world`.
- `Dungeon::generate(seed)` — deterministic; pipeline: per-room `generate_room` → `build_edges` → install clock → spawn paired door entities → `place_door` per edge → `carve_to_nearest_floor` to guarantee connectivity → enemy placement.
- `Dungeon::generate_with_room_count(seed, count)` — same pipeline but skips the random room-count draw. Used by the binary's `--dump-rooms` flag.
- `Dungeon::room`, `room_mut`, `door(id) -> DoorView`, `door_view(id) -> Option<DoorView>`, `door_ids() -> impl Iterator<DoorId>`.
- `step_inward(door_pos, room)` — used by `systems::interact_door` for the spawn tile after traversal.

#### [dungeon/doors.rs](../adventerm_lib/src/dungeon/doors.rs)

- `DoorOwner(RoomId)`, `DoorLink(EntityId)`, `DoorState { open, locked }`. Default state is open & unlocked; reserved for growth.
- `DoorSubsystem { owners, links, states }` — `spawn`, `despawn`, `link_pair`, `view`, `owner_of`, `leads_to`, `state_of`, `ids`.
- `DoorView { id, owner, pos, leads_to, state }` — read-only snapshot returned to callers.

`DoorId` (`pub struct DoorId(pub EntityId)` in [room.rs](../adventerm_lib/src/room.rs)) is embedded inline in `TileKind::Door` so renderer/tile lookups stay O(1).

#### [dungeon/clock.rs](../adventerm_lib/src/dungeon/clock.rs)

- `DungeonClock { entity, turn, log }` — singleton clock entity. `install`, `entity`, `turn`, `advance`, `log`, `log_mut`, `emit`. `TurnCounter(u64)`.
- `log` is `#[serde(skip)]`; `PartialEq` on the clock ignores the log so a freshly loaded clock compares equal to a pre-save one that emitted events.

Generation constants (load-bearing — see CLAUDE.md style rule #2):

| Constant | Value |
| --- | --- |
| Rooms per dungeon | 6–10 |
| Room width | 42–65 |
| Room height | 20–31 |
| Subrooms per room | 3–5 |
| Subroom min size | 8 × 6 |
| Extra edges beyond chain | 1–2 (only when ≥4 rooms) |

Invariants: bidirectional door pairs (the link component always reciprocates), every door has a floor neighbor, all rooms reachable from room 0 via the door graph.

### [room.rs](../adventerm_lib/src/room.rs) — single-room grid

- `Room { id, width, height, tiles, world: World, lighting, items, enemies }` — tiles are flat row-major `Vec<TileKind>`. `world` and the subsystems hold every entity that lives in this room.
- `RoomId(u32)`, `DoorId(EntityId)` — newtypes.
- `TileKind::{Wall, Floor, Door(DoorId)}`.

Methods: `new_filled`, `kind_at`, `set`, `is_walkable`, `in_bounds`, `doors()`, `find_door(DoorId)`, `first_floor()`. Read-only facades: `items_at`, `has_item_at`, `has_light_at`, `enemy_glyph_at`, `has_enemy_at`, `enemies_iter`. **Write paths go through the subsystems** (`room.items.spawn_at(...)`, `room.lighting.add_torch(...)`, `room.enemies.spawn_at(...)`).

### [world.rs](../adventerm_lib/src/world.rs) — frontend-facing primitives

- `Direction::{Up, Down, Left, Right}` (serde) — sole movement input type.
- `Tile::{Wall, Floor, Door, Player}` — what a renderer sees per cell. Distinct from `TileKind`: collapsed for display and includes the synthetic `Player` variant.

### [save.rs](../adventerm_lib/src/save.rs) — persistence

- `Save { version, name, state }` — JSON envelope around `GameState`.
- `SaveSlot { path, name, modified }`.
- `SaveError::{Format, UnsupportedVersion}`.
- `SAVE_VERSION = 9` — bumped for the equipment ComponentStore on `PlayerSubsystem`. Older versions rejected.

Functions: `Save::new`, `Save::to_bytes`, `Save::from_bytes` (validates version, then calls `state.refresh_visibility()` to rehydrate the transient FOV bitmap), `slugify(name)`, `slot_path(dir, name)`, `list_saves(dir)` (sorted newest first; tolerates missing dir / corrupt files / version mismatches), `delete_save(path)`.

### [los.rs](../adventerm_lib/src/los.rs) — line-of-sight

- `LOS_RANGE: usize = 12`, `LIGHT_RANGE: usize = LOS_RANGE`, `CELL_ASPECT_Y_OVER_X: f32 = 2.0`. `compute_visible(room, origin, out)` fills row-major bools per tile at the default `LOS_RANGE`. `compute_visible_with_radius(room, origin, radius, out)` exposes the runtime radius so equipment-driven multipliers (goggles) slot in cleanly.

### [stats/mod.rs](../adventerm_lib/src/stats/mod.rs) — stat block

- `Stats { health, attack, defense, speed, attribute }`, clamped to `[0, 100]`. Default is the player's starter profile (HP 25 / ATK 10 / DEF 5 / SPD 8 / Fire).
- `Attribute::{Fire, Water, Earth, Light, Dark}` — currently display-only.

### [abilities/](../adventerm_lib/src/abilities/) — abilities as ZST behaviors

- `mod.rs`: `Abilities { active_slots, passive_slots, learned_active, learned_passive }`. `ABILITY_SLOTS = 4`, `PASSIVE_SLOTS = 4`. Lives as a component on the player entity.
- `active.rs`: `ActiveAbility` trait, `AbilityCtx { attacker, defender }`, `AbilityOutcome { damage }`, `ability_behavior_for(kind)`.
- `passive.rs`: `PassiveAbility` trait + `passive_behavior_for`. `PassiveKind` is currently `pub enum {}`.
- `impact.rs`: `ImpactAbility` ZST. Damage = `max(1, attacker.attack - defender.defense)`.
- `fireball.rs`: `FireballAbility` ZST. Base damage 8, mitigated by `defender.defense / 2`, floored at 1. Fire affinity is implicit in the ability identity for now.
- `AbilityKind::{Impact, Fireball}`.

### [enemies/](../adventerm_lib/src/enemies/) — enemies + AI

- `mod.rs`: `Enemies` subsystem on `Room`. Methods: `spawn_at`, `despawn`, `kind_of`, `stats_of`, `hp_of`, `set_hp`, `entity_at`, `iter_with_pos`.
- `kind.rs`: `EnemyKind::{Slime}` with `name()`, `glyph()`, `base_stats()`.
- `ai.rs`: `EnemyAi` trait, `AiCtx { enemy_pos, player_pos, room, rng }`, `AiAction::{Wait, Step(Direction)}`, `enemy_behavior_for(kind)`.
- `slime.rs`: `SlimeAi` ZST.
- `movement.rs`: `tick_enemies(room, room_id, player_pos, rng, log: Option<&mut TickLog>) -> EnemyTickOutcome::{Quiet, EncounterTriggered(EntityId)}`. Iteration is sorted by `EntityId` for determinism. `log` receives `EnemyMoved` per actual position change and `EnemyEngaged` for the first adjacency.

### [battle/](../adventerm_lib/src/battle/) — turn-based combat as ECS

- `mod.rs`:
  - `BattleSubsystem { world, turn, combatants, hp, log }` — owns the battle world and component stores.
  - `Battle { sub, entity }` — thin handle over the singleton battle entity. Accessors: `turn()`, `set_turn`, `combatants`, `enemy_id`, `enemy_room`, `hp`, `player_cur_hp`, `enemy_cur_hp`, `log() -> &[String]`, `is_resolved`, `result`. Internal mutators: `set_player_hp`, `set_enemy_hp`, `push_log`.
  - `BattleTurn::{Player, Enemy, Resolved(BattleResult)}`, `BattleResult::{Victory, Defeat, Fled}`, `BATTLE_LOG_LINES = 8`.
  - `Combatants { enemy_entity, enemy_room }`, `HpSnapshot { player, enemy }`, `BattleLog { lines }`.
- `engine.rs`: `start_battle(game, enemy_id) -> Option<Battle>`, `apply_player_ability(game, battle, slot)`, `apply_enemy_turn(game, battle)`. Player turns dispatch through `ability_behavior_for`; enemy turns currently use a stat-based basic attack with a 1-damage floor. The `Battle` is owned by `Screen::Battle` in the binary and dropped on resolve — battles are not serialized (matches the existing "no save mid-battle" rule).

### [rng.rs](../adventerm_lib/src/rng.rs) — seeded PRNG

- `Rng { state: u64 }` — xorshift (13 / 7 / 17). Seed 0 substitutes a constant.
- Methods: `new`, `next_u64`, `next_u32`, `range(low, high_exclusive)`, `chance(num, den)`.

Used by [dungeon.rs](../adventerm_lib/src/dungeon.rs) for generation and by `systems::enemy_tick` (re-seeded from `seed XOR ENEMY_RNG_SALT`) for enemy AI.

## Tests

Each module has unit tests in-file. Coverage hot spots:

- [game.rs](../adventerm_lib/src/game.rs) — movement blocking, door interact, quick-move stop-before-door, room transitions, save round-trip.
- [systems/tests.rs](../adventerm_lib/src/systems/tests.rs) — every mutator emits the expected `DungeonEvent`.
- [events.rs](../adventerm_lib/src/events.rs) — `TickLog` ordering and bounded-queue truncation.
- [dungeon.rs](../adventerm_lib/src/dungeon.rs) — determinism, door bidirectionality, reachability, floor connectivity.
- [dungeon/clock.rs](../adventerm_lib/src/dungeon/clock.rs) — install idempotence, turn advance, emit.
- [save.rs](../adventerm_lib/src/save.rs) — round-trip, version rejection, slugify, listing.
- [rng.rs](../adventerm_lib/src/rng.rs) — determinism, seed divergence, range bounds.

Run scope-targeted: `cargo test -p adventerm_lib`.
