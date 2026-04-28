//! Opaque handle newtypes that own the lib-side game objects.
//!
//! Each handle is a Rust-only wrapper around the inner lib type. Callers
//! only ever see `*mut <Handle>` across the FFI boundary, so cbindgen
//! emits these as opaque forward declarations in the generated header.
//! The `#[repr(C)]` attribute is intentionally omitted: the C side never
//! introspects layout, and adding `#[repr(C)]` would force cbindgen to
//! emit the inner field — which references library types it cannot see
//! (`parse.parse_deps = false`), producing invalid C.
//!
//! Alloc/free pairs live in the topic module that owns the construction
//! sequence (e.g. [`crate::game`] for [`GameHandle`]).

use adventerm_lib::console::ConsoleState;
use adventerm_lib::save::SaveSlot;
use adventerm_lib::{Battle, GameState, Save};

/// Root handle: owns a [`GameState`].
pub struct GameHandle {
    pub(crate) inner: GameState,
}

/// Independent owner of a [`Battle`]. Does not borrow from a [`GameHandle`];
/// the snapshot it carries is cloned at construction.
pub struct BattleHandle {
    pub(crate) inner: Battle,
}

/// Independent owner of a [`Save`].
pub struct SaveHandle {
    pub(crate) inner: Save,
}

/// Listing of saves in a directory.
pub struct SaveListing {
    pub(crate) inner: Vec<SaveSlot>,
}

/// Independent owner of a [`ConsoleState`].
///
/// `inner` mirrors what the binary owns: input buffer, cursor, completion
/// view. `history` is FFI-side because the lib's history is private (no
/// public accessor); we maintain our own log of submitted prompts so the
/// `console_history_*` exports can satisfy Pattern C.
pub struct ConsoleHandle {
    pub(crate) inner: ConsoleState,
    pub(crate) history: Vec<String>,
}
