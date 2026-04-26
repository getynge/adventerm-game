# adventerm_lib

Gameplay logic and state. No TUI dependency. Must not import `ratatui`/`crossterm` or expose frontend-shaped types.

## Gameplay constructs (ECS + behaviors)

Constructs (items, lights, flares, future monsters/traps) are **entities** with **components**, organized into **subsystems**, driven by **behavior traits**. `GameState` dispatches; it does not enumerate.

**Layers:**
1. `ecs::World` — substrate. Owns `EntityId` allocation, lifetime, and the universal `Position` component. **Do not add per-category fields to `World`.** Write a subsystem instead.
2. **Subsystems** (e.g. `lighting::Lighting`, `items::ItemSubsystem`) own category-specific state via `ecs::ComponentStore<T>` and expose a focused write API. They live as fields on `Room` (room-scoped) or `GameState` (global).
3. **Behavior traits** (e.g. `items::ItemBehavior`) define what a player action does. The trait method takes a `*Ctx` struct that borrows only the subsystems it needs. Per-kind impls are ZSTs in their own module, looked up by a single `behavior_for(kind) -> &'static dyn Trait` match.

**Add a new item kind:**
1. Add the variant to `ItemKind` in [src/items/kind.rs](src/items/kind.rs) (update `name()` / `glyph()`).
2. Add a file under [src/items/](src/items/) with a ZST that `impl ItemBehavior`.
3. Add an arm to `behavior_for` in [src/items/behavior.rs](src/items/behavior.rs).
4. If a new side-effect is needed, add a method to `ItemBehavior` (with a default impl) or a new field to `PlaceCtx`.

**Add a new construct category** (e.g. monsters):
1. Create `src/<category>/mod.rs` with a subsystem struct holding `ComponentStore<...>` fields and a narrow write API.
2. Add it as a field on `Room` (room-scoped) or `GameState` (global).
3. If the binary needs to read it, add facade methods on `Room`/`GameState` rather than exposing the subsystem.
4. For interactions, define a behavior trait and dispatch via `behavior_for`.

**`GameState` rules:**
- Holds `World`/inventory and routes actions to the right subsystem or behavior.
- **Never** matches on `ItemKind` or any other construct discriminant. If you write `match item.kind` in `game.rs`, the logic belongs in a behavior impl.

**Binary-visible API:** the read-only facades on `Room`/`GameState` (`items_at`, `has_item_at`, `peek_item_here`, `has_light_at`, ...). The binary must not import subsystem types or `World`.
