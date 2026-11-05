//! BeginTurn stock reset callbacks on the existing action and update owners.
use super::Runtime;
pub use c6_core::game_update::Error;
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::{
        game_update::{self as loaded, round_ai as core},
        LoadedState,
    },
};
pub use core::StockReset as Kind;
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockExternalPoliciesNoAddedConsumers,
}
pub struct Inputs {
    binding: WireBinding,
    pub owner: i32,
    pub listeners: Vec<Kind>,
}
impl Inputs {
    pub fn new(
        state: &LoadedState,
        owner: i32,
        listeners: Vec<Kind>,
        _: Scope,
    ) -> Result<Self, Error> {
        state
            .player_slot_status(owner)
            .map_err(|_| Error::Unsupported("round AI owner"))?;
        Ok(Self {
            binding: state.binding().clone(),
            owner,
            listeners,
        })
    }
}
fn dispatch_all<E>(
    listeners: &[Kind],
    mut callback: impl FnMut(Kind) -> Result<bool, E>,
) -> Result<bool, E> {
    let mut result = false;
    for &kind in listeners {
        result |= callback(kind)?;
    }
    Ok(result)
}
struct Effects<'a> {
    listeners: &'a [Kind],
}
impl core::PriorityEffects for Effects<'_> {
    fn priority_requested(
        &mut self,
        state: &mut LoadedState,
        _: &mut loaded::Runtime,
        owner: i32,
        priority: i32,
    ) -> Result<bool, Error> {
        if priority != -1 {
            return Err(Error::Unsupported("round begin priority"));
        }
        dispatch_all(self.listeners, |kind| core::stock_reset(state, owner, kind))
    }
}
impl<P> Runtime<'_, P> {
    pub fn bind_round_ai(&mut self, state: &LoadedState, inputs: Inputs) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("round AI backend binding"))?;
        if inputs.binding != *state.binding()
            || self.round_ai.iter().any(|v| v.owner == inputs.owner)
        {
            return Err(Error::Unsupported("round AI duplicate owner/binding"));
        }
        self.round_ai.push(inputs);
        Ok(())
    }
    pub fn begin_round_ai(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        owner: i32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("round AI backend binding"))?;
        let inputs = self
            .round_ai
            .iter()
            .find(|v| v.owner == owner)
            .ok_or(Error::Unsupported("current round AI subscribers"))?;
        if inputs.binding != *state.binding() {
            return Err(Error::Unsupported("round AI input binding"));
        }
        core::begin_turn(
            state,
            runtime,
            owner,
            &mut Effects {
                listeners: &inputs.listeners,
            },
        )?;
        self.reached.push("stock AI BeginTurn complete");
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collector_or_visits_every_subscriber_in_initialized_order() {
        let listeners = [Kind::City, Kind::Research, Kind::Culture];
        let mut seen = vec![];
        assert!(dispatch_all::<()>(&listeners, |k| {
            seen.push(k);
            Ok(k == Kind::City)
        })
        .unwrap());
        assert_eq!(seen, listeners);
        assert!(!dispatch_all::<()>(&[], |_| panic!("empty ring")).unwrap());
    }
}
