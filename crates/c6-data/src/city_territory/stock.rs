//! Stock territory callbacks shared by explicit acquisitions and culture upkeep.
//! Registration qualification is supplied by the initialized caller. Gameplay
//! transitions stay in the existing territory, sight and citizen kernels.
use super::{Context, Definitions, Runtime};
use crate::{
    city_turn::current_upkeep::TerritoryWorkers,
    loaded_actions::stock_notifications::{self as stock, movement::Error},
    unit_sight_state as sight,
};
use c6_core::{
    city_territory::{plot_yields, CityId},
    loaded_state::LoadedState,
    unit::PlotCoord,
    unit_sight::VisibilityChange,
    visibility_change::Propagation,
    visibility_propagation,
};
use sight::PlotVisibilityChanged;

/// Actual stock report/signal closure: ownership's Rules Cities subscriber and
/// AnnexPlot's improvement reapplication subscriber, with no added consumers.
pub enum Scope {
    StockLinuxXp2NoAddedConsumers,
}
pub struct Inputs<'s, 'd> {
    pub definitions: &'s Definitions,
    pub sight: &'s crate::loaded_actions::city_range_attack::Definitions,
    pub terrains: &'s c6_core::movement_world::DefinitionTable<
        'd,
        c6_core::movement_world::TerrainProperties,
    >,
    pub workers: TerritoryWorkers<'s>,
    pub scope: Scope,
}
struct Effects<'s, 'runtime, 'd, P> {
    backend: &'s mut stock::Runtime<'runtime, P>,
    inputs: &'s Inputs<'s, 'd>,
}
impl<'runtime, P> sight::Effects for Effects<'_, 'runtime, '_, P> {
    type Error = Error;
    type Contents<'a>
        = <stock::Runtime<'runtime, P> as sight::Effects>::Contents<'a>
    where
        Self: 'a;
    fn preflight_sight(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        actor: (i32, u32),
        add: bool,
    ) -> Result<(), Self::Error> {
        sight::Effects::preflight_sight(&*self.backend, state, runtime, actor, add)
    }
    fn preflight_visibility(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        change: VisibilityChange,
    ) -> Result<(), Self::Error> {
        sight::Effects::preflight_visibility(&*self.backend, state, runtime, change)
    }
    fn water(&mut self, state: &LoadedState, plot: u32) -> Result<bool, Self::Error> {
        sight::Effects::water(&mut *self.backend, state, plot)
    }
    fn feature(&mut self, state: &LoadedState, plot: u32) -> Result<i32, Self::Error> {
        sight::Effects::feature(&mut *self.backend, state, plot)
    }
    fn revealed_feature(
        &mut self,
        state: &LoadedState,
        player: i32,
        feature: i32,
    ) -> Result<bool, Self::Error> {
        sight::Effects::revealed_feature(&mut *self.backend, state, player, feature)
    }
    fn promotion_class(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
    ) -> Result<Option<&str>, Self::Error> {
        sight::Effects::promotion_class(&mut *self.backend, state, actor)
    }
    fn natural_wonder_experience(&mut self, state: &LoadedState) -> Result<i32, Self::Error> {
        sight::Effects::natural_wonder_experience(&mut *self.backend, state)
    }
    fn experience_modifier(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
    ) -> Result<i32, Self::Error> {
        sight::Effects::experience_modifier(&mut *self.backend, state, actor)
    }
    fn change_experience(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        actor: (i32, u32),
        amount: i32,
    ) -> Result<(), Self::Error> {
        sight::Effects::change_experience(&mut *self.backend, state, runtime, actor, amount)
    }
    fn update_resource_revealed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        player: i32,
        plot: u32,
    ) -> Result<(), Self::Error> {
        sight::Effects::update_resource_revealed(&mut *self.backend, state, runtime, player, plot)
    }
    fn preflight_hidden_propagation(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        event: Propagation,
    ) -> Result<(), Self::Error> {
        sight::Effects::preflight_hidden_propagation(&*self.backend, state, runtime, event)
    }
    fn visibility_reporting_blocks(&mut self) -> Result<&mut Vec<u32>, Self::Error> {
        sight::Effects::visibility_reporting_blocks(&mut *self.backend)
    }
    fn unit_visibility_changed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        event: visibility_propagation::UnitVisibilityChanged,
    ) -> Result<(), Self::Error> {
        sight::Effects::unit_visibility_changed(&mut *self.backend, state, runtime, event)
    }
    fn propagate(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        event: Propagation,
    ) -> Result<(), Self::Error> {
        sight::Effects::propagate(&mut *self.backend, state, runtime, event)
    }
    fn contents<'a>(
        &'a mut self,
        state: &'a mut LoadedState,
        runtime: &'a mut sight::Runtime,
    ) -> Result<Self::Contents<'a>, Self::Error> {
        sight::Effects::contents(&mut *self.backend, state, runtime)
    }
    fn plot_visibility_changed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        event: PlotVisibilityChanged,
    ) -> Result<(), Self::Error> {
        sight::Effects::plot_visibility_changed(&mut *self.backend, state, runtime, event)
    }
    fn signal_unit_plot(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<(), Self::Error> {
        sight::Effects::signal_unit_plot(&mut *self.backend, state, runtime, actor, plot)
    }
}
impl<P> super::Effects for Effects<'_, '_, '_, P> {
    fn influence_token_received(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: CityId,
        _: i32,
        _: i32,
    ) -> Result<(), Error> {
        Err(Error::Missing("territory influence callback"))
    }
    fn owner_report(&mut self, _: &mut LoadedState, _: u32, _: i32) -> Result<(), Error> {
        // Stock PlotOwnershipChanged cache and translators publish derived
        // map/placement data; their current getters have no gameplay writes.
        match self.inputs.scope {
            Scope::StockLinuxXp2NoAddedConsumers => Ok(()),
        }
    }
    fn owner_signal(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
        new: CityId,
        old: CityId,
    ) -> Result<(), Error> {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot)
            .ok_or(Error::Missing("territory owner plot"))?;
        super::rules_cities_owner_changed(state, PlotCoord { x, y }, new, old)
            .map_err(|e| Error::Requirement(format!("territory owner requirements: {e:?}")))
    }
    fn plot_yields_created(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: u64,
        _: u32,
    ) -> Result<(), Error> {
        Err(Error::Missing("territory PlotYields Created subscriber"))
    }
    fn plot_yields_destroyed(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: plot_yields::Object,
    ) -> Result<(), Error> {
        Err(Error::Missing("territory PlotYields Destroyed membership"))
    }
    fn coastal_land_one(&mut self, state: &LoadedState, plot: u32) -> Result<bool, Error> {
        use c6_core::city_district::DomainRebuildInputs;
        crate::city_movement::MapDomainInputs::new(state.world().map(), self.inputs.terrains)
            .map_err(|_| Error::Missing("territory domain inputs"))?
            .is_coastal_land_one(plot as usize)
            .map_err(|_| Error::Missing("territory coastal land"))
    }
    fn set_improvement(
        &mut self,
        _: &mut LoadedState,
        _: u32,
        _: i32,
        _: i32,
    ) -> Result<(), Error> {
        Err(Error::Missing(
            "territory improvement transfer continuation",
        ))
    }
    fn expel_from_plot(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: PlotCoord,
    ) -> Result<(), Error> {
        Err(Error::Missing("territory unit expulsion continuation"))
    }
    fn activate_improvement_without_unit(
        &mut self,
        _: &mut LoadedState,
        _: u32,
    ) -> Result<(), Error> {
        Err(Error::Missing(
            "territory improvement activation continuation",
        ))
    }
    fn reallocate_citizens(
        &mut self,
        state: &mut LoadedState,
        territory: &mut Runtime,
        city: CityId,
    ) -> Result<(), Error> {
        self.inputs
            .workers
            .reallocate(state, territory, city.owner, city.id)
            .map_err(|e| Error::Requirement(format!("territory workers: {e:?}")))
    }
    fn annex_report(&mut self, _: &mut LoadedState, _: CityId, _: PlotCoord) -> Result<(), Error> {
        // No native cache subscribers; stock translators/host107 publish DTOs.
        match self.inputs.scope {
            Scope::StockLinuxXp2NoAddedConsumers => Ok(()),
        }
    }
    fn annex_signal(
        &mut self,
        state: &mut LoadedState,
        _: i32,
        _: CityId,
        plot: u32,
        old: CityId,
    ) -> Result<(), Error> {
        if super::annexed_improvement_reapply(state, self.inputs.definitions, plot, old)
            .map_err(|_| Error::Missing("territory annex improvement lookup"))?
            .is_some()
        {
            return Err(Error::Missing("territory annex improvement reapplication"));
        }
        Ok(())
    }
    fn buyable_plots(
        &mut self,
        _: &LoadedState,
        _: &Runtime,
        _: CityId,
    ) -> Result<Vec<u32>, Error> {
        Err(Error::Missing(
            "territory selection belongs to culture upkeep",
        ))
    }
}
/// Resume Culture.AnnexPlot on its already selected plot. No gold debit,
/// PlotPurchased report/Boost signal, or command completion occurs here.
pub fn annex<P>(
    state: &mut LoadedState,
    backend: &mut stock::Runtime<'_, P>,
    territory: &mut Runtime,
    sight: &mut sight::Runtime,
    city: CityId,
    plot: u32,
    inputs: &Inputs<'_, '_>,
) -> Result<(), String> {
    let (x, y) = state
        .world()
        .map()
        .coordinates(plot)
        .ok_or("culture annex plot")?;
    // Territory visibility passes NULL CID; no Unit is selected or queried.
    let actor = (city.owner, u32::MAX);
    let mut effects = Effects { backend, inputs };
    let mut context = Context {
        state,
        runtime: territory,
        sight,
        sight_definitions: inputs.sight,
        definitions: inputs.definitions,
        effects: &mut effects,
        city,
        actor,
    };
    context
        .validate()
        .map_err(|e| format!("culture territory binding: {e:?}"))?;
    c6_core::city_territory::annex_plot(&mut context, PlotCoord { x, y })
        .map_err(|e| format!("culture AnnexPlot: {e:?}"))
}
