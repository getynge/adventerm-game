use crate::enemies::ai::{AiAction, AiCtx, EnemyAi};
use crate::world::Direction;

/// Slime moves on a coin-flip and prefers closing distance to the player on
/// whichever axis is currently farther. Falls back to the other axis if the
/// first choice would walk into a wall.
pub struct SlimeAi;

const MOVE_NUM: u32 = 1;
const MOVE_DEN: u32 = 2;

impl EnemyAi for SlimeAi {
    fn decide(&self, ctx: &mut AiCtx<'_>) -> AiAction {
        if !ctx.rng.chance(MOVE_NUM, MOVE_DEN) {
            return AiAction::Wait;
        }
        let (ex, ey) = ctx.enemy_pos;
        let (px, py) = ctx.player_pos;
        let dx = px as isize - ex as isize;
        let dy = py as isize - ey as isize;

        let primary = if dx.abs() >= dy.abs() {
            horizontal(dx)
        } else {
            vertical(dy)
        };
        let secondary = if dx.abs() >= dy.abs() {
            vertical(dy)
        } else {
            horizontal(dx)
        };

        for candidate in [primary, secondary].into_iter().flatten() {
            if step_is_walkable(ctx, candidate) {
                return AiAction::Step(candidate);
            }
        }
        AiAction::Wait
    }
}

fn horizontal(dx: isize) -> Option<Direction> {
    match dx.signum() {
        1 => Some(Direction::Right),
        -1 => Some(Direction::Left),
        _ => None,
    }
}

fn vertical(dy: isize) -> Option<Direction> {
    match dy.signum() {
        1 => Some(Direction::Down),
        -1 => Some(Direction::Up),
        _ => None,
    }
}

fn step_is_walkable(ctx: &AiCtx<'_>, dir: Direction) -> bool {
    let (x, y) = ctx.enemy_pos;
    let (nx, ny) = match dir {
        Direction::Up => (x as isize, y as isize - 1),
        Direction::Down => (x as isize, y as isize + 1),
        Direction::Left => (x as isize - 1, y as isize),
        Direction::Right => (x as isize + 1, y as isize),
    };
    if !ctx.room.in_bounds(nx, ny) {
        return false;
    }
    matches!(
        ctx.room.kind_at(nx as usize, ny as usize),
        Some(crate::room::TileKind::Floor)
    )
}
