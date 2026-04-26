# adventerm

TUI binary built on `ratatui` 0.30 over `crossterm` 0.29. Renders state from `adventerm_lib` and forwards input. Owns no gameplay logic.

## Layout

**Core gameplay** has three fixed regions:
- A large central window (the world).
- A short window beneath it for dialog/game information.
- A narrow tall window to the right for player actions/options.

This layout applies *only* to core gameplay. Main menu, pause menu, inventory, name/seed entry, options, and save browser are not bound by it.

## UI conventions

- **Screens are the unit of state.** Each screen variant owns the UI-only state it needs (cursor positions, scroll offsets, edit buffers). Don't put per-screen state on a top-level `App`.
- **Read state through library facades** (`items_at`, `has_item_at`, `peek_item_here`, `has_light_at`, ...). Don't import subsystem types or `World` for rendering.
- **Color and styling** flow through [src/ui/colors.rs](src/ui/colors.rs) and the JSON schemes in [schemes/](schemes/). Do not hardcode colors at call sites.
- **Layout helpers** live in [src/ui/layout.rs](src/ui/layout.rs); reuse before splitting `Rect`s by hand.
- **Menu rendering** uses the shared menu helpers (`menu_block`, `MenuColors`, accelerator handling in [src/ui/accel.rs](src/ui/accel.rs)). Don't reinvent.
- **Input** is translated to library calls in [src/input.rs](src/input.rs). Key codes never cross into `adventerm_lib`.
- **No magic numbers.** Sizes, paddings, and durations live as named constants — one source per value.
