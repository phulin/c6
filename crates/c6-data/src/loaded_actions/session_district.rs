//! Explicit district placement choices using the persistent stock owners.
use super::{Error, Session};
use crate::{city_production_turn as production, district_actions::stock};
use c6_core::unit::PlotCoord;
impl<P> Session<'_, '_, P> {
    /// Append a legal fresh Campus choice, create its incomplete canonical
    /// District and lock its current cost. Production advances on that same
    /// District through the ordinary city production adapter.
    pub fn choose_district<'a>(
        &mut self,
        owner: i32,
        city: u32,
        hash: u32,
        location: PlotCoord,
        inputs: &'a stock::Inputs<'a>,
    ) -> Result<Vec<stock::Report>, Error<production::Error>> {
        self.ordinary_player_action(owner)?;
        let before = self.state().action_stamp();
        let (result, reports) = {
            let mut effects = stock::Effects {
                backend: &mut self.context.notifications.backend,
                research_runtime: &mut self.research,
                inputs,
                gossip_history: &self.visibility.history,
                reports: Vec::new(),
                workers: Vec::new(),
            };
            let result = effects.choose(
                self.context.notifications.state,
                owner,
                city,
                hash,
                location,
            );
            (result, effects.reports)
        };
        let partial_failure = self.state().action_stamp() != before;
        self.retain(result.map(|()| reports), partial_failure)
    }
}
