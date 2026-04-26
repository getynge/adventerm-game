# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Adventerm is a TUI adventure game. The workspace has two crates:

- `adventerm_lib` — all gameplay logic and state. No TUI dependency.
- `adventerm` — the TUI binary. Renders current game state and translates terminal input into calls on the library. Uses `ratatui` (0.30) over `crossterm` (0.29).

## Architectural rules

These are load-bearing constraints for this project. Honor them when adding features.

1. **All gameplay logic lives in `adventerm_lib`.** `adventerm` is solely concerned with rendering current game state and forwarding user input. If a behavior describes "what the game does," it belongs in the library; if it describes "how the player sees or drives it," it belongs in the binary.
2. **The TUI must be replaceable.** Structure code so the entire `adventerm` crate could be swapped for a different frontend without touching gameplay logic. The library must not import or depend on `ratatui`/`crossterm`, and must not expose types that assume a specific frontend (e.g. no key codes, no terminal coordinates, no widget concepts in the public API).
3. **Core gameplay layout is fixed.** During core gameplay the screen has three regions: a large central window (the world), a short window beneath it for dialog/game information, and a narrow tall window to the right for player actions/options. This rule applies *only* to core gameplay — main menu, pause menu, inventory, and other interface screens are not bound by this layout.
4. **UI-only state stays in the binary.** State that exists purely to drive rendering (e.g. menu cursor positions, scroll offsets, animation timers) belongs to `adventerm`, not the library. The library should expose available options/data; the binary decides how the user navigates them.

## Gameplay constructs (ECS + behaviors)

Gameplay constructs (items, lights, flares, future monsters/traps) are modeled as **entities** with **components**, organized into **subsystems**, and driven by **behavior traits**. `GameState` is intentionally ignorant of any specific construct's mechanics — its job is dispatch, not enumeration.

**Layers:**
1. `ecs::World` — the substrate. Owns `EntityId` allocation, lifetime, and the universal `Position` component. **Do not add per-category fields to `World`.** If you find yourself wanting to put `monsters` on `World`, write a `Monsters` subsystem instead.
2. **Subsystems** (e.g. `lighting::Lighting`, `items::ItemSubsystem`, future `Monsters`) own category-specific state built from `ecs::ComponentStore<T>` and expose a focused write API (`add_torch`, `spawn_at`, `burn_out_flares`, ...). Subsystems live as fields on `Room` (or on `GameState` for global state).
3. **Behavior traits** (e.g. `items::ItemBehavior`) define what a player action does. The trait method receives a `*Ctx` struct that borrows only the subsystems it legitimately needs. Per-kind impls live in their own module as zero-sized types and are looked up by a single `behavior_for(kind) -> &'static dyn Trait` match.

**How to add a new item kind:**
1. Add the variant to `ItemKind` in [adventerm_lib/src/items/kind.rs](adventerm_lib/src/items/kind.rs) (and update `name()` / `glyph()`).
2. Add a new file under `adventerm_lib/src/items/` containing a ZST struct that `impl ItemBehavior`.
3. Add one arm to `behavior_for` in [adventerm_lib/src/items/behavior.rs](adventerm_lib/src/items/behavior.rs). The compiler will refuse to compile until you do.
4. If the new behavior needs a side-effect not yet covered, extend `ItemBehavior` with a new method (give it a default impl so existing kinds opt in by silence) or add a new field to `PlaceCtx` referencing an existing/new subsystem.

**How to add a new gameplay-construct category** (e.g. monsters):
1. Create `adventerm_lib/src/<category>/mod.rs` with a subsystem struct holding `ComponentStore<...>` fields. Expose an explicit, narrow write API.
2. Add the subsystem as a field on `Room` (room-scoped) or `GameState` (global).
3. If the binary needs to read it, add thin facade methods on `Room`/`GameState` rather than exposing the subsystem fields directly to render code.
4. If players or other constructs interact with it, define a behavior trait in the same module and dispatch through a `behavior_for`-style match.

**What `GameState` is allowed to do:**
- Hold a `World`/inventory and route actions to the right subsystem or behavior.
- **Not** match on `ItemKind` or any other construct's discriminant. If you write `match item.kind` (or equivalent) in `game.rs`, the logic belongs in a behavior impl instead.

**What the binary may know:** the existing read-only facades on `Room`/`GameState` (`items_at`, `has_item_at`, `peek_item_here`, `has_light_at`, ...). It must not import subsystem types or `World` for rendering — keep the lib/binary boundary clean.

## Reference docs

Before planning a new feature or non-trivial change, read [agents/README.md](agents/README.md) and the relevant doc(s) it indexes — they map the codebase's modules, screen state machine, and reusable patterns. Reach for an existing helper from [agents/patterns.md](agents/patterns.md) before introducing a new abstraction.

Keep `agents/` in sync. Any major change — a new screen or `Screen` variant, a new module, a new public type crossing the lib/binary boundary, a new reusable helper, a renamed or moved load-bearing symbol, or a change to the save format / generation constants — must update the relevant `agents/` file(s) in the same change. Smaller edits don't require a doc update, but periodically sweep `agents/` to catch drift.

## Code style principles

Honor these when writing or modifying code:

1. **Single-purpose structures.** A struct/enum should have one responsibility. Compose smaller pieces rather than growing a god-struct. UI state belongs to the screen variant that uses it, not on a top-level `App`.
2. **No magic numbers.** Inline literals (sizes, paddings, ranges, durations) should be named constants — ideally derived from a single source — so editing one item never requires changing several values.
3. **Concise and self-explanatory.** Code should be readable without IDE tooling or deep repo familiarity. Prefer small helpers (`carve_floor`, `menu_block`, `MenuColors`) over inlined boilerplate. Avoid intermediate names that don't earn their keep.
4. **Top-down state flow.** Ownership is hierarchical with a clear path from `main` to any logic point. Each piece of state has one owner. Avoid implicit lifecycles (e.g. status strings cleared in some transitions but not others); make state transitions explicit and ideally enforced by the type system.

## Commands

Run from the workspace root.

- Build: `cargo build`
- Test everything: `cargo test`
- Test the library only: `cargo test -p adventerm_lib`
- Run a single test: `cargo test -p adventerm_lib <test_name>`
