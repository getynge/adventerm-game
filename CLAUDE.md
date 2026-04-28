# CLAUDE.md

Adventerm is a TUI adventure game. The workspace has three crates:

- `adventerm_lib` — gameplay logic and state. No TUI dependency. See [adventerm_lib/CLAUDE.md](adventerm_lib/CLAUDE.md).
- `adventerm` — the TUI binary (`ratatui` 0.30 / `crossterm` 0.29). Renders state, forwards input. See [adventerm/CLAUDE.md](adventerm/CLAUDE.md).
- `adventerm_ffi` — C-compatible FFI surface over `adventerm_lib` for non-Rust hosts (Swift on iOS, etc.). See [adventerm_ffi/CLAUDE.md](adventerm_ffi/CLAUDE.md).

## Architectural rules

1. **Gameplay logic lives in `adventerm_lib`.** "What the game does" → library. "How the player sees or drives it" → binary.
2. **The TUI must be replaceable.** The library must not import `ratatui`/`crossterm` or expose frontend-shaped types (key codes, terminal coordinates, widgets).
3. **UI-only state stays in the binary.** Menu cursors, scroll offsets, animation timers belong to `adventerm`. The library exposes options/data; the binary decides navigation.

## Code style

1. **Single-purpose structures.** Compose smaller pieces; don't grow a god-struct. UI state belongs to the screen that uses it.
2. **No magic numbers.** Name inline literals (sizes, paddings, ranges, durations) — ideally derived from one source.
3. **Self-explanatory.** Prefer small helpers over inlined boilerplate. Avoid intermediate names that don't earn their keep.
4. **Top-down state flow.** One owner per piece of state; explicit transitions, ideally type-enforced.

## Reference docs

Before non-trivial changes, read [agents/README.md](agents/README.md) and the doc(s) it indexes. Reach for an existing helper from [agents/patterns.md](agents/patterns.md) before introducing a new abstraction.

Keep `agents/` in sync with major changes (new screen/module, new public type crossing the lib/binary boundary, renamed load-bearing symbol, save-format or generation-constant changes).

## Commands

- Build: `cargo build`
- Test everything: `cargo test`
- Test the library only: `cargo test -p adventerm_lib`
- Run a single test: `cargo test -p adventerm_lib <test_name>`
