//! Canonical current-state adapter for the Linux plot-contents kernel.
use super::{Error, LoadedState, PlotCoord, Runtime};
use c6_core::{
    unit::ComponentId,
    visibility_change::ViewContents,
    visibility_contents::{self, Discovery, Notification, Signal},
};
pub struct Contents<'a, 'd, P> {
    pub state: &'a mut LoadedState,
    pub backend: &'a mut Runtime<'d, P>,
}

#[allow(unused_variables)]
impl<P> visibility_contents::Context for Contents<'_, '_, P> {
    type Error = Error;
    type Plot = u32;
    type Unit = (i32, u32);
    type UnitList = (u32, usize);
    type Capital = (i32, u32);
    fn preflight_contents(&self, event: ViewContents) -> Result<(), Self::Error> {
        self.backend
            .check(self.state)
            .map_err(|_| Error::Missing("sight contents identity"))
    }
    fn map_present(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn plot(&mut self, index: i32) -> Result<Option<Self::Plot>, Self::Error> {
        Ok(
            (index >= 0 && (index as usize) < self.state.world().map().plots.len())
                .then_some(index as u32),
        )
    }
    fn plot_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        Ok(self
            .current_plot(plot)?
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player)))
    }
    fn location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error> {
        self.current_plot(plot)?;
        let width = self.state.world().map().header.width;
        if width <= 0 {
            return Err(Error::Missing("sight contents map width"));
        }
        Ok(PlotCoord {
            x: plot as i32 % width,
            y: plot as i32 / width,
        })
    }
    fn continent(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        let hash = self.current_plot(plot)?.continent_hash;
        Ok(self
            .definitions()?
            .continents
            .find_index(hash)
            .map_or(-1, |v| v as i32))
    }
    fn feature(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        let hash = self.current_plot(plot)?.feature_hash;
        Ok(self
            .backend
            .movement()?
            .definitions
            .features
            .find_index(hash)
            .map_or(-1, |v| v as i32))
    }
    fn goody_notify(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        Ok(self
            .definitions()?
            .goody_notify
            .contains(&self.current_plot(plot)?.improvement_hash))
    }
    fn natural_wonder(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        Ok(self
            .definitions()?
            .natural_wonders
            .contains(&self.current_plot(plot)?.feature_hash))
    }
    fn player_valid(&mut self, player: i32) -> Result<bool, Self::Error> {
        self.state
            .world()
            .player_is_valid(player)
            .map_err(Error::World)
    }
    fn human(&mut self, player: i32) -> Result<bool, Self::Error> {
        Ok(self
            .state
            .player_slot_status(player)
            .map_err(Error::State)?
            == 3)
    }
    fn is_revealed(&mut self, player: i32, plot: u32) -> Result<bool, Self::Error> {
        match self
            .state
            .world()
            .visibility(player)
            .ok_or(Error::Missing("sight revealed visibility"))?
            .revealed
            .get(plot as usize)
        {
            Some(0) | None => Ok(false),
            Some(1) => Ok(true),
            _ => Err(Error::Missing("sight revealed canonical byte")),
        }
    }
    fn discovery(&self, player: i32) -> Result<&Discovery, Self::Error> {
        Err(Error::Missing("sight discovery discovery"))
    }
    fn discovery_mut(&mut self, player: i32) -> Result<&mut Discovery, Self::Error> {
        Err(Error::Missing("sight discovery discovery_mut"))
    }
    fn find_unit(&mut self, reference: ComponentId) -> Result<Option<Self::Unit>, Self::Error> {
        Ok(self
            .state
            .world()
            .unit(c6_core::map::UnitReference::from_component_id(reference))
            .map_err(Error::World)?
            .map(|u| (u.record.e_owner, u.record.id)))
    }
    fn unit_owner(&mut self, unit: Self::Unit) -> Result<i32, Self::Error> {
        Ok(self
            .state
            .world()
            .player(unit.0)
            .map_err(Error::World)?
            .unit(unit.1)
            .map_err(Error::World)?
            .ok_or(Error::Missing("sight contents unit"))?
            .record
            .e_owner)
    }
    fn player_id(&mut self, owner: i32) -> Result<i32, Self::Error> {
        self.state.world().player(owner).map_err(Error::World)?;
        Ok(owner)
    }
    fn unit_layer_count(&mut self) -> Result<i32, Self::Error> {
        i32::try_from(self.state.world().map().units.layers.len())
            .map_err(|_| Error::Missing("sight unit layer count"))
    }
    fn unit_list(&mut self, plot: Self::Plot, layer: i32) -> Result<Self::UnitList, Self::Error> {
        let ordinal = usize::try_from(layer).map_err(|_| Error::Missing("sight unit layer"))?;
        self.state
            .world()
            .map()
            .units
            .layers
            .get(ordinal)
            .ok_or(Error::Missing("sight unit layer"))?;
        Ok((plot, ordinal))
    }
    fn unit_list_count(&mut self, list: Self::UnitList) -> Result<usize, Self::Error> {
        Ok(self.list(list)?.len())
    }
    fn unit_reference(
        &mut self,
        list: Self::UnitList,
        index: usize,
    ) -> Result<ComponentId, Self::Error> {
        self.list(list)?
            .get(index)
            .map(|v| v.component_id())
            .ok_or(Error::Missing("sight unit list index"))
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
        Ok(self.state.world().map().header.width)
    }
    fn grid_height(&mut self) -> Result<i32, Self::Error> {
        Ok(self.state.world().map().header.height)
    }
    fn revealed_xy(&mut self, player: i32, x: i32, y: i32) -> Result<bool, Self::Error> {
        let width = self.state.world().map().header.width;
        if x < 0 || y < 0 || x >= width || y >= self.state.world().map().header.height {
            return Ok(false);
        }
        self.is_revealed(
            player,
            (y as u32).wrapping_mul(width as u32).wrapping_add(x as u32),
        )
    }
    fn found_continent(&self, player: i32, index: i32) -> Result<bool, Error> {
        self.flag(player, index, false)
    }
    fn found_feature(&self, player: i32, index: i32) -> Result<bool, Error> {
        self.flag(player, index, true)
    }
    fn set_found_continent(&mut self, player: i32, index: i32) -> Result<(), Error> {
        self.set_flag(player, index, false)
    }
    fn set_found_feature(&mut self, player: i32, index: i32) -> Result<(), Error> {
        self.set_flag(player, index, true)
    }
    fn circumnavigated(&self, player: i32) -> Result<bool, Error> {
        Ok(self.player(player)?.header.ordered_flags[4])
    }
    fn set_circumnavigated(&mut self, player: i32) -> Result<(), Error> {
        self.state
            .set_player_circumnavigated(player)
            .map_err(Error::State)
    }
}

impl<P> Contents<'_, '_, P> {
    fn definitions(
        &self,
    ) -> Result<&crate::loaded_actions::stock_notifications::resource_revelation::State, Error>
    {
        self.backend
            .resource_revelation
            .as_ref()
            .ok_or(Error::Missing("sight contents definitions"))
    }
    fn current_plot(&self, index: u32) -> Result<&c6_core::map::Plot, Error> {
        self.state
            .world()
            .map()
            .plot_by_index(index)
            .ok_or(Error::Missing("sight contents plot"))
    }
    fn player(&self, owner: i32) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
        self.state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("sight contents player"))
    }
    fn list(&self, list: (u32, usize)) -> Result<&[c6_core::map::UnitReference], Error> {
        let layer = self
            .state
            .world()
            .map()
            .units
            .layers
            .get(list.1)
            .ok_or(Error::Missing("sight unit layer"))?;
        if list.0 >= layer.maximum {
            return Ok(&[]);
        }
        self.state
            .world()
            .plot_units(layer.layer_id, list.0 as usize)
            .map_err(Error::World)
    }
    fn flag(&self, owner: i32, index: i32, feature: bool) -> Result<bool, Error> {
        let registry = if feature {
            self.backend.movement()?.definitions.features
        } else {
            &self.definitions()?.continents
        };
        let Some(def) = registry.entries().get(index as u32 as usize) else {
            return Ok(false);
        };
        let hash = crate::registry::linux_type_hash(&def.type_name);
        let arrays = self
            .player(owner)?
            .post_heroes_arrays
            .as_ref()
            .ok_or(Error::Missing("sight discovery flags"))?;
        let pairs = if feature {
            &arrays.feature_definitions.pairs
        } else {
            &arrays.continent_definitions.pairs
        };
        pairs
            .iter()
            .rev()
            .find(|p| p.definition_hash == hash)
            .map(|p| p.value)
            .ok_or(Error::Missing("sight discovery flag initializer"))
    }
    fn set_flag(&mut self, owner: i32, index: i32, feature: bool) -> Result<(), Error> {
        let registry = if feature {
            self.backend.movement()?.definitions.features
        } else {
            &self.definitions()?.continents
        };
        let Some(def) = registry.entries().get(index as u32 as usize) else {
            return Ok(());
        };
        let hash = crate::registry::linux_type_hash(&def.type_name);
        let arrays = self
            .state
            .player_discovery_mut(owner)
            .map_err(Error::State)?;
        let pairs = if feature {
            &mut arrays.feature_definitions.pairs
        } else {
            &mut arrays.continent_definitions.pairs
        };
        pairs
            .iter_mut()
            .rev()
            .find(|p| p.definition_hash == hash)
            .ok_or(Error::Missing("sight discovery flag initializer"))?
            .value = true;
        Ok(())
    }
}
