# CLAUDE.md

Adventerm is a TUI adventure game. The workspace has three crates:

- `adventerm_lib` — gameplay logic and state. No TUI dependency. See [adventerm_lib/CLAUDE.md](adventerm_lib/CLAUDE.md).
- `adventerm` — the TUI binary (`ratatui` 0.30 / `crossterm` 0.29). Renders state, forwards input. See [adventerm/CLAUDE.md](adventerm/CLAUDE.md).
- `adventerm_ffi` — C-compatible FFI surface over `adventerm_lib` for non-Rust hosts (Swift on iOS, etc.). See [adventerm_ffi/CLAUDE.md](adventerm_ffi/CLAUDE.md).

## Architectural rules

1. **Gameplay logic lives in `adventerm_lib`.** "What the game does" → library. "How the player sees or drives it" → binary.
2. **The TUI must be replaceable.** The library must not import `ratatui`/`crossterm` or expose frontend-shaped types (key codes, terminal coordinates, widgets).
3. **UI-only state stays in the binary.** Menu cursors, scroll offsets, animation timers belong to `adventerm`. The library exposes options/data; the binary decides navigation.
4. **Public API changes in `adventerm_lib` require matching `adventerm_ffi` updates.** The FFI is a non-Rust consumer of the lib's public surface — adding/renaming/removing a public type, enum variant, function, or facade method that the FFI exposes (or could expose) means updating `adventerm_ffi/src/` (mirror types, shims, conversions) and regenerating `adventerm_ffi/include/adventerm_ffi.h` in the same change. Enum discriminants are wire-stable: APPEND new variants only. See [agents/ffi.md](agents/ffi.md) for the boundary rules and stability contract.

   **How to perform the FFI sync.** Follow the [sync-ffi](.claude/skills/sync-ffi/SKILL.md) skill end-to-end (detect lib delta → edit FFI files → regenerate header → `clang -fsyntax-only` → `cargo test -p adventerm_ffi`). Inline is fine for small changes (one new variant, one new query). For large mirror work — broad refactors, many files touched, lots of diff/build noise — consider delegating to a subagent (`Agent` tool, `subagent_type: general-purpose`) running the same skill, so the build/test churn stays out of the main context. The user can also invoke `/sync-ffi` directly.

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
