//! Dispatch-level tests covering the open-registration pipeline. Each
//! test invokes `dispatch` on the global registry built by
//! `build_registry`, so the bindings exercised match what production
//! callers see.

use std::sync::Mutex;

use crate::action::dispatch;
use crate::actions::{InteractAction, MoveAction, PickUpAction, PlaceItemAction};
use crate::event::EventBus;
use crate::events::{DoorTraversed, ItemEquipped, ItemPlaced, ItemUnequipped, PlayerMoved};
use crate::game::{GameState, MoveOutcome};
use crate::items::ItemKind;
use crate::registry::{build_registry, ActorKind, EventHandler, Registry};
use crate::room::TileKind;
use crate::world::Direction;

/// Serialize tests that read the global registry — they share the
/// process-wide singleton, so concurrent reads of `actor_kind_for` etc.
/// are fine, but tests that mutate `pending_encounter` on the same
/// `GameState` need their own per-test instance (already the case).
static REGISTRY_GUARD: Mutex<()> = Mutex::new(());

fn find_door(state: &GameState) -> (crate::room::DoorId, (usize, usize)) {
    let room = state.current_room();
    for y in 0..room.height {
        for x in 0..room.width {
            if let Some(TileKind::Door(id)) = room.kind_at(x, y) {
                return (id, (x, y));
            }
        }
    }
    panic!("starting room has no door");
}

#[test]
fn dispatch_move_runs_visibility_handler() {
    let _g = REGISTRY_GUARD.lock();
    let mut game = GameState::new_seeded(11);
    let player = game.player.entity();
    let mut moved_to: Option<(usize, usize)> = None;
    for dir in [
        Direction::Right,
        Direction::Down,
        Direction::Left,
        Direction::Up,
    ] {
        let prev = game.player_pos();
        if matches!(
            dispatch(&mut game, player, MoveAction { direction: dir }),
            MoveOutcome::Moved
        ) {
            moved_to = Some(game.player_pos());
            assert_ne!(prev, game.player_pos());
            break;
        }
    }
    assert!(moved_to.is_some(), "no walkable neighbor; bad seed");
    // After dispatch the player's tile must be marked explored, which
    // only happens via the VisibilityHandler responding to PlayerMoved.
    let (px, py) = moved_to.unwrap();
    assert!(game.is_explored(px, py));
}

#[test]
fn dispatch_door_traversed_refreshes_visibility_in_new_room() {
    let _g = REGISTRY_GUARD.lock();
    let mut game = GameState::new_seeded(17);
    let player = game.player.entity();
    let (_, door_pos) = find_door(&game);
    game.player.set_position(door_pos);
    // Pretend nothing in the new room is explored yet.
    let event = dispatch(&mut game, player, InteractAction).expect("door interact emits event");
    // Visibility handler must have run after DoorTraversed.
    assert_eq!(game.current_room, event.new_room);
    let (px, py) = game.player_pos();
    assert!(game.is_explored(px, py), "landing tile should be explored");
}

#[test]
fn dispatch_place_item_refreshes_visibility() {
    let _g = REGISTRY_GUARD.lock();
    let mut game = GameState::new_seeded(11);
    let player = game.player.entity();
    let pos = game.player_pos();
    game.player.inventory_push(ItemKind::Torch);
    let outcome = dispatch(&mut game, player, PlaceItemAction { slot: 0 }).expect("place ok");
    assert!(matches!(
        outcome,
        crate::items::PlaceOutcome::TorchPlaced
    ));
    assert!(game.current_room().has_light_at(pos));
    assert!(game.is_visible(pos.0, pos.1));
}

#[test]
fn dispatch_pickup_emits_picked_up() {
    let _g = REGISTRY_GUARD.lock();
    let mut game = GameState::new_seeded(11);
    let player = game.player.entity();
    let pos = game.player_pos();
    // Drop an item under the player by reaching directly into the room.
    let room = game.dungeon.room_mut(game.current_room);
    room.items
        .spawn_at(&mut room.world, pos, ItemKind::Torch);
    let kind = dispatch(&mut game, player, PickUpAction).expect("picked up");
    assert_eq!(kind, ItemKind::Torch);
    assert!(game.inventory().contains(&ItemKind::Torch));
}

#[test]
fn registry_dispatches_in_subscription_order() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone)]
    struct Tick;
    impl crate::event::Event for Tick {}

    struct Recorder {
        slot: Arc<AtomicUsize>,
        marker: u8,
    }
    impl EventHandler<Tick> for Recorder {
        fn handle(&self, _g: &mut GameState, _e: &Tick, _b: &mut EventBus) {
            // Pack the slot as base-16 digits so we can read both ordering
            // and identity from the final value.
            let prev = self.slot.load(Ordering::SeqCst);
            self.slot
                .store(prev * 16 + self.marker as usize, Ordering::SeqCst);
        }
    }

    let slot = Arc::new(AtomicUsize::new(0));
    let mut reg = Registry::default();
    reg.subscribe::<Tick, _>(Recorder {
        slot: slot.clone(),
        marker: 1,
    });
    reg.subscribe::<Tick, _>(Recorder {
        slot: slot.clone(),
        marker: 2,
    });
    reg.subscribe::<Tick, _>(Recorder {
        slot: slot.clone(),
        marker: 3,
    });

    let mut game = GameState::new_seeded(11);
    let mut bus = EventBus::default();
    let event = Tick;
    reg.dispatch_event(&mut game, &event, &mut bus);

    assert_eq!(slot.load(Ordering::SeqCst), 0x123);
}

#[test]
fn build_registry_records_player_actions() {
    let reg = build_registry();
    assert_eq!(
        reg.actor_kind_for::<MoveAction>(),
        Some(ActorKind::Player)
    );
    assert_eq!(
        reg.actor_kind_for::<InteractAction>(),
        Some(ActorKind::Player)
    );
    assert_eq!(
        reg.actor_kind_for::<PlaceItemAction>(),
        Some(ActorKind::Player)
    );
}

#[test]
fn build_registry_subscribes_visibility_and_enemy_tick_to_player_moved() {
    let reg = build_registry();
    // Visibility + enemy tick both subscribe to PlayerMoved.
    assert_eq!(reg.handler_count::<PlayerMoved>(), 2);
    // Visibility-only subscriptions:
    assert_eq!(reg.handler_count::<DoorTraversed>(), 1);
    assert_eq!(reg.handler_count::<ItemPlaced>(), 1);
    assert_eq!(reg.handler_count::<ItemEquipped>(), 1);
    assert_eq!(reg.handler_count::<ItemUnequipped>(), 1);
}
