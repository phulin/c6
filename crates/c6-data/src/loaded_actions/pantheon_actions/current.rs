//! Concrete stock continuations for a qualified pantheon callback profile.
//! Research and Script borrow the Session owners; historical moments borrow
//! the existing History/Eras effect services. Unknown consumers remain errors.
use super::{boosts, history, stock, Current as Queries, Definitions, Event};
use crate::{
    loaded_actions::{choose_tech, stock_notifications},
    research,
};
use c6_core::{city_power_activation::gossip_delivery::TurnVisibility, loaded_state::LoadedState};
#[derive(Clone, Copy, Debug)]
pub enum Receiver {
    FaithScript,
    Gossip,
    History,
    Summary,
    Boost { row_id: i32 },
}
pub struct Current<'s, 'q, H> {
    pub receivers: &'s [(u64, Receiver)],
    pub queries: Queries<'s>,
    pub history: &'s crate::melee_attack::resolve::completion::history::Definitions,
    pub history_inputs: crate::melee_attack::resolve::completion::history::Inputs<'s>,
    pub history_effects: &'s mut H,
    pub gossip_history: &'s [TurnVisibility],
    pub boosts: &'s boosts::Definitions,
    pub research: &'s research::Definitions,
    pub technologies: &'s choose_tech::Definitions,
    pub culture: Option<&'s stock_notifications::research::CultureInputs<'q>>,
    /// Actual current report/cache/host receivers, including faith/city/history
    /// publications, must be qualified for the stock read-only output closure.
    pub presentation_reports_qualified: bool,
    pub reports: Vec<Event>,
}
impl<'a, P, H: crate::melee_attack::resolve::completion::history::Effects>
    stock::Continuations<'a, P> for Current<'_, '_, H>
where
    H::Error: std::fmt::Debug,
{
    type Error = String;
    fn callback(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        runtime: &mut research::Runtime,
        owner: i32,
        event: Event,
        receiver: u64,
        _: &Definitions,
    ) -> Result<(), String> {
        let route = self
            .receivers
            .iter()
            .find(|r| r.0 == receiver)
            .ok_or("current pantheon receiver")?
            .1;
        match (route, event) {
            (Receiver::FaithScript, Event::FaithChanged { balance_q8 }) => backend
                .script
                .call_empty(state, "OnFaithEarned", &[owner, balance_q8 >> 8])
                .map_err(|e| format!("{e:?}"))?,
            (Receiver::Gossip, Event::PantheonFounded { .. }) => {
                // 29c5d60/29cb050/29c7e20 construct only fixed occurrence fields;
                // AddGossipItem29b25d0 returns before storage/delivery for empty
                // current visibility history. A nonempty history stays explicit.
                if !self.gossip_history.is_empty() {
                    return Err("current pantheon Gossip delivery".into());
                }
            }
            (Receiver::History, Event::PantheonFounded { belief }) => {
                history::founded(
                    state,
                    owner,
                    belief,
                    &self.queries,
                    self.history,
                    self.history_inputs,
                    self.history_effects,
                )
                .map_err(|e| format!("{e:?}"))?;
            }
            (Receiver::Summary, Event::PantheonFounded { .. }) => {
                // 2a24780 only updates a numeric GameSummary dataset via
                // AddValueToDataSet2d3c5e0 (fixed-point add / sequence insert).
                // No gameplay signal, state owner or RNG is reached there.
            }
            (Receiver::Boost { row_id }, Event::PantheonFounded { .. }) => {
                let mut effects = stock_notifications::research::Effects::new(
                    backend,
                    runtime,
                    self.technologies,
                    self.culture,
                );
                boosts::created(
                    state,
                    self.boosts,
                    owner,
                    row_id,
                    &mut crate::melee_attack::resolve::completion::boosts::ResearchTrigger {
                        definitions: self.research,
                        effects: &mut effects,
                    },
                )
                .map_err(|e| format!("{e:?}"))?;
            }
            _ => return Err("pantheon receiver/event mismatch".into()),
        }
        self.reports.push(event);
        Ok(())
    }
    fn report(
        &mut self,
        _: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'a, P>,
        _: &mut research::Runtime,
        _: i32,
        event: Event,
        _: &Definitions,
    ) -> Result<(), String> {
        if !self.presentation_reports_qualified {
            return Err("current pantheon report consumers".into());
        }
        match event {
            Event::ReportFounded { .. }
            | Event::ReportCity { .. }
            | Event::ReportFaithChanged { .. } => {
                self.reports.push(event);
                Ok(())
            }
            _ => Err("unexpected pantheon report".into()),
        }
    }
}
