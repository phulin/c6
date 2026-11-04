//! Computed movement-rule leaves on the authoritative loaded Unit and Map.
//! Definition facts are admitted rules inputs; report handlers operate on the
//! same current state. This module does not accept a complete MoveTo action.
use super::{LoadedState, UnitHandle};
use crate::{
    loaded_actions::{city_range_attack, Error as QueryError},
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    unit::{PlotCoord, Unit},
    unit_placement_tails::{self, BonusContext, SpreadChargesReport},
};

pub struct Definitions {
    binding: WireBinding,
    features: LinuxDefinitionRegistry,
    natural_wonders: Vec<bool>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, QueryError> {
        if state.binding() != bound.destination_binding() {
            return Err(QueryError::ConfigurationIdentity);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(|_| QueryError::Unsupported("placement tail admitted rules/resources"))?;
        let features = collection(rules, "Features", "FeatureType")
            .map_err(|_| QueryError::Unsupported("Features registry"))?;
        let table = rules
            .gameplay()
            .table("Features")
            .ok_or(QueryError::Unsupported("Features"))?;
        let natural_wonders = features
            .entries()
            .iter()
            .map(|entry| {
                match table
                    .row_by_id(entry.database_rowid)
                    .ok_or(QueryError::Unsupported("Features rowid"))?
                    .get("NaturalWonder")
                    .map_err(|_| QueryError::Unsupported("Features.NaturalWonder"))?
                {
                    SqlValue::Integer(0) => Ok(false),
                    SqlValue::Integer(1) => Ok(true),
                    _ => Err(QueryError::Unsupported("Features.NaturalWonder scalar")),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            features,
            natural_wonders,
        })
    }
    pub fn natural_wonder(&self, state: &LoadedState, plot: u32) -> Result<bool, QueryError> {
        if state.binding() != &self.binding {
            return Err(QueryError::ConfigurationIdentity);
        }
        let hash = state
            .world()
            .map()
            .plot_by_index(plot)
            .ok_or(QueryError::Unsupported("current natural wonder plot"))?
            .feature_hash;
        if hash == u32::MAX {
            return Ok(false);
        }
        let Some(index) = self.features.find_index(hash) else {
            return Ok(false);
        };
        Ok(self.natural_wonders[index])
    }
}

pub trait Reports {
    type Error;
    fn preflight_spread_report(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
    ) -> Result<(), Self::Error>;
    fn spread_report(
        &mut self,
        state: &mut LoadedState,
        report: SpreadChargesReport,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Query(QueryError),
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    Report(E),
    MissingPlot,
}
struct Context<'a, E> {
    state: &'a mut LoadedState,
    actor: UnitHandle,
    definitions: &'a Definitions,
    effects: &'a mut E,
}
impl<E: Reports> BonusContext for Context<'_, E> {
    type Error = Error<E::Error>;
    type Map = ();
    type Plot = u32;
    fn map(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor.get(self.state).map_err(Error::World)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.actor.get_mut(self.state).map_err(Error::State)
    }
    fn adjacent(
        &mut self,
        _: (),
        location: PlotCoord,
        direction: i32,
    ) -> Result<Option<u32>, Self::Error> {
        let Some((x, y)) = city_range_attack::adjacent_location(
            self.state,
            (location.x, location.y),
            direction as usize,
        ) else {
            return Ok(None);
        };
        let index = y
            .wrapping_mul(self.state.world().map().header.width)
            .wrapping_add(x) as u32;
        self.state
            .world()
            .map()
            .plot_by_index(index)
            .ok_or(Error::MissingPlot)?;
        Ok(Some(index))
    }
    fn natural_wonder(&mut self, plot: u32) -> Result<bool, Self::Error> {
        self.definitions
            .natural_wonder(self.state, plot)
            .map_err(Error::Query)
    }
    fn preflight_spread_report(&self) -> Result<(), Self::Error> {
        self.effects
            .preflight_spread_report(self.state, self.actor)
            .map_err(Error::Report)
    }
    fn spread_report(&mut self, report: SpreadChargesReport) -> Result<(), Self::Error> {
        self.effects
            .spread_report(self.state, report)
            .map_err(Error::Report)
    }
}
pub fn award_post_move_bonuses<E: Reports>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    destination: PlotCoord,
    definitions: &Definitions,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    if state.binding() != &definitions.binding {
        return Err(Error::Query(QueryError::ConfigurationIdentity));
    }
    unit_placement_tails::award_post_move_bonuses(
        &mut Context {
            state,
            actor: UnitHandle { owner, id },
            definitions,
            effects,
        },
        destination,
    )
}
