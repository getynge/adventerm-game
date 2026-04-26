use crate::los;
use crate::room::Room;

/// Recompute the per-tile `visible` (player LOS) and `lit` (persistent
/// lighting from the `Lighting` subsystem) bitmaps for `room`. Both buffers
/// are resized to `room.width * room.height` row-major.
///
/// Active flares short-circuit `lit` to "every tile" — that's the rule the
/// flare item type encodes via `FlareSource`.
pub fn compute_room_lighting(
    room: &Room,
    player: (usize, usize),
    visible: &mut Vec<bool>,
    lit: &mut Vec<bool>,
) {
    los::compute_visible(room, player, visible);

    let len = room.width * room.height;
    lit.clear();
    lit.resize(len, false);

    if room.lighting.any_flare_active() {
        for v in lit.iter_mut() {
            *v = true;
        }
        return;
    }

    let mut tmp: Vec<bool> = Vec::new();
    for (pos, light) in room.lighting.iter_sources(&room.world) {
        los::compute_visible_with_radius(room, pos, light.radius as usize, &mut tmp);
        for (dst, src) in lit.iter_mut().zip(tmp.iter()) {
            if *src {
                *dst = true;
            }
        }
    }
}
