//! adventerm_ffi: C-compatible FFI for adventerm_lib.
//!
//! See plans/ffi/00-overview.md for the design rationale and rules.

mod action;
mod battle;
mod console;
mod enums;
mod error;
mod game;
mod handle;
mod iter;
mod query;
mod save;
mod structs;

pub use action::*;
pub use battle::*;
pub use console::*;
pub use enums::*;
pub use error::*;
pub use game::*;
pub use handle::*;
pub use iter::*;
pub use query::*;
pub use save::*;
pub use structs::*;
