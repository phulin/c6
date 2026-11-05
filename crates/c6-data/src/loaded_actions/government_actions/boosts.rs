//! Boost.Manager.CheckGovernmentTier2845060 and its mandatory trigger.
use super::Definitions as Governments;
pub use crate::melee_attack::resolve::completion::boosts::{ResearchTrigger, Trigger};
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
pub struct Definitions {
    binding: WireBinding,
    rows: Vec<(i32, usize, Option<u32>)>,
}
#[derive(Debug)]
pub enum Error<E = std::convert::Infallible> {
    Rules(crate::Error),
    Binding,
    Missing(&'static str),
    Trigger(E),
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
            if row.text("BoostClass").map_err(Error::Rules)? != "BOOST_TRIGGER_HAVE_GOVERNMENT_TIER"
            {
                continue;
            }
            let tier = match row.get("GovernmentTierType").map_err(Error::Rules)? {
                SqlValue::Null => None,
                SqlValue::Text(name) => Some(linux_type_hash(name)),
                _ => return Err(Error::Missing("GovernmentTierType")),
            };
            let id = row
                .integer("BoostID")
                .map_err(Error::Rules)?
                .try_into()
                .map_err(|_| Error::Missing("BoostID"))?;
            rows.push((id, index, tier));
        }
        Ok(Self {
            binding: state.binding().clone(),
            rows,
        })
    }
}
pub fn enacted<T: Trigger>(
    state: &mut LoadedState,
    definitions: &Definitions,
    governments: &Governments,
    owner: i32,
    government: i32,
    row_id: i32,
    trigger: &mut T,
) -> Result<(), Error<T::Error>> {
    if state.binding() != &definitions.binding || state.binding() != &governments.binding {
        return Err(Error::Binding);
    }
    let (_, index, tier) = definitions
        .rows
        .iter()
        .find(|r| r.0 == row_id)
        .ok_or(Error::Missing("government Boost definition"))?;
    if government == -1 || tier.is_none() {
        return Ok(());
    }
    if governments.government_tier(government) == *tier {
        trigger
            .trigger(state, owner, *index)
            .map_err(Error::Trigger)?;
    }
    Ok(())
}
