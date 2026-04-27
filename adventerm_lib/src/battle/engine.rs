use crate::abilities::{ability_behavior_for, AbilityCtx};
use crate::battle::{Battle, BattleResult, BattleTurn, Combatants, HpSnapshot};
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
/// enemy abilities can replace it without touching `Battle`.
const ENEMY_MIN_DAMAGE: u8 = 1;

/// Construct a fresh `Battle` from the current world. Snapshots the enemy's
/// current HP and stats; the engine works against the snapshot, not the
/// live entity, until the battle resolves.
pub fn start_battle(game: &GameState, enemy_id: EntityId) -> Option<Battle> {
    let room_id = game.current_room;
    let room = game.dungeon.room(room_id);
    let enemy_hp = room.enemies.hp_of(enemy_id)?;
    let enemy_name = room
        .enemies
        .kind_of(enemy_id)
        .map(|k| k.name())
        .unwrap_or("foe");
    Some(Battle::spawn(
        Combatants {
            enemy_entity: enemy_id,
            enemy_room: room_id,
        },
        HpSnapshot {
            player: game.cur_health(),
            enemy: enemy_hp,
        },
        format!("A {} appears!", enemy_name),
    ))
}

/// Apply the player's chosen active-ability slot to the battle. Caller passes
/// the slot index; the engine looks the ability up and dispatches via
/// `ability_behavior_for`.
pub fn apply_player_ability(
    game: &GameState,
    battle: &mut Battle,
    slot: usize,
) -> Result<(), BattleError> {
    if battle.is_resolved() {
        return Err(BattleError::AlreadyResolved);
    }
    if battle.turn() != BattleTurn::Player {
        return Err(BattleError::NotPlayerTurn);
    }
    let kind = game.abilities().slot(slot).ok_or(BattleError::EmptySlot)?;

    let combatants = battle.combatants();
    let room = game.dungeon.room(combatants.enemy_room);
    let enemy_stats = room
        .enemies
        .stats_of(combatants.enemy_entity)
        .copied()
        .ok_or(BattleError::AlreadyResolved)?;

    let player_stats = game.effective_stats();
    let outcome = {
        let ctx = AbilityCtx {
            attacker: &player_stats,
            defender: &enemy_stats,
        };
        ability_behavior_for(kind).execute(&ctx)
    };

    let new_enemy_hp = battle.enemy_cur_hp().saturating_sub(outcome.damage);
    battle.set_enemy_hp(new_enemy_hp);
    battle.push_log(format!(
        "You use {} for {} damage.",
        kind.name(),
        outcome.damage
    ));

    if new_enemy_hp == 0 {
        battle.set_turn(BattleTurn::Resolved(BattleResult::Victory));
        battle.push_log("You are victorious!");
    } else {
        battle.set_turn(BattleTurn::Enemy);
    }
    Ok(())
}

/// Run the enemy's response. For now every enemy uses a generic basic attack
/// based on its own stats — per-kind abilities slot into `apply_enemy_turn`
/// the same way player abilities slot into `apply_player_ability`.
pub fn apply_enemy_turn(game: &GameState, battle: &mut Battle) {
    if battle.is_resolved() {
        return;
    }
    if battle.turn() != BattleTurn::Enemy {
        return;
    }

    let combatants = battle.combatants();
    let room = game.dungeon.room(combatants.enemy_room);
    let Some(enemy_stats) = room.enemies.stats_of(combatants.enemy_entity).copied() else {
        battle.set_turn(BattleTurn::Resolved(BattleResult::Victory));
        battle.push_log("Your foe vanishes.");
        return;
    };
    let kind = room
        .enemies
        .kind_of(combatants.enemy_entity)
        .map(|k| k.name())
        .unwrap_or("Foe");

    let damage = enemy_stats
        .attack
        .saturating_sub(game.effective_stats().defense)
        .max(ENEMY_MIN_DAMAGE);
    let new_player_hp = battle.player_cur_hp().saturating_sub(damage);
    battle.set_player_hp(new_player_hp);
    battle.push_log(format!("{} hits you for {} damage.", kind, damage));

    if new_player_hp == 0 {
        battle.set_turn(BattleTurn::Resolved(BattleResult::Defeat));
        battle.push_log("You have fallen.");
    } else {
        battle.set_turn(BattleTurn::Player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameState;

    #[test]
    fn player_impact_reduces_enemy_hp() {
        let mut game = GameState::new_seeded(42);
        let player = game.player_pos();
        let room = game.dungeon.room_mut(game.current_room);
        let mut spawn_pos = None;
        for y in 0..room.height {
            for x in 0..room.width {
                if (x, y) != player
                    && matches!(room.kind_at(x, y), Some(crate::room::TileKind::Floor))
                {
                    spawn_pos = Some((x, y));
                    break;
                }
            }
            if spawn_pos.is_some() {
                break;
            }
        }
        let spawn_pos = spawn_pos.expect("test seed had no extra floor tile");
        let enemy = room
            .enemies
            .spawn_at(&mut room.world, spawn_pos, crate::enemies::EnemyKind::Slime);

        let mut battle = start_battle(&game, enemy).expect("start_battle");
        let starting_enemy_hp = battle.enemy_cur_hp();
        apply_player_ability(&game, &mut battle, 0).expect("impact in slot 0");
        assert!(battle.enemy_cur_hp() < starting_enemy_hp);
    }

    #[test]
    fn equipping_gauntlets_raises_player_damage() {
        let mut game = GameState::new_seeded(42);
        let player = game.player_pos();
        let room = game.dungeon.room_mut(game.current_room);
        let mut spawn_pos = None;
        for y in 0..room.height {
            for x in 0..room.width {
                if (x, y) != player
                    && matches!(room.kind_at(x, y), Some(crate::room::TileKind::Floor))
                {
                    spawn_pos = Some((x, y));
                    break;
                }
            }
            if spawn_pos.is_some() {
                break;
            }
        }
        let spawn_pos = spawn_pos.expect("test seed had no extra floor tile");
        let enemy = room
            .enemies
            .spawn_at(&mut room.world, spawn_pos, crate::enemies::EnemyKind::Slime);

        let baseline = {
            let mut g = game.clone();
            let mut b = start_battle(&g, enemy).expect("baseline battle");
            let starting = b.enemy_cur_hp();
            apply_player_ability(&mut g, &mut b, 0).expect("baseline impact");
            starting - b.enemy_cur_hp()
        };

        game.player
            .equipment_mut()
            .equip(crate::items::EquipSlot::Arms, crate::items::ItemKind::Gauntlets);
        let with_gauntlets = {
            let mut b = start_battle(&game, enemy).expect("equipped battle");
            let starting = b.enemy_cur_hp();
            apply_player_ability(&game, &mut b, 0).expect("equipped impact");
            starting - b.enemy_cur_hp()
        };
        assert_eq!(with_gauntlets, baseline + 1);
    }

    #[test]
    fn battle_log_records_opening_line() {
        let mut game = GameState::new_seeded(42);
        let player = game.player_pos();
        let room = game.dungeon.room_mut(game.current_room);
        let mut spawn_pos = None;
        for y in 0..room.height {
            for x in 0..room.width {
                if (x, y) != player
                    && matches!(room.kind_at(x, y), Some(crate::room::TileKind::Floor))
                {
                    spawn_pos = Some((x, y));
                    break;
                }
            }
            if spawn_pos.is_some() {
                break;
            }
        }
        let spawn_pos = spawn_pos.expect("test seed had no extra floor tile");
        let enemy = room
            .enemies
            .spawn_at(&mut room.world, spawn_pos, crate::enemies::EnemyKind::Slime);
        let battle = start_battle(&game, enemy).expect("start_battle");
        assert_eq!(battle.log().len(), 1);
        assert!(battle.log()[0].contains("appears"));
    }
}
