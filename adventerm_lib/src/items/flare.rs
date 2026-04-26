use crate::items::behavior::{ItemBehavior, PlaceCtx, PlaceOutcome};

/// Placing a flare installs a `FlareSource` at the player's tile. While at
/// least one flare exists in the room, every tile is lit; on room exit each
/// flare burns out into a regular torch (handled by `Lighting`).
pub struct FlareBehavior;

impl ItemBehavior for FlareBehavior {
    fn on_place(&self, ctx: &mut PlaceCtx<'_>) -> PlaceOutcome {
        ctx.lighting.add_flare(ctx.world, ctx.player_pos);
        PlaceOutcome::FlarePlaced
    }
}
