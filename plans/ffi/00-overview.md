# 00 — Overview

This directory plans the rollout of a C-compatible FFI layer for `adventerm_lib`. The work spans ~10 PR-sized milestones across a new `adventerm_ffi` crate, build/CI changes, and one `agents/` doc update.

**Audience:** anyone picking up a milestone. Read this file plus the per-milestone file (01–07) you're executing.

## Goal

Expose `adventerm_lib` to non-Rust consumers — primarily an iOS Swift app, secondarily Android NDK and desktop. The exposed surface must let a host app run a complete game session: create a seeded game, dispatch all player actions, query world state, render the dungeon, persist via save/load, run battles, and (optionally) drive the dev console.

## Hard constraints

1. **Rust-first.** `adventerm_lib`'s public Rust API stays idiomatic. No `#[repr(C)]` on its core types, no `extern "C"` exports in its module tree, no `*mut`/`*const` in its public signatures. Pure-Rust consumers (the `adventerm` binary, future Rust integrations) keep getting the API they have today.
2. **Shim Rust-specific features.** Rust enums-with-data, trait objects, generics, lifetimes, `Vec<T>`, `HashMap<K,V>`, `String` need shims so C/Swift can use the underlying behavior. The shim layer lives entirely in the new FFI crate.
3. **Behavior dispatch stays internal.** Trait objects (`&dyn ItemBehavior`, `&dyn ActiveAbility`, `&dyn DevCommand`) never cross the FFI boundary. Behavior is selected by enum discriminant on the Rust side; only outcomes are visible to C.

## Decisions matrix (the non-obvious calls)

| # | Decision | Why |
|---|----------|-----|
| 1 | Separate `adventerm_ffi` sub-crate, not a feature flag on `adventerm_lib` | Feature-gated FFI mutates the build graph for non-FFI consumers (Cargo unifies features per resolver pass) and invites `#[cfg(feature = "ffi")]` to creep into gameplay code. Separate crate enforces the Rust-first constraint structurally. |
| 2 | Opaque handles for everything large; values for everything Copy-small | Avoids exposing struct layouts; sub-handles are independent owners (no parent/child lifetime contract). |
| 3 | No FFI function returns a borrowed pointer into a handle's interior | Sidesteps Swift lifetime questions entirely; small structs returned by value, strings via copy-into-buffer. |
| 4 | `i32` return + out-parameter pattern for every fallible call | Wraps cleanly into Swift `throws`; unambiguous (no sentinel-vs-value confusion). |
| 5 | Every `extern "C"` body wrapped in `catch_unwind` | Panics never cross the FFI boundary — converted to `FfiError::InternalPanic` with detail in `LAST_ERROR`. |
| 6 | Three string patterns, each scoped to one use case | Borrowed `'static` (interned `OnceLock<CString>`), owned variable (caller-allocated buffer), strings INTO FFI (`*const c_char` + UTF-8 validation). No `*_free_string` exports. |
| 7 | All FFI enums are `#[repr(u8)]` mirror types in `adventerm_ffi/src/enums.rs` | Zero `#[repr(C)]` added to `adventerm_lib`. `From`/`TryFrom` between lib and FFI types. |
| 8 | Data-carrying enums become tagged C structs, not `#[repr(C)] union` | Swift bridges C unions awkwardly; tagged structs read like normal Swift structs and the wasted bytes are negligible. |
| 9 | Per-action FFI entries (nine functions), not a central tagged-action union | Clean Swift signatures; no double-discrimination on action-tag plus outcome-tag. |
| 10 | Iteration via copy-into-buffer or count+index pairs, never iterator handles | Snapshot semantics; no stable Rust iterator can cross FFI safely. |
| 11 | Header generated via `cbindgen` from `build.rs`; committed at `adventerm_ffi/include/adventerm_ffi.h` | CI verifies regeneration is a no-op. cbindgen does NOT introspect `adventerm_lib` (`parse.parse_deps = false`). |
| 12 | iOS XCFramework build is a separate script, invoked manually or by release CI | Per-target builds with `cargo build --target <T> --release`, then `lipo` for fat sim slices, then `xcodebuild -create-xcframework`. |
| 13 | Single-threaded per handle | Caller's responsibility to synchronize. `LAST_ERROR` is thread-local; interning tables use `OnceLock` (already thread-safe). No `Send`/`Sync` interlocks in the FFI crate. |

## Milestone summary

Each row is one PR. `Files` lists the focal files; full details in the per-milestone plan file.

| # | Title | Plan file | Files | Verification |
|---|-------|-----------|-------|--------------|
| M1 | Empty FFI crate compiles in workspace | [01](01-crate-and-handles.md) | workspace `Cargo.toml`; `adventerm_ffi/{Cargo.toml,build.rs,cbindgen.toml,src/lib.rs}` | `cargo build --workspace` green; `cargo build -p adventerm_ffi --target aarch64-apple-ios` green on macOS |
| M2 | Error infra + handle infra + first lifecycle exports | [01](01-crate-and-handles.md), [02](02-error-and-strings.md) | `error.rs`, `handle.rs`, `game.rs`, `tests/handle_lifecycle.rs`, `tests/error_codes.rs` | Tests pass; header builds clean under `clang -Wall -Wpedantic` |
| M3 | Enum + struct shims + scalar queries | [03](03-enums-and-structs.md), [04](04-actions-and-queries.md) | `enums.rs`, `structs.rs`, `query.rs`, `tests/enum_round_trip.rs` | Round-trip tests pass for every variant; Miri pass on the round-trip suite |
| M4 | Action dispatch shim | [04](04-actions-and-queries.md) | `action.rs`, extends `structs.rs`, `tests/action_dispatch.rs` | Parity tests pass for all nine actions vs. direct Rust dispatch |
| M5 | Iteration accessors | [05](05-iteration-and-save.md) | `iter.rs`, extends `structs.rs` | Integration test copies inventory + equipment + room contents and asserts equality with Rust reads |
| M6 | Save/load + filesystem helpers | [05](05-iteration-and-save.md) | `save.rs`, extends `handle.rs`, `tests/save_round_trip.rs` | Round-trip preserves equality (matches `save::tests::round_trip_preserves_state`) |
| M7 | Battle subsystem | [06](06-battle-and-console.md) | `battle.rs`, extends `handle.rs`, `tests/battle_flow.rs` | Scripted battle resolves to known outcome on seeded game |
| M8 | Dev console FFI | [06](06-battle-and-console.md) | `console.rs`, extends `handle.rs`, `tests/console_flow.rs` | Submit `fullbright` via FFI; assert `game_fullbright` flips |
| M9 | Header hardening + Linux C smoke test | [07](07-build-and-test.md) | `cbindgen.toml`, `include/adventerm_ffi.h`, `tests/smoke.c`, CI workflow | Smoke test exits 0; `git diff --exit-code include/` passes |
| M10 | iOS XCFramework script + agents/ doc | [07](07-build-and-test.md) | `scripts/build-xcframework.sh`, `agents/ffi.md`, `agents/README.md` | macOS CI builds all four iOS triples; XCFramework produced as artifact |

## Out of scope

- Swift wrapper library (Swift class hierarchy around the C handles). Tracked as a separate workstream once the FFI is stable.
- Android NDK sample app (build target supported in M9 CI matrix; no sample app).
- Concurrent access from multiple threads to the same handle (caller's responsibility).
- Live editing or "hot reload" of `adventerm_lib` from a host app.
- Exposing the event bus (`EventBus`, `ErasedEvent`, `Registry`) to FFI consumers — host apps observe outcomes via per-action return values, not by subscribing to events.

## Reused existing helpers

The FFI is mostly thin shims over existing facades. Per `agents/patterns.md`: reach for the existing helper before introducing new abstractions.

- `GameState::tile_at`, `terrain_at`, `is_visible`, `is_explored`, `player_on_door`, `peek_item_here`, `items_here` — already-existing read-only facades; FFI wraps 1:1.
- `dispatch::<A: Action>(...)` from `adventerm_lib::action` — invoked from inside each `game_action_*` shim.
- `Save::to_bytes` / `Save::from_bytes` — invoked by `save_to_bytes` / `save_from_bytes`.
- `start_battle`, `apply_player_ability`, `apply_enemy_turn` from `adventerm_lib::battle::engine` — invoked by `battle_*` FFI entries.
- `EntityId::from_raw(u32)` / `EntityId::raw() -> u32` — already the opaque-handle pattern; FFI passes `u32` directly.
- `ItemKind::name`, `from_display_name`; `EnemyKind::*`; `AbilityKind::name` — provide string-conversion; FFI wraps with `OnceLock<CString>` interning.
