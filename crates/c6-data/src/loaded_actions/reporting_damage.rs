//! Stock UnitDamageChanged cache update and semantic host publication.
use super::*;
use c6_core::unit_healing::DamageChangedReport;

/// Concrete registered native delegates, in dispatch order. Runtime capture
/// adapters resolve addresses to these implementations before binding them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Consumer {
    CacheUnitUpdate,
    HostUnitDamageChanged,
}
impl Provider<'_> {
    pub fn bind_damage_changed_route(
        &mut self,
        state: &LoadedState,
        consumers: &[Consumer],
        cache_updated_subscribers: usize,
        host_damage_subscribers: usize,
    ) -> Result<(), Error> {
        if state.binding() != &self.definitions.binding
            || !state.is_pristine()
            || self.damage_changed_bound
            || consumers != [Consumer::CacheUnitUpdate, Consumer::HostUnitDamageChanged]
            || cache_updated_subscribers != 0
            || host_damage_subscribers != 0
        {
            return Err(Error::Provider("current stock damage consumer route"));
        }
        self.damage_changed_bound = true;
        Ok(())
    }
    pub fn preflight_damage_changed(&self, state: &LoadedState) -> Result<(), Error> {
        if !self.damage_changed_bound || state.binding() != &self.definitions.binding {
            return Err(Error::Provider(
                "current UnitDamageChanged route unavailable",
            ));
        }
        Ok(())
    }
    /// Native cache refresh precedes host33; the latter has no synchronous
    /// gameplay consumers in the bound route. Presentation DTOs are omitted.
    pub fn damage_changed(
        &mut self,
        state: &LoadedState,
        report: DamageChangedReport,
    ) -> Result<(), Error> {
        if !self.damage_changed_bound
            || report.event_hash != c6_core::unit_healing::DAMAGE_CHANGED
            || report.identity.component_type != 1
        {
            return Err(Error::Provider("UnitDamageChanged route/shape"));
        }
        self.observe_unit_snapshot(state, UnitReference::from_component_id(report.identity))?;
        self.damage_changes.push(report);
        Ok(())
    }
}
