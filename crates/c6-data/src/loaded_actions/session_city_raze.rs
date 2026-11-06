//! Immediate captured-city Raze over the retained Session owner.
use super::{stock_notifications, Error, Session};
use crate::loaded_actions::city_disposition_actions::{self, raze as action};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

/// Raze subscribers borrow the same retained runtime and ongoing owners.
pub trait Services<'a, P> {
    fn command_started(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        city: ComponentId,
        district: ComponentId,
    ) -> Result<(), String>;
    fn city_razed(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        city: ComponentId,
        previous_owner: i32,
    ) -> Result<(), String>;
    fn destroy(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        city: ComponentId,
    ) -> Result<(), String>;
}
struct Current<'s, 'a, P, S> {
    backend: &'s mut stock_notifications::Runtime<'a, P>,
    services: &'s mut S,
}
impl<'a, P, S: Services<'a, P>> action::Effects for Current<'_, 'a, P, S> {
    fn command_started(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        district: ComponentId,
    ) -> Result<(), String> {
        self.services
            .command_started(state, self.backend, city, district)
    }
    fn city_razed(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        previous_owner: i32,
    ) -> Result<(), String> {
        self.services
            .city_razed(state, self.backend, city, previous_owner)
    }
    fn destroy(&mut self, state: &mut LoadedState, city: ComponentId) -> Result<(), String> {
        self.services.destroy(state, self.backend, city)
    }
}
impl<'a, P> Session<'_, 'a, P> {
    /// Execute the pending ordinary combat-capture Raze command. The capture
    /// record remains historical; only the current queue entry is consumed.
    /// Callback failures retain the applied prefix and freeze this Session.
    #[allow(clippy::too_many_arguments)]
    pub fn raze_city<S: Services<'a, P>>(
        &mut self,
        issuer: i32,
        city: ComponentId,
        definitions: &city_disposition_actions::Definitions,
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
                "Raze issuer differs from City owner",
            ));
        }
        let choice = self
            .pending_city_disposition()
            .filter(|choice| choice.city == city)
            .cloned()
            .ok_or(Error::DecisionBoundary(
                "current completed capture disposition required",
            ))?;
        if !city_disposition_actions::allow_strategic(
            phases,
            self.state().retained().core.phase_processor.current_phase,
        )
        .map_err(|e| Error::Action(action::error(action::Stage::Admission, e)))?
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
            action::execute(
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
            // The removed City expires ConsiderRaze while the queue
            // entry stays retained until every continuation succeeds.
            self.context
                .notifications
                .update(issuer, budget)
                .map_err(|e| {
                    action::error(
                        action::Stage::Blocking,
                        (e, self.context.notifications.failure()),
                    )
                })?;
            self.context
                .notifications
                .update(issuer, budget)
                .map_err(|e| {
                    action::error(
                        action::Stage::Blocking,
                        (e, self.context.notifications.failure()),
                    )
                })?;
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
