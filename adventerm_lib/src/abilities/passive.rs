use crate::abilities::PassiveKind;
use crate::stats::Stats;

/// Mutable view of a combatant's stat block exposed to a passive. Empty for
/// now beyond the stats themselves; new fields land here when their first
/// passive needs them.
pub struct PassiveCtx<'a> {
    pub stats: &'a mut Stats,
}

/// Trait every passive kind implements to declare what it does at the start
/// of a turn (or whatever hook the engine wires up later). Empty default
/// methods keep this minimal until the first passive lands.
pub trait PassiveAbility {
    /// Hook fired once per turn while the passive is equipped. Default no-op.
    fn on_turn_start(&self, _ctx: &mut PassiveCtx<'_>) {}
}

/// The single point in the codebase that enumerates `PassiveKind`. Currently
/// the empty match — `PassiveKind` has no variants. Adding a passive: add a
/// variant, add a module with a behavior impl, and add one arm here.
pub fn passive_behavior_for(kind: PassiveKind) -> &'static dyn PassiveAbility {
    match kind {}
}
