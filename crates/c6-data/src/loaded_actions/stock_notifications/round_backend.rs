//! Concrete phase, signal and queued-completion services of the stock round.
use super::{round_turn, round_world, turn_reports, Runtime};
use crate::{
    game_update_state::{
        self as data,
        activation::{requirement_changes as changes, victory_requirements as victory},
        round_phase, round_signals,
    },
    loaded_actions::{configuration::Configuration, notifications::processing},
};
use c6_core::{
    game_update::Error,
    loaded_state::{
        game_update::{self as loaded, activation, round},
        LoadedState,
    },
    melee_attack::proposals::subject_sets,
};
use std::cell::RefCell;

pub struct Current<'a> {
    pub phases: &'a data::activation::Phases,
    pub phase_bindings: round_phase::Bindings,
    pub phase_definitions: round_phase::Definitions,
    pub phase_rules: &'a crate::unit_operation_manager::PhaseDefinitions,
    pub configuration: &'a Configuration<'a>,
    pub notification_inputs: processing::Inputs<'a>,
    pub alive: &'a [i32],
    pub alive_majors: &'a [i32],
    pub visibility_messages_disabled: bool,
    pub signal_bindings: round_signals::Bindings,
    pub requirements: &'a RefCell<changes::Runtime>,
    pub victory_definitions: &'a victory::Definitions,
    pub victory_inputs: &'a victory::Inputs,
    pub victory_receivers: &'a [(u64, victory::Receiver)],
    pub scoring: &'a RefCell<super::scoring::Runtime>,
    pub scoring_definitions: &'a super::scoring::Definitions,
}
fn report<P>(
    state: &LoadedState,
    backend: &mut Runtime<'_, P>,
    event: u32,
    value: i32,
) -> Result<(), Error> {
    backend
        .deliver_round_report(
            state,
            event,
            -1,
            value,
            -1,
            turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
        )
        .map_err(|_| Error::Unsupported("round scalar report"))
}
struct Phase<'a, 'b, 'c, P> {
    current: &'a Current<'c>,
    backend: &'a mut Runtime<'b, P>,
    runtime: Option<&'a mut loaded::Runtime>,
}
impl<P> round_phase::Effects for Phase<'_, '_, '_, P> {
    fn report(&mut self, state: &mut LoadedState, event: u32, value: i32) -> Result<(), Error> {
        report(state, self.backend, event, value)
    }
    fn activate(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        self.backend.activate_selected_player(
            state,
            self.runtime
                .as_deref_mut()
                .ok_or(Error::Unsupported("phase activation current processor"))?,
            self.current.configuration,
            owner,
            self.current.visibility_messages_disabled,
            turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
        )
    }
    fn has_end_turn_blocking(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: u32,
    ) -> Result<bool, Error> {
        let mut provider = processing::Provider::new(
            state,
            self.current.notification_inputs,
            &mut *self.backend,
            c6_core::player_notifications_owner::OccurrenceLimits::default(),
            processing::Scope::SourceInitializedStockXp2,
        );
        provider
            .has_end_turn_blocking(owner, kind)
            .map_err(|_| Error::Unsupported("phase current blocking notification query"))
    }
}
struct Signal<'a, 'b, 'c, P> {
    current: &'a Current<'c>,
    backend: &'a mut Runtime<'b, P>,
}
impl<P> changes::Services for Signal<'_, '_, '_, P> {
    type Error = Error;
    fn test_requirement(
        &mut self,
        state: &mut LoadedState,
        _: &mut subject_sets::Runtime,
        receiver: u64,
    ) -> Result<i32, Error> {
        let descriptor = self
            .current
            .victory_receivers
            .iter()
            .find(|r| r.0 == receiver)
            .map(|r| r.1)
            .ok_or(Error::Unsupported("round current requirement predicate"))?;
        let scoring = self
            .current
            .scoring
            .try_borrow()
            .map_err(|_| Error::Unsupported("round current score owner borrow"))?;
        victory::test(
            state,
            self.current.victory_definitions,
            self.current.victory_inputs,
            &scoring,
            self.current.scoring_definitions,
            descriptor,
        )
        .map_err(|_| Error::Unsupported("round current victory requirement"))
    }
    fn modifier_changed(
        &mut self,
        _: &mut LoadedState,
        _: &mut subject_sets::Runtime,
        _: changes::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), Error> {
        Err(Error::Unsupported(
            "round positive requirement modifier continuation",
        ))
    }
    fn destroy_requirement(
        &mut self,
        _: &mut LoadedState,
        _: &mut subject_sets::Runtime,
        _: u64,
    ) -> Result<(), Error> {
        Err(Error::Unsupported(
            "round requirement destruction continuation",
        ))
    }
    fn destroy_set(
        &mut self,
        _: &mut LoadedState,
        _: &mut subject_sets::Runtime,
        _: u64,
    ) -> Result<(), Error> {
        Err(Error::Unsupported("round set destruction continuation"))
    }
}
impl<P> round_signals::Effects for Signal<'_, '_, '_, P> {
    fn script_turn(&mut self, state: &LoadedState, event: &str, args: &[i32]) -> Result<(), Error> {
        self.backend
            .script
            .call_empty(state, event, args)
            .map_err(|_| Error::Unsupported("round current script subscribers"))
    }
}
impl<P> round_world::Effects<P> for Current<'_> {
    fn manager_turn(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime<'_, P>,
        _: round::WorldManager,
    ) -> Result<(), Error> {
        Err(Error::Unsupported(
            "active Economic round resource calculations",
        ))
    }
}
impl<P> round_turn::Effects<P> for Current<'_> {
    fn phase_automation(
        &mut self,
        state: &mut LoadedState,
        _: &mut Runtime<'_, P>,
    ) -> Result<(), Error> {
        round::processed_automation_pass(state, |s| self.phases.mode(s))
    }
    fn end_phase(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<(), Error> {
        round_phase::end(
            state,
            self.phases,
            &self.phase_bindings,
            &mut Phase {
                current: self,
                backend,
                runtime: None,
            },
        )
    }
    fn phase_time_limit(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<u32, Error> {
        self.phase_definitions.time_limit(
            state,
            self.phases,
            self.alive_majors,
            &mut Phase {
                current: self,
                backend,
                runtime: None,
            },
        )
    }
    fn begin_phase(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
        previous: i32,
    ) -> Result<(), Error> {
        round_phase::begin(
            state,
            self.phases,
            &self.phase_bindings,
            self.alive,
            previous,
            &mut Phase {
                current: self,
                backend,
                runtime: Some(runtime),
            },
        )
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        event: u32,
        value: i32,
    ) -> Result<(), Error> {
        report(state, backend, event, value)
    }
    fn queue_game_turn_complete(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<(), Error> {
        backend.broadcast_game_turn_complete(state)
    }
    fn turn_signal(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        managers: &data::activation::cityless_managers::Runtime,
        begin: bool,
        turn: i32,
    ) -> Result<(), Error> {
        let mut requirements = self
            .requirements
            .try_borrow_mut()
            .map_err(|_| Error::Unsupported("round requirement owner borrow"))?;
        round_signals::run(
            state,
            &self.signal_bindings,
            begin,
            turn,
            managers,
            &mut requirements,
            &mut Signal {
                current: self,
                backend,
            },
        )
    }
    fn activate_at_turn_start(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
    ) -> Result<(), Error> {
        round_phase::set_turn_start_flags(state, self.alive)?;
        let selection = backend.select_next_player(state, self.configuration, self.phases)?;
        let owner = match selection {
            activation::Selection::Sequential(owner) => Some(owner),
            activation::Selection::AdvancePhase => {
                let mut changed = false;
                let phases = self.phases;
                let phase_rules = self.phase_rules;
                let result = data::activation::round::advance_phase(
                    state,
                    phases,
                    phase_rules,
                    &mut changed,
                    &mut Advance {
                        current: self,
                        backend,
                        runtime,
                    },
                )?;
                if result == round::PhaseAdvance::Exhausted {
                    return Err(Error::Unsupported("round start has no enabled phase"));
                }
                activation::next_sequential_in_phase(state, |s| self.phases.mode(s))?
            }
            activation::Selection::NoSequentialPlayer => None,
        };
        if let Some(owner) = owner {
            backend.activate_selected_player(
                state,
                runtime,
                self.configuration,
                owner,
                self.visibility_messages_disabled,
                turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
            )?;
        }
        Ok(())
    }
    fn autoplay_end(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
    ) -> Result<(), Error> {
        round::inactive_autoplay_end(runtime)
    }
}
struct Advance<'a, 'b, 'c, P> {
    current: &'a mut Current<'c>,
    backend: &'a mut Runtime<'b, P>,
    runtime: &'a mut loaded::Runtime,
}
impl<P> data::activation::round::Effects for Advance<'_, '_, '_, P> {
    fn automation(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        round::processed_automation_pass(state, |s| self.current.phases.mode(s))
    }
    fn end_phase(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        <Current<'_> as round_turn::Effects<P>>::end_phase(self.current, state, self.backend)
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
        <Current<'_> as round_turn::Effects<P>>::phase_time_limit(self.current, state, self.backend)
    }
    fn begin_phase(&mut self, state: &mut LoadedState, previous: i32) -> Result<(), Error> {
        <Current<'_> as round_turn::Effects<P>>::begin_phase(
            self.current,
            state,
            self.backend,
            self.runtime,
            previous,
        )
    }
}
