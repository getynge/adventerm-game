//! Developer-mode mutators. Reachable only from the dev console — the
//! gameplay loop never calls these. They route through the same per-room
//! subsystem APIs as ordinary gameplay (`Room::items.spawn_at`,
//! `Room::enemies.spawn_at`) so they cannot bypass invariants.

use crate::ecs::EntityId;
use crate::enemies::EnemyKind;
use crate::game::GameState;
use crate::items::ItemKind;

/// Spawn a ground-item entity on the player's current tile. Items are
/// stackable across the floor (the renderer surfaces only the first), so
/// landing on a tile that already has one is fine.
pub fn spawn_item_at_player(game: &mut GameState, kind: ItemKind) -> EntityId {
    let pos = game.player.position();
    let room_id = game.current_room;
    let room = game.dungeon.room_mut(room_id);
    room.items.spawn_at(&mut room.world, pos, kind)
}

/// Spawn an enemy on the first walkable, enemy-free tile adjacent to the
/// player (4-neighborhood, then 8). Returns `None` if every neighbor is a
/// wall or already occupied. The player's own tile is intentionally not
/// considered — overlapping the player would either occupy the same square
/// (renderer hides one) or trigger an immediate encounter, neither of which
/// matches "spawn next to me".
pub fn spawn_enemy_near_player(game: &mut GameState, kind: EnemyKind) -> Option<EntityId> {
    let (px, py) = game.player.position();
    let room_id = game.current_room;
    let room = game.dungeon.room_mut(room_id);

    const NEIGHBORS_4: &[(isize, isize)] = &[(0, -1), (0, 1), (-1, 0), (1, 0)];
    const DIAGONALS: &[(isize, isize)] = &[(-1, -1), (1, -1), (-1, 1), (1, 1)];

    for offsets in [NEIGHBORS_4, DIAGONALS] {
        for (dx, dy) in offsets {
            let nx = px as isize + dx;
            let ny = py as isize + dy;
            if !room.in_bounds(nx, ny) {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if !room.is_walkable(nx, ny) {
                continue;
            }
            if room.enemies.entity_at(&room.world, (nx, ny)).is_some() {
                continue;
            }
            return Some(room.enemies.spawn_at(&mut room.world, (nx, ny), kind));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::TileKind;

    #[test]
    fn spawn_item_lands_under_player() {
        let mut game = GameState::new_seeded(11);
        let pos = game.player.position();
        let entity = spawn_item_at_player(&mut game, ItemKind::Torch);
        let room = game.current_room();
        assert!(room.has_item_at(pos));
        let kinds: Vec<_> = room.items_at(pos).collect();
        assert!(kinds.contains(&ItemKind::Torch));
        // entity is alive in the room's world
        assert!(room.world.is_alive(entity));
    }

    #[test]
    fn spawn_enemy_picks_adjacent_floor() {
        let mut game = GameState::new_seeded(11);
        let (px, py) = game.player.position();
        let entity =
            spawn_enemy_near_player(&mut game, EnemyKind::Slime).expect("space available");
        let room = game.current_room();
        let pos = room.world.position_of(entity).unwrap();
        let dx = (pos.0 as isize - px as isize).abs();
        let dy = (pos.1 as isize - py as isize).abs();
        assert!(dx <= 1 && dy <= 1, "spawned tile must be adjacent");
        assert!((dx, dy) != (0, 0), "must not overlap the player");
        assert!(matches!(
            room.kind_at(pos.0, pos.1),
            Some(TileKind::Floor) | Some(TileKind::Door(_))
        ));
    }

    #[test]
    fn spawn_enemy_returns_none_when_walled_in() {
        let mut game = GameState::new_seeded(11);
        let (px, py) = game.player.position();
        let room_id = game.current_room;
        let room = game.dungeon.room_mut(room_id);
        for dx in -1isize..=1 {
            for dy in -1isize..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = px as isize + dx;
                let ny = py as isize + dy;
                if room.in_bounds(nx, ny) {
                    room.set(nx as usize, ny as usize, TileKind::Wall);
                }
            }
        }
        assert!(spawn_enemy_near_player(&mut game, EnemyKind::Slime).is_none());
    }
}
