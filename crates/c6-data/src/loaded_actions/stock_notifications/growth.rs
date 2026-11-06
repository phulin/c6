//! Population callbacks during ordinary city growth on the persistent stock
//! runtime. Registration order is supplied by current inputs; gameplay owners
//! are shared with founding, production and movement.
use super::Runtime;
use crate::city_turn::{self, religion, religion_callbacks as callbacks, Error, GrowthRules};
use c6_core::loaded_state::LoadedState;

pub struct Inputs {
    pub rules: GrowthRules,
    /// Source-qualified default manual ComponentID/text notice types. Unknown
    /// factories or added gameplay consumers remain unsupported when reached.
    pub notification_types: Vec<u32>,
    /// Current concrete diplomatic Subject modifier owners reached by growth.
    /// Absence rejects changed requirements after preserving their state write.
    pub requirement_effects:
        Option<crate::loaded_actions::declare_war::requirement_modifiers::Modifiers>,
    /// Actual stock report subscribers and null synchronous host hooks, with
    /// the existing source-qualified warm domestic query closure.
    pub population_active_trade_report: bool,
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

    /// Borrow the sole current diplomatic modifier owner across a callback.
    /// Restore it after errors so partial native transitions remain inspectable.
    pub(crate) fn with_growth_requirement_effects<T>(
        &mut self,
        state: &mut LoadedState,
        apply: impl FnOnce(
            &mut Self,
            &mut LoadedState,
            &mut crate::loaded_actions::declare_war::requirement_modifiers::Modifiers,
        ) -> Result<T, &'static str>,
    ) -> Result<T, &'static str> {
        self.check(state)
            .map_err(|_| "growth modifier runtime binding")?;
        let mut effects = self
            .growth
            .as_mut()
            .and_then(|inputs| inputs.requirement_effects.take())
            .ok_or("current growth modifier owner")?;
        let result = apply(self, state, &mut effects);
        self.growth
            .as_mut()
            .expect("bound growth owner")
            .requirement_effects = Some(effects);
        result
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
    fn publish_growth_notification(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        notification: u32,
    ) -> Result<(), Error> {
        if !matches!(
            notification,
            0xba4af0fb | 0x9b27fd19 | 0x9efbc217 | 0xffad0d16
        ) || !self
            .growth
            .as_ref()
            .is_some_and(|input| input.notification_types.contains(&notification))
        {
            return Err(Error::Unsupported(
                "current growth notification factory/consumers",
            ));
        }
        let current = state.city(owner, city)?;
        let location = [current.header.location_x, current.header.location_y];
        // Native 2d4b510 sets the City ComponentID, message and summary before
        // pointer Send. Plain setters/localization and default OnAdd have no
        // gameplay/RNG effects in this qualified scope. Preserve semantic output.
        self.publish_notice(
            state,
            super::Notice::CityGrowth {
                owner,
                city,
                type_hash: notification,
                location,
            },
        )
        .map_err(|error| {
            self.last_error = Some(format!("{error:?}"));
            Error::Unsupported("stock growth notification publication")
        })
    }
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
        // Dispatch individually so a positive Boost can reenter the canonical
        // research/requirements owners between the surrounding recipients.
        let listeners = self
            .growth
            .as_ref()
            .ok_or(Error::Unsupported("current growth registrations"))?
            .population
            .clone();
        for listener in &listeners {
            if matches!(listener, callbacks::PopulationListener::Boost { .. }) {
                if let Some(definition) = callbacks::population_boost(state, owner, city, listener)
                    .map_err(Error::ReligionCallback)?
                {
                    self.population_research_boost(state, owner, definition)?;
                }
                continue;
            }
            let inputs = self.growth.as_mut().unwrap();
            let owners = self
                .modifier_owners
                .as_mut()
                .ok_or(Error::Unsupported("current growth modifier owner"))?;
            callbacks::population_changed(
                state,
                owner,
                city,
                delta,
                std::slice::from_ref(listener),
                owners.requirements.sets_mut(),
                &mut Script(&mut self.script),
                &mut RequirementEffects(inputs.requirement_effects.as_mut()),
            )
            .map_err(Error::ReligionCallback)?;
        }
        Ok(())
    }

    fn report_population(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
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
        // snapshots. Qualified warm domestic branches execute current numerical
        // guards; unknown consumers and unsupported route queries remain errors.
        if !state
            .trade_graph()
            .map_err(|_| Error::Unsupported("current growth trade graph"))?
            .graph()
            .origins()
            .is_empty()
        {
            if !self
                .growth
                .as_ref()
                .is_some_and(|input| input.population_active_trade_report)
            {
                return Err(Error::Unsupported("population report route queries"));
            }
            self.population_trade_queries(state, owner)
                .map_err(|error| {
                    self.last_error = Some(error);
                    Error::Unsupported("population current active trade query")
                })?;
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
