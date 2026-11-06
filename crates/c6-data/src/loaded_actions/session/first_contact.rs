//! Response delivery with the mandatory update on the retained Session owners.
use super::{advance::TurnServices, deactivation, stock_notifications as stock, Error, Session};
use c6_core::diplomacy_first_meeting::{Choice, ChoiceProgress};
pub use stock::first_contact_update::{Error as UpdateError, Outcome};

impl<'a, P> Session<'_, 'a, P> {
    /// The response has delivered its result but has not completed its required
    /// Game.Update and close. Inspect pending_choice and failure for its boundary.
    pub fn pending_first_meeting_update(
        &self,
    ) -> Option<c6_core::diplomacy_statement::SessionHeader> {
        self.context
            .notifications
            .backend
            .first_meeting_update
            .as_ref()
            .map(|pending| pending.result.session())
    }
    pub(super) fn first_meeting_requires_update(&self, choice: Choice) -> bool {
        choice == Choice::Positive
            && self
                .pending_first_meetings()
                .first()
                .is_some_and(|decision| {
                    self.state()
                        .diplomacy_manager()
                        .ok()
                        .and_then(|manager| manager.statement_inventory())
                        .is_some_and(|inventory| {
                            inventory.iter().any(|statement| {
                                statement.handle == decision.statement
                                    && matches!(statement.kind,
                        c6_core::diplomacy_statement::StatementKind::FirstMeeting { type_hash, .. }
                        if type_hash == c6_core::diplomacy_first_meeting::INFO_EXCHANGE)
                            })
                        })
                })
    }

    /// Deliver an external response and run its mandatory action-message update.
    /// A policy choice retains the result continuation before session close.
    #[allow(clippy::too_many_arguments)]
    pub fn respond_first_meeting_with_update<S: TurnServices<'a, P>>(
        &mut self,
        issuer: i32,
        choice: Choice,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
    ) -> Result<Outcome, Error<UpdateError<S::Error>>> {
        if let Some(failure) = &self.failed {
            return Err(Error::Failed(failure.clone()));
        }
        if self
            .context
            .notifications
            .backend
            .first_meeting_update
            .is_some()
        {
            return Err(Error::DecisionBoundary(
                "resume first-contact result update",
            ));
        }
        if self
            .pending_first_meetings()
            .first()
            .is_none_or(|d| d.player != issuer)
        {
            return Err(Error::DecisionBoundary("first-contact response owner"));
        }
        if self.first_meeting_requires_update(choice) {
            stock::first_contact_update::preflight_ingress(self.state())
                .map_err(|e| Error::Action(UpdateError::Response(e)))?;
        }
        let before = self.state().action_stamp();
        let decision = self.pending_first_meetings().first().copied();
        let result = self
            .context
            .notifications
            .backend
            .respond_first_meeting(self.context.notifications.state, issuer, choice)
            .map_err(UpdateError::Response);
        let partial = self.state().action_stamp() != before
            || self.pending_first_meetings().first().copied() != decision;
        match self.retain(result, partial)? {
            ChoiceProgress::Pending(decision) => Ok(Outcome::Diplomacy(
                c6_core::diplomacy_first_meeting::ChoiceOutcome::Pending(decision),
            )),
            ChoiceProgress::Closed(session) => Ok(Outcome::Diplomacy(
                c6_core::diplomacy_first_meeting::ChoiceOutcome::Closed(session),
            )),
            ChoiceProgress::RequiresResultActionUpdate(result) => {
                self.context.notifications.backend.first_meeting_update =
                    Some(stock::first_contact_update::Continuation {
                        result,
                        choice: None,
                    });
                self.drive_first_meeting_update(inputs, services, budget, false)
            }
        }
    }

    /// Resume the same suspended update after an external policy resolves the
    /// reached callback. No response, prefix or friendly store is replayed.
    pub fn resume_first_meeting_update<S: TurnServices<'a, P>>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
    ) -> Result<Outcome, Error<UpdateError<S::Error>>> {
        if let Some(failure) = &self.failed {
            return Err(Error::Failed(failure.clone()));
        }
        if self
            .context
            .notifications
            .backend
            .first_meeting_update
            .is_none()
        {
            return Err(Error::DecisionBoundary("no first-contact result update"));
        }
        if self
            .context
            .notifications
            .backend
            .first_meeting_update
            .as_ref()
            .is_some_and(|pending| pending.choice.is_some())
        {
            return Err(Error::DecisionBoundary(
                "resolve first-contact update policy choice",
            ));
        }
        self.drive_first_meeting_update(inputs, services, budget, true)
    }

    fn drive_first_meeting_update<S: TurnServices<'a, P>>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
        resuming: bool,
    ) -> Result<Outcome, Error<UpdateError<S::Error>>> {
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
        .map_err(UpdateError::Update);
        match self.retain(result, true)? {
            Some(choice) => {
                self.context
                    .notifications
                    .backend
                    .first_meeting_update
                    .as_mut()
                    .expect("result continuation retained during update")
                    .choice = Some(choice.clone());
                Ok(Outcome::Choice(choice))
            }
            None => {
                let pending = self
                    .context
                    .notifications
                    .backend
                    .first_meeting_update
                    .take()
                    .expect("result continuation retained during update");
                let result = self
                    .context
                    .notifications
                    .backend
                    .resume_first_meeting_after_result_update(
                        self.context.notifications.state,
                        pending.result,
                    )
                    .map(Outcome::Diplomacy)
                    .map_err(UpdateError::Response);
                self.retain(result, true)
            }
        }
    }
}
