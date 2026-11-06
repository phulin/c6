//! First-activation maintenance on the same persistent stock action owner.
//! No ScriptState, world or rule cache is cloned. Positive callbacks outside
//! the implemented current profile stop after their actual preceding stores.
use super::Runtime;
use crate::{
    game_parameters::{IntegerParameter, Parameters},
    game_update_state::activation::maintenance as data,
    loaded_actions::{city_range_attack as combat, notifications::processing},
    resource_binding::ResourceAuthority,
    Rules,
};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

#[derive(Debug)]
pub enum DefinitionError {
    Maintenance(data::Error<std::convert::Infallible>),
    Combat(crate::loaded_actions::Error),
    Parameters(crate::game_parameters::Error),
}
/// Build once from the initialized world's admitted definitions. Military
/// strength needs no terrain, pathfinder or placement runtime.
pub struct Definitions {
    binding: WireBinding,
    pub maintenance: data::Definitions,
    pub combat: combat::Definitions,
    pub max_hit_points: i32,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, DefinitionError> {
        let maintenance = data::Definitions::from_rules(rules, authority)
            .map_err(DefinitionError::Maintenance)?;
        let combat = combat::Definitions::from_rules(state, rules, authority)
            .map_err(DefinitionError::Combat)?;
        let max_hit_points = Parameters::from_rules(rules)
            .map_err(DefinitionError::Parameters)?
            .integer(IntegerParameter::CombatMaxHitPoints)
            .map_err(DefinitionError::Parameters)?;
        Ok(Self {
            binding: state.binding().clone(),
            maintenance,
            combat,
            max_hit_points,
        })
    }
}
#[derive(Debug)]
pub enum Error {
    Backend(processing::Error),
    ReportingBlock(&'static str),
    FuelUsage {
        owner: i32,
        resource: usize,
        demand: i32,
    },
    CityYield {
        owner: i32,
        city: u32,
        yield_index: i32,
    },
    ScienceVictoryChanged {
        owner: i32,
    },
    EraChanged {
        owner: i32,
        old: i32,
        new: i32,
    },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock activation maintenance: {self:?}")
    }
}
impl std::error::Error for Error {}
impl<P> data::Effects for Runtime<'_, P> {
    type Error = Error;
    fn report_block(
        &mut self,
        state: &mut LoadedState,
        hash: u32,
        _begin: bool,
    ) -> Result<(), Self::Error> {
        self.check(state).map_err(Error::Backend)?;
        data::stock_fuel_report_block(hash).map_err(Error::ReportingBlock)
    }
    fn fuel_usage(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        resource: usize,
        demand: i32,
    ) -> Result<(), Self::Error> {
        self.check(state).map_err(Error::Backend)?;
        Err(Error::FuelUsage {
            owner,
            resource,
            demand,
        })
    }
    fn city_yield(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        yield_index: i32,
    ) -> Result<i32, Self::Error> {
        self.check(state).map_err(Error::Backend)?;
        self.current_city_yield(state, owner, city, yield_index, 0)
            .map_err(|_| Error::CityYield {
                owner,
                city,
                yield_index,
            })
    }
    fn science_victory_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<(), Self::Error> {
        self.check(state).map_err(Error::Backend)?;
        Err(Error::ScienceVictoryChanged { owner })
    }
    fn era_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        old: i32,
        new: i32,
    ) -> Result<(), Self::Error> {
        self.check(state).map_err(Error::Backend)?;
        self.last_error = None;
        self.player_era_changed(state, owner, old, new)
            .map_err(|error| {
                self.last_error
                    .get_or_insert_with(|| format!("era change: {error}"));
                Error::EraChanged { owner, old, new }
            })
    }
}
/// Borrow at the source phase; these entry points do not reorder surrounding
/// activation, choose policy orders, or duplicate later per-Unit healing.
pub struct Context<'s, 'a, P> {
    pub state: &'s mut LoadedState,
    pub backend: &'s mut Runtime<'a, P>,
    pub definitions: &'s Definitions,
}
impl<P> Context<'_, '_, P> {
    fn check(&self) -> Result<(), data::Error<Error>> {
        self.backend
            .check(self.state)
            .map_err(|e| data::Error::Effect(Error::Backend(e)))?;
        if self.definitions.binding != *self.state.binding() {
            return Err(data::Error::Missing("stock maintenance definition binding"));
        }
        Ok(())
    }
    pub fn units(
        &mut self,
        owner: i32,
        current_unit_order: &[u32],
        major: bool,
    ) -> Result<(), data::Error<Error>> {
        self.check()?;
        data::units_turn_activate(
            self.state,
            &self.definitions.maintenance,
            owner,
            current_unit_order,
            major,
            self.backend,
        )
    }
    pub fn stats(
        &mut self,
        owner: i32,
        current_unit_order: &[u32],
        current_city_order: Option<&[u32]>,
    ) -> Result<(), data::Error<Error>> {
        self.check()?;
        data::stats_turn_activate(
            self.state,
            &self.definitions.maintenance,
            owner,
            current_unit_order,
            current_city_order,
            &self.definitions.combat,
            self.definitions.max_hit_points,
            self.backend,
        )
    }
    pub fn eras(&mut self, owner: i32) -> Result<(), data::Error<Error>> {
        self.check()?;
        data::eras_turn_activate(
            self.state,
            &self.definitions.maintenance,
            owner,
            self.backend,
        )
    }
    /// Major AnalyzeForces' demand prefix immediately before AnalyzeExplorers.
    /// These current input values are supplied by the initialized runtime,
    /// independently of any observed final role/count fields.
    pub fn roles(
        &mut self,
        owner: i32,
        current_unit_order: &[u32],
        major: bool,
        global_ai_initialized: bool,
        inputs: &data::ScoutInputs,
    ) -> Result<(), data::Error<Error>> {
        self.check()?;
        if !major {
            return Ok(());
        }
        data::prepare_explorer_demand(
            self.state,
            &self.definitions.maintenance,
            owner,
            major,
            inputs,
        )?;
        data::analyze_explorers(
            self.state,
            &self.definitions.maintenance,
            owner,
            current_unit_order,
            global_ai_initialized,
        )
    }
}
