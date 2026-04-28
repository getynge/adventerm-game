# agents/

Reference docs for Claude Code sessions. These map the codebase's structure so a session can orient quickly without reloading the whole repo or relying on stale memory. Rules of engagement live in [../CLAUDE.md](../CLAUDE.md); the layout of the territory lives here.

## Index

- [architecture.md](architecture.md) — workspace topology, screen FSM, lib/binary boundary
- [library.md](library.md) — `adventerm_lib` module-by-module reference
- [tui.md](tui.md) — `adventerm` binary: app, screens, input, rendering, helpers
- [ffi.md](ffi.md) — `adventerm_ffi` C-ABI surface: handles, error codes, header workflow, Swift consumer guide
- [patterns.md](patterns.md) — reusable helpers and conventions to leverage before writing new code

## When to read what

| Task | Read |
| --- | --- |
| Adding a gameplay rule, dungeon feature, or save change | [library.md](library.md), then [architecture.md](architecture.md) for the boundary |
| Adding/changing a screen, menu, keybind, color scheme | [tui.md](tui.md) and [patterns.md](patterns.md) |
| Adding/changing an FFI export, handle, error code, or Swift consumer pattern | [ffi.md](ffi.md), then [library.md](library.md) for the lib type being shimmed |
| Cross-cutting refactor or new feature spanning both crates | [architecture.md](architecture.md) first, then the relevant crate doc |
| Reviewing whether a helper already exists | [patterns.md](patterns.md) |

## Keeping these docs current

These files are a living reference. When making the kinds of changes called out in the "Reference docs" section of [../CLAUDE.md](../CLAUDE.md), update the affected file(s) in the same change. Smaller edits don't require a doc update — but if something feels stale while reading, fix it.
