# 01 — Crate skeleton and handle infrastructure

Covers **M1** (workspace + empty crate compiles) and the handle half of **M2** (lifecycle exports).

## Workspace change

Edit [Cargo.toml](../../Cargo.toml) — add `"adventerm_ffi"` to `members`. Currently:

```toml
[workspace]
resolver = "3"
members = ["adventerm", "adventerm_lib"]
```

Becomes:

```toml
members = ["adventerm", "adventerm_lib", "adventerm_ffi"]
```

`[profile.release]` is left alone — its current `lto = true`, `opt-level = "s"`, `codegen-units = 1`, `strip = true` settings are correct for the FFI dylib too.

## `adventerm_ffi/Cargo.toml`

```toml
[package]
name = "adventerm_ffi"
version = "0.1.0"
edition = "2024"

[lib]
name = "adventerm_ffi"
crate-type = ["staticlib", "cdylib", "rlib"]

[dependencies]
adventerm_lib = { path = "../adventerm_lib" }

[build-dependencies]
cbindgen = "0.27"
```

**Crate-type rationale:**
- `staticlib` — iOS XCFrameworks consume `.a` archives.
- `cdylib` — Android (`.so`), desktop Linux/macOS (`.dylib`/`.so`), Windows (`.dll`), the C smoke test.
- `rlib` — lets `adventerm_ffi/tests/*.rs` link the FFI surface as a normal Rust dependency without going through the C ABI for setup.

## `adventerm_ffi/build.rs`

```rust
fn main() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let config = cbindgen::Config::from_file(format!("{crate_dir}/cbindgen.toml")).unwrap();
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("cbindgen failed")
        .write_to_file(format!("{crate_dir}/include/adventerm_ffi.h"));
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
}
```

(Full `cbindgen.toml` lives in [07-build-and-test.md](07-build-and-test.md). M1 ships with a minimal config sufficient to produce an empty header.)

## `adventerm_ffi/src/lib.rs`

Module wiring only. No logic in `lib.rs` itself — every export lives in a topic module.

```rust
//! adventerm_ffi: C-compatible FFI for adventerm_lib.
//!
//! See plans/ffi/00-overview.md for the design rationale and rules.

mod error;
mod handle;
mod enums;
mod structs;
mod game;
mod query;
mod action;
mod iter;
mod save;
mod battle;
mod console;

pub use error::*;
pub use handle::*;
pub use enums::*;
pub use structs::*;
pub use game::*;
pub use query::*;
pub use action::*;
pub use iter::*;
pub use save::*;
pub use battle::*;
pub use console::*;
```

For M1, all submodules are empty stubs (`// placeholder`) so the crate compiles. Each milestone adds real content to the relevant submodules.

## Handle definitions

All handles live in `adventerm_ffi/src/handle.rs`. Each is a tuple-newtype around the inner Rust type, `Box`-allocated, and exposed to C as a forward-declared opaque struct.

```rust
use std::os::raw::c_void;
use adventerm_lib::{GameState, Battle, Save, save::SaveSlot};
use crate::error::FfiError;

/// Opaque root handle; owns a GameState.
#[repr(C)]
pub struct GameHandle { pub(crate) inner: GameState }

/// Independent owner of a Battle (does NOT borrow from GameHandle).
#[repr(C)]
pub struct BattleHandle { pub(crate) inner: Battle }

/// Independent owner of a Save.
#[repr(C)]
pub struct SaveHandle { pub(crate) inner: Save }

/// Listing of saves in a directory; owns a Vec<SaveSlot>.
#[repr(C)]
pub struct SaveListing { pub(crate) inner: Vec<SaveSlot> }

/// Independent owner of a ConsoleState.
#[repr(C)]
pub struct ConsoleHandle { pub(crate) inner: adventerm_lib::console::ConsoleState }
```

The `#[repr(C)]` here is *only* for ABI stability of the wrapper struct — the inner Rust types remain untouched. cbindgen forward-declares each as `typedef struct GameHandle GameHandle;` (no fields) since we never expose internals.

## Alloc / free discipline

For each handle, exactly one alloc function returns `Box::into_raw(Box::new(...))`, paired with a free function that does `Box::from_raw(...)`. Free is null-safe (matches C's `free(NULL)` convention).

```rust
#[no_mangle]
pub extern "C" fn game_new_seeded(seed: u64) -> *mut GameHandle {
    Box::into_raw(Box::new(GameHandle { inner: GameState::new_seeded(seed) }))
}

#[no_mangle]
pub extern "C" fn game_free(handle: *mut GameHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)); }
    }
}
```

**Rules:**
- `game_new_seeded` is the only constructor that doesn't take an `out_*` pointer (it can't fail). Every other handle constructor takes `out: *mut *mut XxxHandle` and returns `i32` (see `02-error-and-strings.md`).
- Sub-handles (`Battle`, `Save`, `Console`) are independent owners. `BattleHandle` does NOT borrow from `GameHandle` — it clones the snapshot it needs from `GameState` at construction. This eliminates the parent/child lifetime contract.
- A sub-handle freed before or after its "parent" is fine; freeing is a no-op on null.
- No `*_clone` functions. If a host needs a copy, it serializes via `save_*` and deserializes.

## First lifecycle exports (M2 deliverable)

Land in `adventerm_ffi/src/game.rs`:

```rust
#[no_mangle]
pub extern "C" fn game_new_seeded(seed: u64) -> *mut GameHandle;

#[no_mangle]
pub extern "C" fn game_free(handle: *mut GameHandle);

#[no_mangle]
pub extern "C" fn game_player_pos(
    handle: *const GameHandle,
    out_x: *mut usize,
    out_y: *mut usize,
) -> i32;  // 0 = Ok; -1 = NullArgument

#[no_mangle]
pub extern "C" fn game_cur_health(
    handle: *const GameHandle,
    out_hp: *mut u8,
) -> i32;

#[no_mangle]
pub extern "C" fn game_set_cur_health(
    handle: *mut GameHandle,
    hp: u8,
) -> i32;

#[no_mangle]
pub extern "C" fn game_vision_radius(
    handle: *const GameHandle,
    out_radius: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn game_refresh_visibility(handle: *mut GameHandle) -> i32;
```

Each function's body uses the `ffi_try!` macro from `error.rs`, which wraps in `catch_unwind` and writes any error detail to `LAST_ERROR`. See [02-error-and-strings.md](02-error-and-strings.md) for the macro definition.

## Null-pointer policy

Every entry point that takes a handle null-checks at the top:

```rust
let handle = match unsafe { handle.as_ref() } {
    Some(h) => h,
    None => return FfiError::NullArgument as i32,
};
```

`*_free` functions silently no-op on null (C convention). All other functions return `FfiError::NullArgument` (-1) when given a null handle.

## Threading contract

Documented in `agents/ffi.md` (M10) and the cbindgen-generated header preamble:

> Handles may be moved between threads, but a single handle must only be accessed by one thread at a time. Concurrent access from multiple threads is undefined behavior — the consumer is responsible for synchronization.
>
> `ffi_last_error_message()` returns thread-local data; it's safe to call from any thread but only reflects errors that occurred on the calling thread.
>
> Internal statics (`OnceLock<CString>` interning tables) are thread-safe; concurrent calls from different threads to functions that read them (e.g., `item_kind_name`) are sound.

No `Send`/`Sync` interlocks in the FFI crate. `LAST_ERROR` is `thread_local!`. Interning tables are `OnceLock`-initialized.

## M1 verification

```bash
cargo build --workspace
# On macOS:
rustup target add aarch64-apple-ios
cargo build -p adventerm_ffi --target aarch64-apple-ios
```

Both must succeed. The header at `adventerm_ffi/include/adventerm_ffi.h` is created (likely near-empty after M1 since no exports yet).

## M2 verification

```bash
cargo test -p adventerm_ffi --test handle_lifecycle
cargo test -p adventerm_ffi --test error_codes
clang -Wall -Wpedantic -fsyntax-only -Iadventerm_ffi/include adventerm_ffi/include/adventerm_ffi.h
```

All three must succeed. Tests cover:
- `game_new_seeded` returns non-null; `game_free(NULL)` is a no-op; double-free protection (see test docs — we don't actually prevent double-free; we test that `_free(NULL)` is safe).
- `game_player_pos(NULL, ...)` returns `-1` (`NullArgument`).
- `game_cur_health` round-trips via `set_cur_health`.
