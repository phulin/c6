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
    Update {
        outcome: Outcome,
        choice: Option<PolicyChoice>,
    },
}
impl Continuation {
    pub(crate) fn policy_choice(&self) -> Option<&PolicyChoice> {
        match self {
            Self::Update { choice, .. } => choice.as_ref(),
            Self::Decision(_) => None,
        }
    }
}
#[derive(Debug)]
pub enum Progress {
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
