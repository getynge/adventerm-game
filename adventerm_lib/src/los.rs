use crate::room::{Room, TileKind};

pub const LOS_RANGE: usize = 6;

/// Terminal cells are roughly twice as tall as they are wide, so a literal
/// `dx² + dy²` disc looks like a vertical ellipse on screen. Scaling dy by
/// this factor in the distance check stretches the disc horizontally so it
/// appears round to the player.
pub const CELL_ASPECT_Y_OVER_X: f32 = 2.0;

/// Recompute currently-visible tiles for `room` from `origin`.
///
/// Resizes/clears `out` to `room.width * room.height` and writes a `bool` per
/// tile (row-major, matching `Room::idx`). The origin is always visible. For
/// each tile within Euclidean distance `LOS_RANGE`, a Bresenham line is walked
/// from origin outward; the endpoint is visible if no *intermediate* tile is a
/// wall. Endpoint walls remain visible — you see the wall, not past it.
pub fn compute_visible(room: &Room, origin: (usize, usize), out: &mut Vec<bool>) {
    let len = room.width * room.height;
    out.clear();
    out.resize(len, false);

    let (ox, oy) = origin;
    if ox >= room.width || oy >= room.height {
        return;
    }
    out[room.idx(ox, oy)] = true;

    let y_radius = (LOS_RANGE as f32 / CELL_ASPECT_Y_OVER_X).ceil() as usize;
    let lo_x = ox.saturating_sub(LOS_RANGE);
    let lo_y = oy.saturating_sub(y_radius);
    let hi_x = (ox + LOS_RANGE).min(room.width - 1);
    let hi_y = (oy + y_radius).min(room.height - 1);
    let r2 = (LOS_RANGE as f32) * (LOS_RANGE as f32);

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
        let room = open_room(15, 15);
        let v = vis(&room, (7, 7));
        assert!(v[room.idx(7, 7)]);
    }

    #[test]
    fn range_is_aspect_corrected_disc() {
        let room = open_room(20, 20);
        let v = vis(&room, (10, 10));
        // Horizontal axis: full LOS_RANGE tiles reachable.
        assert!(v[room.idx(16, 10)]);
        assert!(!v[room.idx(17, 10)]);
        // Vertical axis: dy is stretched ×2 in the distance check, so only
        // ~3 tiles up/down stay inside the disc.
        assert!(v[room.idx(10, 13)]);
        assert!(!v[room.idx(10, 14)]);
        // A short-and-wide diagonal stays inside the visual circle.
        assert!(v[room.idx(14, 12)]);
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
        room.set(5, 6, TileKind::Door(crate::room::DoorId(0)));
        let v = vis(&room, (5, 5));
        assert!(v[room.idx(5, 6)]);
        assert!(v[room.idx(5, 7)]);
    }
}
