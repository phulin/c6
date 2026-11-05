//! Source-ordered camp creation and tribe upkeep over the existing stock owners.
//! Definitions are prepared once. Each unit factory call borrows the same state,
//! sight and stock backend for that call, including reentrant modifier effects.
use super::queries;
use crate::{
    barbarian_activation as activation, barbarian_camp_placement as placement,
    barbarian_camp_units::FactoryInputs,
    loaded_actions::stock_notifications::{self as stock, npc_leaves},
    unit_sight_state,
};
use c6_core::{
    game_capabilities::Membership,
    game_update::Error,
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord},
    unit_sight::VisibilityChange,
};

/// The retained constructed-signal route. The Barbarian subscriber is executed
/// at its actual position, rather than before or after the whole signal ring.
#[derive(Clone, Copy, Debug)]
pub enum Constructed {
    Barbarian,
    Gameplay(u64),
}
/// Remaining constructed/camp callbacks and current yield providers. These
/// methods have no successful defaults; stock handlers must preserve their own
/// source guards and any gameplay decisions or RNG they reach.
pub trait Services<P> {
    fn constructed_order(&self) -> &[Constructed];
    fn constructed_gameplay(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        plot: usize,
        receiver: u64,
    ) -> Result<(), Error>;
    fn spawn_turns(&self, state: &LoadedState, turns: i32) -> Result<i32, Error>;
    fn local_food(&self, state: &LoadedState, location: PlotCoord) -> Result<i32, Error>;
    fn camp_created(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        index: usize,
    ) -> Result<(), Error>;
    fn monopolies(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
    ) -> Result<(), Error>;
    fn revealed_notifications(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        location: PlotCoord,
    ) -> Result<(), Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        plot: usize,
        event: u32,
        argument: i32,
    ) -> Result<(), Error>;
    fn behavior_threshold(
        &self,
        state: &LoadedState,
        index: usize,
        behavior: activation::Behavior,
    ) -> Result<i32, Error>;
    fn start_behavior(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        index: usize,
        target: ComponentId,
        behavior: activation::Behavior,
    ) -> Result<bool, Error>;
}

pub struct Camps<'a, 'd, 'r, S> {
    pub definitions: &'a queries::Definitions,
    pub inputs: queries::Inputs<'a>,
    pub placement: &'a placement::Rules,
    pub factory: FactoryInputs<'a, 'd, 'r>,
    pub sight: &'a mut unit_sight_state::Runtime,
    pub capabilities: &'a Membership,
    pub parameters: activation::Parameters,
    pub services: S,
}
struct Current<'a, 'b, 'd, 'r, 's, P, S> {
    camps: &'a mut Camps<'b, 'd, 'r, S>,
    backend: &'a mut stock::Runtime<'s, P>,
}
fn fail<P, E: std::fmt::Debug>(
    backend: &mut stock::Runtime<'_, P>,
    message: &'static str,
    error: E,
) -> Error {
    backend.last_error = Some(format!("{message}: {error:?}"));
    Error::Unsupported(message)
}
fn location(state: &LoadedState, index: usize) -> Result<PlotCoord, Error> {
    let map = state.world().map();
    if map.header.width <= 0 || index >= map.plots.len() {
        return Err(Error::Unsupported("camp plot location"));
    }
    Ok(PlotCoord {
        x: (index % map.header.width as usize) as i32,
        y: (index / map.header.width as usize) as i32,
    })
}
fn index(state: &LoadedState, at: PlotCoord) -> Result<usize, Error> {
    crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(state, at)
        .map_err(|_| Error::Unsupported("camp coordinate lookup"))?
        .map(|v| v as usize)
        .ok_or(Error::Unsupported("camp coordinate"))
}
impl<P, S: Services<P>> npc_leaves::Camps<P> for Camps<'_, '_, '_, S> {
    fn visibility(&self) -> &unit_sight_state::Runtime {
        self.sight
    }
    fn run(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        _: &mut c6_core::loaded_state::game_update::Runtime,
        owner: i32,
    ) -> Result<(), Error> {
        if owner != state.retained().barbarian.player_ordinal {
            return Err(Error::Unsupported("camp controller owner"));
        }
        let definitions = self.definitions;
        let inputs = queries::Inputs {
            alive_ids: self.inputs.alive_ids,
            alive_major_ids: self.inputs.alive_major_ids,
            civilization_levels: self.inputs.civilization_levels,
            national_parks: self.inputs.national_parks,
        };
        let parameters = self.parameters;
        let capabilities = self.capabilities;
        let mut current = Current {
            camps: self,
            backend,
        };
        {
            let mut query = queries::Queries {
                definitions,
                inputs,
                placement: &mut current,
            };
            super::create(
                state,
                &mut query,
                definitions.parameters,
                definitions.improvement(),
            )
            .map_err(|e| fail(current.backend, "Barbarian CreateCamps", e))?;
        }
        activation::after_camps(state, &mut current, parameters, capabilities, true)
            .map_err(|e| fail(current.backend, "Barbarian camp upkeep", e))
    }
}
impl<P, S: Services<P>> queries::Placement for Current<'_, '_, '_, '_, '_, P, S> {
    type Error = Error;
    fn visibility(&self) -> Result<&unit_sight_state::Runtime, Error> {
        Ok(self.camps.sight)
    }
    fn set_camp(
        &mut self,
        state: &mut LoadedState,
        at: PlotCoord,
        improvement: i32,
        owner: i32,
    ) -> Result<(), Error> {
        if improvement != self.camps.definitions.improvement() {
            return Err(Error::Unsupported("camp native improvement"));
        }
        let hash = state.retained().barbarian.improvement_hash;
        let rules = self.camps.placement;
        placement::set_camp(state, self, rules, index(state, at)?, hash, owner)
            .map_err(|e| fail(self.backend, "Set camp improvement", e))
    }
}
impl<P, S: Services<P>> placement::Context for Current<'_, '_, '_, '_, '_, P, S> {
    type Error = Error;
    fn is_coastal(&self, state: &LoadedState, at: PlotCoord) -> Result<bool, Error> {
        self.camps
            .definitions
            .coastal(state, at)
            .map_err(|_| Error::Unsupported("camp coast query"))
    }
    fn required_resource(
        &self,
        state: &LoadedState,
        at: PlotCoord,
        hash: u32,
        radius: u32,
    ) -> Result<bool, Error> {
        let map = state.world().map();
        for (i, p) in map.plots.iter().enumerate() {
            if p.resource_hash != hash || p.city_attributes.as_ref().is_some_and(|a| a.player != -1)
            {
                continue;
            }
            let other = location(state, i)?;
            let distance = c6_core::geometry::plot_distance(
                map.header.width,
                map.header.wrap_x,
                at.x,
                at.y,
                other.x,
                other.y,
            );
            if distance >= 0 && distance as u32 <= radius {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn scaled_spawn_turns(&self, state: &LoadedState, turns: i32) -> Result<i32, Error> {
        self.camps.services.spawn_turns(state, turns)
    }
    fn set_owner(&mut self, state: &mut LoadedState, plot: usize, owner: i32) -> Result<(), Error> {
        placement::set_improvement_owner(state, self, plot, owner)
            .map_err(|e| fail(self.backend, "Set camp owner", e))
    }
    fn constructed(
        &mut self,
        state: &mut LoadedState,
        rules: &placement::Rules,
        plot: usize,
    ) -> Result<(), Error> {
        let count = self.camps.services.constructed_order().len();
        for i in 0..count {
            let item = self.camps.services.constructed_order()[i];
            match item {
                Constructed::Barbarian => placement::on_created(state, self, rules, plot)
                    .map_err(|e| fail(self.backend, "Barbarian improvement created", e))?,
                Constructed::Gameplay(receiver) => {
                    self.camps
                        .services
                        .constructed_gameplay(state, self.backend, plot, receiver)?
                }
            }
        }
        Ok(())
    }
    fn spawn_initial_units(
        &mut self,
        state: &mut LoadedState,
        i: usize,
        at: PlotCoord,
        scouts: i32,
    ) -> Result<(), Error> {
        self.camps
            .factory
            .spawn_initial(state, self.camps.sight, self.backend, i, at, scouts)
            .map_err(|e| fail(self.backend, "Camp initial units", e))
    }
    fn local_food(&self, state: &LoadedState, at: PlotCoord) -> Result<i32, Error> {
        self.camps.services.local_food(state, at)
    }
    fn camp_created(&mut self, state: &mut LoadedState, i: usize) -> Result<(), Error> {
        self.camps.services.camp_created(state, self.backend, i)
    }
    fn restore_visual_state(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        hash: u32,
    ) -> Result<(), Error> {
        if hash != state.retained().barbarian.improvement_hash {
            return Err(Error::Unsupported("Restored camp definition"));
        }
        let improvement = self.camps.definitions.improvement();
        placement::restore_visual_state(state, self, plot, improvement)
            .map_err(|e| fail(self.backend, "Restore camp visibility", e))
    }
    fn resource_monopolies(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        self.camps.services.monopolies(state, self.backend)
    }
    fn revealed_major_notifications(
        &mut self,
        state: &mut LoadedState,
        at: PlotCoord,
    ) -> Result<(), Error> {
        self.camps
            .services
            .revealed_notifications(state, self.backend, at)
    }
}
impl<P, S: Services<P>> placement::OwnerContext for Current<'_, '_, '_, '_, '_, P, S> {
    type Error = Error;
    fn valid_player(&self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        state
            .world()
            .valid_player(owner)
            .map(|p| p.is_some())
            .map_err(|_| Error::Unsupported("camp valid improvement owner"))
    }
    fn adjacent(
        &self,
        state: &LoadedState,
        plot: usize,
        direction: u8,
    ) -> Result<Option<usize>, Error> {
        queries::adjacent(state, location(state, plot)?, usize::from(direction))
            .map_err(|_| Error::Unsupported("camp adjacent plot"))
    }
    fn visibility(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        plot: usize,
    ) -> Result<(), Error> {
        let source = ComponentId {
            owner: -1,
            component_type: -1,
            id: u32::MAX,
        };
        let mut context = unit_sight_state::Context {
            state,
            runtime: self.camps.sight,
            effects: self.backend,
            sight_definitions: self.camps.factory.placement.sight,
            actor: (i32::from(source.owner), source.id),
        };
        c6_core::visibility_change::change_visibility_count(
            &mut context,
            VisibilityChange {
                owner,
                plot_index: plot as u32,
                delta: 1,
                hidden_type: -1,
                source_flag: true,
                near_origin: false,
                origin_index: -1,
                sight_range: 1,
                source,
            },
        )
        .map(|_| ())
        .map_err(|e| fail(self.backend, "Camp owner sight", e))
    }
    fn owner_changed(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        owner: i32,
    ) -> Result<(), Error> {
        self.camps
            .services
            .report(state, self.backend, plot, 0xa625c726, owner)
    }
}
impl<P, S: Services<P>> placement::VisualContext for Current<'_, '_, '_, '_, '_, P, S> {
    type Error = Error;
    fn alive_ids(&self) -> Result<Vec<i32>, Error> {
        Ok(self.camps.inputs.alive_ids.to_vec())
    }
    fn messages_disabled(&self) -> bool {
        self.camps.sight.messages_disabled
    }
    fn local_observer(&self) -> Option<i32> {
        self.camps.sight.local_observer
    }
    fn is_revealed(&self, state: &LoadedState, player: i32, plot: usize) -> Result<bool, Error> {
        match state
            .world()
            .visibility(player)
            .and_then(|v| v.revealed.get(plot))
            .copied()
        {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(Error::Unsupported("camp revealed visibility")),
        }
    }
    fn revealed_state(&self, state: &LoadedState, player: i32, plot: usize) -> Result<i32, Error> {
        let layers = self
            .camps
            .sight
            .shared(player)
            .map_err(Error::Unsupported)?;
        let normal = layers
            .0
            .iter()
            .find(|l| l.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
            .ok_or(Error::Unsupported("camp normal visibility"))?;
        let width = state.world().map().header.width as usize;
        let cell = (plot / width) * normal.grid_width as usize + plot % width;
        match normal.values.get(cell) {
            Some(v) if *v > 0 => Ok(2),
            Some(_) => self.is_revealed(state, player, plot).map(i32::from),
            None => Err(Error::Unsupported("camp normal visibility cell")),
        }
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        event: u32,
        arg: i32,
    ) -> Result<(), Error> {
        self.camps
            .services
            .report(state, self.backend, plot, event, arg)
    }
}
impl<P, S: Services<P>> activation::Context for Current<'_, '_, '_, '_, '_, P, S> {
    type Error = Error;
    fn local_food(&mut self, state: &LoadedState, i: usize) -> Result<i32, Error> {
        let instance = state
            .retained()
            .barbarian
            .instances
            .get(i)
            .ok_or(Error::Unsupported("camp food instance"))?;
        self.camps.services.local_food(
            state,
            PlotCoord {
                x: instance.camp_location.x,
                y: instance.camp_location.y,
            },
        )
    }
    fn create_scouts(
        &mut self,
        state: &mut LoadedState,
        i: usize,
        at: PlotCoord,
        range: i32,
        count: i32,
    ) -> Result<(), Error> {
        let range = u32::try_from(range).map_err(|_| Error::Unsupported("camp scout range"))?;
        self.camps
            .factory
            .create_scouts(state, self.camps.sight, self.backend, i, at, range, count)
            .map(|_| ())
            .map_err(|e| fail(self.backend, "Camp scouts", e))
    }
    fn create_tag(
        &mut self,
        state: &mut LoadedState,
        i: usize,
        at: PlotCoord,
        tag: &[u8],
        range: i32,
        count: i32,
    ) -> Result<(), Error> {
        let range = u32::try_from(range).map_err(|_| Error::Unsupported("camp unit range"))?;
        self.camps
            .factory
            .create_tag(
                state,
                self.camps.sight,
                self.backend,
                i,
                at,
                tag,
                range,
                count,
            )
            .map(|_| ())
            .map_err(|e| fail(self.backend, "Camp tagged units", e))
    }
    fn create_unit(
        &mut self,
        state: &mut LoadedState,
        i: usize,
        at: PlotCoord,
        kind: activation::UnitKind,
        range: i32,
    ) -> Result<bool, Error> {
        let range = u32::try_from(range).map_err(|_| Error::Unsupported("camp military range"))?;
        self.camps
            .factory
            .create_military(state, self.camps.sight, self.backend, i, at, kind, range)
            .map_err(|e| fail(self.backend, "Camp military unit", e))
    }
    fn behavior_threshold(
        &self,
        state: &LoadedState,
        i: usize,
        kind: activation::Behavior,
    ) -> Result<i32, Error> {
        self.camps.services.behavior_threshold(state, i, kind)
    }
    fn start_behavior(
        &mut self,
        state: &mut LoadedState,
        i: usize,
        target: ComponentId,
        kind: activation::Behavior,
    ) -> Result<bool, Error> {
        self.camps
            .services
            .start_behavior(state, self.backend, i, target, kind)
    }
}
