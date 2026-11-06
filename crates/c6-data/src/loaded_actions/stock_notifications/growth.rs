//! Population callbacks during ordinary city growth on the persistent stock
//! runtime. Registration order is supplied by current inputs; gameplay owners
//! are shared with founding, production and movement.
use super::Runtime;
use crate::city_turn::{self, religion, religion_callbacks as callbacks, Error, GrowthRules};
use c6_core::loaded_state::LoadedState;

pub struct Inputs {
    pub rules: GrowthRules,
    /// Current concrete diplomatic Subject modifier owners reached by growth.
    /// Absence rejects changed requirements after preserving their state write.
    pub requirement_effects:
        Option<crate::loaded_actions::declare_war::requirement_modifiers::Modifiers>,
    pub population: Vec<callbacks::PopulationListener>,
    pub religion: Vec<callbacks::CityListener>,
    pub quests: Vec<callbacks::CurrentQuest>,
    /// The qualified stock PlayerReligionChanged recipient returns for NONE.
    pub player_religion_stock: bool,
    pub scope: Scope,
}
pub enum Scope {
    StockPlainHeadlessNoAddedConsumers,
}
impl<P> Runtime<'_, P> {
    /// Current lifecycle state for the concrete growth requirement effects.
    pub fn growth_requirement_effects(
        &self,
    ) -> Option<&crate::loaded_actions::declare_war::requirement_modifiers::Modifiers> {
        self.growth.as_ref()?.requirement_effects.as_ref()
    }

    pub fn bind_growth(&mut self, state: &LoadedState, inputs: Inputs) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("growth runtime binding"))?;
        if self.growth.is_some() {
            return Err(Error::Unsupported("growth callbacks already bound"));
        }
        self.growth = Some(inputs);
        Ok(())
    }
}
struct RequirementEffects<'a>(
    Option<&'a mut crate::loaded_actions::declare_war::requirement_modifiers::Modifiers>,
);
impl callbacks::RequirementEffects for RequirementEffects<'_> {
    fn changed(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut c6_core::melee_attack::proposals::subject_sets::Runtime,
        label: u64,
        old: i32,
        new: i32,
    ) -> Result<(), callbacks::Error> {
        let effects = self
            .0
            .as_mut()
            .ok_or(callbacks::Error::ChangedRequirement { label, old, new })?;
        effects
            .requirement_changed(state, requirements, label, old, new)
            .map_err(|e| callbacks::Error::RequirementContinuation(format!("{e:?}")))
    }
}
struct Script<'a, P>(&'a mut crate::loaded_actions::stock_script::Runtime<P>);
impl<P> callbacks::PopulationScript for Script<'_, P> {
    fn on_city_population_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        delta: i32,
    ) -> Result<(), callbacks::Error> {
        self.0
            .call_empty(
                state,
                "OnCityPopulationChanged",
                &[owner, city as i32, delta],
            )
            .map_err(|_| callbacks::Error::Unsupported("population Lua subscriber"))
    }
}
impl<P> city_turn::PopulationEffects for Runtime<'_, P> {
    fn assign_citizens(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: bool,
    ) -> Result<(), Error> {
        Err(Error::Unsupported(
            "population assignment needs current upkeep worker services",
        ))
    }
    fn population_changed_signal(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        delta: i32,
    ) -> Result<(), Error> {
        let inputs = self
            .growth
            .as_mut()
            .ok_or(Error::Unsupported("current growth registrations"))?;
        let owners = self
            .modifier_owners
            .as_mut()
            .ok_or(Error::Unsupported("current growth modifier owner"))?;
        callbacks::population_changed(
            state,
            owner,
            city,
            delta,
            &inputs.population,
            owners.requirements.sets_mut(),
            &mut Script(&mut self.script),
            &mut RequirementEffects(inputs.requirement_effects.as_mut()),
        )
        .map_err(Error::ReligionCallback)
    }
    fn report_population(
        &mut self,
        state: &mut LoadedState,
        _: i32,
        _: u32,
        event: u32,
        _: i32,
    ) -> Result<(), Error> {
        self.growth
            .as_ref()
            .ok_or(Error::Unsupported("current growth report scope"))?;
        if event != city_turn::POPULATION_REPORT {
            return Err(Error::Unsupported("unknown population report"));
        }
        // Stock City/Culture/Religion/Tech/Treasury consumers publish current
        // snapshots. The already-qualified no-route branch has no retained
        // gameplay or RNG work; route-query branches remain explicit.
        if !state
            .trade_graph()
            .map_err(|_| Error::Unsupported("current growth trade graph"))?
            .graph()
            .origins()
            .is_empty()
        {
            return Err(Error::Unsupported("population report route queries"));
        }
        Ok(())
    }
}
impl<P> religion::Effects for Runtime<'_, P> {
    fn civilization_level(&mut self, _: &LoadedState, owner: i32) -> Result<u32, Error> {
        self.movement()
            .map_err(|_| Error::Unsupported("current growth levels"))?
            .civilization_levels
            .iter()
            .find(|v| v.0 == owner)
            .map(|v| v.1)
            .ok_or(Error::Unsupported("current growth civilization level"))
    }
    fn give_free_envoy(&mut self, _: &mut LoadedState, _: i32, _: i32) -> Result<(), Error> {
        Err(Error::Unsupported("growth religion envoy"))
    }
    fn city_religion_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        old: religion::ReligionIdentity,
        new: religion::ReligionIdentity,
    ) -> Result<(), Error> {
        let inputs = self
            .growth
            .as_mut()
            .ok_or(Error::Unsupported("current growth religion registrations"))?;
        let owners = self
            .modifier_owners
            .as_mut()
            .ok_or(Error::Unsupported("current growth modifier owner"))?;
        callbacks::city_changed(
            state,
            owner,
            city,
            old.native_type,
            new.native_type,
            &inputs.religion,
            &inputs.quests,
            &inputs.rules,
            owners.requirements.sets_mut(),
            &mut RequirementEffects(inputs.requirement_effects.as_mut()),
        )
        .map_err(Error::ReligionCallback)
    }
    fn report_city_religion(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        event: u32,
        _: Option<religion::ReligionIdentity>,
    ) -> Result<(), Error> {
        self.growth
            .as_ref()
            .ok_or(Error::Unsupported("current growth report scope"))?;
        match event {
            // Both source handlers forward to the same derived City cache.
            0x51c84cba | 0x99e85ec6 => Ok(()),
            _ => Err(Error::Unsupported("unknown growth religion report")),
        }
    }
    fn player_religion_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: i32,
        new: religion::ReligionIdentity,
    ) -> Result<(), Error> {
        if !self
            .growth
            .as_ref()
            .ok_or(Error::Unsupported("current growth registrations"))?
            .player_religion_stock
        {
            return Err(Error::Unsupported("growth PlayerReligionChanged recipient"));
        }
        callbacks::player_changed_no_religion(new.native_type).map_err(Error::ReligionCallback)
    }
    fn city_converted_by(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: i32,
        _: u32,
    ) -> Result<(), Error> {
        Err(Error::Unsupported("growth city conversion"))
    }
    fn alive_players(&mut self, _: &LoadedState) -> Result<Vec<i32>, Error> {
        Ok(self
            .player_services
            .as_ref()
            .and_then(|v| v.alive_players.as_ref())
            .ok_or(Error::Unsupported("current growth alive players"))?
            .values()
            .to_vec())
    }
}
