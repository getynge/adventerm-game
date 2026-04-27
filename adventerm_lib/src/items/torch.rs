use crate::items::behavior::{ItemBehavior, PlaceCtx, PlaceOutcome};
use crate::items::category::ItemCategory;

/// Placing a torch installs a persistent `LightSource` at the player's tile.
pub struct TorchBehavior;

impl ItemBehavior for TorchBehavior {
    fn category(&self) -> ItemCategory {
        ItemCategory::Placeable
    }

    fn on_place(&self, ctx: &mut PlaceCtx<'_>) -> Option<PlaceOutcome> {
        ctx.lighting.add_torch(ctx.world, ctx.player_pos);
        Some(PlaceOutcome::TorchPlaced)
    }
}
