//! Ordered powered-gossip delivery against the existing owned LoadedState.
//! The current Gossip manager and PlayerManager alive-major vector are supplied
//! as actual logical state, not inferred from sorted player IDs or saved counts.
use super::Definitions;
use c6_core::{
    city_power_activation::{gossip_delivery as core, stock_signals::Occurrence},
    loaded_state::LoadedState,
};

#[derive(Debug)]
pub enum InputError<E> {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Effect(E),
    RulesIdentity,
    MissingRuntime,
    MissingCity { owner: i32, id: u32 },
}
/// Final synchronous localization/status effects after the computed gameplay
/// counter writes. They may reenter all current owners; failures preserve prefix.
pub trait Effects<T> {
    type Error;
    type Formatted;
    fn format_gossip(
        &mut self,
        state: &mut LoadedState,
        manager: &mut core::Manager<T>,
        alive_majors: &mut Vec<i32>,
        occurrence: Occurrence,
        visibility: core::TurnVisibility,
    ) -> Result<Self::Formatted, Self::Error>;
    fn send_status(
        &mut self,
        state: &mut LoadedState,
        manager: &mut core::Manager<T>,
        alive_majors: &mut Vec<i32>,
        formatted: Self::Formatted,
        status: core::Status,
    ) -> Result<(), Self::Error>;
}
struct Context<'a, T, E> {
    state: &'a mut LoadedState,
    manager: &'a mut core::Manager<T>,
    alive: &'a mut Vec<i32>,
    definitions: &'a Definitions,
    effects: &'a mut E,
    city: Option<(i32, u32)>,
}
impl<T, E: Effects<T>> core::Context for Context<'_, T, E> {
    type Error = InputError<E::Error>;
    type OtherOccurrence = T;
    type Formatted = E::Formatted;
    fn manager(&mut self) -> Result<&mut core::Manager<T>, Self::Error> {
        Ok(self.manager)
    }
    fn alive_major_ids(&mut self) -> Result<&[i32], Self::Error> {
        Ok(self.alive)
    }
    fn definition(&mut self, key: u32) -> Result<Option<core::Definition>, Self::Error> {
        Ok(self.definitions.gossip_definition(key))
    }
    fn player_is_valid(&mut self, owner: i32) -> Result<bool, Self::Error> {
        self.state
            .world()
            .player_is_valid(owner)
            .map_err(InputError::World)
    }
    fn relations(
        &mut self,
    ) -> Result<&c6_core::game_diplomatic_relations::GameDiplomaticRelations, Self::Error> {
        self.state.game_relations().map_err(InputError::State)
    }
    fn gossip_reports(
        &mut self,
        owner: i32,
    ) -> Result<&mut c6_core::player_diplomacy::GossipReports, Self::Error> {
        self.state
            .player_gossip_reports_mut(owner)
            .map_err(InputError::State)
    }
    fn current_turn(&mut self) -> Result<i32, Self::Error> {
        Ok(self.state.retained().core.current_turn)
    }
    fn format_gossip(
        &mut self,
        occurrence: Occurrence,
        visibility: core::TurnVisibility,
    ) -> Result<Self::Formatted, Self::Error> {
        self.effects
            .format_gossip(self.state, self.manager, self.alive, occurrence, visibility)
            .map_err(InputError::Effect)
    }
    fn send_status(
        &mut self,
        formatted: Self::Formatted,
        status: core::Status,
    ) -> Result<(), Self::Error> {
        self.effects
            .send_status(self.state, self.manager, self.alive, formatted, status)
            .map_err(InputError::Effect)
    }
}
fn check<E>(
    definitions: &Definitions,
    state: &LoadedState,
) -> Result<(), core::Error<InputError<E>>> {
    if definitions.binding.as_ref() != Some(state.binding()) {
        return Err(core::Error::Input(InputError::RulesIdentity));
    }
    Ok(())
}
/// Full AddGossipItem -> ReportGossip -> SendGossip join, preserving current
/// manager/player owners and actual mutable callback state (no World clone).
pub fn add_powered<T, E: Effects<T>>(
    state: &mut LoadedState,
    manager: &mut core::Manager<T>,
    alive_majors: &mut Vec<i32>,
    definitions: &Definitions,
    effects: &mut E,
    occurrence: Occurrence,
) -> Result<(), core::Error<InputError<E::Error>>> {
    check(definitions, state)?;
    core::add_powered(
        &mut Context {
            state,
            manager,
            alive: alive_majors,
            definitions,
            effects,
            city: None,
        },
        occurrence,
    )
}
pub fn report_powered<T, E: Effects<T>>(
    state: &mut LoadedState,
    manager: &mut core::Manager<T>,
    alive_majors: &mut Vec<i32>,
    definitions: &Definitions,
    effects: &mut E,
    occurrence: Occurrence,
) -> Result<(), core::Error<InputError<E::Error>>> {
    check(definitions, state)?;
    core::report_powered(
        &mut Context {
            state,
            manager,
            alive: alive_majors,
            definitions,
            effects,
            city: None,
        },
        occurrence,
    )
}

/// Actual owners used by the stock callback when first-generation is true.
/// None is sufficient on the source-proven false early-return branch; unknown
/// current runtime is never manufactured as empty visibility/history.
pub struct Runtime<'a, T, E> {
    pub manager: &'a mut core::Manager<T>,
    pub alive_major_ids: &'a mut Vec<i32>,
    pub effects: &'a mut E,
}
impl<T, E: Effects<T>> Context<'_, T, E> {
    fn current_city(&self) -> Result<&c6_core::city::City, InputError<E::Error>> {
        let (owner, id) = self.city.ok_or(InputError::MissingRuntime)?;
        self.state
            .world()
            .player(owner)
            .map_err(InputError::World)?;
        self.state
            .retained()
            .players
            .iter()
            .find(|player| player.requested_owner == owner)
            .and_then(|player| {
                player
                    .cities
                    .records
                    .entries
                    .iter()
                    .find(|slot| id != u32::MAX && slot.index as u32 == id & 0xffff)
            })
            .map(|slot| &slot.value)
            .ok_or(InputError::MissingCity { owner, id })
    }
}
impl<T, E: Effects<T>> c6_core::city_power_activation::stock_signals::GossipContext
    for Context<'_, T, E>
{
    type Error = core::Error<InputError<E::Error>>;
    fn city_owner(&mut self) -> Result<i32, Self::Error> {
        self.current_city()
            .map(|city| city.header.owner)
            .map_err(core::Error::Input)
    }
    fn city_id(&mut self) -> Result<u32, Self::Error> {
        self.current_city()
            .map(|city| city.header.id)
            .map_err(core::Error::Input)
    }
    fn turn_visibility_count(&mut self) -> Result<usize, Self::Error> {
        Ok(self.manager.history.len())
    }
    fn current_turn(&mut self) -> Result<i32, Self::Error> {
        Ok(self.state.retained().core.current_turn)
    }
    fn append_powered_city(&mut self, occurrence: Occurrence) -> Result<(), Self::Error> {
        self.manager
            .pending
            .try_reserve(1)
            .map_err(core::Error::Allocation)?;
        self.manager
            .pending
            .push(core::Item::PoweredCity(occurrence));
        Ok(())
    }
    fn report_powered_city(&mut self, occurrence: Occurrence) -> Result<(), Self::Error> {
        core::report_powered(self, occurrence)
    }
}
/// Stock signal callback -> actual current City fields -> occurrence factory ->
/// current pending/visibility distribution -> owned player counters -> status.
/// Runtime is required only after the computed first-generation true branch.
pub fn powered_city_changed<T, E: Effects<T>>(
    state: &mut LoadedState,
    definitions: &Definitions,
    runtime: Option<Runtime<'_, T, E>>,
    city: (i32, u32),
    first_generation: bool,
) -> Result<(), core::Error<InputError<E::Error>>> {
    if !first_generation {
        return Ok(());
    }
    check(definitions, state)?;
    let runtime = runtime.ok_or(core::Error::Input(InputError::MissingRuntime))?;
    c6_core::city_power_activation::stock_signals::powered_city_changed(
        &mut Context {
            state,
            manager: runtime.manager,
            alive: runtime.alive_major_ids,
            definitions,
            effects: runtime.effects,
            city: Some(city),
        },
        first_generation,
    )
}

/// Render-free arena implementation for the source-qualified stock-owned
/// localization domain. It is a semantic provider, not native capture authority.
/// Current bindings are ordinary mutable state; none are inferred as empty.
pub struct ArenaPresentation {
    pub localization: c6_core::city_power_activation::status_projection::Localization,
    pub status: c6_core::city_power_activation::status_projection::StatusState,
}
/// No formatted strings or desktop state are constructed for an unobserved
/// presentation-only result. Reachable/unknown status callbacks still refuse.
pub struct UnrenderedGossip(());
impl<T> Effects<T> for ArenaPresentation {
    type Error = c6_core::city_power_activation::status_projection::Error;
    type Formatted = UnrenderedGossip;
    fn format_gossip(
        &mut self,
        _: &mut LoadedState,
        _: &mut core::Manager<T>,
        _: &mut Vec<i32>,
        _: Occurrence,
        _: core::TurnVisibility,
    ) -> Result<Self::Formatted, Self::Error> {
        if !self.localization.permits_render_omission() {
            return Err(Self::Error::UnmodeledLocalization);
        }
        Ok(UnrenderedGossip(()))
    }
    fn send_status(
        &mut self,
        _: &mut LoadedState,
        _: &mut core::Manager<T>,
        _: &mut Vec<i32>,
        _: Self::Formatted,
        status: core::Status,
    ) -> Result<(), Self::Error> {
        self.status.omit_status(status.recipient).map(|_| ())
    }
}
