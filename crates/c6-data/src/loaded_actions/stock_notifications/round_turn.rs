//! Game.EndTurn/BeginTurn using the persistent stock player and world owners.
use super::{round_end, round_world, Runtime};
use c6_core::{
    game_update::Error,
    loaded_state::{
        game_update::{self as loaded, round},
        LoadedState,
    },
};

/// Synchronous processor signals and the queued completion message retain their
/// native ordering. The completion broadcast must not execute reentrantly.
pub trait Effects<P>: round_world::Effects<P> {
    fn phase_automation(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<(), Error>;
    fn end_phase(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<(), Error>;
    fn phase_time_limit(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<u32, Error>;
    fn begin_phase(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
        previous: i32,
    ) -> Result<(), Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        event: u32,
        value: i32,
    ) -> Result<(), Error>;
    fn queue_game_turn_complete(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<(), Error>;
    fn turn_signal(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        managers: &crate::game_update_state::activation::cityless_managers::Runtime,
        begin: bool,
        turn: i32,
    ) -> Result<(), Error>;
    fn activate_at_turn_start(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
    ) -> Result<(), Error>;
    fn autoplay_end(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
    ) -> Result<(), Error>;
}

struct Phase<'a, 'b, P, E> {
    backend: &'a mut Runtime<'b, P>,
    effects: &'a mut E,
    runtime: &'a mut loaded::Runtime,
}
impl<P, E: Effects<P>> crate::game_update_state::activation::round::Effects
    for Phase<'_, '_, P, E>
{
    fn automation(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        self.effects.phase_automation(state, self.backend)
    }
    fn end_phase(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        self.effects.end_phase(state, self.backend)
    }
    fn segment_disabled(&mut self, state: &LoadedState, hash: u32) -> Result<bool, Error> {
        Ok(state
            .retained()
            .core
            .phase_processor
            .turn_segments
            .iter()
            .find(|s| s.hash == hash)
            .is_none_or(|s| s.disabled))
    }
    fn time_limit(&mut self, state: &mut LoadedState) -> Result<u32, Error> {
        self.effects.phase_time_limit(state, self.backend)
    }
    fn begin_phase(&mut self, state: &mut LoadedState, previous: i32) -> Result<(), Error> {
        self.effects
            .begin_phase(state, self.backend, self.runtime, previous)
    }
}

/// Persistent world owners and borrowed current player queries. Activation and
/// world upkeep borrow the same cityless manager for their respective stages.
pub struct State<'a, 'd, 'r, E> {
    pub map: crate::game_update_state::round_map::Runtime,
    pub climate: crate::game_update_state::round_climate::Runtime,
    pub cityless:
        &'a std::cell::RefCell<crate::game_update_state::activation::cityless_managers::Runtime>,
    pub carbon: crate::game_update_state::round_climate::CarbonRules<'a>,
    pub alive_ids: &'a [i32],
    pub alive_major_ids: &'a [i32],
    /// Actions and round upkeep share this one manager; only the Rc handle
    /// is copied. Failed/reentrant mutable borrowing is an incomplete boundary.
    pub trade: std::rc::Rc<std::cell::RefCell<crate::game_update_state::round_trade::Runtime>>,
    pub random_definitions: crate::game_update_state::round_random_events::Definitions,
    pub random: crate::game_update_state::round_random_events::Runtime,
    pub fallout: crate::game_update_state::round_fallout::Runtime,
    pub great_people_definitions: crate::game_update_state::round_great_people::Definitions,
    pub great_people: crate::game_update_state::round_great_people::Runtime,
    pub quests: &'a std::cell::RefCell<crate::game_update_state::round_quests::Runtime>,
    pub diplomatic_definitions: crate::game_update_state::round_diplomatic_relations::Definitions,
    pub era_definitions: crate::game_update_state::round_eras::Definitions,
    pub congress_definitions: crate::game_update_state::round_congress::Definitions,
    pub favor_definitions: crate::game_update_state::round_congress::favor::Definitions,
    pub capabilities: &'a c6_core::game_capabilities::Membership,
    pub effects: E,
    pub player_end: round_end::Inputs<'a, 'd, 'r>,
}
impl<P, E: Effects<P>> super::round_dispatch::Services<P> for State<'_, '_, '_, E> {
    fn advance(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
        inputs: &super::deactivation::Inputs<'_, '_, '_>,
        entry: super::round_dispatch::Entry,
        budget: &mut usize,
    ) -> Result<super::round_dispatch::Progress, Error> {
        use super::round_dispatch::{Entry, Progress};
        let mut managers = self
            .cityless
            .try_borrow_mut()
            .map_err(|_| Error::Unsupported("round current manager borrow"))?;
        if entry == Entry::AdvancePhase {
            let mut changed = false;
            let phase = crate::game_update_state::activation::round::advance_phase(
                state,
                inputs.phases,
                inputs.phase_definitions,
                &mut changed,
                &mut Phase {
                    backend,
                    effects: &mut self.effects,
                    runtime,
                },
            )?;
            if phase != round::PhaseAdvance::Exhausted {
                return Err(Error::Unsupported("round next-phase player selection"));
            }
            if state.retained().core.player_processor.active_count != 0
                || self.alive_major_ids.is_empty()
                || round::unprocessed_automation(state, self.alive_ids)? != 0
            {
                return Err(Error::Unsupported(
                    "round end activation/automation continuation",
                ));
            }
        }
        let mut trade = self
            .trade
            .try_borrow_mut()
            .map_err(|_| Error::Unsupported("round current Trade manager borrow"))?;
        let mut current = Current {
            world: round_world::Current {
                backend,
                map: &mut self.map,
                climate: &mut self.climate,
                cityless: &mut managers,
                carbon: crate::game_update_state::round_climate::CarbonRules {
                    resources: self.carbon.resources,
                    game_speed_cost_multiplier: self.carbon.game_speed_cost_multiplier,
                    removable_feature_hashes: self.carbon.removable_feature_hashes,
                },
                alive_major_ids: self.alive_major_ids,
                trade: &mut trade,
                random_definitions: &self.random_definitions,
                random: &mut self.random,
                fallout: &mut self.fallout,
                great_people_definitions: &self.great_people_definitions,
                great_people: &mut self.great_people,
                quests: &mut self.quests.borrow_mut(),
                diplomatic_definitions: &self.diplomatic_definitions,
                era_definitions: &self.era_definitions,
                congress_definitions: &self.congress_definitions,
                favor_definitions: &self.favor_definitions,
                alive_ids: self.alive_ids,
                capabilities: self.capabilities,
                effects: &mut self.effects,
            },
            player_end: &self.player_end,
        };
        if entry == Entry::AdvancePhase {
            round::end_turn(state, runtime, &mut current)?;
            let backend = &mut *current.world.backend;
            let mut c =
                crate::game_update_state::context(state, runtime, inputs.readiness, backend)?;
            c.finish_round_update(|state, services| {
                state
                    .process_pending_diplomacy(&mut *services.effects, budget)
                    .map_err(|_| Error::Unsupported("round end pending diplomacy"))
            })?;
            return Ok(Progress::Returned);
        }
        let begun = round::begin_turn(state, runtime, &mut current)?;
        let backend = &mut *current.world.backend;
        let mut c = crate::game_update_state::context(state, runtime, inputs.readiness, backend)?;
        match round::after_begin_turn(&mut c, begun)? {
            round::AfterBegin::PlayerUpdates => Ok(Progress::PlayerUpdates),
            round::AfterBegin::HumanExit => {
                c.state
                    .process_pending_diplomacy(&mut *c.services.effects, budget)
                    .map_err(|_| Error::Unsupported("round begin pending diplomacy"))?;
                Ok(Progress::Returned)
            }
        }
    }
}

pub struct Current<'a, 'b, 'd, 'r, P, E> {
    pub world: round_world::Current<'a, 'b, P, E>,
    pub player_end: &'a round_end::Inputs<'a, 'd, 'r>,
}
impl<P, E: Effects<P>> round::TurnEffects for Current<'_, '_, '_, '_, P, E> {
    fn report(&mut self, state: &mut LoadedState, event: u32, value: i32) -> Result<(), Error> {
        self.world
            .effects
            .report(state, self.world.backend, event, value)
    }
    fn player_begin(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        owner: i32,
    ) -> Result<(), Error> {
        self.world.backend.begin_round_ai(state, runtime, owner)
    }
    fn player_end(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        self.world
            .backend
            .end_player_round(state, self.player_end, owner)
            .map_err(|e| {
                self.world.backend.last_error = Some(e.to_string());
                Error::Unsupported("player round end continuation")
            })
    }
    fn player_end_tail(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        self.world
            .effects
            .queue_game_turn_complete(state, self.world.backend)
    }
    fn world_begin(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        round::xp2_begin_world(state, &mut self.world)
    }
    fn turn_signal(
        &mut self,
        state: &mut LoadedState,
        begin: bool,
        turn: i32,
    ) -> Result<(), Error> {
        self.world
            .effects
            .turn_signal(state, self.world.backend, self.world.cityless, begin, turn)
    }
    fn activate_at_turn_start(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
    ) -> Result<(), Error> {
        self.world
            .effects
            .activate_at_turn_start(state, self.world.backend, runtime)
    }
    fn autoplay_end(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
    ) -> Result<(), Error> {
        self.world
            .effects
            .autoplay_end(state, self.world.backend, runtime)
    }
}
