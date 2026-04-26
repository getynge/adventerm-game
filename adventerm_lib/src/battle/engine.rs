use crate::abilities::{ability_behavior_for, AbilityCtx};
use crate::battle::{BattleResult, BattleState, BattleTurn};
use crate::ecs::EntityId;
use crate::game::GameState;

/// Reasons the engine refuses to apply a player turn. The screen turns these
/// into status messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleError {
    EmptySlot,
    NotPlayerTurn,
    AlreadyResolved,
}

/// Simple "basic attack" damage when an enemy retaliates. Mirrors `Impact`
/// for the player but uses the enemy's stats. Lives here so future per-kind
/// enemy abilities can replace it without touching `BattleState`.
const ENEMY_MIN_DAMAGE: u8 = 1;

/// Construct a `BattleState` from the current world. Snapshots the enemy's
/// current HP and stats; the engine works against the snapshot, not the
/// live entity, until the battle resolves.
pub fn start_battle(game: &GameState, enemy_id: EntityId) -> Option<BattleState> {
    let room_id = game.current_room;
    let room = game.dungeon.room(room_id);
    let enemy_hp = room.enemies.hp_of(enemy_id)?;
    Some(BattleState {
        enemy_id,
        enemy_room: room_id,
        player_cur_hp: game.cur_health,
        enemy_cur_hp: enemy_hp,
        turn: BattleTurn::Player,
        log: vec![format!(
            "A {} appears!",
            room.enemies
                .kind_of(enemy_id)
                .map(|k| k.name())
                .unwrap_or("foe")
        )],
    })
}

/// Apply the player's chosen active-ability slot to the battle. Caller passes
/// the slot index; the engine looks the ability up and dispatches via
/// `ability_behavior_for`.
pub fn apply_player_ability(
    game: &GameState,
    state: &mut BattleState,
    slot: usize,
) -> Result<(), BattleError> {
    if state.is_resolved() {
        return Err(BattleError::AlreadyResolved);
    }
    if state.turn != BattleTurn::Player {
        return Err(BattleError::NotPlayerTurn);
    }
    let kind = game.abilities.slot(slot).ok_or(BattleError::EmptySlot)?;

    let room = game.dungeon.room(state.enemy_room);
    let enemy_stats = room
        .enemies
        .stats_of(state.enemy_id)
        .copied()
        .ok_or(BattleError::AlreadyResolved)?;

    let outcome = {
        let ctx = AbilityCtx {
            attacker: &game.stats,
            defender: &enemy_stats,
        };
        ability_behavior_for(kind).execute(&ctx)
    };

    state.enemy_cur_hp = state.enemy_cur_hp.saturating_sub(outcome.damage);
    state.push_log(format!(
        "You use {} for {} damage.",
        kind.name(),
        outcome.damage
    ));

    if state.enemy_cur_hp == 0 {
        state.turn = BattleTurn::Resolved(BattleResult::Victory);
        state.push_log("You are victorious!");
    } else {
        state.turn = BattleTurn::Enemy;
    }
    Ok(())
}

/// Run the enemy's response. For now every enemy uses a generic basic attack
/// based on its own stats — per-kind abilities slot into `apply_enemy_turn`
/// the same way player abilities slot into `apply_player_ability`.
pub fn apply_enemy_turn(game: &GameState, state: &mut BattleState) {
    if state.is_resolved() {
        return;
    }
    if state.turn != BattleTurn::Enemy {
        return;
    }

    let room = game.dungeon.room(state.enemy_room);
    let Some(enemy_stats) = room.enemies.stats_of(state.enemy_id).copied() else {
        state.turn = BattleTurn::Resolved(BattleResult::Victory);
        state.push_log("Your foe vanishes.");
        return;
    };
    let kind = room
        .enemies
        .kind_of(state.enemy_id)
        .map(|k| k.name())
        .unwrap_or("Foe");

    let damage = enemy_stats
        .attack
        .saturating_sub(game.stats.defense)
        .max(ENEMY_MIN_DAMAGE);
    state.player_cur_hp = state.player_cur_hp.saturating_sub(damage);
    state.push_log(format!("{} hits you for {} damage.", kind, damage));

    if state.player_cur_hp == 0 {
        state.turn = BattleTurn::Resolved(BattleResult::Defeat);
        state.push_log("You have fallen.");
    } else {
        state.turn = BattleTurn::Player;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameState;

    #[test]
    fn player_impact_reduces_enemy_hp() {
        let mut game = GameState::new_seeded(42);
        // Force-spawn an enemy in the current room at a known free tile.
        let player = game.player;
        let target = (player.0 + 0, player.1 + 0); // overwritten below if blocked
        let room = game.dungeon.room_mut(game.current_room);
        // Find any walkable floor that isn't the player's tile.
        let mut spawn_pos = None;
        for y in 0..room.height {
            for x in 0..room.width {
                if (x, y) != player
                    && matches!(
                        room.kind_at(x, y),
                        Some(crate::room::TileKind::Floor)
                    )
                {
                    spawn_pos = Some((x, y));
                    break;
                }
            }
            if spawn_pos.is_some() {
                break;
            }
        }
        let spawn_pos = spawn_pos.unwrap_or(target);
        let enemy = room
            .enemies
            .spawn_at(&mut room.world, spawn_pos, crate::enemies::EnemyKind::Slime);

        let mut battle = start_battle(&game, enemy).expect("start_battle");
        let starting_enemy_hp = battle.enemy_cur_hp;
        apply_player_ability(&game, &mut battle, 0).expect("impact in slot 0");
        assert!(battle.enemy_cur_hp < starting_enemy_hp);
    }
}
