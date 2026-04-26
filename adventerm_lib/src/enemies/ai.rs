use crate::enemies::kind::EnemyKind;
use crate::enemies::slime::SlimeAi;
use crate::rng::Rng;
use crate::room::Room;
use crate::world::Direction;

/// What an enemy chooses to do on its turn. The engine applies the action;
/// the AI itself never mutates the room directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiAction {
    Wait,
    Step(Direction),
}

/// Read-only view handed to an enemy's AI. The AI does not own movement —
/// that's the movement system's job. Mutating randomness is the only
/// non-pure piece, so the `&mut Rng` is the only `&mut` here.
pub struct AiCtx<'a> {
    pub enemy_pos: (usize, usize),
    pub player_pos: (usize, usize),
    pub room: &'a Room,
    pub rng: &'a mut Rng,
}

/// Trait every enemy kind implements to declare what it does on its turn.
pub trait EnemyAi {
    fn decide(&self, ctx: &mut AiCtx<'_>) -> AiAction;
}

/// The single point in the codebase that enumerates `EnemyKind`. Adding a
/// new enemy: add a variant, add a new module with an AI impl, and add one
/// arm here.
pub fn enemy_behavior_for(kind: EnemyKind) -> &'static dyn EnemyAi {
    match kind {
        EnemyKind::Slime => &SlimeAi,
    }
}
