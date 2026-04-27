# adventerm (TUI binary)

Renders [`adventerm_lib`](../adventerm_lib/) state and forwards input. `ratatui` 0.30 over `crossterm` 0.29.

Main flow ([main.rs](../adventerm/src/main.rs)): if `--dump-rooms <count> <path> [--seed <n>]` is on argv, [dump.rs](../adventerm/src/dump.rs) writes a single dungeon as ASCII art and exits before the TUI starts; otherwise `relaunch::maybe_relaunch_in_terminal()` → `App::new()` (loads saves, config, schemes) → loop { draw → poll event → `App::handle_event()` } until `Screen::Quit`.

## Core

### [app.rs](../adventerm/src/app.rs) — state machine

`App` owns `screen`, `save_dir`, `config`, `config_path`, `scheme_registry`, `any_saves`. The `Screen` enum is the FSM (see [architecture.md](architecture.md) for the variant table). Dispatch: `handle_event` matches on `screen` and routes to `handle_main_menu` / `handle_load_game` / `handle_playing` / `handle_pause_menu` / `handle_slot_picker` / `handle_name_entry` / `handle_seed_entry` / `handle_options` / `handle_options_advanced` / `handle_rebind_capture` / `handle_developer_console`.

Developer-console overlay: `Screen::DeveloperConsole { underlying: Box<Screen>, console: ConsoleState }` wraps whichever screen was active when backtick fired. While wrapped, every key routes through `input::translate_console` and `handle_developer_console`. `Screen::game_mut()` / `Screen::game()` recurse through the wrapper so commands reach the underlying gameplay state.

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

- `Action` — game-facing actions (`Up/Down/Left/Right`, `QuickUp/Down/Left/Right`, `Confirm`, `Escape`, `Delete`, `Inventory`, `ToggleConsole`, `Hotkey(char)`)
- `TextInputAction` — `Char(char)`, `Backspace`, `Confirm`, `Escape` for `NameEntry`
- `ConsoleInputAction` — `Char(char)`, `Backspace`, `Submit`, `Cancel`, `Tab`, `HistoryUp`, `HistoryDown`, `Toggle` (backtick again to close)
- `translate(event, keybinds, console_enabled) -> Option<Action>` — main path; Shift + movement maps to `Quick*`. When `console_enabled` is true, backtick maps to `Action::ToggleConsole`; otherwise it falls through as `Hotkey('`')`.
- `translate_text(event) -> Option<TextInputAction>` — used in `NameEntry` and `SeedEntry`
- `translate_console(event) -> Option<ConsoleInputAction>` — used while the developer console overlay is active

### [config.rs](../adventerm/src/config.rs) — config + schemes

- `Config { keybinds, color_scheme, dev_console_enabled: Option<bool> }` persisted as JSON to `{save_dir}/config.json`. `Config::dev_console_enabled()` resolves `None` to `cfg!(debug_assertions)` so debug builds default the console on and release builds default it off — toggling via Options → Advanced persists the choice as `Some(true|false)`.
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
- [inventory.rs](../adventerm/src/ui/inventory.rs) — three-tab popup (Items / Abilities / Stats) with a tab header row. The active tab is one of `InventoryTab` from [menu.rs](../adventerm/src/menu.rs); each tab keeps its own cursor on `Screen::Inventory`. The Items tab splits horizontally into the inventory list (left) and an equipment sidebar (right) showing the 5 fixed `EquipSlot::ALL` rows; `ItemsFocus::{List, Sidebar}` tracks which pane has the cursor. Tab key (translated to `Action::Inventory` while inside the popup) cycles `Items(List) → Items(Sidebar) → Abilities → Stats`; Up/Down moves within the active panel. Confirm in Items dispatches by category (`category_of(kind)`): Placeable → `PlaceItemAction` and close; Equipment → `EquipItemAction` (stay open); Consumable → either `ConsumeItemAction` immediately (`ConsumeIntent::Immediate`) or enters the pending-consume substate (`PendingConsume { intent }`) which auto-switches to the Abilities tab and asks the player to pick a slot before firing `ConsumeItemAction { target: ConsumeTarget::AbilitySlot(...) }`. Confirm in the equipment sidebar fires `UnequipItemAction { slot }`. Stats tab reads `effective_stats()` and shows `Total (base ± bonus)` plus a `Vision N tiles` row sourced from `vision_radius()`.
- [main_menu.rs](../adventerm/src/ui/main_menu.rs) — title + centered options + status; uses `accel::assign` + `accel::line`
- [gameplay.rs](../adventerm/src/ui/gameplay.rs) — three-region layout per CLAUDE.md rule #3 (world, dialog beneath, actions panel right). `scroll_offset` keeps the player centered. Tile glyphs: `@` player, `.` floor, `#` wall, `+` door. Each cell picks one of three render paths via `GameState::is_visible` / `is_explored`: in-LOS uses full-brightness `WorldColors`, out-of-LOS-but-explored uses the dimmed `memory_*` colors, unseen tiles render as a blank space.
- [pause_menu.rs](../adventerm/src/ui/pause_menu.rs) — fixed-width 24 popup, height from option count
- [name_entry.rs](../adventerm/src/ui/name_entry.rs) — fixed 44×5 popup; renders buffer + trailing `_` cursor
- [seed_entry.rs](../adventerm/src/ui/seed_entry.rs) — fixed 44×5 popup for new-game seed input; renders buffer + trailing `_` cursor over a dimmed main-menu underlay
- [save_browser.rs](../adventerm/src/ui/save_browser.rs) — list with relative-time formatting; overlays a confirm prompt when `pending_delete` is set; `Mode` differentiates Load vs SavePicker (the latter prepends "+ New save...")
- [options.rs](../adventerm/src/ui/options.rs) — full panel with color scheme + 7 keybind rows + Advanced... + Reset/Back; overlays a "Press a key for…" prompt during `RebindCapture`
- [options_advanced.rs](../adventerm/src/ui/options_advanced.rs) — sub-panel for the Options → Advanced sub-screen. Today: dev-console toggle + Back. Adding a future advanced toggle = one variant on `OptionsAdvancedRow` + one arm in `activate_options_advanced_row` + one row in `options_advanced_row_label`.
- [console.rs](../adventerm/src/ui/console.rs) — developer-console overlay. Three vertical regions: log pane (level-colored, tail of the `console::log_sink::LOG_BUFFER`), input row with prompt + ghost-text completion, footer hint. Use the `Tab` key to accept the ghost (or cycle when ambiguous), Up/Down for history, Enter to submit, Esc/backtick to close.

### UI helpers

- [accel.rs](../adventerm/src/ui/accel.rs) — `assign(labels)` picks unique accelerator letters skipping reserved WASD/hjkl; `line(label, accel, selected, colors)` renders the row; `find_by_hotkey(labels, key)` resolves a press to an option index; `matches` for case-insensitive char comparison
- [colors.rs](../adventerm/src/ui/colors.rs) — `MenuColors`, `WorldColors`, `SchemeColors`; `menu_block(title, colors)` returns a styled bordered `Block`; `rgb_to_color` converts a palette `Rgb` to ratatui `Color`. `WorldColors` also exposes derived `memory_floor` / `memory_wall` / `memory_interactive` (palette × `MEMORY_DIM_FACTOR`) for fog-of-war rendering — schemes don't list these, they're computed from the existing world palette.
- [layout.rs](../adventerm/src/ui/layout.rs) — `popup_rect(area, w, h)` centers; named constants for popup/panel padding, fixed widths (`PAUSE_MENU_WIDTH`, `NAME_ENTRY_WIDTH/HEIGHT`, `MAIN_MENU_OPTIONS_WIDTH`, `SAVE_BROWSER_HORIZONTAL_PAD`, `STATUS_POPUP_HEIGHT`, `POPUP_MIN_WIDTH`, `POPUP_BORDER_PAD`, `PANEL_*`, `CONSOLE_*`)

When adding a new screen or widget, the helpers above already cover most of what you need — see [patterns.md](patterns.md) before writing new ones.

## Developer console

The console runtime lives in the **library** at [adventerm_lib/src/console/](../adventerm_lib/src/console/) — `ConsoleState`, the parser, completer, command trait, and every command implementation are gameplay code. The **binary** owns only what's needed to render the popup and capture log output:
- [adventerm/src/console/log_sink.rs](../adventerm/src/console/log_sink.rs) — installs a `log::Log` impl into a global `OnceLock<Mutex<VecDeque<LogEntry>>>` (capacity `LOG_BUFFER_CAPACITY = 256`). `init()` runs once from `main.rs`; the renderer reads via `snapshot(n)`.
- [adventerm/src/ui/console.rs](../adventerm/src/ui/console.rs) — the popup renderer (described above).
- `adventerm/src/console/mod.rs` re-exports `adventerm_lib::console::ConsoleState` so existing call sites still see `crate::console::ConsoleState`.

The console is always compiled in; the runtime gate is `Config::dev_console_enabled()`.

Library-side surface ([adventerm_lib/src/console/](../adventerm_lib/src/console/)):

- [`command::DevCommand`](../adventerm_lib/src/console/command.rs) — trait with `name`, `help`, `arg_completions`, `execute`. Adding a new command: drop a file under [adventerm_lib/src/console/commands/](../adventerm_lib/src/console/commands/) with a `static` ZST that `impl DevCommand`, then add one `&NAME` entry to the `REGISTRY` slice in [command.rs](../adventerm_lib/src/console/command.rs). Lookup is `find(name)`; the completer iterates the same `registry()` so command names and arg completions stay in lockstep.
- [`parse::tokenize`](../adventerm_lib/src/console/parse.rs) — whitespace tokenizer with `"…"` quoted-string support. Used by both the executor and the completer.
- [`complete::Completion::from_input(input, game)`](../adventerm_lib/src/console/complete.rs) — single entry point that returns the candidate list and the ghost-text suffix. Tab handling lives on `Completion::accept_into(input, cycle_index)`.
- [`commands::{fullbright,spawn,give}`](../adventerm_lib/src/console/commands/) — the three initial commands. `give` and `spawn item` parse argument strings via `ItemKind::from_display_name`; `spawn enemy` parses via `EnemyKind::from_display_name`. `spawn item` with no name reuses `items::random::random_item_kind(player.enemy_rng)` so the distribution matches dungeon generation.

Library and binary code emit through `log::{info,warn,error,debug,trace}!`; the library only depends on the `log` *facade* and does **not** install a logger — the binary's `log_sink::init()` is the single registration site.
