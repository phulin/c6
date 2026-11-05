//! AdjustUnitSeeHidden235b2a0/235b2c0 and SetSeeHiddenType2c28ed0.
//! Initialize reads its boolean, but Apply always stores HIDDEN_VISIBILITY(0),
//! and Remove stores NO_HIDDEN_VISIBILITY(-1), independently of that argument.
use super::{Error, LoadedState};
/// Fresh initialization is the complete reached branch: the constructor has an
/// invalid location, so the setter has no sight callback or RNG. For an already
/// placed Unit, retain the native field prefix and expose the required UpdateMap
/// continuation rather than claiming that a sight refresh was executed.
pub fn apply(state: &mut LoadedState, actor: (i32, u32), remove: bool) -> Result<bool, Error> {
    let unit = super::super::actor::<Error>(state, actor.0, actor.1)
        .map_err(|_| Error::Missing("SeeHidden current Unit"))?;
    let target = if remove { -1 } else { 0 };
    if unit.k_sight_modifiers.e_can_see_hidden_type == target {
        return Ok(true);
    }
    let located = c6_core::skip_turn::placement::coordinate_valid(unit.k_location);
    state
        .unit_mut(actor.0, actor.1)
        .map_err(Error::State)?
        .k_sight_modifiers
        .e_can_see_hidden_type = target;
    if located {
        return Err(Error::Missing(
            "SeeHidden placed Unit Sight.UpdateMap continuation",
        ));
    }
    Ok(true)
}
