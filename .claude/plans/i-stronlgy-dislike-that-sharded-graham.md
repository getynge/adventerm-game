---
title: Decouple item & gameplay-construct behavior from `GameState` via a tiny ECS
status: draft
---

# Context

Today, `GameState::place_item` ([adventerm_lib/src/game.rs:168-189](adventerm_lib/src/game.rs#L168-L189)) matches on `ItemKind` and reaches into `Room` to call kind-specific mutators (`add_light`, `add_flare`). Lights/flares are stored as bare `(usize,usize)` tuples on `Room` and their lifecycle (flare → torch on room exit, full-room illumination while present) is hard-coded into `GameState::interact` and `GameState::refresh_visibility`. This violates the project's decoupling principle: gameplay constructs cannot exist or evolve without `game.rs` learning their internals.

The refactor introduces a small in-house ECS plus a behavior trait that gameplay constructs implement. `GameState` becomes ignorant of *what* a Torch, Flare, or future construct does — it just dispatches actions through a kind-keyed registry that resolves to a per-kind behavior impl. The ECS gives us a uniform data layer (entities + components, scoped per room) that future constructs (traps, monsters, breakable doors) can plug into without touching `game.rs`.

User-confirmed decisions:
- Hand-roll a minimal ECS in `adventerm_lib/src/ecs/`. No third-party crate.
- Phase 1 includes items **and** lights/flares — the visibility code is rewritten as a system in this phase.
- Bump `SAVE_VERSION` to 5 and reject v4 saves with a clear error. No migration.
- Each `Room` owns its own `World`. Inventory stays on `GameState` (entities aren't needed for inventory in phase 1; behaviors take `ItemKind` + `Item` data, not entities).

# Approach

## New module layout

```
adventerm_lib/src/
  ecs/
    mod.rs            // EntityId, World (substrate), Position, ComponentStore<T>
  lighting/
    mod.rs            // Lighting subsystem: LightSource, FlareSource, methods
  items/
    mod.rs            // re-exports Item, ItemId, ItemKind; pub use behavior::*
    kind.rs           // ItemKind, Item, ItemId (moved from items.rs)
    storage.rs        // ItemSubsystem (per-room ground-item store)
    behavior.rs       // ItemBehavior trait, PlaceCtx, PlaceOutcome, behavior_for(kind)
    torch.rs          // TorchBehavior (ZST) impls ItemBehavior
    flare.rs          // FlareBehavior (ZST) impls ItemBehavior
  visibility.rs       // compute_room_lighting(room, player, &mut visible, &mut lit)
```

`lib.rs` re-exports stay stable for the binary: `Item`, `ItemId`, `ItemKind`, `PlaceOutcome` keep their paths. `World` and `EntityId` are exposed only if the binary genuinely needs them (it shouldn't).

## ECS core — substrate only, no per-category storage

`World` knows about entities and a small set of *substrate* components (things that almost any construct needs, like `Position`). Category-specific state — light sources, flares, items, future traps/monsters — lives in dedicated subsystems alongside `World`, not as fields on `World`. The user's principle: `World` should not enumerate every category of thing in the game.

```rust
// ecs/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(u32);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct World {
    next: u32,
    alive: HashSet<EntityId>,
    pub positions: ComponentStore<Position>,
}

impl World {
    pub fn spawn(&mut self) -> EntityId { /* bump next, insert into alive */ }
    pub fn despawn(&mut self, e: EntityId) { /* remove from alive + positions */ }
    pub fn is_alive(&self, e: EntityId) -> bool;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentStore<T>(HashMap<EntityId, T>); // generic, reusable by subsystems

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position(pub (usize, usize));
```

`ComponentStore<T>` is the reusable building block: subsystems compose their own state out of it (or out of plain collections) without `World` having to know they exist.

## Subsystems own per-category state

Each construct category gets its own module that owns a focused store. Subsystems are added to `Room` as named fields, not threaded through `World`. This keeps `World` stable as the game grows: adding monsters means a new `monsters` subsystem on `Room`, not a new field on `World`.

```rust
// adventerm_lib/src/lighting/mod.rs
pub struct Lighting {
    sources: ComponentStore<LightSource>,
    flares:  ComponentStore<FlareSource>,
}
pub struct LightSource { pub radius: u8 }
pub struct FlareSource;

impl Lighting {
    pub fn add_torch(&mut self, world: &mut World, pos: (usize, usize)) -> EntityId;
    pub fn add_flare(&mut self, world: &mut World, pos: (usize, usize)) -> EntityId;
    pub fn burn_out_flares(&mut self);              // flares → sources at same entity
    pub fn any_flare_active(&self) -> bool;
    pub fn iter_sources<'a>(&'a self, world: &'a World)
        -> impl Iterator<Item = ((usize,usize), &'a LightSource)> + 'a;
}

// adventerm_lib/src/items/storage.rs
pub struct ItemSubsystem {
    kinds: ComponentStore<ItemKind>,   // entity → kind for items currently on the ground
}

impl ItemSubsystem {
    pub fn spawn_at(&mut self, world: &mut World, pos: (usize,usize), kind: ItemKind) -> EntityId;
    pub fn take_at(&mut self, world: &mut World, pos: (usize,usize)) -> Option<ItemKind>;
    pub fn iter_at<'a>(&'a self, world: &'a World, pos: (usize,usize))
        -> impl Iterator<Item = ItemKind> + 'a;
    pub fn any_at(&self, world: &World, pos: (usize,usize)) -> bool;
}
```

`Room` composes them:

```rust
pub struct Room {
    pub id: RoomId,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TileKind>,
    pub world:    World,
    pub lighting: Lighting,
    pub items:    ItemSubsystem,
}
```

Visibility's flare check, today written `room.flares.is_empty()`, becomes `room.lighting.any_flare_active()`. Light iteration becomes `room.lighting.iter_sources(&room.world)`. Generation calls `room.items.spawn_at(&mut room.world, pos, kind)` instead of `room.add_item(pos, item)`. The renderer's `room.items_at(pos)` becomes a one-line facade over `room.items.iter_at(&room.world, pos)`.

This is the structural answer to "World should not enumerate flares": `World` exposes only generic facilities (`spawn`, `despawn`, `Position`); each gameplay category brings its own subsystem with its own components and methods.

## Behavior trait + dispatch (uses `dyn`)

Behavior is a trait object. `dyn` is the right tool here because the set of behaviors grows over time and we want a uniform call site that doesn't care which kind it's invoking. Each kind has a zero-sized struct impl, and a single `behavior_for` match returns `&'static dyn ItemBehavior` — no allocation, no `typetag`, no serde concerns (the trait object is derived from `ItemKind` at runtime, not stored).

```rust
// items/behavior.rs
pub enum PlaceOutcome { TorchPlaced, FlarePlaced }

pub struct PlaceCtx<'a> {
    pub player_pos: (usize, usize),
    pub world:    &'a mut World,
    pub lighting: &'a mut Lighting,
    // Future: monsters, traps, etc. added here as new subsystems land.
    // Each behavior touches only the subsystems relevant to it.
}

pub trait ItemBehavior {
    fn on_place(&self, ctx: &mut PlaceCtx<'_>) -> PlaceOutcome;
    // Future hooks (added when needed, with default impls so kinds opt in):
    // fn on_pick_up(&self, ctx: &mut PickUpCtx) {}
    // fn on_use(&self, ctx: &mut UseCtx) -> UseOutcome { UseOutcome::Noop }
}

pub fn behavior_for(kind: ItemKind) -> &'static dyn ItemBehavior {
    match kind {
        ItemKind::Torch => &torch::TorchBehavior,
        ItemKind::Flare => &flare::FlareBehavior,
    }
}
```

`&'static dyn ItemBehavior` is the simplest design that satisfies the constraint that `GameState` does not know what individual items do. The match in `behavior_for` is the *only* place that enumerates kinds; everything else dispatches dynamically. ZSTs mean the trait object is fat-pointer-only (no heap), so this is as cheap as a function pointer.

Per-kind impls (in `items/torch.rs`, `items/flare.rs`) call into the relevant subsystem:

```rust
impl ItemBehavior for TorchBehavior {
    fn on_place(&self, ctx: &mut PlaceCtx) -> PlaceOutcome {
        ctx.lighting.add_torch(ctx.world, ctx.player_pos);
        PlaceOutcome::TorchPlaced
    }
}
impl ItemBehavior for FlareBehavior {
    fn on_place(&self, ctx: &mut PlaceCtx) -> PlaceOutcome {
        ctx.lighting.add_flare(ctx.world, ctx.player_pos);
        PlaceOutcome::FlarePlaced
    }
}
```

The match in `behavior_for` is the **only** place in the codebase that knows the full set of `ItemKind`s. Adding a new item kind: add an enum variant, add a new module under `items/`, add one arm to `behavior_for`. The Rust compiler enforces exhaustiveness.

## `GameState::place_item` shrinks

```rust
pub fn place_item(&mut self, slot: usize) -> Option<PlaceOutcome> {
    let item = self.inventory.get(slot)?.clone();
    let room = self.dungeon.room_mut(self.current_room);
    let mut ctx = PlaceCtx {
        player_pos: self.player,
        world:    &mut room.world,
        lighting: &mut room.lighting,
    };
    let outcome = items::behavior::behavior_for(item.kind).on_place(&mut ctx);
    self.inventory.remove(slot);
    self.refresh_visibility();
    Some(outcome)
}
```

No `match ItemKind` in `game.rs` and no calls to `add_light`/`add_flare`.

## `Room` changes

`Room` **loses** the `lights`, `flares`, `items` `Vec` fields and their helper methods (`add_light`, `add_flare`, `burn_out_flares`, `add_item`, `take_item_at`, `items_at`, `has_light_at`, `has_item_at`). It gains `world: World`, `lighting: Lighting`, `items: ItemSubsystem`. The old method names get re-implemented as one-line facades that delegate to the appropriate subsystem so the binary needs **zero** changes:

```rust
impl Room {
    pub fn items_at(&self, pos: (usize, usize)) -> impl Iterator<Item = ItemKind> + '_ {
        self.items.iter_at(&self.world, pos)
    }
    pub fn has_item_at(&self, pos: (usize, usize)) -> bool {
        self.items.any_at(&self.world, pos)
    }
    pub fn has_light_at(&self, pos: (usize, usize)) -> bool {
        self.lighting.iter_sources(&self.world).any(|(p, _)| p == pos)
    }
}
```

Note that `add_light` / `add_flare` / `burn_out_flares` / `add_item` / `take_item_at` are intentionally **not** re-exposed as facades — they are write paths and should now go through behaviors or generation calling the subsystems directly. This is the decoupling: write paths funnel through subsystem APIs; read paths keep their old shape so render code is unaffected.

## Visibility becomes a system

Move the lighting logic out of `GameState::refresh_visibility` into `visibility.rs`:

```rust
pub fn compute_room_lighting(
    room: &Room,
    player: (usize, usize),
    visible: &mut Vec<bool>,
    lit: &mut Vec<bool>,
) {
    los::compute_visible(room, player, visible);
    lit.clear();
    lit.resize(room.width * room.height, false);

    if room.lighting.any_flare_active() {
        for v in lit.iter_mut() { *v = true; }
        return;
    }
    let mut tmp = Vec::new();
    for (pos, light) in room.lighting.iter_sources(&room.world) {
        los::compute_visible_with_radius(room, pos, light.radius as usize, &mut tmp);
        for (dst, src) in lit.iter_mut().zip(tmp.iter()) {
            if *src { *dst = true; }
        }
    }
}
```

`GameState::refresh_visibility` becomes a 4-line wrapper that calls this and ORs into the explored bitmap.

## Flare burnout lives on `Lighting`

Replace `room.burn_out_flares()` with `room.lighting.burn_out_flares()` — the subsystem swaps each `FlareSource` for a `LightSource` at the same entity (Position is preserved). `GameState::interact` calls this on the leaving room. Same end-state as today's behavior (flare → torch at the same tile).

## Dungeon generation

[adventerm_lib/src/dungeon.rs:338-358](adventerm_lib/src/dungeon.rs#L338-L358) `place_room_items` and `place_wall_lights` (line 293-306) currently call `room.add_item` / `room.add_light`. They become:

```rust
room.items.spawn_at(&mut room.world, pos, kind);     // ground items
room.lighting.add_torch(&mut room.world, pos);       // wall lights
```

Open mini-decision during implementation: whether `ItemId(u32)` is still useful given `EntityId` exists. Lean toward dropping it (one less concept) but keep `Item { id: ItemId, kind: ItemKind }` for the inventory `Vec<Item>` so display logic doesn't change.

## Save format

`SAVE_VERSION` → 5. `Room` serializes its `world`, `lighting`, and `items` subsystems. `GameState::inventory` is unchanged shape (`Vec<Item>`). The transient `visible`/`lit` caches stay `#[serde(skip)]` and are rebuilt by `Save::from_bytes` calling `state.refresh_visibility()` (already does this).

`Save::load` checks version; for `< 5` returns `Err(SaveError::UnsupportedVersion)` with a message. Update [adventerm_lib/src/save.rs](adventerm_lib/src/save.rs) version constant and the error variant.

# Critical files

- [adventerm_lib/src/game.rs](adventerm_lib/src/game.rs) — shrink `place_item`, `interact`, `refresh_visibility`
- [adventerm_lib/src/room.rs](adventerm_lib/src/room.rs) — replace `lights`/`flares`/`items` fields with `world: World`; keep `items_at` as a thin facade
- [adventerm_lib/src/items.rs](adventerm_lib/src/items.rs) — split into `items/` module
- [adventerm_lib/src/dungeon.rs](adventerm_lib/src/dungeon.rs) — generation writes to `room.world`
- [adventerm_lib/src/save.rs](adventerm_lib/src/save.rs) — version bump + reject v4
- [adventerm_lib/src/lib.rs](adventerm_lib/src/lib.rs) — module list, re-exports
- New: `adventerm_lib/src/ecs/mod.rs`, `adventerm_lib/src/items/{mod,kind,behavior,torch,flare}.rs`, `adventerm_lib/src/visibility.rs`
- Update: [CLAUDE.md](CLAUDE.md) — add a section explaining the ECS + subsystem + behavior pattern (see "CLAUDE.md updates" below)
- Update: [agents/library.md](agents/library.md), [agents/architecture.md](agents/architecture.md), [agents/patterns.md](agents/patterns.md) — document the ECS + behavior pattern in detail; cross-link from CLAUDE.md

## CLAUDE.md updates

Add a new top-level section, "Gameplay constructs (ECS + behaviors)", after "Architectural rules". It must give a future session enough to extend the system correctly without reading every source file. Suggested content:

> ### Gameplay constructs (ECS + behaviors)
>
> Gameplay constructs (items, lights, flares, future monsters/traps) are modeled as **entities** with **components**, organized into **subsystems**, and driven by **behavior traits**. `GameState` is intentionally ignorant of any specific construct's mechanics.
>
> **Layers:**
> 1. `ecs::World` — the substrate. Owns `EntityId` allocation, lifetime, and the universal `Position` component. **Do not add per-category fields here.** If you find yourself wanting to put `monsters` on `World`, write a `Monsters` subsystem instead.
> 2. **Subsystems** (e.g. `Lighting`, `ItemSubsystem`, future `Monsters`) — own category-specific component stores (built from `ComponentStore<T>`) and expose a focused write API (`add_torch`, `spawn_at`, `burn_out_flares`, ...). Subsystems live as fields on `Room` (or on `GameState` for global state).
> 3. **Behavior traits** (e.g. `ItemBehavior`) — define what a player action does. The trait method receives a `*Ctx` struct that borrows the subsystems it legitimately needs. Per-kind impls live in their own module (one ZST per kind) and are looked up by a single `behavior_for(kind) -> &'static dyn Trait` match.
>
> **How to add a new item kind:**
> 1. Add the variant to `ItemKind` in [adventerm_lib/src/items/kind.rs](adventerm_lib/src/items/kind.rs).
> 2. Add a new module under `adventerm_lib/src/items/` with a ZST struct that `impl ItemBehavior`.
> 3. Add one arm to `behavior_for` in `items/behavior.rs`. The compiler will yell until you do.
> 4. If the new behavior needs a new kind of side-effect (e.g. damage), extend `ItemBehavior` (give the new method a default impl) or add a new ctx field referencing an existing/new subsystem.
>
> **How to add a new gameplay-construct category** (e.g. monsters):
> 1. Create `adventerm_lib/src/<category>/mod.rs` with a subsystem struct holding `ComponentStore<...>` fields. Expose an explicit, narrow write API.
> 2. Add the subsystem as a field on `Room` (or `GameState` for global state).
> 3. If the binary needs to read it, add thin facade methods on `Room` rather than exposing the subsystem fields directly to render code.
> 4. If players or other constructs interact with it, define a behavior trait in the same module and dispatch through a `behavior_for`-style match.
>
> **What `GameState` is allowed to do:**
> - Hold a `World`/inventory and route actions to the right subsystem or behavior.
> - **Not** match on `ItemKind` or any other construct's discriminant. If you find yourself writing `match item.kind` in `game.rs`, that logic belongs in a behavior impl.
>
> **What the binary is allowed to know:** existing read-only facades on `Room`/`GameState` (`items_at`, `has_item_at`, `peek_item_here`, ...). It must not import subsystem types or `World` for rendering — keep the lib/binary boundary clean.

# Reusable functions/utilities

- `los::compute_visible`, `los::compute_visible_with_radius` ([adventerm_lib/src/los.rs](adventerm_lib/src/los.rs)) — keep using as-is; the visibility system reads them.
- `Rng` ([adventerm_lib/src/rng.rs](adventerm_lib/src/rng.rs)) — generation continues to thread the seeded RNG through.
- `Save::from_bytes` post-load `refresh_visibility` hook ([adventerm_lib/src/save.rs](adventerm_lib/src/save.rs)) — keep; transient caches still need rehydration.

# Verification

1. `cargo build` — workspace compiles.
2. `cargo test` — all existing tests pass. The torch and flare tests in `game.rs` ([game.rs:523-579](adventerm_lib/src/game.rs#L523-L579)) are the load-bearing checks: torch placement still adds a persistent light, flare placement lights the full room, flare burnout on room transition still produces a regular torch at the same tile.
3. Add a small new test: spawn an item entity directly via `behavior_for(ItemKind::Torch).on_place(&mut ctx)` and assert a `LightSource` component appears at the expected `Position`. This proves the trait surface works without going through `GameState::place_item`.
4. Add a test that loading a v4 save bytes returns `SaveError::UnsupportedVersion`.
5. Manual: `cargo run -p adventerm`, walk into a room with a torch, pick it up, place it, leave the room, return — torch glow persists. Place a flare, leave the room, return — the flare position is now a regular torch. (CLAUDE.md requires manual UI verification for changes that affect rendering.)
6. Confirm the binary diff is empty or near-empty: only `app.rs` and `ui/gameplay.rs` callsites should still compile against the same `Room`/`GameState` method names.

# Out of scope (future phases)

- Doors / room transitions as entities (still procedural).
- Monsters, traps, breakable terrain — natural extensions once the trait surface and ECS are in place.
- Per-entity behavior data (e.g., flares with `fuel_turns` countdowns). Phase 1 keeps current semantics; richer behaviors land later.
- Inventory items as entities. Today's `Vec<Item>` is fine; if inventory ever needs richer state (stacks, equipped slots) we can promote it to an ECS storage on `GameState` later.
