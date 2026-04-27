//! Open-set action substrate.
//!
//! An [`Action`] is a typed value representing "an entity wants to do X".
//! The binary builds a concrete action and hands it to [`dispatch`], which
//! invokes the action's `perform`, then drains every event the action
//! emitted through the global handler registry. Handlers may emit further
//! events; the loop runs until the bus is empty.
//!
//! Action types live in their own modules and need no central enumeration.

use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::game::GameState;
use crate::registry::registry;

/// Concrete action invokable on a specific actor entity. The action body
/// performs the canonical mutation directly and emits any number of events
/// into `bus`; subscribers in the [`registry`] react to those events
/// asynchronously after `perform` returns.
pub trait Action: Send + Sync + 'static {
    type Outcome;
    fn perform(self, game: &mut GameState, actor: EntityId, bus: &mut EventBus) -> Self::Outcome;
}

/// Run `action` performed by `actor`, then drain any events it produced.
/// Returns `action`'s outcome verbatim — handlers cannot rewrite it. If a
/// handler needs to surface information back to the caller, it should
/// stash it on `GameState` and the caller should read it after dispatch.
pub fn dispatch<A: Action>(game: &mut GameState, actor: EntityId, action: A) -> A::Outcome {
    let mut bus = EventBus::default();
    let outcome = action.perform(game, actor, &mut bus);
    drain_bus(game, &mut bus);
    outcome
}

/// Drain every event currently in `bus` through the global registry.
/// Exposed crate-internal so multi-step actions (e.g. `QuickMoveAction`)
/// can settle their handlers between steps without finishing dispatch.
pub(crate) fn drain_bus(game: &mut GameState, bus: &mut EventBus) {
    let reg = registry();
    while let Some(event) = bus.pop() {
        reg.dispatch_event(game, event.as_ref(), bus);
    }
}
