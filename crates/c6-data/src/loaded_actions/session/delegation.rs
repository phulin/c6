//! Retain an interrupted delegation command and its mandatory single Game.Update.
use super::{advance::TurnServices, deactivation, stock_notifications as stock, Error, Session};
use crate::{
    game_update_state::activation::upkeep::PolicyChoice,
    loaded_actions::{configuration::Configuration, delegation_actions as action},
};
pub use action::{Definitions, Effects, Outcome, PendingDecision, Response};

/// Stored on the existing backend, so extracting Session parts retains it.
pub(crate) enum Continuation {
    Decision(action::Request),
    HumanDecision(c6_core::diplomacy_delegation::PendingDecision),
    HumanUpdate {
        phase: HumanPhase,
        choice: Option<PolicyChoice>,
    },
    Update {
        outcome: Outcome,
        choice: Option<PolicyChoice>,
    },
}
impl Continuation {
    pub(crate) fn policy_choice(&self) -> Option<&PolicyChoice> {
        match self {
            Self::Update { choice, .. } => choice.as_ref(),
            Self::HumanUpdate { choice, .. } => choice.as_ref(),
            Self::Decision(_) | Self::HumanDecision(_) => None,
        }
    }
}
#[derive(Debug)]
pub enum Progress {
    HumanDecision(c6_core::diplomacy_delegation::PendingDecision),
    HumanComplete(HumanOutcome),
    Complete(Outcome),
    Choice(PolicyChoice),
}
#[derive(Debug)]
pub enum ResponseError<E, U> {
    Command(action::Error<E>),
    Ingress(stock::movement::Error),
    Update(stock::turn_loop::Error<U>),
}
pub type ResponseResult<E, U> = Result<Progress, Error<ResponseError<E, U>>>;
impl<E: std::fmt::Debug, U: std::fmt::Debug> std::fmt::Display for ResponseError<E, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "delegation response/update: {self:?}")
    }
}
impl<E: std::fmt::Debug, U: std::fmt::Debug> std::error::Error for ResponseError<E, U> {}
impl<'a, P> Session<'_, 'a, P> {
    pub fn pending_delegation(&self) -> Option<PendingDecision> {
        match self.context.notifications.backend.delegation.as_ref() {
            Some(Continuation::Decision(request)) => Some(request.decision),
            _ => None,
        }
    }
    pub fn pending_delegation_update(&self) -> Option<Outcome> {
        match self.context.notifications.backend.delegation.as_ref() {
            Some(Continuation::Update { outcome, .. }) => Some(*outcome),
            _ => None,
        }
    }
    /// Execute a SetDelegation command up to the recipient's external response.
    /// This bounded command surface does not implement the game's broader
    /// IsDiplomaticActionValid UI offer query. Human targets remain unsupported.
    pub fn request_diplomatic_delegation(
        &mut self,
        sender: i32,
        recipient: i32,
        definitions: &Definitions,
        configuration: &Configuration<'_>,
    ) -> Result<PendingDecision, Error<action::Error<std::convert::Infallible>>> {
        self.ordinary_player_action(sender)?;
        definitions
            .cost(self.state(), configuration)
            .map_err(Error::Action)?;
        let request = action::request(self.state(), sender, recipient, configuration)
            .map_err(Error::Action)?;
        self.context.notifications.backend.delegation = Some(Continuation::Decision(request));
        Ok(request.decision)
    }
    /// Resume the same command after an externally selected acceptance/refusal,
    /// then enter exactly the action message's Game.Update. It does not end turns.
    #[allow(clippy::too_many_arguments)]
    pub fn respond_diplomatic_delegation<
        E: Effects<stock::Runtime<'a, P>>,
        S: TurnServices<'a, P>,
    >(
        &mut self,
        recipient: i32,
        response: Response,
        definitions: &Definitions,
        configuration: &Configuration<'_>,
        effects: &mut E,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
    ) -> ResponseResult<E::Error, S::Error>
    where
        E::Error: std::fmt::Debug,
    {
        if let Some(failure) = &self.failed {
            return Err(Error::Failed(failure.clone()));
        }
        let request = match self.context.notifications.backend.delegation.as_ref() {
            Some(Continuation::Decision(request)) if request.decision.recipient == recipient => {
                *request
            }
            _ => return Err(Error::DecisionBoundary("delegation response owner/stage")),
        };
        stock::first_contact_update::preflight_ingress(self.state())
            .map_err(|e| Error::Action(ResponseError::Ingress(e)))?;
        let before = self.state().action_stamp();
        let result = action::respond(
            self.context.notifications.state,
            &mut self.context.notifications.backend,
            &mut self.visibility,
            request,
            response,
            definitions,
            configuration,
            effects,
        )
        .map_err(ResponseError::Command);
        let partial = self.state().action_stamp() != before;
        let outcome = self.retain(result, partial)?;
        self.context.notifications.backend.delegation = Some(Continuation::Update {
            outcome,
            choice: None,
        });
        self.drive_delegation_update(inputs, services, budget, false)
    }
    /// Continue a yielded update after its external policy choice is resolved.
    pub fn resume_diplomatic_delegation_update<S: TurnServices<'a, P>>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
    ) -> ResponseResult<std::convert::Infallible, S::Error> {
        if let Some(failure) = &self.failed {
            return Err(Error::Failed(failure.clone()));
        }
        match self.context.notifications.backend.delegation.as_ref() {
            Some(Continuation::Update { choice: None, .. }) => {}
            _ => {
                return Err(Error::DecisionBoundary(
                    "delegation update missing or policy unresolved",
                ))
            }
        }
        self.drive_delegation_update(inputs, services, budget, true)
    }
    fn drive_delegation_update<E: std::fmt::Debug, S: TurnServices<'a, P>>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
        resuming: bool,
    ) -> ResponseResult<E, S::Error> {
        let result = stock::first_contact_update::execute(
            &mut self.context,
            &mut self.update,
            self.research
                .as_mut()
                .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?,
            inputs,
            &mut self.visibility,
            services,
            budget,
            resuming,
        )
        .map_err(ResponseError::Update);
        match self.retain(result, true)? {
            Some(choice) => {
                let Some(Continuation::Update {
                    choice: pending, ..
                }) = self.context.notifications.backend.delegation.as_mut()
                else {
                    unreachable!("delegation update owner retained")
                };
                *pending = Some(choice.clone());
                Ok(Progress::Choice(choice))
            }
            None => {
                let Some(Continuation::Update { outcome, .. }) =
                    self.context.notifications.backend.delegation.take()
                else {
                    unreachable!("delegation update owner retained")
                };
                Ok(Progress::Complete(outcome))
            }
        }
    }
}

/// Initial statement choices are external; the sole-Exit acknowledgment closes
/// deterministically after the embedded command's mandatory Update returns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HumanOutcome {
    pub choice: c6_core::diplomacy_delegation::Choice,
    pub delegation: Option<Outcome>,
}
pub(crate) enum HumanPhase {
    Initial,
    Action {
        outcome: HumanOutcome,
        session: c6_core::diplomacy_statement::SessionHeader,
    },
}
pub(crate) fn human_context(
    state: &c6_core::loaded_state::LoadedState,
) -> Result<c6_core::diplomacy_statement::Context, &'static str> {
    let manager = state
        .diplomacy_manager()
        .map_err(|_| "delegation manager")?;
    let (game_mode, logging) = manager
        .execution_profile()
        .ok_or("delegation execution profile")?;
    Ok(c6_core::diplomacy_statement::Context {
        local_player: state.retained().core.local_player,
        game_mode,
        logging,
        active_session: manager
            .active_session()
            .map_err(|_| "delegation active session")?,
    })
}
impl<'a, P> Session<'_, 'a, P> {
    pub fn pending_human_delegation(
        &self,
    ) -> Option<c6_core::diplomacy_delegation::PendingDecision> {
        match self.context.notifications.backend.delegation.as_ref() {
            Some(Continuation::HumanDecision(d)) => Some(*d),
            _ => None,
        }
    }
    /// Execute an externally requested AI offer to the local human. This is a
    /// diplomacy command even when its sender is inactive; it grants no ordinary
    /// inactive-player action or turn. Initial ShowUI semantics are preserved.
    #[allow(clippy::too_many_arguments)]
    pub fn request_diplomatic_delegation_to_human<S: TurnServices<'a, P>>(
        &mut self,
        sender: i32,
        recipient: i32,
        definitions: &Definitions,
        configuration: &Configuration<'_>,
        capabilities: &c6_core::game_capabilities::Membership,
        effects: Box<dyn stock::delegation_human::Effects<P> + 'a>,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
    ) -> ResponseResult<std::convert::Infallible, S::Error> {
        self.check()?;
        if self.pending_choice().is_some() || self.update.pending_player_effect().is_some() {
            return Err(Error::DecisionBoundary(
                "human delegation request during another policy continuation",
            ));
        }
        action::request_human(
            self.state(),
            sender,
            recipient,
            definitions,
            configuration,
            capabilities,
        )
        .map_err(|e| Error::Action(ResponseError::Command(e)))?;
        stock::first_contact_update::preflight_ingress(self.state())
            .map_err(|e| Error::Action(ResponseError::Ingress(e)))?;
        effects
            .preflight(self.state(), &self.context.notifications.backend)
            .map_err(|e| Error::Action(ResponseError::Ingress(e)))?;
        let context = human_context(self.state()).map_err(Error::DecisionBoundary)?;
        let manager = self
            .state()
            .diplomacy_manager()
            .map_err(|_| Error::DecisionBoundary("delegation manager"))?;
        if !manager.open_sessions().is_empty()
            || context.active_session.is_some()
            || manager.first_meeting_closed_sessions().is_none()
            || manager.pending_actions_known().is_none()
        {
            return Err(Error::DecisionBoundary(
                "fresh delegation session and current pending action/closed inventory required",
            ));
        }
        let start = self
            .state()
            .diplomacy_manager()
            .map_err(|_| Error::DecisionBoundary("delegation inventory"))?
            .statement_inventory()
            .ok_or(Error::DecisionBoundary("delegation inventory"))?
            .iter()
            .map(|s| s.handle.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(Error::DecisionBoundary(
                "delegation statement identity exhausted",
            ))?;
        self.context.notifications.backend.delegation_human =
            Some(stock::delegation_human::State {
                effects,
                decision: None,
            });
        self.context.notifications.backend.delegation = Some(Continuation::HumanUpdate {
            phase: HumanPhase::Initial,
            choice: None,
        });
        let result = self
            .context
            .notifications
            .state
            .diplomacy_manager_mut()
            .map_err(|e| ResponseError::Command(action::Error::State(e)))
            .and_then(|m| {
                m.consume_delegation_action(sender, recipient)
                    .map_err(|_| {
                        ResponseError::Command(action::Error::Missing(
                            "delegation pending action inventory",
                        ))
                    })?;
                m.request_delegation(
                    sender,
                    recipient,
                    c6_core::diplomacy_manager::StatementHandle(start),
                    context,
                )
                .map_err(|_| {
                    ResponseError::Command(action::Error::Missing("delegation request ownership"))
                })
            });
        let queued = self.retain(result, true)?;
        if !queued {
            return Err(Error::DecisionBoundary(
                "existing open diplomacy session rejected delegation request",
            ));
        }
        self.drive_human_delegation_update(inputs, services, budget, false)
    }
    /// Positive and Negative have different response subtypes but both send the
    /// original embedded enable command with flags3. Exit sends only CLOSE.
    #[allow(clippy::too_many_arguments)]
    pub fn respond_human_diplomatic_delegation<
        E: Effects<stock::Runtime<'a, P>>,
        S: TurnServices<'a, P>,
    >(
        &mut self,
        recipient: i32,
        choice: c6_core::diplomacy_delegation::Choice,
        definitions: &Definitions,
        configuration: &Configuration<'_>,
        effects: &mut E,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
    ) -> ResponseResult<E::Error, S::Error>
    where
        E::Error: std::fmt::Debug,
    {
        if let Some(failure) = &self.failed {
            return Err(Error::Failed(failure.clone()));
        }
        let decision = self
            .pending_human_delegation()
            .filter(|d| d.player == recipient)
            .ok_or(Error::DecisionBoundary(
                "human delegation response owner/stage",
            ))?;
        let context = human_context(self.state()).map_err(Error::DecisionBoundary)?;
        self.state()
            .diplomacy_manager()
            .map_err(|_| Error::DecisionBoundary("delegation manager"))?
            .validate_delegation_choice(decision, context)
            .map_err(|_| Error::DecisionBoundary("delegation response ownership"))?;
        let human = self
            .context
            .notifications
            .backend
            .delegation_human
            .as_ref()
            .ok_or(Error::DecisionBoundary("delegation human consumers"))?;
        human
            .effects
            .preflight(self.state(), &self.context.notifications.backend)
            .map_err(|e| Error::Action(ResponseError::Ingress(e)))?;
        if choice == c6_core::diplomacy_delegation::Choice::Exit {
            let result = self
                .context
                .notifications
                .backend
                .close_human_delegation(self.context.notifications.state, decision.session)
                .map_err(ResponseError::Ingress);
            self.retain(result, true)?;
            self.context.notifications.backend.delegation = None;
            self.context.notifications.backend.delegation_human = None;
            return Ok(Progress::HumanComplete(HumanOutcome {
                choice,
                delegation: None,
            }));
        }
        stock::first_contact_update::preflight_ingress(self.state())
            .map_err(|e| Error::Action(ResponseError::Ingress(e)))?;
        // Qualify callbacks and current cost before committing the response.
        definitions
            .cost(self.state(), configuration)
            .map_err(|e| Error::Action(ResponseError::Command(e)))?;
        effects
            .preflight(
                self.state(),
                &self.context.notifications.backend,
                &self.visibility,
                decision.session.from,
                recipient,
            )
            .map_err(|e| Error::Action(ResponseError::Command(action::Error::Effect(e))))?;
        let start = self
            .state()
            .diplomacy_manager()
            .unwrap()
            .statement_inventory()
            .unwrap()
            .iter()
            .map(|s| s.handle.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(Error::DecisionBoundary(
                "delegation response identity exhausted",
            ))?;
        let last = start.checked_add(2).ok_or(Error::DecisionBoundary(
            "delegation response identity exhausted",
        ))?;
        let handles = [
            c6_core::diplomacy_manager::StatementHandle(start),
            c6_core::diplomacy_manager::StatementHandle(start + 1),
            c6_core::diplomacy_manager::StatementHandle(last),
        ];
        let result = self
            .context
            .notifications
            .state
            .diplomacy_manager_mut()
            .map_err(|e| ResponseError::Command(action::Error::State(e)))
            .and_then(|m| {
                m.respond_delegation(decision, choice, context, handles)
                    .map_err(|_| {
                        ResponseError::Command(action::Error::Missing(
                            "delegation response ownership",
                        ))
                    })
            });
        let report = self
            .retain(result, true)?
            .ok_or(Error::DecisionBoundary("delegation response report"))?;
        let result = self
            .context
            .notifications
            .backend
            .report_human_delegation(self.context.notifications.state, report)
            .map_err(ResponseError::Ingress);
        self.retain(result, true)?;
        let result = self
            .context
            .notifications
            .state
            .diplomacy_manager_mut()
            .map_err(|e| ResponseError::Command(action::Error::State(e)))
            .and_then(|m| {
                m.consume_delegation_action(decision.session.from, recipient)
                    .map_err(|_| {
                        ResponseError::Command(action::Error::Missing(
                            "delegation pending action inventory",
                        ))
                    })
            });
        self.retain(result, true)?;
        let result = action::apply_human_response(
            self.context.notifications.state,
            &mut self.context.notifications.backend,
            &mut self.visibility,
            decision.session.from,
            recipient,
            definitions,
            configuration,
            effects,
        )
        .map_err(ResponseError::Command);
        let outcome = HumanOutcome {
            choice,
            delegation: Some(self.retain(result, true)?),
        };
        self.context.notifications.backend.delegation = Some(Continuation::HumanUpdate {
            phase: HumanPhase::Action {
                outcome,
                session: decision.session,
            },
            choice: None,
        });
        self.drive_human_delegation_update(inputs, services, budget, false)
    }
    pub fn resume_human_diplomatic_delegation_update<S: TurnServices<'a, P>>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
    ) -> ResponseResult<std::convert::Infallible, S::Error> {
        if let Some(failure) = &self.failed {
            return Err(Error::Failed(failure.clone()));
        }
        if !matches!(
            self.context.notifications.backend.delegation,
            Some(Continuation::HumanUpdate { choice: None, .. })
        ) {
            return Err(Error::DecisionBoundary(
                "human delegation update missing or policy unresolved",
            ));
        }
        self.drive_human_delegation_update(inputs, services, budget, true)
    }
    fn drive_human_delegation_update<E: std::fmt::Debug, S: TurnServices<'a, P>>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
        resuming: bool,
    ) -> ResponseResult<E, S::Error> {
        let result = stock::first_contact_update::execute(
            &mut self.context,
            &mut self.update,
            self.research
                .as_mut()
                .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?,
            inputs,
            &mut self.visibility,
            services,
            budget,
            resuming,
        )
        .map_err(ResponseError::Update);
        if let Some(choice) = self.retain(result, true)? {
            let Some(Continuation::HumanUpdate {
                choice: pending, ..
            }) = self.context.notifications.backend.delegation.as_mut()
            else {
                unreachable!("human update retained")
            };
            *pending = Some(choice.clone());
            return Ok(Progress::Choice(choice));
        }
        let Some(Continuation::HumanUpdate { phase, .. }) =
            self.context.notifications.backend.delegation.take()
        else {
            unreachable!("human update retained")
        };
        match phase {
            HumanPhase::Initial => {
                let decision = self
                    .context
                    .notifications
                    .backend
                    .delegation_human
                    .as_mut()
                    .and_then(|h| h.decision.take())
                    .ok_or(Error::DecisionBoundary(
                        "delegation initial Update did not deliver human statement",
                    ))?;
                self.context.notifications.backend.delegation =
                    Some(Continuation::HumanDecision(decision));
                Ok(Progress::HumanDecision(decision))
            }
            HumanPhase::Action { outcome, session } => {
                let result = self
                    .context
                    .notifications
                    .backend
                    .close_human_delegation(self.context.notifications.state, session)
                    .map_err(ResponseError::Ingress);
                self.retain(result, true)?;
                self.context.notifications.backend.delegation_human = None;
                Ok(Progress::HumanComplete(outcome))
            }
        }
    }
}
