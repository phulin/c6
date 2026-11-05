//! Source-ordered completion/continuation of the selected processor pass.
//! Next-player activation is performed here; its first-activation upkeep and
//! the next Game.Update remain mandatory caller continuations.
use super::{turn_reports, Runtime};
use crate::{
    game_update_state as data,
    loaded_actions::{
        configuration::Configuration, current_aftermath, diplomacy_pending_deals, reporting,
    },
    unit_operation_manager::PhaseDefinitions,
    Rules,
};
use c6_core::{
    game_update,
    loaded_state::{game_update as loaded, LoadedState},
};
use std::convert::Infallible;

pub struct Inputs<'a, 'q, 'c> {
    pub readiness: &'a reporting::Provider<'q>,
    pub phase_definitions: &'a PhaseDefinitions,
    pub phases: &'a data::activation::Phases,
    pub configuration: &'a Configuration<'c>,
    pub rules: &'a Rules,
    pub pending_deals: &'a diplomacy_pending_deals::CurrentPendingDeals,
    pub host_reactivation_requested: Option<bool>,
    pub game_speed_type: u32,
    pub game_uninitializing: bool,
    pub visibility_messages_disabled: bool,
    pub npc_services: Option<NpcServices<'a>>,
}
pub struct NpcServices<'a> {
    pub visibility: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
    pub maintenance: &'a super::maintenance::Definitions,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Completed {
    pub deactivated_owner: i32,
    pub selection: loaded::activation::Selection,
    pub units_advanced: usize,
    pub visibility_records: usize,
    pub resources_changed: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "Continue variants remain inside the current Game.Update"]
pub enum Outcome {
    Deactivated(Completed),
    /// An active human remains. The current processor pass, final Autoplay
    /// update and pending diplomacy have completed without selecting anyone.
    NoDeactivation,
    /// Another PlayerUpdates pass inside this same original Game.Update.
    Continue(Completed),
    ContinueWithoutDeactivation,
    /// No selected player remains; phase/round work continues in this Update.
    AdvancePhase,
}
#[derive(Debug)]
pub enum Error {
    Prefix(game_update::Error),
    State(c6_core::loaded_state::Error),
    Report(loaded::deactivation::ReportError<turn_reports::Error>),
    Automation(c6_core::turn_completion::Error),
    Unit(data::automation::PlayerError<Infallible, Infallible>),
    Diplomacy(loaded::automation::diplomacy::Error<DiplomacyError>),
    Visibility(data::deactivation::Error),
    NpcControl(game_update::Error),
    IncompleteNpcControl(i32),
    UnsupportedSelection(Vec<i32>),
    RequiresPhaseAdvance,
    UnsupportedAutomation,
    RequiresQueuedHandler,
    SelectedOwnerStillActive(i32),
    UnsupportedNextSelection(loaded::activation::Selection),
    Budget,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock processor pass: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Debug)]
pub enum DiplomacyError {
    PendingDeals(diplomacy_pending_deals::Error),
    Configuration(crate::loaded_actions::configuration::Error),
    MissingHostReactivation,
    RequestUnready { owner: i32, reason: u32 },
    SetActive(i32),
}
struct Diplomacy<'a, 'c> {
    configuration: &'a Configuration<'c>,
    pending: &'a diplomacy_pending_deals::CurrentPendingDeals,
    host_reactivation: Option<bool>,
}
impl loaded::automation::diplomacy::Services for Diplomacy<'_, '_> {
    type Error = DiplomacyError;
    fn pending_deal(&mut self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error> {
        diplomacy_pending_deals::has_pending_deal(state, owner, self.pending)
            .map_err(DiplomacyError::PendingDeals)
    }
    fn any_multiplayer(&mut self, state: &LoadedState) -> Result<bool, Self::Error> {
        self.configuration
            .is_any_multiplayer(state.binding())
            .map_err(DiplomacyError::Configuration)
    }
    fn network_multiplayer(&mut self, state: &LoadedState) -> Result<bool, Self::Error> {
        self.configuration
            .is_network_multiplayer(state.binding())
            .map_err(DiplomacyError::Configuration)
    }
    fn reactivation_requested(
        &mut self,
        _state: &LoadedState,
        _owner: i32,
    ) -> Result<bool, Self::Error> {
        self.host_reactivation
            .ok_or(DiplomacyError::MissingHostReactivation)
    }
    fn request_unready(
        &mut self,
        _state: &mut LoadedState,
        owner: i32,
        reason: u32,
    ) -> Result<bool, Self::Error> {
        Err(DiplomacyError::RequestUnready { owner, reason })
    }
    fn set_active(&mut self, _state: &mut LoadedState, owner: i32) -> Result<(), Self::Error> {
        Err(DiplomacyError::SetActive(owner))
    }
}
fn finish_update<P>(
    c: &mut loaded::Context<'_, data::Services<'_, '_, Runtime<'_, P>>>,
    inputs: &Inputs<'_, '_, '_>,
    budget: &mut usize,
) -> Result<loaded::ProcessorPass, Error> {
    c.finish_exclusive_processor_pass(
        |state, _| {
            inputs
                .configuration
                .is_any_multiplayer(state.binding())
                .map_err(|_| {
                    game_update::Error::Unsupported("Game.Update footer current multiplayer mode")
                })
        },
        |_, _, _| {
            Err(game_update::Error::Unsupported(
                "Game.Update single-player active diplomacy footer",
            ))
        },
        |state, services| {
            state
                .process_pending_diplomacy(&mut *services.effects, budget)
                .map_err(|_| {
                    game_update::Error::Unsupported("Game.Instance.Update pending diplomacy")
                })
        },
    )
    .map_err(Error::Prefix)
}
/// Continue after all selected Player.Update calls have completed. Every
/// callback uses the existing action backend and canonical state. On failure,
/// the native prefix remains visible and this update runtime cannot be replayed.
pub fn finish_completed_human_pass<P>(
    context: &mut current_aftermath::Context<'_, Runtime<'_, P>>,
    runtime: &mut loaded::Runtime,
    inputs: &Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    budget: &mut usize,
) -> Result<Outcome, Error> {
    let result = (|| {
        let mut c = data::context(
            context.notifications.state,
            runtime,
            inputs.readiness,
            &mut context.notifications.backend,
        )
        .map_err(Error::Prefix)?;
        let mut pending = loaded::deactivation::begin(&mut c).map_err(Error::Prefix)?;
        if pending.selected().is_empty() {
            if c.state.retained().core.player_processor.active_count == 0 {
                return Ok(Outcome::AdvancePhase);
            }
            return Ok(match finish_update(&mut c, inputs, budget)? {
                loaded::ProcessorPass::ContinuePlayerUpdates => {
                    Outcome::ContinueWithoutDeactivation
                }
                loaded::ProcessorPass::Returned => Outcome::NoDeactivation,
            });
        }
        let [owner] = pending.selected() else {
            return Err(Error::UnsupportedSelection(pending.selected().to_vec()));
        };
        let owner = *owner;
        if matches!(
            pending.effect(),
            Some(game_update::deactivation::Effect::TurnActiveLog { .. })
        ) {
            loaded::deactivation::omit_stock_file_log(
                c.state,
                c.runtime,
                &mut pending,
                loaded::deactivation::LoggingScope::StockFileSink,
            )
            .map_err(Error::Prefix)?;
        }
        let backend = &mut *c.services.effects;
        loaded::deactivation::deliver_turn_active_report(
            c.state,
            c.runtime,
            &mut pending,
            |state, event, owner| {
                backend.deliver_turn_report(
                    state,
                    event,
                    owner,
                    turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
                )
            },
        )
        .map_err(Error::Report)?;
        let loaded::automation::Begin::Units(mut frame) =
            data::automation::begin_after_deactivation_report(
                c.state,
                c.runtime,
                &mut pending,
                inputs.phase_definitions,
            )
            .map_err(Error::Automation)?
        else {
            return Err(Error::UnsupportedAutomation);
        };
        let mut units_advanced = 0;
        loop {
            if *budget == 0 {
                return Err(Error::Budget);
            }
            *budget -= 1;
            match data::automation::advance_idle_player_unit(
                c.state,
                &mut frame,
                inputs.readiness,
                Some(loaded::deactivation::LoggingScope::StockFileSink),
            )
            .map_err(Error::Unit)?
            {
                data::automation::IdleProgress::Advanced => units_advanced += 1,
                data::automation::IdleProgress::RequiresQueuedHandler => {
                    return Err(Error::RequiresQueuedHandler)
                }
                data::automation::IdleProgress::NoUnit => break,
            }
        }
        let diplomacy = frame.finish(c.state).map_err(Error::Prefix)?;
        let mut services = Diplomacy {
            configuration: inputs.configuration,
            pending: inputs.pending_deals,
            host_reactivation: inputs.host_reactivation_requested,
        };
        loaded::automation::diplomacy::finish(c.state, diplomacy, &mut services)
            .map_err(Error::Diplomacy)?;
        if c.state.turn_active(owner).map_err(Error::State)? {
            return Err(Error::SelectedOwnerStillActive(owner));
        }
        let visibility_records = visibility
            .record(c.state, owner)
            .map_err(Error::Visibility)?;
        if c.state.player_slot_status(owner).map_err(Error::State)? == 3 {
            loaded::human_control::deactivate_with_priority_signal(
                c.state,
                owner,
                |state, owner, _priority| {
                    let services = inputs.npc_services.as_ref().ok_or(
                        game_update::Error::Unsupported("current human priority signal services"),
                    )?;
                    let sight = services.visibility.try_borrow().map_err(|_| {
                        game_update::Error::Unsupported("current human priority signal sight borrow")
                    })?;
                    super::human_activation::knowledge::refresh_visible_units(
                        state,
                        owner,
                        services.maintenance.maintenance.unit_registry(),
                        &sight,
                        c.services.effects.movement().ok().map(|m|m.definitions.formation),
                    )
                },
            )
            .map_err(Error::Prefix)?;
        } else {
            let services = inputs.npc_services.as_ref().ok_or(Error::Prefix(
                game_update::Error::Unsupported("current NPC deactivation services"),
            ))?;
            match c
                .services
                .effects
                .deactivate_npc_control(
                    c.state,
                    c.runtime,
                    owner,
                    &*services.visibility.try_borrow().map_err(|_| {
                        Error::Prefix(game_update::Error::Unsupported(
                            "current NPC deactivation sight borrow",
                        ))
                    })?,
                    services.maintenance.maintenance.unit_registry(),
                )
                .map_err(Error::NpcControl)?
            {
                super::npc_deactivation::Progress::Complete => {
                    if let Some(frame) = loaded::npc_activation::controller(c.runtime) {
                        if frame.owner() != owner {
                            return Err(Error::Prefix(game_update::Error::Unsupported(
                                "NPC deactivation activation-owner differs",
                            )));
                        }
                        loaded::npc_activation::retire_after_deactivation(c.state, c.runtime)
                            .map_err(Error::Prefix)?;
                    }
                }
                super::npc_deactivation::Progress::Pending => {
                    return Err(Error::IncompleteNpcControl(owner))
                }
            }
        }
        let resources_changed = c
            .services
            .effects
            .finish_deactivation_resources(
                c.state,
                inputs.rules,
                owner,
                inputs.game_speed_type,
                inputs.game_uninitializing,
                turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
            )
            .map_err(Error::Prefix)?;
        let selection = c
            .services
            .effects
            .select_next_player(c.state, inputs.configuration, inputs.phases)
            .map_err(Error::Prefix)?;
        let loaded::activation::Selection::Sequential(next) = selection else {
            if selection == loaded::activation::Selection::AdvancePhase {
                return Ok(Outcome::AdvancePhase);
            }
            return Err(Error::UnsupportedNextSelection(selection));
        };
        c.services
            .effects
            .activate_selected_player(
                c.state,
                c.runtime,
                inputs.configuration,
                next,
                inputs.visibility_messages_disabled,
                turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
            )
            .map_err(Error::Prefix)?;
        let footer = finish_update(&mut c, inputs, budget)?;
        let completed = Completed {
            deactivated_owner: owner,
            selection,
            units_advanced,
            visibility_records,
            resources_changed,
        };
        Ok(match footer {
            loaded::ProcessorPass::ContinuePlayerUpdates => Outcome::Continue(completed),
            loaded::ProcessorPass::Returned => Outcome::Deactivated(completed),
        })
    })();
    if result.is_err() {
        runtime.fail_player_callback();
    }
    result
}
