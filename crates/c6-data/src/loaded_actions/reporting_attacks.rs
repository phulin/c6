//! Stock changed-attacks cache snapshot followed by host81 scalar output.
use super::*;
use c6_core::unit_turn_upkeep::{AttacksRemainingReport, ATTACKS_REMAINING_CHANGED};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Consumer {
    CacheUnitUpdate,
    HostUnitAttacksRemainingChanged,
}
impl Provider<'_> {
    /// Bind the actual ordered stock route and its synchronous consumer census.
    /// This does not admit unknown/custom snapshot, cache or host consumers.
    pub fn bind_attacks_changed_route(
        &mut self,
        state: &LoadedState,
        consumers: &[Consumer],
        cache_updated_subscribers: usize,
        host_attacks_subscribers: usize,
    ) -> Result<(), Error> {
        if state.binding() != &self.definitions.binding
            || !state.is_pristine()
            || self.attacks_changed_bound
            || consumers
                != [
                    Consumer::CacheUnitUpdate,
                    Consumer::HostUnitAttacksRemainingChanged,
                ]
            || cache_updated_subscribers != 0
            || host_attacks_subscribers != 0
        {
            return Err(Error::Provider("current stock attacks consumer route"));
        }
        self.attacks_changed_bound = true;
        Ok(())
    }
    pub fn preflight_attacks_changed(&self, state: &LoadedState) -> Result<(), Error> {
        if !self.attacks_changed_bound || state.binding() != &self.definitions.binding {
            return Err(Error::Provider(
                "current UnitAttacksRemainingChanged route unavailable",
            ));
        }
        Ok(())
    }
    /// Execute native UnitOnUpdate's current snapshot query dependencies before
    /// retaining host81's exact signed integer payload. DTO/cache storage and
    /// client playback are omitted at the qualified headless output boundary.
    pub fn attacks_changed(
        &mut self,
        state: &LoadedState,
        report: AttacksRemainingReport,
    ) -> Result<(), Error> {
        self.preflight_attacks_changed(state)?;
        if report.event_hash != ATTACKS_REMAINING_CHANGED || report.identity.component_type != 1 {
            return Err(Error::Provider("UnitAttacksRemainingChanged report shape"));
        }
        self.observe_unit_snapshot(state, UnitReference::from_component_id(report.identity))?;
        self.attack_changes.push(report);
        Ok(())
    }
}
