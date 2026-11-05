//! Explicit unavailable sight discovery leaves; no invented callback values.
use super::{sight, Error, LoadedState, PlotCoord, Runtime};
use c6_core::{
    unit::ComponentId,
    unit_sight::VisibilityChange,
    visibility_change::{Propagation, ViewContents},
    visibility_contents::{self, Discovery, Notification, Signal},
    visibility_propagation,
};
use sight::PlotVisibilityChanged;
pub struct UnboundContents;
#[allow(unused_variables)]
impl visibility_contents::Context for UnboundContents {
    type Error = Error;
    type Plot = u32;
    type Unit = (i32, u32);
    type UnitList = ();
    type Capital = (i32, u32);
    fn preflight_contents(&self, event: ViewContents) -> Result<(), Self::Error> {
        Err(Error::Missing("sight discovery preflight_contents"))
    }
    fn map_present(&mut self) -> Result<bool, Self::Error> {
        Err(Error::Missing("sight discovery map_present"))
    }
    fn plot(&mut self, index: i32) -> Result<Option<Self::Plot>, Self::Error> {
        Err(Error::Missing("sight discovery plot"))
    }
    fn plot_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery plot_owner"))
    }
    fn location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error> {
        Err(Error::Missing("sight discovery location"))
    }
    fn continent(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery continent"))
    }
    fn feature(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery feature"))
    }
    fn goody_notify(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        Err(Error::Missing("sight discovery goody_notify"))
    }
    fn natural_wonder(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        Err(Error::Missing("sight discovery natural_wonder"))
    }
    fn player_valid(&mut self, player: i32) -> Result<bool, Self::Error> {
        Err(Error::Missing("sight discovery player_valid"))
    }
    fn human(&mut self, player: i32) -> Result<bool, Self::Error> {
        Err(Error::Missing("sight discovery human"))
    }
    fn is_revealed(&mut self, player: i32, plot: u32) -> Result<bool, Self::Error> {
        Err(Error::Missing("sight discovery is_revealed"))
    }
    fn discovery(&self, player: i32) -> Result<&Discovery, Self::Error> {
        Err(Error::Missing("sight discovery discovery"))
    }
    fn discovery_mut(&mut self, player: i32) -> Result<&mut Discovery, Self::Error> {
        Err(Error::Missing("sight discovery discovery_mut"))
    }
    fn find_unit(&mut self, reference: ComponentId) -> Result<Option<Self::Unit>, Self::Error> {
        Err(Error::Missing("sight discovery find_unit"))
    }
    fn unit_owner(&mut self, unit: Self::Unit) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery unit_owner"))
    }
    fn player_id(&mut self, owner: i32) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery player_id"))
    }
    fn unit_layer_count(&mut self) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery unit_layer_count"))
    }
    fn unit_list(&mut self, plot: Self::Plot, layer: i32) -> Result<Self::UnitList, Self::Error> {
        Err(Error::Missing("sight discovery unit_list"))
    }
    fn unit_list_count(&mut self, list: Self::UnitList) -> Result<usize, Self::Error> {
        Err(Error::Missing("sight discovery unit_list_count"))
    }
    fn unit_reference(
        &mut self,
        list: Self::UnitList,
        index: usize,
    ) -> Result<ComponentId, Self::Error> {
        Err(Error::Missing("sight discovery unit_reference"))
    }
    fn meet(
        &mut self,
        first: i32,
        second: i32,
        plot: Option<Self::Plot>,
        source: ComponentId,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("sight discovery meet"))
    }
    fn view_unit_rules(
        &mut self,
        player: i32,
        unit: Self::Unit,
        origin: i32,
        first_reveal: bool,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("sight discovery view_unit_rules"))
    }
    fn signal(&mut self, event: Signal<Self::Unit>) -> Result<(), Self::Error> {
        Err(Error::Missing("sight discovery signal"))
    }
    fn notify(
        &mut self,
        player: i32,
        plot: Self::Plot,
        kind: Notification,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("sight discovery notify"))
    }
    fn natural_wonder_relic_count(&mut self, player: i32) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery natural_wonder_relic_count"))
    }
    fn capital(&mut self, player: i32) -> Result<Option<Self::Capital>, Self::Error> {
        Err(Error::Missing("sight discovery capital"))
    }
    fn capital_location(&mut self, city: Self::Capital) -> Result<PlotCoord, Self::Error> {
        Err(Error::Missing("sight discovery capital_location"))
    }
    fn create_relic(
        &mut self,
        player: i32,
        location: PlotCoord,
        source: u32,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("sight discovery create_relic"))
    }
    fn grid_width(&mut self) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery grid_width"))
    }
    fn grid_height(&mut self) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight discovery grid_height"))
    }
    fn revealed_xy(&mut self, player: i32, x: i32, y: i32) -> Result<bool, Self::Error> {
        Err(Error::Missing("sight discovery revealed_xy"))
    }
}
#[allow(unused_variables)]
impl<P> sight::Effects for Runtime<'_, P> {
    type Error = Error;
    type Contents<'a>
        = UnboundContents
    where
        Self: 'a;
    fn preflight_sight(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        actor: (i32, u32),
        add: bool,
    ) -> Result<(), Self::Error> {
        self.current_actor(state, actor)?;
        self.visibility_owner(state, runtime, actor.0, None)
    }
    fn preflight_visibility(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        change: VisibilityChange,
    ) -> Result<(), Self::Error> {
        self.visibility_owner(state, runtime, change.owner, Some(change.plot_index))
    }
    fn water(&mut self, state: &LoadedState, plot: u32) -> Result<bool, Self::Error> {
        let plot = state
            .world()
            .map()
            .plot_by_index(plot)
            .ok_or(Error::Missing("sight plot"))?;
        self.movement()?
            .definitions
            .placement
            .water(state, plot)
            .map_err(Error::Placement)
    }
    fn feature(&mut self, state: &LoadedState, plot: u32) -> Result<i32, Self::Error> {
        let plot = state
            .world()
            .map()
            .plot_by_index(plot)
            .ok_or(Error::Missing("sight plot"))?;
        Ok(self
            .movement()?
            .definitions
            .features
            .find_index(plot.feature_hash)
            .map_or(-1, |i| i32::from(i as i16)))
    }
    fn revealed_feature(
        &mut self,
        state: &LoadedState,
        player: i32,
        feature: i32,
    ) -> Result<bool, Self::Error> {
        sight::revealed_feature::get(
            state,
            self.movement()?.definitions.features,
            player,
            feature,
        )
        .map_err(|e| Error::Requirement(format!("{e:?}")))
    }
    fn promotion_class(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
    ) -> Result<Option<&str>, Self::Error> {
        Err(Error::Missing("sight promotion_class"))
    }
    fn natural_wonder_experience(&mut self, state: &LoadedState) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight natural_wonder_experience"))
    }
    fn experience_modifier(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
    ) -> Result<i32, Self::Error> {
        Err(Error::Missing("sight experience_modifier"))
    }
    fn change_experience(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        actor: (i32, u32),
        amount: i32,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("sight change_experience"))
    }
    fn update_resource_revealed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        player: i32,
        plot: u32,
    ) -> Result<(), Self::Error> {
        self.update_revealed_resource(state, runtime, player, plot)
    }
    fn preflight_hidden_propagation(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        event: Propagation,
    ) -> Result<(), Self::Error> {
        self.visibility_owner(state, runtime, event.player, Some(event.plot))
    }
    fn visibility_reporting_blocks(&mut self) -> Result<&mut Vec<u32>, Self::Error> {
        self.sight_visibility_blocks()
    }
    fn unit_visibility_changed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        event: visibility_propagation::UnitVisibilityChanged,
    ) -> Result<(), Self::Error> {
        self.sight_unit_visibility(event)
    }
    fn propagate(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        event: Propagation,
    ) -> Result<(), Self::Error> {
        self.propagate_sight(state, event)
    }
    fn contents<'a>(
        &'a mut self,
        state: &'a mut LoadedState,
        runtime: &'a mut sight::Runtime,
    ) -> Result<Self::Contents<'a>, Self::Error> {
        Err(Error::Missing("sight contents"))
    }
    fn plot_visibility_changed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        event: PlotVisibilityChanged,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("sight plot_visibility_changed"))
    }
    fn signal_unit_plot(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("sight signal_unit_plot"))
    }
}
