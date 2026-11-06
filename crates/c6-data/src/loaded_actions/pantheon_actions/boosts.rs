//! Source Boost.Manager.Initialize lambda2845b90 triggers its captured
//! BOOST_TRIGGER_CREATE_PANTHEON definition for the explicit founding owner.
use crate::{melee_attack::resolve::completion::boosts::Trigger, Rules};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error<E = std::convert::Infallible> {
    Binding,
    Missing(&'static str),
    Rules(crate::Error),
    Trigger(E),
}
pub struct Definitions {
    binding: WireBinding,
    rows: Vec<(i32, usize)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let mut rows = vec![];
        for (index, row) in rules
            .gameplay()
            .table("Boosts")
            .ok_or(Error::Missing("Boosts"))?
            .rows()
            .enumerate()
        {
            if row.text("BoostClass").map_err(Error::Rules)? == "BOOST_TRIGGER_CREATE_PANTHEON" {
                rows.push((
                    row.integer("BoostID")
                        .map_err(Error::Rules)?
                        .try_into()
                        .map_err(|_| Error::Missing("BoostID i32"))?,
                    index,
                ));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            rows,
        })
    }
}
pub fn created<T: Trigger>(
    state: &mut LoadedState,
    definitions: &Definitions,
    owner: i32,
    row_id: i32,
    trigger: &mut T,
) -> Result<(), Error<T::Error>> {
    if state.binding() != &definitions.binding {
        return Err(Error::Binding);
    }
    let index = definitions
        .rows
        .iter()
        .find(|r| r.0 == row_id)
        .ok_or(Error::Missing("captured pantheon Boost definition"))?
        .1;
    trigger.trigger(state, owner, index).map_err(Error::Trigger)
}
