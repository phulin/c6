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

/// Coordinate IsVisible at Linux 0x2004030 reads only the initialized normal
/// shared layer. This borrows the current authoritative runtime used by sight
/// mutations. The Visibility instance's actual +0xd0 width is an explicit raw
/// input; it is not inferred from own/save layers or a per-unit answer.
pub fn plot_visible(
    runtime: &crate::unit_sight_state::Runtime,
    viewer: i32,
    native_visibility_width: u32,
    location: PlotCoord,
) -> Result<bool, &'static str> {
    let layers = runtime.shared(viewer)?;
    let normal = layers
        .0
        .iter()
        .find(|layer| layer.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
        .ok_or("missing initialized current shared normal visibility")?;
    let index = native_visibility_width
        .wrapping_mul(location.y as u32)
        .wrapping_add(location.x as u32);
    if index >= normal.count {
        return Ok(false);
    }
    normal
        .values
        .get(index as usize)
        .map(|count| *count > 0)
        .ok_or("incomplete current shared normal visibility count array")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinate_visibility_borrows_live_shared_counts_and_distinguishes_missing_data() {
        let mut runtime = crate::unit_sight_state::Runtime {
            shared: vec![crate::unit_sight_state::SharedVisibility {
                owner: 7,
                layers: c6_core::visibility_change::SharedLayers(vec![
                    c6_core::map::VisibilityLayer {
                        type_hash: c6_core::visibility_change::BASE_LAYER_HASH,
                        index: 0,
                        count: 3,
                        grid_width: 1,
                        values: vec![0, -1, 1],
                    },
                ]),
            }],
            hidden_layer_hashes: vec![],
            messages_disabled: false,
            local_observer: None,
        };
        let p = PlotCoord { x: 0, y: 1 };
        assert_eq!(plot_visible(&runtime, 7, 2, p), Ok(true));
        runtime.shared[0].layers.0[0].values[2] = -1;
        assert_eq!(plot_visible(&runtime, 7, 2, p), Ok(false));
        runtime.shared[0].layers.0[0].values.pop();
        assert!(plot_visible(&runtime, 7, 2, p).is_err());
        assert_eq!(plot_visible(&runtime, 7, 3, p), Ok(false));
        assert!(plot_visible(&runtime, 8, 2, p).is_err());
    }
}
