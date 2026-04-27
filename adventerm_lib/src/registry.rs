//! Open-set handler registry.
//!
//! Modules register their handlers (and, optionally, the actions an entity
//! kind can perform) at boot via free `register(&mut Registry)` functions.
//! Once built, the registry is consulted by `dispatch` to find the handlers
//! that subscribe to a given concrete event type. The registry has no
//! knowledge of what those events or handlers *do* — it only routes by
//! `TypeId`.
//!
//! There is one process-wide singleton initialized lazily by [`registry()`].
//! Tests that need a fresh, isolated registry can call [`build_registry`]
//! directly without touching the singleton.

use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::OnceLock;

use crate::event::{ErasedEvent, Event, EventBus};
use crate::game::GameState;

/// Identifies the kind of actor an action can be invoked on. Action types
/// are looked up here for introspection only — `dispatch` itself does not
/// consult this map. Keeping it lightweight means new actor categories
/// (enemies, traps, etc.) can be added without disturbing existing code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorKind {
    Player,
    Enemy,
}

/// Trait every concrete event handler implements. The registry stores
/// type-erased adapters around these so it can call them by `TypeId`.
pub trait EventHandler<E: Event>: Send + Sync + 'static {
    fn handle(&self, game: &mut GameState, event: &E, bus: &mut EventBus);
}

/// Object-safe handler shim used inside the registry's storage.
pub trait ErasedHandler: Send + Sync {
    fn handle(&self, game: &mut GameState, event: &dyn ErasedEvent, bus: &mut EventBus);
}

struct HandlerAdapter<E: Event, H: EventHandler<E>> {
    handler: H,
    _marker: PhantomData<fn(E)>,
}

impl<E: Event, H: EventHandler<E>> ErasedHandler for HandlerAdapter<E, H> {
    fn handle(&self, game: &mut GameState, event: &dyn ErasedEvent, bus: &mut EventBus) {
        let typed = event
            .as_any()
            .downcast_ref::<E>()
            .expect("registry routed event whose TypeId did not match handler's E");
        self.handler.handle(game, typed, bus);
    }
}

/// Pair of static registries:
/// - `handlers`: subscriber lists keyed by emitted-event `TypeId`.
/// - `actions`: which `ActorKind` is allowed to invoke each action `TypeId`.
///   Stored for introspection / future validation; not consulted by dispatch.
#[derive(Default)]
pub struct Registry {
    handlers: HashMap<TypeId, Vec<Box<dyn ErasedHandler>>>,
    actions: HashMap<TypeId, ActorKind>,
}

impl Registry {
    /// Subscribe `handler` to every event of type `E`. Handlers are invoked
    /// in subscription order during `dispatch_event`.
    pub fn subscribe<E, H>(&mut self, handler: H)
    where
        E: Event,
        H: EventHandler<E>,
    {
        self.handlers
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Box::new(HandlerAdapter::<E, H> {
                handler,
                _marker: PhantomData,
            }));
    }

    /// Record that `actor_kind` may invoke action type `A`. This is a
    /// pure-introspection facility; nothing in the dispatch path checks it.
    pub fn register_action<A: 'static>(&mut self, actor_kind: ActorKind) {
        self.actions.insert(TypeId::of::<A>(), actor_kind);
    }

    /// Returns the registered actor kind for action type `A`, if any.
    pub fn actor_kind_for<A: 'static>(&self) -> Option<ActorKind> {
        self.actions.get(&TypeId::of::<A>()).copied()
    }

    /// Number of handlers registered for event type `E`. Mostly useful for
    /// tests that want to assert a module wired up the subscriptions it
    /// promised.
    pub fn handler_count<E: Event>(&self) -> usize {
        self.handlers
            .get(&TypeId::of::<E>())
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Invoke every handler subscribed to the runtime type of `event`. If no
    /// handler is subscribed, the call is a no-op — opt-in coverage replaces
    /// exhaustiveness here.
    pub fn dispatch_event(
        &self,
        game: &mut GameState,
        event: &dyn ErasedEvent,
        bus: &mut EventBus,
    ) {
        if let Some(handlers) = self.handlers.get(&event.event_type_id()) {
            for handler in handlers {
                handler.handle(game, event, bus);
            }
        }
    }
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field(
                "handlers",
                &self
                    .handlers
                    .iter()
                    .map(|(k, v)| (k, v.len()))
                    .collect::<Vec<_>>(),
            )
            .field("actions", &self.actions)
            .finish()
    }
}

/// Compose the process-wide registry by inviting every module that owns
/// handlers / actions to wire its bindings. Each call below is the *only*
/// place the listed module participates in the registry — its concrete
/// handlers, events, and actions are otherwise opaque to this function.
pub fn build_registry() -> Registry {
    let mut reg = Registry::default();
    crate::systems::visibility::register(&mut reg);
    crate::systems::enemy_tick::register(&mut reg);
    crate::player::register(&mut reg);
    reg
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Process-wide registry. First call lazily runs [`build_registry`]. All
/// subsequent calls hand back the same `&'static Registry`.
pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(build_registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;

    #[derive(Debug, Clone)]
    struct Beep;
    impl Event for Beep {}

    struct Counter;
    impl EventHandler<Beep> for Counter {
        fn handle(&self, _game: &mut GameState, _event: &Beep, _bus: &mut EventBus) {
            // exercised only via Registry::dispatch_event; intentionally a no-op
        }
    }

    #[test]
    fn handler_count_reflects_subscriptions() {
        let mut reg = Registry::default();
        assert_eq!(reg.handler_count::<Beep>(), 0);
        reg.subscribe::<Beep, _>(Counter);
        reg.subscribe::<Beep, _>(Counter);
        assert_eq!(reg.handler_count::<Beep>(), 2);
    }

    #[test]
    fn register_action_records_actor_kind() {
        let mut reg = Registry::default();
        struct Walk;
        reg.register_action::<Walk>(ActorKind::Player);
        assert_eq!(reg.actor_kind_for::<Walk>(), Some(ActorKind::Player));
    }

    #[test]
    fn unsubscribed_event_drains_cleanly() {
        let reg = Registry::default();
        let mut state = GameState::new_seeded(99);
        let mut bus = EventBus::default();
        // Direct call — nothing registered, must not panic.
        let event = Beep;
        reg.dispatch_event(&mut state, &event, &mut bus);
        assert!(bus.is_empty());
    }
}
