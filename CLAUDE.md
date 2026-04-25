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

## Commands

Run from the workspace root.

- Build: `cargo build`
- Test everything: `cargo test`
- Test the library only: `cargo test -p adventerm_lib`
- Run a single test: `cargo test -p adventerm_lib <test_name>`
