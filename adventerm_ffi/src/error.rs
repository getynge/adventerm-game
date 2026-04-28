//! Error model for the FFI surface.
//!
//! Every fallible `extern "C"` function returns an `i32` whose value is one
//! of the [`FfiError`] discriminants. Detail strings (`serde_json` messages,
//! out-of-range descriptions, panic payloads) are stashed in the thread-local
//! [`LAST_ERROR`] and retrieved via [`ffi_last_error_message`].

use std::cell::RefCell;
use std::ffi::{c_char, CString};

/// Stable wire codes for FFI return values.
///
/// `Ok` is `0`, every error is negative, and positive values are reserved.
/// The discriminants are part of the public ABI: only ever append, never
/// reorder or repurpose.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiError {
    Ok = 0,
    NullArgument = -1,
    InvalidUtf8 = -2,
    OutOfRange = -3,
    EmptySlot = -4,
    NotPlayerTurn = -5,
    AlreadyResolved = -6,
    NoSuchEntity = -7,
    NoPendingEncounter = -8,
    SaveFormat = -9,
    UnsupportedSaveVersion = -10,
    IoFailure = -11,
    BufferTooSmall = -12,
    InternalPanic = -13,
    Unknown = -99,
}

thread_local! {
    /// Last detail message produced on this thread. Cleared by callers that
    /// want to start a fresh "error scope"; populated by `From` conversions
    /// and by [`ffi_try!`] on panic.
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Stash a detail message for the next `ffi_last_error_message()` call.
///
/// `set_last_error` is `pub(crate)` so other FFI modules (and the
/// [`ffi_try!`] macro) can record context without exposing the storage.
pub(crate) fn set_last_error(msg: impl Into<Vec<u8>>) {
    let cstr =
        CString::new(msg).unwrap_or_else(|_| CString::new("<contains nul>").unwrap());
    LAST_ERROR.with(|cell| *cell.borrow_mut() = Some(cstr));
}

/// Drop any prior detail message on the current thread.
#[allow(dead_code)] // Used by later milestones; kept here so `error.rs` owns the API.
pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|cell| *cell.borrow_mut() = None);
}

/// Borrow a UTF-8 string from a caller-owned `*const c_char`.
///
/// Returns [`FfiError::NullArgument`] for null pointers and
/// [`FfiError::InvalidUtf8`] (with byte-offset detail in [`LAST_ERROR`])
/// for non-UTF-8 input. The returned slice borrows for the call's duration.
pub(crate) fn cstr_to_str<'a>(ptr: *const c_char) -> Result<&'a str, FfiError> {
    if ptr.is_null() {
        return Err(FfiError::NullArgument);
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
    cstr.to_str().map_err(|e| {
        set_last_error(format!("invalid utf-8: {e}"));
        FfiError::InvalidUtf8
    })
}

/// Pointer to the current thread's last error detail, or null when none is
/// set. The pointer is valid until the next FFI call on this thread; callers
/// that need the message past that point must copy it immediately.
#[unsafe(no_mangle)]
pub extern "C" fn ffi_last_error_message() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

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

impl From<std::io::Error> for FfiError {
    fn from(e: std::io::Error) -> Self {
        set_last_error(format!("{e}"));
        FfiError::IoFailure
    }
}

impl From<adventerm_lib::save::SaveError> for FfiError {
    fn from(e: adventerm_lib::save::SaveError) -> Self {
        use adventerm_lib::save::SaveError as S;
        match e {
            S::Format(err) => {
                set_last_error(format!("{err}"));
                FfiError::SaveFormat
            }
            S::UnsupportedVersion { found, expected } => {
                set_last_error(format!("found {found}, expected {expected}"));
                FfiError::UnsupportedSaveVersion
            }
        }
    }
}

/// Wrap an `extern "C"` body in `catch_unwind`, converting any panic into
/// [`FfiError::InternalPanic`] and recording the payload to [`LAST_ERROR`].
///
/// Every fallible FFI export uses this macro — no exceptions.
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
