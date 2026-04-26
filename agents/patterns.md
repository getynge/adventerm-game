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
| Tile a renderer should draw at `(x, y)` | `GameState::tile_at` in [game.rs](../adventerm_lib/src/game.rs) |
| "Press Enter" hint when standing on a door | `GameState::player_on_door` in [game.rs](../adventerm_lib/src/game.rs) |
| Move the player one tile or slide them | `move_player` / `quick_move` in [game.rs](../adventerm_lib/src/game.rs) |
| Traverse a door | `interact` in [game.rs](../adventerm_lib/src/game.rs) |
| Random number for any new gameplay logic | `Rng` in [rng.rs](../adventerm_lib/src/rng.rs) — keep determinism by threading the seeded RNG, never `rand::random()` |

If a new gameplay query is needed, add it as a method on `GameState` (or another lib type) rather than reaching into `Dungeon`/`Room` from the binary — that preserves CLAUDE.md rules #1 and #2.
