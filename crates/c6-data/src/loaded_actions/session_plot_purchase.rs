//! City plot purchase borrows the persistent stock/modifier/territory owners.
use super::{Error, Session};
use crate::city_plot_purchase::{self, Current, Domains, Inputs, Outcome, Report};
use c6_core::{city_territory::CityId, unit::PlotCoord};
impl<'a> Session<'_, 'a> {
    pub fn city_plot_purchase_gold_cost(
        &self,
        owner: i32,
        city: u32,
        location: PlotCoord,
        inputs: &Inputs<'_>,
    ) -> Result<i32, Error<String>> {
        self.ordinary_player_action(owner)?;
        city_plot_purchase::gold_cost(
            self.state(),
            CityId { owner, id: city },
            location,
            false,
            inputs,
        )
        .map_err(Error::Action)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn purchase_city_plot<D: Domains<'a>>(
        &mut self,
        owner: i32,
        city: u32,
        location: PlotCoord,
        inputs: &Inputs<'_>,
        current: &mut Current<'_>,
        domains: &mut D,
        reports: &mut Vec<Report>,
    ) -> Result<Outcome, Error<String>> {
        self.ordinary_player_action(owner)?;
        let before = self.state().action_stamp();
        let result = city_plot_purchase::purchase_current(
            self.context.notifications.state,
            &mut self.context.notifications.backend,
            CityId { owner, id: city },
            location,
            inputs,
            current,
            domains,
            reports,
        );
        let partial = self.state().action_stamp() != before || result.is_err();
        self.retain(result, partial)
    }
}
