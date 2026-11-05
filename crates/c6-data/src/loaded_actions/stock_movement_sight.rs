//! Explicit unavailable sight discovery leaves; no invented callback values.
use super::{sight, Error, LoadedState, PlotCoord, Runtime};
use c6_core::{
    unit_sight::VisibilityChange, visibility_change::Propagation, visibility_propagation,
};
use sight::PlotVisibilityChanged;
#[path = "stock_sight_contents.rs"]
mod contents;
#[allow(unused_variables)]
impl<'d, P> sight::Effects for Runtime<'d, P> {
    type Error = Error;
    type Contents<'a>
        = contents::Contents<'a, 'd, P>
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
        Ok(contents::Contents {
            state,
            backend: self,
        })
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
