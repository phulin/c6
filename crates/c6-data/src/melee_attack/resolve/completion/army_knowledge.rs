//! Player.Signals+78 stock Targets observation over the canonical knowledge.
//!
//! Linux XP2 callback 21abc70/21abcc0 reads the current attacker fields and
//! reaches CivKnowledge.UpdateUnit2001230 -> ArmyKnowledge.AddUnit1ffed10.
//! Creation and attacked observations use the same type buckets, first-ID
//! refresh/append ordering, current Area/optional Region and age reset. The
//! shared implementation preserves that state; this leaf has no RNG or policy.
use crate::registry::LinuxDefinitionRegistry;
use c6_core::{loaded_state::LoadedState, map::UnitReference};

pub type Error = crate::unit_creation::Error<std::convert::Infallible>;

/// Observe the live current Unit, including its post-combat damage. Unknown
/// canonical buckets or map dependencies are explicit errors in the shared
/// adapter, rather than replacement knowledge or a successful omission.
pub fn observe(
    state: &mut LoadedState,
    units: &LinuxDefinitionRegistry,
    observer: i32,
    attacker: UnitReference,
) -> Result<(), Error> {
    let targets = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == observer)
        .and_then(|p| p.ai_targets.as_ref())
        .ok_or(Error::Missing("current attacked Targets knowledge"))?;
    if targets.player_id != observer || targets.knowledge_base.owner != observer {
        return Err(Error::Missing("attacked Targets owner binding"));
    }
    crate::unit_creation::production_effects::observe_current_unit(
        state,
        units,
        observer,
        (attacker.player(), attacker.id),
    )
}
