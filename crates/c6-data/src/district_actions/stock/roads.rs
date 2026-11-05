//! City.Manager.SetComplete road continuation, reusing the founding kernel.
use super::{error, Effects};
use crate::{
    city_production_turn as production,
    found_city::{route_callbacks, routes},
    unit_placement_services::Services,
};
use c6_core::{loaded_state::LoadedState, unit::PlotCoord};
pub struct Inputs<'a> {
    pub definitions: &'a routes::Definitions,
    pub sight: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
    pub callbacks: &'a route_callbacks::Inputs,
}
// Keep the adapter's borrow independent of the stock owner's borrow.
struct Adapter<'s, 'r, 'a, P> {
    backend: &'s mut crate::loaded_actions::stock_notifications::Runtime<'r, P>,
    definitions: &'a super::super::Definitions,
    callbacks: &'a route_callbacks::Inputs,
}
impl<P> routes::Effects for Adapter<'_, '_, '_, P> {
    type Error = production::Error;
    fn water(&self, state: &LoadedState, plot: u32) -> Result<bool, Self::Error> {
        let hash = state
            .world()
            .map()
            .plots
            .get(plot as usize)
            .ok_or(production::Error::Unsupported("district road plot"))?
            .terrain_hash;
        self.definitions
            .terrains
            .iter()
            .find(|r| r.hash == hash)
            .map(|r| r.water)
            .ok_or(production::Error::Unsupported("district road terrain"))
    }
    fn alive_ids(&self) -> Result<Vec<i32>, Self::Error> {
        Ok(self
            .backend
            .placement_service_runtime()
            .map_err(|e| error("district road players", e))?
            .alive_players
            .as_ref()
            .ok_or(production::Error::Unsupported("current AlivePlayers"))?
            .values()
            .to_vec())
    }
    fn visible(
        &self,
        state: &LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        owner: i32,
        plot: u32,
    ) -> Result<bool, Self::Error> {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot)
            .ok_or(production::Error::Unsupported("district road coordinate"))?;
        let width = self
            .backend
            .placement_service_runtime()
            .map_err(|e| error("district road players", e))?
            .player::<production::Error>(owner)
            .map_err(|e| error("district road viewer", e))?
            .native_visibility_width
            .ok_or(production::Error::Unsupported("current visibility width"))?;
        crate::loaded_upkeep::placement_tails::plot_visible(sight, owner, width, PlotCoord { x, y })
            .map_err(production::Error::Unsupported)
    }
    fn report(&mut self, _: &LoadedState, _: u32, event: u32) -> Result<(), Self::Error> {
        match event {
            0x5d0f4014 | 0x33d8725a => {
                self.backend
                    .reached
                    .push("district road headless connectivity publication");
                Ok(())
            }
            _ => Err(production::Error::Unsupported("district road report")),
        }
    }
    fn route_changed(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
        route: i32,
        owner: i32,
    ) -> Result<(), Self::Error> {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot)
            .ok_or(production::Error::Unsupported("district road coordinate"))?;
        route_callbacks::route_changed(state, self.callbacks, PlotCoord { x, y }, route, owner)
            .map_err(|e| error("district road callbacks", e))
    }
}
pub(super) fn complete<P>(
    effects: &mut Effects<'_, '_, '_, P>,
    state: &mut LoadedState,
    d: crate::district_initialization::DistrictReference,
) -> Result<(), production::Error> {
    let location = state
        .district(d.owner, d.id)
        .map_err(production::Error::District)?
        .location;
    let index = (location.1 as u32)
        .wrapping_mul(state.world().map().header.width as u32)
        .wrapping_add(location.0 as u32);
    let inputs = effects.inputs.roads;
    routes::complete(
        state,
        &inputs.sight.borrow(),
        inputs.definitions,
        index,
        d.owner,
        &mut Adapter {
            backend: effects.backend,
            definitions: effects.inputs.definitions,
            callbacks: inputs.callbacks,
        },
    )
    .map_err(|e| error("City.Manager.SetComplete road", e))?;
    Ok(())
}
