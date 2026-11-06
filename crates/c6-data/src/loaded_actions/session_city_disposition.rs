//! Captured-city policy boundary over the SAME retained combat owner.
use super::Session;
use crate::city_capture::PendingDisposition;
impl<P> Session<'_, '_, P> {
    /// The earliest completed capture whose disposition has not been resolved.
    /// Melee completion (advance, callbacks, RNG and queue cleanup) finishes
    /// before this choice becomes visible. Extracting and rewrapping Session
    /// parts preserves it with the retained combat record. Typed initialized
    /// completed decisions are also visible without inventing combat history.
    ///
    /// Keep is available through `keep_city`; unsupported directives remain
    /// pending and other actions/turns stay blocked until resolution.
    pub fn pending_city_disposition(&self) -> Option<&PendingDisposition> {
        let backend = &self.context.notifications.backend;
        backend
            .pending_city_dispositions()
            .iter()
            .find(|choice| backend.completed_city_disposition(choice))
    }
}

use super::{stock_notifications, Error};
use crate::loaded_actions::city_disposition_actions as action;
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

/// Synchronous Keep consumers borrow the SAME retained runtime. In particular,
/// CityOccupied listeners, expulsion, loyalty and any RNG must use its ongoing
/// requirement/modifier/territory/script owners. No callback has an empty default.
pub trait Services<'a, P> {
    fn event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        event: action::Event,
    ) -> Result<(), String>;
}
struct Current<'s, 'a, P, S> {
    backend: &'s mut stock_notifications::Runtime<'a, P>,
    services: &'s mut S,
}
impl<'a, P, S: Services<'a, P>> action::Effects for Current<'_, 'a, P, S> {
    fn event(&mut self, state: &mut LoadedState, event: action::Event) -> Result<(), String> {
        self.services.event(state, self.backend, event)
    }
}
impl<'a, P> Session<'_, 'a, P> {
    /// Execute the pending ordinary combat-capture Keep command. The capture
    /// record remains historical; only the current queue entry is consumed.
    /// Callback failures retain the applied prefix and freeze this Session.
    #[allow(clippy::too_many_arguments)]
    pub fn keep_city<S: Services<'a, P>>(
        &mut self,
        issuer: i32,
        city: ComponentId,
        definitions: &action::Definitions,
        levels: &[(i32, u32)],
        phases: &crate::unit_operation_manager::PhaseDefinitions,
        services: &mut S,
        budget: &mut usize,
    ) -> Result<(), Error<action::Error>> {
        // The generic ordinary-action guard rejects this very pending choice.
        // Recheck its other conditions directly without removing the choice or
        // temporarily altering policy/runtime state.
        if let Some(failure) = &self.failed {
            return Err(Error::Failed(failure.clone()));
        }
        let controls = self
            .state()
            .partial_player_controls(issuer)
            .map_err(|_| Error::DecisionBoundary("current player controls unavailable"))?;
        if controls.turn_active != Some(true) {
            return Err(Error::DecisionBoundary("requested player is not active"));
        }
        match self.pending_choice() {
            Some(choice)
                if choice.player == issuer
                    && choice.kind
                        == crate::game_update_state::activation::upkeep::PolicyKind::PlayerActions
                    && self.update.pending_player_effect()
                        == Some(c6_core::game_update::PlayerEffect::ActiveControl {
                            player: issuer,
                        }) => {}
            Some(_) => {
                return Err(Error::DecisionBoundary(
                    "request differs from suspended player-action owner",
                ))
            }
            None if controls.slot_status == 3 && self.update.pending_player_effect().is_none() => {}
            None => {
                return Err(Error::DecisionBoundary(
                    "ordinary human action or suspended player action required",
                ))
            }
        }
        if i32::from(city.owner) != issuer {
            return Err(Error::DecisionBoundary(
                "Keep issuer differs from City owner",
            ));
        }
        let choice = self
            .pending_city_disposition()
            .filter(|choice| choice.city == city)
            .cloned()
            .ok_or(Error::DecisionBoundary(
                "current completed capture disposition required",
            ))?;
        if !action::allow_strategic(
            phases,
            self.state().retained().core.phase_processor.current_phase,
        )
        .map_err(Error::Action)?
        {
            return Err(Error::DecisionBoundary(
                "strategic City commands unavailable",
            ));
        }
        if controls.slot_status == 3
            && self
                .state()
                .pause_storage()
                .map_err(|e| Error::Action(action::error(action::Stage::Admission, e)))?
                != 0
        {
            return Err(Error::DecisionBoundary("human City command while paused"));
        }
        let before = self.state().action_stamp();
        let result = (|| {
            action::keep(
                self.context.notifications.state,
                &choice,
                definitions,
                levels,
                &mut Current {
                    backend: &mut self.context.notifications.backend,
                    services,
                },
            )?;
            // Destroy.UpdatesEndTurnBlockingState is true. Manager.Start and
            // CityCommand.Execute independently refresh the ordinary blockers.
            // The cleared native markers expire ConsiderRaze while the queue
            // entry stays retained until every continuation succeeds.
            self.context
                .notifications
                .update(issuer, budget)
                .map_err(|e| action::error(action::Stage::Blocking, (e, self.context.notifications.failure())))?;
            self.context
                .notifications
                .update(issuer, budget)
                .map_err(|e| action::error(action::Stage::Blocking, (e, self.context.notifications.failure())))?;
            self.context
                .messages_processed()
                .map_err(|e| action::error(action::Stage::MessagesProcessed, e))?;
            self.context
                .notifications
                .backend
                .complete_city_disposition(self.context.notifications.state, &choice)
                .map_err(|e| action::error(action::Stage::Queue, e))
        })();
        let partial = self.state().action_stamp() != before;
        self.retain(result, partial)
    }
}
