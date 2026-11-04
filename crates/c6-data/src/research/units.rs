//! Player.Units.DoTechUpdates over the current native-order retained roster.
//! Formation updates execute for every live unit, including shared formations;
//! callbacks observe the same mutable state and may append subsequent members.
use crate::{loaded_upkeep, unit_movement_state::Inputs};
use c6_core::{formation_upkeep::FormationUpkeepError, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error<E> {
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    Formation(FormationUpkeepError<loaded_upkeep::Error<E>>),
}
pub fn update_formations<E: loaded_upkeep::Effects>(
    state: &mut LoadedState,
    owner: i32,
    inputs: &Inputs<'_>,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    let mut index = 0;
    loop {
        let player = state.world().player(owner).map_err(Error::World)?;
        let slots = player.unit_records().map_err(Error::World)?;
        let Some(slot) = slots.get(index) else {
            return Ok(());
        };
        if !slot.value.b_delayed_death {
            let id = slot.value.id;
            if let Some(found) = player.unit(id).map_err(Error::World)? {
                if let Some(formation) = state
                    .unit_runtime(owner, found.resolved_id)
                    .map_err(Error::State)?
                    .formation
                {
                    loaded_upkeep::update_formation(
                        state,
                        formation.owner,
                        formation.requested_id,
                        inputs,
                        effects,
                    )
                    .map_err(Error::Formation)?;
                }
            }
        }
        index += 1;
    }
}
