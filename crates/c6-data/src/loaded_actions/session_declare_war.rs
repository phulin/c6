//! One explicit diplomatic player decision on the persistent Session owners.
use super::{Error, Session};
use crate::loaded_actions::declare_war::{self as war, Effects, Inputs, Legality, Outcome};
impl<'a, P> Session<'_, 'a, P> {
    pub fn can_declare_surprise_war(
        &self,
        issuer: i32,
        target: i32,
        inputs: &Inputs<'_, '_>,
    ) -> Result<Legality, Error<war::Error<std::convert::Infallible>>> {
        self.ordinary_player_action(issuer)?;
        war::can_declare_surprise_war(self.state(), issuer, target, inputs).map_err(Error::Action)
    }
    pub fn declare_surprise_war<E: Effects<super::stock_notifications::Runtime<'a, P>>>(
        &mut self,
        issuer: i32,
        target: i32,
        inputs: &Inputs<'_, '_>,
        effects: &mut E,
    ) -> Result<Outcome, Error<war::Error<E::Error>>>
    where
        E::Error: std::fmt::Debug,
    {
        self.ordinary_player_action(issuer)?;
        let before = self.state().action_stamp();
        let result = war::execute(
            self.context.notifications.state,
            &mut self.context.notifications.backend,
            self.research
                .as_mut()
                .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?,
            &mut self.visibility,
            issuer,
            target,
            inputs,
            effects,
        );
        let partial = self.state().action_stamp() != before;
        self.retain(result, partial)
    }
}
