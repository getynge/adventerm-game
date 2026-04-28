//! Save/load surface and filesystem helpers.
//!
//! [`SaveHandle`] is an independent owner of an [`adventerm_lib::Save`];
//! creating, serializing, and restoring saves never borrows the parent
//! [`GameHandle`]. The bytes round-trip uses `Save::to_bytes` /
//! `Save::from_bytes`; the directory listing wraps `save::list_saves`.
//!
//! All string outputs use the caller-allocated buffer pattern from
//! [`crate::error`] (pattern 2 in `plans/ffi/02-error-and-strings.md`):
//! callers may pass `buf == null` with `out_required != null` to discover
//! the required size before allocating.

use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use adventerm_lib::save::{self, SAVE_VERSION};
use adventerm_lib::Save;

use crate::error::{cstr_to_str, set_last_error, FfiError};
use crate::ffi_try;
use crate::handle::{GameHandle, SaveHandle, SaveListing};

/// Copy `bytes` into a caller-allocated buffer using the two-call discovery
/// pattern. When `with_nul` is true, an extra terminating NUL is appended
/// (used for string outputs); when false the raw bytes are written verbatim
/// (used for serialized save payloads).
fn copy_into_buf(
    bytes: &[u8],
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
    with_nul: bool,
) -> i32 {
    let needed = bytes.len() + if with_nul { 1 } else { 0 };
    if !out_required.is_null() {
        unsafe {
            *out_required = needed;
        }
    }
    if buf.is_null() || cap < needed {
        return FfiError::BufferTooSmall as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
        if with_nul {
            *buf.add(bytes.len()) = 0;
        }
    }
    FfiError::Ok as i32
}

/// Resolve `*const SaveHandle` or short-circuit with `NullArgument`.
macro_rules! save_ref_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_ref() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        }
    };
}

/// Resolve `*const SaveListing` or short-circuit with `NullArgument`.
macro_rules! listing_ref_or_null {
    ($handle:expr) => {
        match unsafe { $handle.as_ref() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        }
    };
}

/// Bounds-check `index` for a listing-shaped accessor.
fn check_index(idx: usize, len: usize) -> Result<(), i32> {
    if idx >= len {
        set_last_error(format!("index {idx} >= len {len}"));
        Err(FfiError::OutOfRange as i32)
    } else {
        Ok(())
    }
}

// ---- Bytes round-trip -----------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn save_new_from_game(
    handle: *const GameHandle,
    name: *const c_char,
    out_save: *mut *mut SaveHandle,
) -> i32 {
    ffi_try!({
        if out_save.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe { *out_save = std::ptr::null_mut(); }
        let Some(h) = (unsafe { handle.as_ref() }) else {
            return FfiError::NullArgument as i32;
        };
        let name = match cstr_to_str(name) {
            Ok(s) => s.to_string(),
            Err(e) => return e as i32,
        };
        let save = Save::new(name, h.inner.clone());
        unsafe {
            *out_save = Box::into_raw(Box::new(SaveHandle { inner: save }));
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_to_bytes(
    save: *const SaveHandle,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let s = save_ref_or_null!(save);
        let bytes = s.inner.to_bytes();
        copy_into_buf(&bytes, buf, cap, out_required, false)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_from_bytes(
    bytes: *const u8,
    len: usize,
    out_save: *mut *mut SaveHandle,
) -> i32 {
    ffi_try!({
        if out_save.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe { *out_save = std::ptr::null_mut(); }
        if bytes.is_null() {
            return FfiError::NullArgument as i32;
        }
        let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
        let save = match Save::from_bytes(slice) {
            Ok(s) => s,
            Err(e) => return FfiError::from(e) as i32,
        };
        unsafe {
            *out_save = Box::into_raw(Box::new(SaveHandle { inner: save }));
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_to_game(
    save: *const SaveHandle,
    out_game: *mut *mut GameHandle,
) -> i32 {
    ffi_try!({
        if out_game.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe { *out_game = std::ptr::null_mut(); }
        let s = save_ref_or_null!(save);
        // Clone keeps the SaveHandle valid for further use after restoration.
        let game = GameHandle { inner: s.inner.state.clone() };
        unsafe {
            *out_game = Box::into_raw(Box::new(game));
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_name(
    save: *const SaveHandle,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let s = save_ref_or_null!(save);
        copy_into_buf(s.inner.name.as_bytes(), buf, cap, out_required, true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_version(
    save: *const SaveHandle,
    out_version: *mut u32,
) -> i32 {
    ffi_try!({
        let s = save_ref_or_null!(save);
        if out_version.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe { *out_version = s.inner.version; }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_free(save: *mut SaveHandle) {
    if !save.is_null() {
        unsafe { drop(Box::from_raw(save)); }
    }
}

// ---- Filesystem helpers ---------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn save_list_open(
    dir: *const c_char,
    out_listing: *mut *mut SaveListing,
) -> i32 {
    ffi_try!({
        if out_listing.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe { *out_listing = std::ptr::null_mut(); }
        let dir_str = match cstr_to_str(dir) {
            Ok(s) => s,
            Err(e) => return e as i32,
        };
        let slots = match save::list_saves(Path::new(dir_str)) {
            Ok(v) => v,
            Err(e) => return FfiError::from(e) as i32,
        };
        unsafe {
            *out_listing = Box::into_raw(Box::new(SaveListing { inner: slots }));
        }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_list_count(
    listing: *const SaveListing,
    out_count: *mut usize,
) -> i32 {
    ffi_try!({
        let l = listing_ref_or_null!(listing);
        if out_count.is_null() {
            return FfiError::NullArgument as i32;
        }
        unsafe { *out_count = l.inner.len(); }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_list_name(
    listing: *const SaveListing,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let l = listing_ref_or_null!(listing);
        if let Err(code) = check_index(index, l.inner.len()) {
            return code;
        }
        copy_into_buf(l.inner[index].name.as_bytes(), buf, cap, out_required, true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_list_path(
    listing: *const SaveListing,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let l = listing_ref_or_null!(listing);
        if let Err(code) = check_index(index, l.inner.len()) {
            return code;
        }
        let path = l.inner[index].path.to_string_lossy();
        copy_into_buf(path.as_bytes(), buf, cap, out_required, true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_list_modified_unix(
    listing: *const SaveListing,
    index: usize,
    out_unix_seconds: *mut i64,
) -> i32 {
    ffi_try!({
        let l = listing_ref_or_null!(listing);
        if out_unix_seconds.is_null() {
            return FfiError::NullArgument as i32;
        }
        if let Err(code) = check_index(index, l.inner.len()) {
            return code;
        }
        let modified = l.inner[index].modified;
        let seconds = match modified.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => -(e.duration().as_secs() as i64),
        };
        unsafe { *out_unix_seconds = seconds; }
        FfiError::Ok as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_list_free(listing: *mut SaveListing) {
    if !listing.is_null() {
        unsafe { drop(Box::from_raw(listing)); }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn save_delete(path: *const c_char) -> i32 {
    ffi_try!({
        let path_str = match cstr_to_str(path) {
            Ok(s) => s,
            Err(e) => return e as i32,
        };
        match save::delete_save(Path::new(path_str)) {
            Ok(()) => FfiError::Ok as i32,
            Err(e) => FfiError::from(e) as i32,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_slugify(
    name: *const c_char,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let name_str = match cstr_to_str(name) {
            Ok(s) => s,
            Err(e) => return e as i32,
        };
        let slug = save::slugify(name_str);
        copy_into_buf(slug.as_bytes(), buf, cap, out_required, true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_slot_path(
    dir: *const c_char,
    name: *const c_char,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32 {
    ffi_try!({
        let dir_str = match cstr_to_str(dir) {
            Ok(s) => s,
            Err(e) => return e as i32,
        };
        let name_str = match cstr_to_str(name) {
            Ok(s) => s,
            Err(e) => return e as i32,
        };
        let path: PathBuf = save::slot_path(Path::new(dir_str), name_str);
        let path_str = path.to_string_lossy();
        copy_into_buf(path_str.as_bytes(), buf, cap, out_required, true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_format_version() -> u32 {
    SAVE_VERSION
}
