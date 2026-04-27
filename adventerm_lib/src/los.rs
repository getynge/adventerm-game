use crate::room::{Room, TileKind};

/// Player line-of-sight radius. Doubled from the original 6 so the player can
/// see meaningfully more of a room at once. Light sources use [`LIGHT_RANGE`]
/// instead, which is the smaller historical value.
pub const LOS_RANGE: usize = 12;

/// Radius used by persistent light sources (wall lights, placed torches).
/// Matches `LOS_RANGE` so a torch or wall light reveals the same disc the
/// player would see standing on that tile.
pub const LIGHT_RANGE: usize = LOS_RANGE;

/// Terminal cells are roughly twice as tall as they are wide, so a literal
/// `dx² + dy²` disc looks like a vertical ellipse on screen. Scaling dy by
/// this factor in the distance check stretches the disc horizontally so it
/// appears round to the player.
pub const CELL_ASPECT_Y_OVER_X: f32 = 2.0;

/// Recompute currently-visible tiles for `room` from `origin` at the player's
/// vision range. Thin wrapper around [`compute_visible_with_radius`].
pub fn compute_visible(room: &Room, origin: (usize, usize), out: &mut Vec<bool>) {
    compute_visible_with_radius(room, origin, LOS_RANGE, out);
}

/// Recompute visible tiles from `origin` within Euclidean distance `radius`.
///
/// Resizes/clears `out` to `room.width * room.height` and writes a `bool` per
/// tile (row-major, matching `Room::idx`). The origin is always visible. For
/// each tile within the aspect-corrected disc, a Bresenham line is walked from
/// origin outward; the endpoint is visible if no *intermediate* tile is a
/// wall. Endpoint walls remain visible — you see the wall, not past it.
pub fn compute_visible_with_radius(
    room: &Room,
    origin: (usize, usize),
    radius: usize,
    out: &mut Vec<bool>,
) {
    let len = room.width * room.height;
    out.clear();
    out.resize(len, false);

    let (ox, oy) = origin;
    if ox >= room.width || oy >= room.height {
        return;
    }
    out[room.idx(ox, oy)] = true;

    if radius == 0 || room.width == 0 || room.height == 0 {
        return;
    }

    let y_radius = (radius as f32 / CELL_ASPECT_Y_OVER_X).ceil() as usize;
    let lo_x = ox.saturating_sub(radius);
    let lo_y = oy.saturating_sub(y_radius);
    let hi_x = (ox + radius).min(room.width - 1);
    let hi_y = (oy + y_radius).min(room.height - 1);
    let r2 = (radius as f32) * (radius as f32);

    for ty in lo_y..=hi_y {
        for tx in lo_x..=hi_x {
            if (tx, ty) == (ox, oy) {
                continue;
            }
            let dx = tx as f32 - ox as f32;
            let dy = (ty as f32 - oy as f32) * CELL_ASPECT_Y_OVER_X;
            if dx * dx + dy * dy > r2 {
                continue;
            }
            if line_clear(room, ox, oy, tx, ty) {
                out[room.idx(tx, ty)] = true;
            }
        }
    }
}

/// Walk a Bresenham line from `(x0, y0)` to `(x1, y1)` and return true if the
/// endpoint is reachable: no wall sits *strictly between* origin and endpoint.
/// Endpoint walls return true — they are visible themselves.
fn line_clear(room: &Room, x0: usize, y0: usize, x1: usize, y1: usize) -> bool {
    let mut x = x0 as isize;
    let mut y = y0 as isize;
    let x1i = x1 as isize;
    let y1i = y1 as isize;

    let dx = (x1i - x).abs();
    let dy = -(y1i - y).abs();
    let sx: isize = if x < x1i { 1 } else { -1 };
    let sy: isize = if y < y1i { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        if x == x1i && y == y1i {
            return true;
        }
        if matches!(
            room.kind_at(x as usize, y as usize),
            Some(TileKind::Wall) | None
        ) {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::{Room, RoomId, TileKind};

    fn open_room(w: usize, h: usize) -> Room {
        Room::new_filled(RoomId(0), w, h, TileKind::Floor)
    }

    fn vis(room: &Room, origin: (usize, usize)) -> Vec<bool> {
        let mut out = Vec::new();
        compute_visible(room, origin, &mut out);
        out
    }

    #[test]
    fn origin_is_always_visible() {
        let room = open_room(30, 30);
        let v = vis(&room, (15, 15));
        assert!(v[room.idx(15, 15)]);
    }

    #[test]
    fn range_is_aspect_corrected_disc() {
        let room = open_room(40, 40);
        let v = vis(&room, (20, 20));
        // Horizontal axis: full LOS_RANGE tiles reachable.
        let r = LOS_RANGE;
        assert!(v[room.idx(20 + r, 20)]);
        assert!(!v[room.idx(20 + r + 1, 20)]);
        // Vertical axis: dy is stretched ×2 in the distance check, so only
        // ~r/2 tiles up/down stay inside the disc.
        let half = r / 2;
        assert!(v[room.idx(20, 20 + half)]);
        assert!(!v[room.idx(20, 20 + half + 1)]);
    }

    #[test]
    fn light_range_matches_player_range() {
        assert_eq!(LIGHT_RANGE, LOS_RANGE);
        let room = open_room(40, 40);
        let mut out = Vec::new();
        compute_visible_with_radius(&room, (20, 20), LIGHT_RANGE, &mut out);
        assert!(out[room.idx(20 + LIGHT_RANGE, 20)]);
        assert!(!out[room.idx(20 + LIGHT_RANGE + 1, 20)]);
    }

    #[test]
    fn wall_blocks_tile_behind_it() {
        let mut room = open_room(15, 15);
        // Drop a wall directly between origin (5,5) and target (5,8).
        room.set(5, 6, TileKind::Wall);
        let v = vis(&room, (5, 5));
        // The wall itself is visible…
        assert!(v[room.idx(5, 6)]);
        // …but tiles directly behind it along the same column are not.
        assert!(!v[room.idx(5, 7)]);
        assert!(!v[room.idx(5, 8)]);
    }

    #[test]
    fn doors_do_not_block_line_of_sight() {
        let mut room = open_room(15, 15);
        room.set(
            5,
            6,
            TileKind::Door(crate::room::DoorId(crate::ecs::EntityId::from_raw(0))),
        );
        let v = vis(&room, (5, 5));
        assert!(v[room.idx(5, 6)]);
        assert!(v[room.idx(5, 7)]);
    }
}
