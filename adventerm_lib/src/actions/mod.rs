//! Concrete [`Action`] types performed by the player. Each variant lives
//! in its own module so adding a new action means dropping a new file in
//! and adding one `register_action::<NewAction>` call in
//! [`crate::player::register`] — nothing else changes.
//!
//! Actions own the canonical mutation of game state for their intent and
//! emit zero or more typed events into the [`crate::event::EventBus`] so
//! registered handlers (visibility, enemy AI, …) can react during the
//! same dispatch.
//!
//! [`Action`]: crate::action::Action

pub mod combat_action;
pub mod consume_action;
pub mod equip_action;
pub mod interact_action;
pub mod move_action;
pub mod pickup_action;
pub mod place_action;
pub mod quick_move_action;
pub mod unequip_action;

pub use combat_action::DefeatEnemyAction;
pub use consume_action::ConsumeItemAction;
pub use equip_action::EquipItemAction;
pub use interact_action::InteractAction;
pub use move_action::MoveAction;
pub use pickup_action::PickUpAction;
pub use place_action::PlaceItemAction;
pub use quick_move_action::QuickMoveAction;
pub use unequip_action::UnequipItemAction;

#[cfg(test)]
mod tests;
