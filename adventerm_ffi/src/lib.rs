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
