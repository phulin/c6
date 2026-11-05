//! Player.TurnManager.DoEndTurn over the same unit, city and effect owners.
use super::{movement, unit_creation, Runtime};
use crate::{
    game_update_state::{round_cities, round_units},
    unit_kill,
};
use c6_core::loaded_state::LoadedState;

/// Current Game.Signals+1518 registration admitted from the actual initial
/// owner. A nonempty ring requires its ordered concrete handlers.
pub struct UnitHealedSignals {
    binding: c6_core::core_player::initialization::WireBinding,
}
impl UnitHealedSignals {
    pub fn from_initial(
        state: &LoadedState,
        binding: &c6_core::core_player::initialization::WireBinding,
        subscription_count: usize,
    ) -> Result<Self, movement::Error> {
        if state.binding() != binding || !state.is_pristine() {
            return Err(movement::Error::Missing("UnitHealed initial binding"));
        }
        if subscription_count != 0 {
            return Err(movement::Error::Missing("UnitHealed ordered handlers"));
        }
        Ok(Self {
            binding: binding.clone(),
        })
    }
}
pub struct Inputs<'a, 'd, 'r> {
    pub units: round_units::Inputs<'a, 'd, 'r>,
    pub cities: round_cities::Inputs<'a>,
    pub combat: &'a crate::loaded_actions::city_range_attack::Definitions,
    pub sight: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
}
#[derive(Debug)]
pub struct Error {
    pub stage: &'static str,
    pub cause: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round end {}: {}", self.stage, self.cause)
    }
}
impl std::error::Error for Error {}
fn failure(stage: &'static str, cause: impl std::fmt::Debug) -> Error {
    Error {
        stage,
        cause: format!("{cause:?}"),
    }
}
impl<P> crate::unit_placement_state::round::Effects for Runtime<'_, P> {
    fn set_unit_activity(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        activity: i32,
    ) -> Result<(), movement::Error> {
        crate::loaded_upkeep::set_unit_activity(state, actor.0, actor.1, self, activity)
            .map_err(|error| movement::Error::Requirement(format!("round activity: {error:?}")))
    }
    fn unit_healed(
        &mut self,
        state: &mut LoadedState,
        _: (i32, u32),
        _: c6_core::unit_healing::UnitHealedSignal,
    ) -> Result<(), movement::Error> {
        let signals = self
            .round_unit_healed
            .as_ref()
            .ok_or(movement::Error::Missing(
                "round current UnitHealed registrations",
            ))?;
        if state.binding() != &signals.binding {
            return Err(movement::Error::Missing("round UnitHealed binding"));
        }
        // HealUnit2bf5090 dispatches even when requested healing clamps to
        // actual zero. The admitted current empty ring has no callbacks/RNG.
        Ok(())
    }
}
impl<P> round_cities::Effects for Runtime<'_, P> {
    type Error = movement::Error;
    fn damage_changed(
        &mut self,
        _: &mut LoadedState,
        _: round_cities::DamageChanged,
    ) -> Result<(), Self::Error> {
        Err(movement::Error::Missing(
            "round positive city damage report consumers",
        ))
    }
}
struct Cleanup<'a>(&'a mut unit_creation::ModifierOwners);
impl<P> unit_kill::Continuations<Runtime<'_, P>> for Cleanup<'_> {
    type Error = movement::Error;
}
impl<P> unit_kill::objects::Services<Runtime<'_, P>> for Cleanup<'_> {
    fn unit_cleanup_runtime(
        &self,
    ) -> Result<
        &crate::melee_attack::backend::EffectRuntime,
        crate::melee_attack::backend::QueryError,
    > {
        Ok(&self.0.effects)
    }
    fn unit_cleanup_runtime_mut(
        &mut self,
    ) -> Result<
        &mut crate::melee_attack::backend::EffectRuntime,
        crate::melee_attack::backend::QueryError,
    > {
        Ok(&mut self.0.effects)
    }
}
impl<P> Runtime<'_, P> {
    pub fn bind_round_unit_healed(&mut self, signals: UnitHealedSignals) {
        self.round_unit_healed = Some(signals);
    }
    /// Units (including their gathered immediate Destroy list), then Cities,
    /// then Culture. A failure retains the executed prefix and actual owners.
    pub fn end_player_round(
        &mut self,
        state: &mut LoadedState,
        inputs: &Inputs<'_, '_, '_>,
        owner: i32,
    ) -> Result<(), Error> {
        let mut sight = inputs
            .sight
            .try_borrow_mut()
            .map_err(|e| failure("round sight", e))?;
        let mut frame =
            round_units::begin(state, owner).map_err(|e| failure("round units begin", e))?;
        while matches!(
            round_units::advance(state, &mut frame, &inputs.units, &mut sight, self)
                .map_err(|e| failure("round unit", e))?,
            round_units::Progress::Advanced
        ) {}
        for actor in frame
            .into_delayed(state)
            .map_err(|e| failure("round delayed list", e))?
        {
            let mut owners = self
                .modifier_owners
                .take()
                .ok_or_else(|| failure("round destruction", "current modifier owner"))?;
            let result = unit_kill::immediate::destroy_delayed(
                state,
                &mut sight,
                self,
                &mut Cleanup(&mut owners),
                inputs.combat,
                actor,
            );
            self.modifier_owners = Some(owners);
            result.map_err(|e| failure("round delayed destruction", (actor, e)))?;
        }
        drop(sight);
        let mut frame =
            round_cities::begin(state, owner).map_err(|e| failure("round cities begin", e))?;
        while !matches!(
            round_cities::advance(state, &mut frame, &inputs.cities, self)
                .map_err(|e| failure("round city", e))?,
            round_cities::Progress::Complete
        ) {}
        c6_core::loaded_state::game_update::round::end_player_culture(state, owner)
            .map_err(|e| failure("round culture", e))
    }
}
