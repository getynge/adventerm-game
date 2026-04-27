//! Systems that observe the dispatch event stream.
//!
//! After the open-registration migration, gameplay mutations live in
//! [`crate::actions`] (one struct per intent). Files here host only the
//! handler ZSTs and their `register(&mut Registry)` entry points — both
//! the `VisibilityHandler` (subscribes to `PlayerMoved` /
//! `DoorTraversed` / `ItemPlaced`) and the `EnemyTickHandler`
//! (subscribes to `PlayerMoved`).
//!
//! `visibility::refresh_visibility` and `enemy_tick::tick_current_room`
//! remain as the worker functions the handlers call; they're also used
//! during `GameState::new_seeded` and post-deserialize rehydration where
//! dispatch is not running.

pub mod enemy_tick;
pub mod visibility;

pub use enemy_tick::EnemyTickHandler;
pub use visibility::{refresh_visibility, VisibilityHandler};
