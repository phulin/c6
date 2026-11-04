//! Common UnitEffect live lookup plus concrete healing/movement scalar leaves.
//! DynamicModifier.ApplyEffect callbacks, suppression/stack gating and subject
//! bookkeeping remain with the enclosing modifier owner.
pub use c6_core::unit_modifier_effect::{Direction, Effect};
use c6_core::{loaded_state::LoadedState, map::UnitReference};
#[derive(Debug)]
pub enum Error {
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnitEffect: {self:?}")
    }
}
impl std::error::Error for Error {}
/// 2371be0/2371c20. None means a source-proven absent IUnitInterface;
/// unavailable interface/class inputs must remain errors in the caller.
/// A known null current Unit returns false without mutation. Each invocation
/// applies its stored amount once: the common wrapper ignores its int argument.
pub fn update(
    state: &mut LoadedState,
    subject: Option<UnitReference>,
    effect: Effect,
    direction: Direction,
) -> Result<bool, Error> {
    let Some(reference) = subject else {
        return Ok(false);
    };
    let Some(found) = state.world().unit(reference).map_err(Error::World)? else {
        return Ok(false);
    };
    let owner = found.record.e_owner;
    let id = found.record.id;
    if !effect.writes_unit() {
        return Ok(true);
    }
    let unit = state.unit_mut(owner, id).map_err(Error::State)?;
    // Native edit helpers only add archive dirty-pointer transport metadata;
    // typed clearDelta targets are RET in the audited Unit wrapper family.
    Ok(c6_core::unit_modifier_effect::update(
        unit, effect, direction,
    ))
}
pub fn apply(
    state: &mut LoadedState,
    subject: Option<UnitReference>,
    effect: Effect,
) -> Result<bool, Error> {
    update(state, subject, effect, Direction::Apply)
}
pub fn remove(
    state: &mut LoadedState,
    subject: Option<UnitReference>,
    effect: Effect,
) -> Result<bool, Error> {
    update(state, subject, effect, Direction::Remove)
}
