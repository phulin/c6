//! Stock changed-Formation movement publication. Both native delegates are
//! cache-side; current member Unit snapshot queries remain mandatory.
use super::*;
use c6_core::unit_turn_start::FormationRestoredReport;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Consumer {
    FormationMemberSnapshots,
    CacheUpdated,
}
impl Provider<'_> {
    pub fn bind_formation_movement_changed_route(
        &mut self,
        state: &LoadedState,
        consumers: &[Consumer],
        cache_updated_subscribers: usize,
    ) -> Result<(), Error> {
        if state.binding() != &self.definitions.binding
            || !state.is_pristine()
            || self.formation_movement_changed_bound
            || consumers != [Consumer::FormationMemberSnapshots, Consumer::CacheUpdated]
            || cache_updated_subscribers != 0
        {
            return Err(Error::Provider(
                "current stock Formation movement consumer route",
            ));
        }
        self.formation_movement_changed_bound = true;
        Ok(())
    }
    /// OnFormationUpdate ignores the raw movement value and publishes member
    /// snapshots in the actual current Formation order. Retain the exact scalar
    /// alongside those observations without constructing native cache objects.
    pub fn formation_movement_changed(
        &mut self,
        state: &LoadedState,
        report: FormationRestoredReport,
    ) -> Result<(), Error> {
        if !self.formation_movement_changed_bound
            || state.binding() != &self.definitions.binding
            || report.event_hash != 0x3d75b3ed
            || report.identity.component_type != 0x2c
        {
            return Err(Error::Provider(
                "current Formation movement route/shape unavailable",
            ));
        }
        self.observe_formation_snapshot(state, report.identity)?;
        self.formation_movement_changes.push(report);
        Ok(())
    }
}
