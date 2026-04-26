use crate::ecs::World;
use crate::items::flare::FlareBehavior;
use crate::items::kind::ItemKind;
use crate::items::torch::TorchBehavior;
use crate::lighting::Lighting;

/// Side-effect description returned by a placement behavior so the caller
/// can surface a status string. The behavior itself has already mutated the
/// world by the time this is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceOutcome {
    TorchPlaced,
    FlarePlaced,
}

/// Mutable view of the world handed to an `ItemBehavior` when it acts. Only
/// the subsystems a behavior is allowed to touch are exposed here — a torch
/// can change lighting but not, say, monsters. Add new subsystems here as
/// they land.
pub struct PlaceCtx<'a> {
    pub player_pos: (usize, usize),
    pub world: &'a mut World,
    pub lighting: &'a mut Lighting,
}

/// Trait every item kind implements to declare what happens when the player
/// places it from their inventory. `GameState` dispatches through
/// `behavior_for(kind)` and never matches on the kind itself.
pub trait ItemBehavior {
    fn on_place(&self, ctx: &mut PlaceCtx<'_>) -> PlaceOutcome;
}

/// The single point in the codebase that enumerates `ItemKind`. Adding a new
/// item: add a variant, add a new module with a behavior impl, and add one
/// arm here. Exhaustiveness is compiler-enforced.
pub fn behavior_for(kind: ItemKind) -> &'static dyn ItemBehavior {
    match kind {
        ItemKind::Torch => &TorchBehavior,
        ItemKind::Flare => &FlareBehavior,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_behavior_spawns_a_light_source_at_player_position() {
        let mut world = World::default();
        let mut lighting = Lighting::default();
        let mut ctx = PlaceCtx {
            player_pos: (3, 4),
            world: &mut world,
            lighting: &mut lighting,
        };
        let outcome = behavior_for(ItemKind::Torch).on_place(&mut ctx);
        assert_eq!(outcome, PlaceOutcome::TorchPlaced);
        let positions: Vec<(usize, usize)> =
            lighting.iter_sources(&world).map(|(p, _)| p).collect();
        assert_eq!(positions, vec![(3, 4)]);
    }

    #[test]
    fn flare_behavior_marks_the_room_as_flare_lit() {
        let mut world = World::default();
        let mut lighting = Lighting::default();
        let mut ctx = PlaceCtx {
            player_pos: (1, 1),
            world: &mut world,
            lighting: &mut lighting,
        };
        let outcome = behavior_for(ItemKind::Flare).on_place(&mut ctx);
        assert_eq!(outcome, PlaceOutcome::FlarePlaced);
        assert!(lighting.any_flare_active());
    }
}
