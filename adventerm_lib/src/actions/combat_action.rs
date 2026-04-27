use crate::action::Action;
use crate::ecs::EntityId;
use crate::event::EventBus;
use crate::events::EnemyDefeated;
use crate::game::GameState;
use crate::room::RoomId;

/// "Despawn the named enemy from `room`." Emitted by the binary after a
/// battle ends in the player's favour. Emits [`EnemyDefeated`].
#[derive(Debug, Clone, Copy)]
pub struct DefeatEnemyAction {
    pub room: RoomId,
    pub entity: EntityId,
}

impl Action for DefeatEnemyAction {
    type Outcome = ();

    fn perform(self, game: &mut GameState, _actor: EntityId, bus: &mut EventBus) {
        let r = game.dungeon.room_mut(self.room);
        r.enemies.despawn(&mut r.world, self.entity);
        bus.emit(EnemyDefeated {
            entity: self.entity,
            room: self.room,
        });
    }
}
