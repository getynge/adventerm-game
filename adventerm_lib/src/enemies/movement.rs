use crate::ecs::EntityId;
use crate::enemies::ai::{enemy_behavior_for, AiAction, AiCtx};
use crate::rng::Rng;
use crate::room::Room;
use crate::world::Direction;

/// Result of running one enemy-movement tick. The movement system applies
/// enemy steps directly to the room, but it does not start a battle — the
/// caller decides what to do when an enemy ends adjacent to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyTickOutcome {
    Quiet,
    EncounterTriggered(EntityId),
}

/// Run one tick of enemy AI. Each enemy decides, the chosen step is applied
/// if the destination is walkable and not occupied by another enemy or by
/// the player. Iteration order is sorted by `EntityId` so the result is
/// deterministic given the seeded `Rng`.
pub fn tick_enemies(
    room: &mut Room,
    player_pos: (usize, usize),
    rng: &mut Rng,
) -> EnemyTickOutcome {
    let mut entities: Vec<EntityId> = room.enemies.entities().collect();
    entities.sort();

    let mut encounter: Option<EntityId> = None;

    for entity in entities {
        let Some(pos) = room.world.position_of(entity) else {
            continue;
        };

        let action = {
            let mut ctx = AiCtx {
                enemy_pos: pos,
                player_pos,
                room,
                rng,
            };
            enemy_behavior_for(room.enemies.kind_of(entity).unwrap_or_else(|| {
                // Should never happen: an entity in `enemies` always has a kind.
                unreachable!("enemy entity without kind")
            }))
            .decide(&mut ctx)
        };

        let new_pos = match action {
            AiAction::Wait => pos,
            AiAction::Step(dir) => {
                let target = step_pos(pos, dir);
                if let Some(dest) = step_destination(room, target, player_pos) {
                    room.world.set_position(entity, dest);
                    dest
                } else {
                    pos
                }
            }
        };

        if encounter.is_none() && is_adjacent(new_pos, player_pos) {
            encounter = Some(entity);
        }
    }

    match encounter {
        Some(e) => EnemyTickOutcome::EncounterTriggered(e),
        None => EnemyTickOutcome::Quiet,
    }
}

fn step_pos(pos: (usize, usize), dir: Direction) -> (isize, isize) {
    let (x, y) = pos;
    match dir {
        Direction::Up => (x as isize, y as isize - 1),
        Direction::Down => (x as isize, y as isize + 1),
        Direction::Left => (x as isize - 1, y as isize),
        Direction::Right => (x as isize + 1, y as isize),
    }
}

fn step_destination(
    room: &Room,
    target: (isize, isize),
    player_pos: (usize, usize),
) -> Option<(usize, usize)> {
    if !room.in_bounds(target.0, target.1) {
        return None;
    }
    let pos = (target.0 as usize, target.1 as usize);
    if !matches!(room.kind_at(pos.0, pos.1), Some(crate::room::TileKind::Floor)) {
        return None;
    }
    if pos == player_pos {
        return None;
    }
    if room.enemies.entity_at(&room.world, pos).is_some() {
        return None;
    }
    Some(pos)
}

fn is_adjacent(a: (usize, usize), b: (usize, usize)) -> bool {
    let dx = (a.0 as isize - b.0 as isize).abs();
    let dy = (a.1 as isize - b.1 as isize).abs();
    dx + dy == 1
}
