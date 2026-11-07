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
        self.current_actor(state, actor)?;
        let index = usize::try_from(
            state
                .unit_runtime(actor.0, actor.1)
                .map_err(Error::State)?
                .definition_index,
        )
        .map_err(|_| Error::Missing("sight Unit definition index"))?;
        let classes = &self
            .resource_revelation
            .as_ref()
            .and_then(|r| r.wonders.as_ref())
            .ok_or(Error::Missing("sight current Unit promotion rules"))?
            .promotion_classes;
        let (hash, class) = classes
            .get(index)
            .ok_or(Error::Missing("sight Unit promotion definition"))?;
        let unit = state
            .world()
            .player(actor.0)
            .map_err(Error::World)?
            .unit(actor.1)
            .map_err(Error::World)?
            .ok_or(Error::Missing("sight current Unit"))?
            .record;
        if unit.e_type_index != *hash {
            return Err(Error::Missing("sight Unit definition identity"));
        }
        Ok(class.as_deref())
    }
    fn natural_wonder_experience(&mut self, state: &LoadedState) -> Result<i32, Self::Error> {
        Ok(self
            .resource_revelation
            .as_ref()
            .and_then(|r| r.wonders.as_ref())
            .ok_or(Error::Missing("sight experience rules"))?
            .experience)
    }
    fn experience_modifier(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
    ) -> Result<i32, Self::Error> {
        let unit = c6_core::map::UnitReference::from_component_id(c6_core::unit::ComponentId {
            owner: actor.0 as i16,
            component_type: 1,
            id: actor.1,
        });
        let bonuses = &self
            .resource_revelation
            .as_ref()
            .and_then(|r| r.wonders.as_ref())
            .ok_or(Error::Missing("sight experience bonus definitions"))?
            .bonuses;
        crate::melee_attack::experience::general_modifier(state, bonuses, unit)
            .map_err(|e| Error::Requirement(format!("sight experience modifier: {e:?}")))
    }
    fn change_experience(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        actor: (i32, u32),
        amount: i32,
    ) -> Result<(), Self::Error> {
        if amount <= 0 {
            return Ok(());
        }
        let unit = c6_core::map::UnitReference::from_component_id(c6_core::unit::ComponentId {
            owner: actor.0 as i16,
            component_type: 1,
            id: actor.1,
        });
        let maximum = self
            .resource_revelation
            .as_ref()
            .and_then(|r| r.wonders.as_ref())
            .ok_or(Error::Missing("sight experience maximum damage"))?
            .maximum_damage;
        let levels = self.movement()?.civilization_levels.clone();
        if !crate::melee_attack::experience::change_eligible::<Error>(
            state,
            &levels,
            unit,
            |unit| crate::formation_movement_cost::unit_maximum_damage(unit, maximum),
        )
        .map_err(|e| Error::Requirement(format!("sight experience guard: {e:?}")))?
        {
            self.reached
                .push("stock sight experience native early guard");
            return Ok(());
        }
        // Move the immutable definitions out while Runtime owns the reached
        // report callbacks, and restore them on both success and failure.
        let definitions = self
            .resource_revelation
            .as_mut()
            .and_then(|r| r.wonders.as_mut())
            .and_then(|w| w.change_definitions.take())
            .ok_or(Error::Missing("positive sight experience definitions"))?;
        let result = crate::melee_attack::experience::change_with_maximum(
            state,
            &definitions,
            |unit| crate::formation_movement_cost::unit_maximum_damage(unit, maximum),
            &levels,
            unit,
            amount,
            self,
        );
        self.resource_revelation
            .as_mut()
            .and_then(|r| r.wonders.as_mut())
            .expect("wonder definitions owner retained during XP reports")
            .change_definitions = Some(definitions);
        result.map_err(|e| Error::Requirement(format!("sight experience change: {e:?}")))
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
        self.on_plot_visibility_changed(state, event)
    }
    fn signal_unit_plot(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<(), Self::Error> {
        self.dispatch_unit_plot_visibility(state, actor, plot)
    }
}
