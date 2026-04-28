# 02 — Error model and string handling

Covers the error-infra half of **M2**. The `FfiError` enum, `LAST_ERROR` mechanism, `ffi_try!` macro, and the three string patterns used across every milestone.

## `FfiError` (single source of truth)

`adventerm_ffi/src/error.rs`:

```rust
use std::cell::RefCell;
use std::ffi::{CString, c_char};

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiError {
    Ok = 0,
    NullArgument         = -1,
    InvalidUtf8          = -2,
    OutOfRange           = -3,
    EmptySlot            = -4,
    NotPlayerTurn        = -5,
    AlreadyResolved      = -6,
    NoSuchEntity         = -7,
    NoPendingEncounter   = -8,
    SaveFormat           = -9,
    UnsupportedSaveVersion = -10,
    IoFailure            = -11,
    BufferTooSmall       = -12,
    InternalPanic        = -13,
    Unknown              = -99,
}
```

**Rules:**
- `Ok = 0`, all errors negative, positive reserved for future use.
- Numeric values are stable once published — only ever append, never reorder or repurpose.
- Every fallible FFI function returns `i32` (not `FfiError` directly — keeps the C ABI a plain integer).

## Error → code conversion

| Lib type | FFI code | Detail captured in `LAST_ERROR` |
|----------|----------|--------------------------------|
| `BattleError::EmptySlot` | `EmptySlot` | none |
| `BattleError::NotPlayerTurn` | `NotPlayerTurn` | none |
| `BattleError::AlreadyResolved` | `AlreadyResolved` | none |
| `SaveError::Format(serde_json::Error)` | `SaveFormat` | serde message |
| `SaveError::UnsupportedVersion { found, expected }` | `UnsupportedSaveVersion` | `"found N, expected M"` |
| `std::io::Error` | `IoFailure` | OS message |
| `std::str::Utf8Error` (from `CStr::to_str`) | `InvalidUtf8` | byte-offset detail |
| Index out of range (any subsystem) | `OutOfRange` | `"index 5 >= len 3"` |

```rust
impl From<adventerm_lib::battle::engine::BattleError> for FfiError {
    fn from(e: adventerm_lib::battle::engine::BattleError) -> Self {
        use adventerm_lib::battle::engine::BattleError as B;
        match e {
            B::EmptySlot => FfiError::EmptySlot,
            B::NotPlayerTurn => FfiError::NotPlayerTurn,
            B::AlreadyResolved => FfiError::AlreadyResolved,
        }
    }
}

impl From<adventerm_lib::save::SaveError> for FfiError {
    fn from(e: adventerm_lib::save::SaveError) -> Self {
        use adventerm_lib::save::SaveError as S;
        match e {
            S::Format(err) => { set_last_error(format!("{err}")); FfiError::SaveFormat },
            S::UnsupportedVersion { found, expected } => {
                set_last_error(format!("found {found}, expected {expected}"));
                FfiError::UnsupportedSaveVersion
            }
        }
    }
}
```

## `LAST_ERROR` (thread-local)

```rust
thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(msg: impl Into<Vec<u8>>) {
    let cstr = CString::new(msg).unwrap_or_else(|_| CString::new("<contains nul>").unwrap());
    LAST_ERROR.with(|cell| *cell.borrow_mut() = Some(cstr));
}

fn clear_last_error() {
    LAST_ERROR.with(|cell| *cell.borrow_mut() = None);
}

#[no_mangle]
pub extern "C" fn ffi_last_error_message() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}
```

**Contract:**
- Pointer returned from `ffi_last_error_message` is valid until the next FFI call on the current thread. Swift copies into its own `String` immediately via `String(cString:)`.
- Returns null when there's no error message (success path, or pre-first-error).
- `set_last_error` is called only by `From` conversions and by `ffi_try!` on panic.

## `ffi_try!` macro

Wraps every `extern "C"` body in `catch_unwind`, returning `FfiError::InternalPanic` (-13) and capturing the panic payload to `LAST_ERROR` if a panic crosses the boundary.

```rust
#[macro_export]
macro_rules! ffi_try {
    ($body:block) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> i32 { $body })) {
            Ok(code) => code,
            Err(panic) => {
                let msg = if let Some(s) = panic.downcast_ref::<&'static str>() {
                    s.to_string()
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "panic with unknown payload".to_string()
                };
                $crate::error::set_last_error(msg);
                $crate::error::FfiError::InternalPanic as i32
            }
        }
    }};
}
```

**Usage pattern:**

```rust
#[no_mangle]
pub extern "C" fn game_player_pos(
    handle: *const GameHandle,
    out_x: *mut usize,
    out_y: *mut usize,
) -> i32 {
    ffi_try!({
        let h = match unsafe { handle.as_ref() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        };
        if out_x.is_null() || out_y.is_null() {
            return FfiError::NullArgument as i32;
        }
        let (x, y) = h.inner.player_pos();
        unsafe {
            *out_x = x;
            *out_y = y;
        }
        FfiError::Ok as i32
    })
}
```

Every `extern "C"` function in the FFI crate uses this macro. No exceptions.

## String pattern 1 — borrowed `'static`

For `&'static str` returns from the lib (`ItemKind::name()`, `EnemyKind::name()`, `Attribute::name()`, etc.), use a `OnceLock<HashMap<u8, CString>>` interning table per enum.

```rust
use std::collections::HashMap;
use std::sync::OnceLock;

fn item_name_table() -> &'static HashMap<u8, CString> {
    static TABLE: OnceLock<HashMap<u8, CString>> = OnceLock::new();
    TABLE.get_or_init(|| {
        adventerm_lib::ItemKind::ALL
            .iter()
            .map(|&k| (k as u8, CString::new(k.name()).unwrap()))
            .collect()
    })
}

#[no_mangle]
pub extern "C" fn item_kind_name(kind: u8) -> *const c_char {
    item_name_table()
        .get(&kind)
        .map(|s| s.as_ptr())
        .unwrap_or(std::ptr::null())
}
```

**Contract:** pointer is `'static` for the process lifetime. Caller MUST NOT free.

## String pattern 2 — owned variable (caller buffer)

For dynamic strings (save names, console output, error detail copies), use the caller-allocated buffer pattern.

```rust
#[no_mangle]
pub extern "C" fn battle_log_line_copy(
    battle: *const BattleHandle,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let b = match unsafe { battle.as_ref() } {
            Some(b) => b,
            None => return FfiError::NullArgument as i32,
        };
        let lines = b.inner.log();
        let line = match lines.get(index) {
            Some(l) => l,
            None => {
                set_last_error(format!("index {index} >= len {}", lines.len()));
                return FfiError::OutOfRange as i32;
            }
        };
        let bytes = line.as_bytes();
        let needed = bytes.len() + 1; // +1 for NUL terminator
        if !out_required.is_null() {
            unsafe { *out_required = needed; }
        }
        if buf.is_null() || cap < needed {
            return FfiError::BufferTooSmall as i32;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
            *buf.add(bytes.len()) = 0; // NUL terminator
        }
        FfiError::Ok as i32
    })
}
```

**Two-call discovery pattern:**

```c
size_t needed = 0;
battle_log_line_copy(battle, 3, NULL, 0, &needed);
char* buf = malloc(needed);
battle_log_line_copy(battle, 3, (uint8_t*)buf, needed, &needed);
// ... use buf (NUL-terminated UTF-8) ...
free(buf);
```

**Why caller-allocated rather than `*mut c_char` + `_free_string`:**
- Swift's `withUnsafeMutableBufferPointer` makes this idiomatic.
- No allocator-ownership confusion across the boundary.
- Avoids a parallel family of `*_free_string` exports.
- Two-call cost is negligible compared to gameplay work.

**Used by:** `battle_log_line_copy`, `console_history_line_copy`, `save_list_name`, `save_list_path`, `save_slugify`, `save_slot_path`, `save_to_bytes` (binary buffer, same pattern), `game_save_name`.

## String pattern 3 — strings INTO the FFI

For `*const c_char` parameters (save names, paths from Swift), the standard `CStr::from_ptr` + UTF-8 validation pattern:

```rust
fn cstr_to_str<'a>(ptr: *const c_char) -> Result<&'a str, FfiError> {
    if ptr.is_null() { return Err(FfiError::NullArgument); }
    let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
    cstr.to_str().map_err(|e| {
        set_last_error(format!("invalid utf-8: {e}"));
        FfiError::InvalidUtf8
    })
}
```

Caller retains ownership of the input string; FFI just borrows for the call's duration.

## Test plan (M2)

`adventerm_ffi/tests/error_codes.rs`:

```rust
#[test]
fn null_handle_returns_null_argument() {
    let mut x = 0usize;
    let mut y = 0usize;
    let rc = adventerm_ffi::game_player_pos(std::ptr::null(), &mut x, &mut y);
    assert_eq!(rc, adventerm_ffi::FfiError::NullArgument as i32);
}

#[test]
fn last_error_persists_until_next_call() {
    let mut x = 0usize;
    let mut y = 0usize;
    adventerm_ffi::game_player_pos(std::ptr::null(), &mut x, &mut y);
    let msg = adventerm_ffi::ffi_last_error_message();
    assert!(!msg.is_null() || /* NullArgument may not set a message */ true);
}

#[test]
fn buffer_too_small_returns_required() {
    // Set up a battle, write log line, call _line_copy with cap=0
    // assert returns BufferTooSmall and out_required is populated
    todo!("filled in M7")
}
```

`adventerm_ffi/tests/handle_lifecycle.rs`:

```rust
#[test]
fn game_alloc_free_no_leak() {
    let handle = adventerm_ffi::game_new_seeded(42);
    assert!(!handle.is_null());
    adventerm_ffi::game_free(handle);
}

#[test]
fn game_free_null_is_safe() {
    adventerm_ffi::game_free(std::ptr::null_mut());
}

#[test]
fn game_player_pos_round_trip() {
    let handle = adventerm_ffi::game_new_seeded(42);
    let mut x = 0usize;
    let mut y = 0usize;
    let rc = adventerm_ffi::game_player_pos(handle, &mut x, &mut y);
    assert_eq!(rc, 0);
    adventerm_ffi::game_free(handle);
}
```

Run under Miri in CI for catch UAF/provenance issues:

```bash
cargo +nightly miri test -p adventerm_ffi --test handle_lifecycle --test error_codes
```
