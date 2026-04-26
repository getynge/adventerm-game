# adventerm (TUI binary)

Renders [`adventerm_lib`](../adventerm_lib/) state and forwards input. `ratatui` 0.30 over `crossterm` 0.29.

Main flow ([main.rs](../adventerm/src/main.rs)): if `--dump-rooms <count> <path> [--seed <n>]` is on argv, [dump.rs](../adventerm/src/dump.rs) writes a single dungeon as ASCII art and exits before the TUI starts; otherwise `relaunch::maybe_relaunch_in_terminal()` → `App::new()` (loads saves, config, schemes) → loop { draw → poll event → `App::handle_event()` } until `Screen::Quit`.

## Core

### [app.rs](../adventerm/src/app.rs) — state machine

`App` owns `screen`, `save_dir`, `config`, `config_path`, `scheme_registry`, `any_saves`. The `Screen` enum is the FSM (see [architecture.md](architecture.md) for the variant table). Dispatch: `handle_event` matches on `screen` and routes to `handle_main_menu` / `handle_load_game` / `handle_playing` / `handle_pause_menu` / `handle_slot_picker` / `handle_name_entry` / `handle_seed_entry` / `handle_options` / `handle_rebind_capture`.

Per-screen behavior (high level):

| Screen | Inputs | Notable transitions |
| --- | --- | --- |
| `MainMenu` | Up/Down, Confirm, Hotkey | New Game → `SeedEntry`, Load Game → `LoadGame`, Options → `Options`, Quit → `Quit` |
| `LoadGame` | Up/Down, Confirm, Delete/`x`, Esc | Confirm loads save → `Playing`; Delete sets `pending_delete`, next Confirm deletes |
| `SeedEntry` | Char, Backspace, Enter, Esc | Enter resolves seed (blank → clock, else hashed text) → `Playing`; Esc → `MainMenu` |
| `Playing` | Movement (+ Shift = quick), Confirm = `interact`, Tab = inventory, Esc | A move whose `MoveOutcome` is `Encounter` opens `Battle`. Esc → `Paused`. |
| `Battle` | Up/Down on player turn, Confirm to use the selected ability or advance the enemy turn, Esc to flee | Resolves to `Playing` on victory/flee (enemy despawned on victory) or `MainMenu` on defeat. |
| `Inventory` | Tab cycles tabs (Items / Abilities / Stats), Up/Down, Confirm, Esc | Confirm on Items places that item; Abilities and Stats are read-only for now. |
| `Paused` | Up/Down, Confirm, Hotkey, Esc | Resume / Save (→ `SaveSlotPicker`) / Quit |
| `SaveSlotPicker` | Up/Down, Confirm, Esc | Index 0 ("+ New save...") → `NameEntry`; otherwise overwrite |
| `NameEntry` | Char, Backspace, Enter, Esc | Enter validates + writes save → `Paused` |
| `Options` | Up/Down, Confirm, Hotkey, Esc | ColorScheme cycles; Keybind row → `RebindCapture`; Reset/Back |
| `RebindCapture` | any raw key, Esc | Stores key for `BoundAction`, persists, → `Options` |

### [input.rs](../adventerm/src/input.rs) — event translation

- `Action` — game-facing actions (`Up/Down/Left/Right`, `QuickUp/Down/Left/Right`, `Confirm`, `Escape`, `Delete`, `Hotkey(char)`)
- `TextInputAction` — `Char(char)`, `Backspace`, `Confirm`, `Cancel` for `NameEntry`
- `translate(event, keybinds) -> Option<Action>` — main path; Shift + movement maps to `Quick*`
- `translate_text(event) -> Option<TextInputAction>` — used in `NameEntry` and `SeedEntry`

### [config.rs](../adventerm/src/config.rs) — config + schemes

- `Config { keybinds, color_scheme }` persisted as JSON to `{save_dir}/config.json`
- `KeybindMap` — slots for `BoundAction::{Up, Down, Left, Right, Confirm, Escape, Delete}`; defaults include arrows + WASD + hjkl. `set(action, key)` replaces and unbinds from other actions.
- `Key` / `NamedKey` — keybind serialization primitives (no `KeyCode` leaks across the lib boundary)
- `SchemeRegistry` loads built-ins (`default`, `high-contrast`, `dim`) plus user JSON from `{save_dir}/schemes/*.json`. `next_after(name)` cycles alphabetically.

### [menu.rs](../adventerm/src/menu.rs) — generic menu plumbing

- `MenuState<T>` — cursor + options vec; `up()`, `down()`, `current()`, `select_hotkey(labels, key)` with wrap navigation
- `MainMenuOption`, `PauseMenuOption`, `OptionsRow` — variant lists for the three menu screens; `MainMenuOption::available(any_saves)` filters Load Game
- `Status::{None, Info(String), Error(String)}` — per-screen status string, cleared on transitions
- `SaveBrowser { cursor, pending_delete: Option<usize> }` — used by both `LoadGame` and `SaveSlotPicker`

### [relaunch.rs](../adventerm/src/relaunch.rs) — terminal autospawn

If stdin/stdout aren't a TTY (file-manager launch), spawn a terminal and re-exec. Per-platform fallback chain: macOS uses `osascript`; Windows tries `wt.exe` then `cmd /C start`; Linux iterates `gnome-terminal`, `konsole`, `xfce4-terminal`, `alacritty`, `kitty`, etc. The `ADVENTERM_RELAUNCHED` env var prevents loops.

## Rendering

### [ui.rs](../adventerm/src/ui.rs) — render dispatcher

`render(frame, app)` converts the current `ColorScheme` into `SchemeColors` once per frame (avoid recomputing inside widgets), then matches on the screen and delegates. `render_main_menu_underlay` paints the dimmed main menu behind `LoadGame` / `Options` / `RebindCapture`.

### Battle screen

`adventerm/src/ui/battle.rs` paints a single full-frame popup with three vertical regions:

1. **Header** — player and enemy names with HP bars (`[####···] cur / max`). HP-bar width is the named constant `HP_BAR_WIDTH` (default 20).
2. **Actions** — list of the player's active ability slots (read from `GameState::abilities`). Cursor only renders during the player turn.
3. **Log** — last `BATTLE_LOG_LINES` (default 8) lines from `BattleState::log`, plus a status overlay (e.g. "That slot is empty.") and a hint footer that adapts to whose turn it is.

All input dispatch lives in `App::handle_battle` — Up/Down moves the cursor among ability slots on the player turn, Confirm fires `battle::apply_player_ability` (or `apply_enemy_turn` when it's the enemy's turn), and Esc on the player turn flags the battle as `Resolved(Fled)`. When `BattleState::result()` becomes `Some`, any subsequent Confirm/Esc routes through `App::finish_battle`.

### Per-screen renderers (`adventerm/src/ui/`)

- [battle.rs](../adventerm/src/ui/battle.rs) — covered in the previous section. Reuses `menu_block`, `MenuColors`, `Layout::vertical`. Enemy glyph is read via `room.enemies.kind_of(entity)`.
- [inventory.rs](../adventerm/src/ui/inventory.rs) — three-tab popup (Items / Abilities / Stats) with a tab header row. The active tab is one of `InventoryTab` from [menu.rs](../adventerm/src/menu.rs); each tab keeps its own cursor on `Screen::Inventory`. Tab key (translated to `Action::Inventory` while inside the popup) cycles tabs; Up/Down moves within the active panel; Confirm in Items places that item.
- [main_menu.rs](../adventerm/src/ui/main_menu.rs) — title + centered options + status; uses `accel::assign` + `accel::line`
- [gameplay.rs](../adventerm/src/ui/gameplay.rs) — three-region layout per CLAUDE.md rule #3 (world, dialog beneath, actions panel right). `scroll_offset` keeps the player centered. Tile glyphs: `@` player, `.` floor, `#` wall, `+` door. Each cell picks one of three render paths via `GameState::is_visible` / `is_explored`: in-LOS uses full-brightness `WorldColors`, out-of-LOS-but-explored uses the dimmed `memory_*` colors, unseen tiles render as a blank space.
- [pause_menu.rs](../adventerm/src/ui/pause_menu.rs) — fixed-width 24 popup, height from option count
- [name_entry.rs](../adventerm/src/ui/name_entry.rs) — fixed 44×5 popup; renders buffer + trailing `_` cursor
- [seed_entry.rs](../adventerm/src/ui/seed_entry.rs) — fixed 44×5 popup for new-game seed input; renders buffer + trailing `_` cursor over a dimmed main-menu underlay
- [save_browser.rs](../adventerm/src/ui/save_browser.rs) — list with relative-time formatting; overlays a confirm prompt when `pending_delete` is set; `Mode` differentiates Load vs SavePicker (the latter prepends "+ New save...")
- [options.rs](../adventerm/src/ui/options.rs) — full panel with color scheme + 7 keybind rows + Reset/Back; overlays a "Press a key for…" prompt during `RebindCapture`

### UI helpers

- [accel.rs](../adventerm/src/ui/accel.rs) — `assign(labels)` picks unique accelerator letters skipping reserved WASD/hjkl; `line(label, accel, selected, colors)` renders the row; `find_by_hotkey(labels, key)` resolves a press to an option index; `matches` for case-insensitive char comparison
- [colors.rs](../adventerm/src/ui/colors.rs) — `MenuColors`, `WorldColors`, `SchemeColors`; `menu_block(title, colors)` returns a styled bordered `Block`; `rgb_to_color` converts a palette `Rgb` to ratatui `Color`. `WorldColors` also exposes derived `memory_floor` / `memory_wall` / `memory_interactive` (palette × `MEMORY_DIM_FACTOR`) for fog-of-war rendering — schemes don't list these, they're computed from the existing world palette.
- [layout.rs](../adventerm/src/ui/layout.rs) — `popup_rect(area, w, h)` centers; named constants for popup/panel padding, fixed widths (`PAUSE_MENU_WIDTH`, `NAME_ENTRY_WIDTH/HEIGHT`, `MAIN_MENU_OPTIONS_WIDTH`, `SAVE_BROWSER_HORIZONTAL_PAD`, `STATUS_POPUP_HEIGHT`, `POPUP_MIN_WIDTH`, `POPUP_BORDER_PAD`, `PANEL_*`)

When adding a new screen or widget, the helpers above already cover most of what you need — see [patterns.md](patterns.md) before writing new ones.
