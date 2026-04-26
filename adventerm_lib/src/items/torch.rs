use crate::items::behavior::{ItemBehavior, PlaceCtx, PlaceOutcome};

/// Placing a torch installs a persistent `LightSource` at the player's tile.
pub struct TorchBehavior;

impl ItemBehavior for TorchBehavior {
    fn on_place(&self, ctx: &mut PlaceCtx<'_>) -> PlaceOutcome {
        ctx.lighting.add_torch(ctx.world, ctx.player_pos);
        PlaceOutcome::TorchPlaced
    }
}
