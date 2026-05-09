pub mod abilities;
pub mod action;
pub mod actions;
pub mod battle;
pub mod console;
pub mod dungeon;
pub mod ecs;
pub mod enemies;
pub mod equipment;
pub mod event;
pub mod events;
pub mod explored;
pub mod game;
pub mod items;
pub mod lighting;
pub mod los;
pub mod player;
pub mod registry;
pub mod rng;
pub mod room;
pub mod save;
pub mod stats;
pub mod systems;
pub mod visibility;
pub mod world;

pub use abilities::{AbilityKind, PassiveKind};
pub use action::{Action, dispatch};
pub use actions::{
    ConsumeItemAction, DefeatEnemyAction, EquipItemAction, InteractAction, MoveAction,
    PickUpAction, PlaceItemAction, QuickMoveAction, UnequipItemAction,
};
pub use battle::{
    Battle, BattleLog, BattleResult, BattleSubsystem, BattleTurn, Combatants, HpSnapshot,
};
pub use dungeon::{DoorSubsystem, DoorView, Dungeon, DungeonClock};
pub use ecs::{EntityId, World};
pub use enemies::EnemyKind;
pub use equipment::Equipment;
pub use event::{Event, EventBus};
pub use events::{
    DoorTraversed, EnemyDefeated, EnemyEngaged, EnemyMoved, FlareBurnedOut, ItemConsumed,
    ItemEquipped, ItemPickedUp, ItemPlaced, ItemUnequipped, PlayerMoved,
};
pub use game::{DoorEvent, GameState, MoveOutcome, PlaceOutcome};
pub use items::{
    ConsumeIntent, ConsumeOutcome, ConsumeTarget, EquipEffect, EquipSlot, ItemCategory, ItemKind,
    category_of, consume_intent_of,
};
pub use los::{LIGHT_RANGE, LOS_RANGE};
pub use registry::{ActorKind, EventHandler, Registry, build_registry, registry};
pub use room::{DoorId, Room, RoomId, TileKind};
pub use save::{SAVE_VERSION, Save, SaveError, SaveSlot};
pub use stats::{Attribute, Stats};
pub use world::{Direction, Tile};
