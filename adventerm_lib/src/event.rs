//! Open-set event substrate.
//!
//! An [`Event`] is any concrete type emitted into the [`EventBus`] by an
//! action or by another handler reacting to a prior event. The bus is
//! type-erased so the registry can dispatch every emitted event to the
//! handlers that subscribed for that exact concrete type via `TypeId`.
//!
//! Events themselves carry no central catalog: each module owns the event
//! types it emits, registers any handlers it ships with, and is otherwise
//! invisible to the rest of the library.

use std::any::{Any, TypeId};
use std::collections::VecDeque;
use std::fmt::Debug;

/// Marker trait every concrete event type implements. The bounds are what
/// the registry, the bus, and the diagnostic log need: any-cast for typed
/// handler dispatch, `Debug` for ad-hoc inspection, thread-safe + `'static`
/// so handlers can be `Send + Sync` and freely shared across threads.
pub trait Event: Any + Debug + Send + Sync + 'static {}

/// Object-safe view of an emitted event. Stored inside the bus and handed
/// to `Registry::dispatch_event` so the dispatcher can both read the
/// runtime [`TypeId`] (to look up handlers) and downcast to the concrete
/// event type (to invoke them).
///
/// The accessor is named `event_type_id` rather than `type_id` so that
/// calls on `Box<dyn ErasedEvent>` cannot be silently routed to
/// [`std::any::Any::type_id`] (which would return the boxed wrapper's
/// type, not the wrapped event's).
pub trait ErasedEvent: Debug + Send + Sync {
    fn event_type_id(&self) -> TypeId;
    fn as_any(&self) -> &dyn Any;
    fn type_name(&self) -> &'static str;
}

impl<E: Event> ErasedEvent for E {
    fn event_type_id(&self) -> TypeId {
        TypeId::of::<E>()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<E>()
    }
}

/// FIFO queue of events emitted during a single `dispatch` invocation. The
/// dispatch loop drains the queue front-to-back; handlers may emit further
/// events into the back of the same queue, which the loop will then drain
/// in turn.
#[derive(Debug, Default)]
pub struct EventBus {
    queue: VecDeque<Box<dyn ErasedEvent>>,
}

impl EventBus {
    pub fn emit<E: Event>(&mut self, event: E) {
        self.queue.push_back(Box::new(event));
    }

    pub fn pop(&mut self) -> Option<Box<dyn ErasedEvent>> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Ping(u32);
    impl Event for Ping {}

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Pong(&'static str);
    impl Event for Pong {}

    #[test]
    fn emit_and_pop_preserves_order_across_types() {
        let mut bus = EventBus::default();
        bus.emit(Ping(1));
        bus.emit(Pong("a"));
        bus.emit(Ping(2));

        let first = bus.pop().unwrap();
        assert_eq!(first.event_type_id(), TypeId::of::<Ping>());
        assert_eq!(first.as_any().downcast_ref::<Ping>(), Some(&Ping(1)));

        let second = bus.pop().unwrap();
        assert_eq!(second.event_type_id(), TypeId::of::<Pong>());

        let third = bus.pop().unwrap();
        assert_eq!(third.as_any().downcast_ref::<Ping>(), Some(&Ping(2)));

        assert!(bus.pop().is_none());
    }
}
