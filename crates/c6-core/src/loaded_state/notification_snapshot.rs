//! Initial typed current notification storage, independent of saved transport.
use super::{Error, LoadedState, ObservedSnapshotWitness};
use crate::{
    notifications::PlayerNotifications,
    player_notifications_owner::{OccurrenceLimits, Owner},
};

impl LoadedState {
    /// Install observed ordered semantic records once, before gameplay writes.
    /// Text may explicitly carry `OmittedPresentation`; live runtime fields are
    /// installed separately. No saved byte range or native pool state is inferred.
    pub fn install_current_player_notifications(
        &mut self,
        witness: &ObservedSnapshotWitness,
        collections: Vec<PlayerNotifications>,
    ) -> Result<(), Error> {
        if self.player_notifications.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if self.observed_snapshot.as_ref() != Some(witness) {
            return Err(Error::Binding);
        }
        let owner = Owner::from_decoded_with_limits(
            collections,
            OccurrenceLimits {
                max_records: self.limits.max_roster_records,
                max_occurrences: self.limits.max_runtime_array_items,
            },
        )
        .map_err(Error::PlayerNotifications)?;
        self.advance_admission_generation()?;
        self.player_notifications = Some(owner);
        Ok(())
    }
}

impl LoadedState {
    /// Install an actual typed initial graph from the same observed runtime.
    /// This does not supply omitted live Route fields or captured yield answers.
    pub fn install_current_trade_graph(
        &mut self,
        witness: &ObservedSnapshotWitness,
        graph: crate::trade::Graph,
    ) -> Result<(), Error> {
        if self.trade_graph.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if self.observed_snapshot.as_ref() != Some(witness) {
            return Err(Error::Binding);
        }
        let records = graph
            .edge_count()
            .checked_add(graph.origins().len())
            .ok_or(Error::ResourceLimit)?;
        if records > self.limits.max_runtime_array_items {
            return Err(Error::ResourceLimit);
        }
        let graph =
            crate::trade_graph_state::GraphState::from_graph(graph).map_err(Error::TradeGraph)?;
        self.advance_admission_generation()?;
        self.trade_graph = Some(graph);
        Ok(())
    }
}
