use crate::items::behavior::{ItemBehavior, PlaceCtx, PlaceOutcome};
use crate::items::category::ItemCategory;

/// Placing a flare installs a `FlareSource` at the player's tile. While at
/// least one flare exists in the room, every tile is lit; on room exit each
/// flare burns out into a regular torch (handled by `Lighting`).
pub struct FlareBehavior;

impl ItemBehavior for FlareBehavior {
    fn category(&self) -> ItemCategory {
        ItemCategory::Placeable
    }

    fn on_place(&self, ctx: &mut PlaceCtx<'_>) -> Option<PlaceOutcome> {
        ctx.lighting.add_flare(ctx.world, ctx.player_pos);
        Some(PlaceOutcome::FlarePlaced)
    }
}
