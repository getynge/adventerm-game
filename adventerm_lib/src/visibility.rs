use crate::los;
use crate::room::Room;

/// Recompute the per-tile `visible` (player LOS) and `lit` (persistent
/// lighting from the `Lighting` subsystem) bitmaps for `room` using the
/// player's default LOS radius. Thin wrapper around
/// [`compute_room_lighting_with_radius`] for callers that don't care about
/// equipment-driven vision changes.
pub fn compute_room_lighting(
    room: &Room,
    player: (usize, usize),
    visible: &mut Vec<bool>,
    lit: &mut Vec<bool>,
) {
    compute_room_lighting_with_radius(room, player, los::LOS_RANGE, visible, lit);
}

/// Recompute the per-tile `visible` and `lit` bitmaps with an explicit
/// player vision `radius`. Light sources still use their own
/// (`LightSource::radius`) constants — the player's vision multiplier
/// affects only the player's LOS disc.
///
/// Active flares short-circuit `lit` to "every tile" — that's the rule the
/// flare item type encodes via `FlareSource`.
pub fn compute_room_lighting_with_radius(
    room: &Room,
    player: (usize, usize),
    radius: usize,
    visible: &mut Vec<bool>,
    lit: &mut Vec<bool>,
) {
    los::compute_visible_with_radius(room, player, radius, visible);

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
