# Architecture

## Workspace

Two crates, root [Cargo.toml](../Cargo.toml):

- [adventerm_lib/](../adventerm_lib/) — gameplay logic and state. No TUI deps. See [library.md](library.md).
- [adventerm/](../adventerm/) — TUI binary on `ratatui` 0.30 / `crossterm` 0.29. See [tui.md](tui.md).

The lib/binary split is enforced by CLAUDE.md rules #1 and #2: gameplay logic belongs in the library, and the library must not expose frontend-specific types.

## State ownership

Top-down chain (CLAUDE.md style rule #4):

```
main()
 └─ App                                   (adventerm/src/app.rs)
     ├─ screen: Screen                    (the FSM — single source of truth for "what's on screen")
     ├─ save_dir, config_path             (paths)
     ├─ config: Config                    (keybinds + active scheme name)
     ├─ scheme_registry: SchemeRegistry   (built-in + user color schemes)
     └─ any_saves: bool                   (cached toggle for "Load Game" visibility)
```

Each `Screen` variant owns its own UI state (cursor, scroll, status). The gameplay `GameState` lives inside whichever variant is currently using it (`Playing`, `Paused`, `SaveSlotPicker`, `NameEntry`) and is moved between variants on transition — there is no global mutable game state on `App`.

## Screen FSM

Defined as the [`Screen`](../adventerm/src/app.rs) enum. Variants and the `GameState` they carry:

| Variant | Owns `GameState`? | Notes |
| --- | --- | --- |
| `MainMenu` | no | Entry point; menu cursor + status |
| `LoadGame` | no | Browses save files; can delete with confirm |
| `SeedEntry` | no | Text buffer (≤32 chars) for an optional new-game seed; blank → clock seed |
| `Playing(GameState)` | yes | Active gameplay |
| `Paused` | yes (retained) | Overlay on Playing; offers Resume / Save / Quit |
| `SaveSlotPicker` | yes | Pick existing slot to overwrite, or "+ New save..." |
| `NameEntry` | yes | Text buffer (≤32 chars) for new save name |
| `Options` | no | Color scheme + 7 keybinds + Reset / Back |
| `RebindCapture` | no | Modal awaiting next raw key press |
| `Quit` | — | Sentinel; main loop exits |

Transitions are pattern-matched in `App::handle_*` methods per screen. Triggers:

- **Confirm / Esc / Hotkey** — driven by `BoundAction` lookups via [`input::translate`](../adventerm/src/input.rs)
- **Char / Backspace / Enter / Esc** — in `NameEntry` and `SeedEntry`, via [`input::translate_text`](../adventerm/src/input.rs)
- **Movement / interact** — only in `Playing`; calls into `GameState::move_player`, `quick_move`, `interact`

Status messages on each screen are an enum (`Status::None | Info | Error`) cleared explicitly on transitions — see CLAUDE.md style rule #4.

## Library/binary boundary

What crosses (library types used in the binary):

- [`GameState`](../adventerm_lib/src/game.rs), [`MoveOutcome`](../adventerm_lib/src/game.rs), [`DoorEvent`](../adventerm_lib/src/game.rs)
- [`Direction`](../adventerm_lib/src/world.rs), [`Tile`](../adventerm_lib/src/world.rs) — `Tile` is the only rendering primitive the library exposes
- [`Save`](../adventerm_lib/src/save.rs), [`SaveSlot`](../adventerm_lib/src/save.rs), [`SaveError`](../adventerm_lib/src/save.rs), [`SAVE_VERSION`](../adventerm_lib/src/save.rs), `slugify`, `slot_path`, `list_saves`, `delete_save`
- [`RoomId`](../adventerm_lib/src/room.rs), [`DoorId`](../adventerm_lib/src/room.rs), [`TileKind`](../adventerm_lib/src/room.rs) — used indirectly through `Tile` queries and door interaction

What must not cross:

- `ratatui`/`crossterm` types (no key codes, colors, rects in the library API)
- Layout constants, popup sizes, color palettes
- Menu cursor positions, scroll offsets, status strings

If you find yourself reaching for a `KeyCode` in the library or a `Dungeon` field in a renderer, that's a signal the design is bending around the boundary — restructure instead.

## Determinism and persistence

- Dungeon generation is fully seeded (xorshift `Rng`); same seed → identical dungeon.
- Saves are JSON with a version field (`SAVE_VERSION = 2`); load rejects mismatches cleanly.
- Config (keybinds + active scheme) is JSON at `{save_dir}/config.json`; user-defined color schemes live in `{save_dir}/schemes/*.json`.
