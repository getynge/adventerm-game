//! Typed gameplay events.
//!
//! Each event lives in its own submodule. Subscribers are registered
//! against the concrete type via [`crate::registry::Registry::subscribe`];
//! the dispatcher routes by `TypeId`, so adding an event needs no
//! central enum and no central match.

pub mod door_traversed;
pub mod enemy_defeated;
pub mod enemy_engaged;
pub mod enemy_moved;
pub mod flare_burned_out;
pub mod item_consumed;
pub mod item_equipped;
pub mod item_picked_up;
pub mod item_placed;
pub mod item_unequipped;
pub mod player_moved;

pub use door_traversed::DoorTraversed;
pub use enemy_defeated::EnemyDefeated;
pub use enemy_engaged::EnemyEngaged;
pub use enemy_moved::EnemyMoved;
pub use flare_burned_out::FlareBurnedOut;
pub use item_consumed::ItemConsumed;
pub use item_equipped::ItemEquipped;
pub use item_picked_up::ItemPickedUp;
pub use item_placed::ItemPlaced;
pub use item_unequipped::ItemUnequipped;
pub use player_moved::PlayerMoved;
