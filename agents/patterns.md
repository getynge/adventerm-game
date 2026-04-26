# Patterns to leverage

Helpers and conventions that already exist. Reach for these before writing new code — see CLAUDE.md style rules #1 (single-purpose) and #3 (concise, prefer small helpers).

## Menus

| Need | Use |
| --- | --- |
| Cursor-driven option list with wrap navigation | `MenuState<T>` in [menu.rs](../adventerm/src/menu.rs) |
| Auto-assigned single-letter shortcuts on menu items | `accel::assign` in [accel.rs](../adventerm/src/ui/accel.rs) |
| Render an option row (`> label <` selected, underlined accelerator) | `accel::line` in [accel.rs](../adventerm/src/ui/accel.rs) |
| Resolve a key press to an option index | `accel::find_by_hotkey` in [accel.rs](../adventerm/src/ui/accel.rs) |
| Per-screen status (info/error) string with explicit clearing on transitions | `Status` in [menu.rs](../adventerm/src/menu.rs) |

When adding a new menu screen, model it as `MenuState<NewOptionEnum>` plus a `Status` and dispatch through `accel::find_by_hotkey` for letter shortcuts. Don't reimplement cursor wrap, accelerator assignment, or status styling.

## Layout and popups

| Need | Use |
| --- | --- |
| Center any popup on the frame | `popup_rect(area, w, h)` in [layout.rs](../adventerm/src/ui/layout.rs) |
| Bordered popup block with title styling | `menu_block(title, colors)` in [colors.rs](../adventerm/src/ui/colors.rs) |
| Sizing for popups, panels, save browser, name entry, status overlay | named constants in [layout.rs](../adventerm/src/ui/layout.rs) |

Never hardcode popup positions or sizes — extend the constants instead (CLAUDE.md style rule #2).

## Colors

| Need | Use |
| --- | --- |
| Pre-computed ratatui `Color` values for the active scheme | `SchemeColors` / `MenuColors` / `WorldColors` in [colors.rs](../adventerm/src/ui/colors.rs) |
| Convert a palette `Rgb` to a `Color` | `rgb_to_color` in [colors.rs](../adventerm/src/ui/colors.rs) |
| Cycle to the next color scheme | `SchemeRegistry::next_after` in [config.rs](../adventerm/src/config.rs) |

`SchemeColors` is built once per frame in [ui.rs](../adventerm/src/ui.rs) — pass the struct down rather than re-deriving colors in widgets.

## Input

| Need | Use |
| --- | --- |
| Key event → game `Action` for any keybind-driven screen | `input::translate` in [input.rs](../adventerm/src/input.rs) |
| Key event → text-entry action (only in `NameEntry`) | `input::translate_text` in [input.rs](../adventerm/src/input.rs) |
| Bind / rebind a keyboard shortcut | `KeybindMap::set` in [config.rs](../adventerm/src/config.rs) (replaces and unbinds from other actions) |

If a new screen needs raw key capture (like `RebindCapture`), don't add a third translator — handle the raw event inline and route everything else through `translate`.

## Saves

| Need | Use |
| --- | --- |
| Filesystem-safe filename from a display name | `slugify` in [save.rs](../adventerm_lib/src/save.rs) |
| Path for a save slot | `slot_path(dir, name)` in [save.rs](../adventerm_lib/src/save.rs) |
| List existing saves (sorted, version-checked, tolerant) | `list_saves(dir)` in [save.rs](../adventerm_lib/src/save.rs) |
| Delete a save | `delete_save(path)` in [save.rs](../adventerm_lib/src/save.rs) |
| Serialize / deserialize a save (with version validation) | `Save::to_bytes` / `Save::from_bytes` in [save.rs](../adventerm_lib/src/save.rs) |

Never reimplement filename munging or directory scanning — these helpers already handle missing dirs, malformed JSON, and version mismatches.

## Config

| Need | Use |
| --- | --- |
| Persist after mutating | `Config::save(path)` in [config.rs](../adventerm/src/config.rs) (call after every mutation that should survive a restart) |
| Restore defaults | `Config::default()` in [config.rs](../adventerm/src/config.rs) |
| Load built-in + user color schemes | `SchemeRegistry::load` in [config.rs](../adventerm/src/config.rs) |

## Library boundary

| Need | Use |
| --- | --- |
| Tile a renderer should draw at `(x, y)` (with player overlay) | `GameState::tile_at` in [game.rs](../adventerm_lib/src/game.rs) |
| Underlying terrain at `(x, y)` (no player overlay — for memory cells) | `GameState::terrain_at` in [game.rs](../adventerm_lib/src/game.rs) |
| Is `(x, y)` currently in line of sight | `GameState::is_visible` in [game.rs](../adventerm_lib/src/game.rs) |
| Has the player ever seen `(x, y)` in this room | `GameState::is_explored` in [game.rs](../adventerm_lib/src/game.rs) |
| LOS radius constant | `LOS_RANGE` from [los.rs](../adventerm_lib/src/los.rs) |
| "Press Enter" hint when standing on a door | `GameState::player_on_door` in [game.rs](../adventerm_lib/src/game.rs) |
| Move the player one tile or slide them | `move_player` / `quick_move` in [game.rs](../adventerm_lib/src/game.rs) |
| Traverse a door | `interact` in [game.rs](../adventerm_lib/src/game.rs) |
| Random number for any new gameplay logic | `Rng` in [rng.rs](../adventerm_lib/src/rng.rs) — keep determinism by threading the seeded RNG, never `rand::random()` |

If a new gameplay query is needed, add it as a method on `GameState` (or another lib type) rather than reaching into `Dungeon`/`Room` from the binary — that preserves CLAUDE.md rules #1 and #2.

## Gameplay constructs (ECS + behaviors)

See [CLAUDE.md "Gameplay constructs"](../CLAUDE.md) for the full how-to. Quick map:

| Need | Use |
|------|-----|
| Spawn / despawn an entity, get/set its position | `World` in [ecs/mod.rs](../adventerm_lib/src/ecs/mod.rs) |
| Generic per-component sparse storage | `ComponentStore<T>` in [ecs/mod.rs](../adventerm_lib/src/ecs/mod.rs) |
| Add a torch / flare / iterate light sources / burn out flares | `Lighting` in [lighting/mod.rs](../adventerm_lib/src/lighting/mod.rs) |
| Spawn / take / iterate ground items | `ItemSubsystem` in [items/storage.rs](../adventerm_lib/src/items/storage.rs) |
| Define what happens when an item is placed | implement `ItemBehavior` in a new file under [items/](../adventerm_lib/src/items/) and add an arm to `behavior_for` in [items/behavior.rs](../adventerm_lib/src/items/behavior.rs) |
| Define what an ability does in battle | implement `ActiveAbility` in a new file under [abilities/](../adventerm_lib/src/abilities/) and add an arm to `ability_behavior_for` in [abilities/active.rs](../adventerm_lib/src/abilities/active.rs) |
| Define a passive effect | implement `PassiveAbility` and add a variant to `PassiveKind` plus an arm to `passive_behavior_for` (currently `match kind {}` because the enum has no variants yet) |
| Define an enemy AI | implement `EnemyAi` in a new file under [enemies/](../adventerm_lib/src/enemies/) and add an arm to `enemy_behavior_for` in [enemies/ai.rs](../adventerm_lib/src/enemies/ai.rs) |
| Spawn / despawn / read enemies in a room | `room.enemies.spawn_at(...)`, `room.enemies.despawn(...)`, `room.enemies_iter()` (binary side, no `EntityId`) — `Enemies` itself is internal |
| Tick enemy AI after a player move | `enemies::tick_enemies(room, player_pos, rng)` — already wired into `GameState::move_player` and `quick_move`, don't call directly |
| Start / step a battle | `battle::start_battle(game, enemy_id)`, `battle::apply_player_ability(game, state, slot)`, `battle::apply_enemy_turn(game, state)` |
| Recompute lit tiles for the current room | `visibility::compute_room_lighting` in [visibility.rs](../adventerm_lib/src/visibility.rs) |

**Don't** add per-category fields to `World`. Write a new subsystem instead — `World` stays a stable substrate as the game grows.

**Don't** match on `ItemKind`, `AbilityKind`, `PassiveKind`, or `EnemyKind` outside a `behavior_for`-style registry. The whole point of the trait is that `GameState`, `BattleState`, and other generic call sites never need to learn about specific kinds.
