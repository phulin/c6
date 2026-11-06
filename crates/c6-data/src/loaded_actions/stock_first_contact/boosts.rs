//! Definition-bound PlayersMet boost subscribers (Linux classes zero and35).
use super::Error;
use crate::{melee_attack::resolve::completion::boosts::Trigger, Rules, SqlValue};
use c6_core::loaded_state::LoadedState;
struct Boost {
    row: i32,
    index: usize,
    class: i32,
    count: i32,
}
pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    boosts: Vec<Boost>,
}
fn error(value: impl std::fmt::Debug) -> Error {
    Error::Requirement(format!("first-contact boost: {value:?}"))
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let names = rules
            .gameplay()
            .table("BoostNames")
            .ok_or(Error::Missing("first-contact BoostNames"))?;
        let rows = rules
            .gameplay()
            .table("Boosts")
            .ok_or(Error::Missing("first-contact Boosts"))?;
        let mut boosts = Vec::new();
        for (index, row) in rows.rows().enumerate() {
            let name = row.text("BoostClass").map_err(error)?;
            let class = names
                .rows()
                .find(|r| r.text("BoostType").ok() == Some(name))
                .ok_or(Error::Missing("first-contact BoostName"))?
                .integer("BoostValue")
                .map_err(error)?;
            if ![0, 13, 35].contains(&class) {
                continue;
            }
            let count = match row.get("NumItems").map_err(error)? {
                SqlValue::Integer(n) => i32::try_from(*n).map_err(error)?,
                SqlValue::Null => 0,
                _ => return Err(Error::Missing("first-contact NumItems")),
            };
            boosts.push(Boost {
                row: i32::try_from(row.integer("BoostID").map_err(error)?).map_err(error)?,
                index,
                class: i32::try_from(class).map_err(error)?,
                count,
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            boosts,
        })
    }
    /// `ever_alive_minors` is the actual GetWasEverAliveMinors roster, including
    /// dead players; an alive-only filter changes the native count.
    #[allow(clippy::too_many_arguments)]
    pub fn on_met<T: Trigger>(
        &self,
        state: &mut LoadedState,
        row: i32,
        levels: &[(i32, u32)],
        ever_alive_minors: &[i32],
        first: i32,
        second: i32,
        trigger: &mut T,
    ) -> Result<(), Error>
    where
        T::Error: std::fmt::Debug,
    {
        if state.binding() != &self.binding {
            return Err(Error::Missing("first-contact boost identity"));
        }
        let boost = self
            .boosts
            .iter()
            .find(|b| b.row == row)
            .ok_or(Error::Missing("first-contact subscribed boost definition"))?;
        if boost.class == 0 {
            for owner in [first, second] {
                if levels
                    .iter()
                    .find(|p| p.0 == owner)
                    .ok_or(Error::Missing("first-contact boost civilization level"))?
                    .1
                    != c6_core::diplomacy_meeting::MAJOR_LEVEL
                {
                    return Ok(());
                }
            }
            for owner in [first, second] {
                trigger.trigger(state, owner, boost.index).map_err(error)?;
            }
        } else if boost.class == 35 {
            for owner in [first, second] {
                let mut count = 0i32;
                for &other in ever_alive_minors {
                    if other != owner
                        && state
                            .game_relations()
                            .map_err(Error::State)?
                            .has_met(owner, other)
                            .map_err(error)?
                    {
                        count = count.wrapping_add(1);
                    }
                }
                if count >= boost.count {
                    trigger.trigger(state, owner, boost.index).map_err(error)?;
                }
            }
        } else {
            return Err(Error::Missing(
                "first-contact percent-civs/district boost continuation",
            ));
        }
        Ok(())
    }
}
