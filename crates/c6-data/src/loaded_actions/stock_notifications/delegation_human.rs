//! Current report consumers for the bounded human-recipient delegation flow.
use super::{movement::Error, Runtime};
use c6_core::{
    diplomacy_delegation::{PendingDecision, Report},
    diplomacy_manager::StatementHandle,
    loaded_state::LoadedState,
};
/// Implement the actual statement-report/session-close consumers against these
/// same canonical gameplay/RNG owners. There are no successful defaults.
pub trait Effects<P> {
    fn preflight(&self, state: &LoadedState, backend: &Runtime<'_, P>) -> Result<(), Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        report: Report,
    ) -> Result<(), Error>;
    fn session_closed(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        session_id: u32,
    ) -> Result<(), Error>;
}
pub(crate) struct State<'a, P> {
    pub effects: Box<dyn Effects<P> + 'a>,
    pub decision: Option<PendingDecision>,
}
impl<P> Runtime<'_, P> {
    pub(super) fn process_initial_human_delegation(
        &mut self,
        state: &mut LoadedState,
        handle: StatementHandle,
    ) -> Result<(), Error> {
        let mut current = self
            .delegation_human
            .take()
            .ok_or(Error::Missing("delegation human report consumers"))?;
        let result = (|| {
            current.effects.preflight(state, self)?;
            let context = super::super::session::delegation::human_context(state)
                .map_err(|_| Error::Missing("delegation human current manager profile"))?;
            let (report, decision) = state
                .diplomacy_manager_mut()
                .map_err(Error::State)?
                .process_delegation_initial(handle, context)
                .map_err(|e| Error::Requirement(format!("delegation initial ownership: {e:?}")))?;
            current.decision = Some(decision);
            current.effects.report(state, self, report)
        })();
        self.delegation_human = Some(current);
        result
    }
    pub(crate) fn report_human_delegation(
        &mut self,
        state: &mut LoadedState,
        report: Report,
    ) -> Result<(), Error> {
        let mut current = self
            .delegation_human
            .take()
            .ok_or(Error::Missing("delegation human report consumers"))?;
        let result = current.effects.report(state, self, report);
        self.delegation_human = Some(current);
        result
    }
    pub(crate) fn close_human_delegation(
        &mut self,
        state: &mut LoadedState,
        header: c6_core::diplomacy_statement::SessionHeader,
    ) -> Result<(), Error> {
        let mut current = self
            .delegation_human
            .take()
            .ok_or(Error::Missing("delegation human close consumers"))?;
        let result = (|| {
            current.effects.preflight(state, self)?;
            state
                .diplomacy_manager_mut()
                .map_err(Error::State)?
                .close_delegation(header)
                .map_err(|e| Error::Requirement(format!("delegation close ownership: {e:?}")))?;
            current.effects.session_closed(state, self, header.id)
        })();
        self.delegation_human = Some(current);
        result
    }
}
