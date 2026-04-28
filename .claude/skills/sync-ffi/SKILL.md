---
name: sync-ffi
description: Update and test adventerm_ffi after changes to adventerm_lib's public API. Use when adventerm_lib gains/renames/removes a public type, function, field, or enum variant, when CI fails on the FFI header diff, or when the user says "sync the FFI", "update the FFI", "regenerate the FFI header", or asks whether FFI updates are needed after a lib change.
user-invocable: true
allowed-tools:
  - Read
  - Edit
  - Write
  - Bash(cargo build*)
  - Bash(cargo test*)
  - Bash(cargo check*)
  - Bash(git diff*)
  - Bash(git status*)
  - Bash(git log*)
  - Bash(git show*)
  - Bash(rg *)
  - Bash(clang *)
  - Bash(ls *)
  - Bash(cc *)
  - Bash(wc *)
  - Bash(rg *)
---

# sync-ffi — Mirror lib changes into adventerm_ffi

`adventerm_ffi` is a second consumer of `adventerm_lib`'s public surface (alongside the `adventerm` binary). When the lib's public API changes, the FFI must be updated **in the same change** and the regenerated header committed. CI fails the PR if `git diff --exit-code adventerm_ffi/include/` is non-empty after `cargo build -p adventerm_ffi`.

This skill walks the sync end-to-end: detect what changed in the lib, apply the mirror updates in the FFI crate, regenerate the header, and verify with build + tests + standalone clang syntax check.

Read [agents/ffi.md](../../../agents/ffi.md) before adding a new export, handle type, or error variant — it is the source of truth for handle conventions, error codes, and stability rules.

---

## 1. Identify the lib delta

Find the public-API changes that need mirroring. Use whichever applies:

- `git diff main -- adventerm_lib/src/` — for branch work
- `git diff HEAD~ -- adventerm_lib/src/` — for the most recent commit
- `git status -- adventerm_lib/src/` — for uncommitted work
- A specific commit / range the user names

Look for changes to anything `pub`:

- New / renamed / removed `pub` types, functions, fields, methods.
- New / renamed / removed enum variants (discriminants are wire-stable — see § 4).
- Changes to facade methods on `Room` / `GameState` (the binary-visible API).
- `SAVE_VERSION` bumps in [adventerm_lib/src/save.rs](../../../adventerm_lib/src/save.rs).

Internal-only changes (private fns, refactors that don't touch `pub` items) need no FFI work — confirm and stop.

## 2. Map each change to its FFI module

| Lib change | FFI file to touch |
| --- | --- |
| New / changed `pub enum` (kinds, statuses, tags) | [adventerm_ffi/src/enums.rs](../../../adventerm_ffi/src/enums.rs) — append a `CXxx` mirror variant; update the `From` / `Into` conversion |
| New / changed `pub struct` returned across the boundary | [adventerm_ffi/src/structs.rs](../../../adventerm_ffi/src/structs.rs) — `#[repr(C)]` mirror; conversion helper |
| New game-state query (read-only facade) | [adventerm_ffi/src/query.rs](../../../adventerm_ffi/src/query.rs) |
| New player action / dispatch path | [adventerm_ffi/src/action.rs](../../../adventerm_ffi/src/action.rs) |
| New iteration over a collection (count + index pattern) | [adventerm_ffi/src/iter.rs](../../../adventerm_ffi/src/iter.rs) |
| Save format / `SAVE_VERSION` change | [adventerm_ffi/src/save.rs](../../../adventerm_ffi/src/save.rs) — `UnsupportedSaveVersion` already propagates; update detail formatting if needed |
| Battle flow change | [adventerm_ffi/src/battle.rs](../../../adventerm_ffi/src/battle.rs) |
| Dev-console command surface change | [adventerm_ffi/src/console.rs](../../../adventerm_ffi/src/console.rs) |
| New handle type | [adventerm_ffi/src/handle.rs](../../../adventerm_ffi/src/handle.rs) — opaque newtype, no `#[repr(C)]`; matching `*_new` / `*_free` (free is null-safe) |
| New error condition | [adventerm_ffi/src/error.rs](../../../adventerm_ffi/src/error.rs) — append a discriminant; never reorder |

For each new fallible export: wrap the body in `ffi_try!` (no exceptions), return `i32`, populate `LAST_ERROR` for caller-readable detail. Strings use the two-call buffer pattern (pass `buf == NULL` with `out_required` to discover size, then re-call). See [adventerm_ffi/src/save.rs](../../../adventerm_ffi/src/save.rs) `copy_into_buf` for the shared helper.

## 3. Hard rules — do not violate

These are restated from [agents/ffi.md](../../../agents/ffi.md) § 7–8 and the FFI [CLAUDE.md](../../../adventerm_ffi/CLAUDE.md). Stop and ask the user if a change appears to require breaking any of them.

- **No `#[repr(C)]` on lib types, no `extern "C"` in lib code, no `*mut` / `*const` in lib signatures.** All C-ABI shaping happens in `adventerm_ffi`.
- **Trait objects don't cross.** `&dyn ItemBehavior`, `&dyn ActiveAbility`, `&dyn DevCommand`, `&dyn EnemyAi` stay lib-side. Hosts see outcomes, not behavior.
- **Enum discriminants are append-only.** Reordering or repurposing an existing discriminant of `FfiError`, any `CXxx` mirror in `enums.rs`, or any tagged-struct `tag` field in `structs.rs` is a hard ABI break.
- **No interior pointers.** Never return a pointer that aliases a handle's interior. Strings copy out; small structs return by value.
- **One `*_new` ↔ one `*_free`.** `*_free(NULL)` is a no-op. Sub-handles are independent owners (no parent / child borrows).

## 4. Apply the updates

Edit the FFI files identified in § 2. When mirroring an enum:

1. Append the new `CXxx` variant **at the end** of the mirror enum in `enums.rs`. Match the variant's documented intent — name and discriminant ordering follow the lib enum's append order, never repurposing an old slot.
2. Add the `From<LibEnum> for CXxx` arm (and reverse if the FFI converts back).
3. If the variant carries data, decide the wire shape (tagged struct in `structs.rs`, or a separate query export).

When mirroring a struct: `#[repr(C)]`, primitive / `Copy` fields only, conversion helper alongside.

## 5. Build → regenerate header → diff check

The header is generated by [adventerm_ffi/build.rs](../../../adventerm_ffi/build.rs) on every `cargo build -p adventerm_ffi` and written verbatim to [adventerm_ffi/include/adventerm_ffi.h](../../../adventerm_ffi/include/adventerm_ffi.h). The committed header must match a fresh build.

Run from the workspace root:

```bash
cargo build -p adventerm_ffi
git diff --stat adventerm_ffi/include/
```

If the header changed, the regenerated file goes in the same commit as the source change. **Do not** hand-edit `adventerm_ffi.h` — re-run the build instead.

## 6. Standalone header syntax check

```bash
clang -Wall -Wpedantic -fsyntax-only -x c \
    -Iadventerm_ffi/include adventerm_ffi/include/adventerm_ffi.h
```

Catches malformed cbindgen output (forward-decl ordering, missing `typedef`, etc.) before a downstream Swift / C consumer hits it.

## 7. Test

```bash
cargo test -p adventerm_ffi
```

The integration tests in [adventerm_ffi/tests/](../../../adventerm_ffi/tests/) link against the `rlib` crate-type and exercise:

- [handle_lifecycle.rs](../../../adventerm_ffi/tests/handle_lifecycle.rs) — alloc / free pairing, `*_free(NULL)` no-op.
- [enum_round_trip.rs](../../../adventerm_ffi/tests/enum_round_trip.rs) — mirror enum ↔ lib enum conversions. **Add a case here when you append a variant in § 4.**
- [error_codes.rs](../../../adventerm_ffi/tests/error_codes.rs) — fallible exports return the documented discriminant; `LAST_ERROR` populated.
- [action_dispatch.rs](../../../adventerm_ffi/tests/action_dispatch.rs), [queries.rs](../../../adventerm_ffi/tests/queries.rs), [iteration.rs](../../../adventerm_ffi/tests/iteration.rs), [save_round_trip.rs](../../../adventerm_ffi/tests/save_round_trip.rs), [battle_flow.rs](../../../adventerm_ffi/tests/battle_flow.rs), [console_flow.rs](../../../adventerm_ffi/tests/console_flow.rs) — per-module behavior.
- [smoke.c](../../../adventerm_ffi/tests/smoke.c) — only consumer that crosses the actual C ABI in CI.

If a new export was added in § 4, add a focused test under the matching file (or a new file) before declaring done.

## 8. Final report

Summarize for the user:

- Lib symbols that changed (1 line each).
- FFI files edited.
- Whether the header diff is non-empty (and committed).
- `cargo test -p adventerm_ffi` result.
- Anything that violated § 3 and needs a design discussion (do not silently break ABI).

Mention `SAVE_VERSION` bumps explicitly — they invalidate older saves identically through the FFI.
