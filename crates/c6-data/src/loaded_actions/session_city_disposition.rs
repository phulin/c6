//! Captured-city policy boundary over the SAME retained combat owner.
use super::Session;
use crate::city_capture::PendingDisposition;
impl<P> Session<'_, '_, P> {
    /// The earliest completed capture whose disposition has not been resolved.
    /// Melee completion (advance, callbacks, RNG and queue cleanup) finishes
    /// before this choice becomes visible. Extracting and rewrapping Session
    /// parts preserves it with the retained combat record.
    ///
    /// The bounded action surface exposes the legal directives; executing a
    /// disposition is not yet supported. Other actions/turns remain blocked.
    pub fn pending_city_disposition(&self) -> Option<&PendingDisposition> {
        let backend = &self.context.notifications.backend;
        let combat = backend.combat().ok()?;
        backend.pending_city_dispositions().iter().find(|choice| {
            combat.city_melees.iter().any(|row| {
                row.captured
                    .as_ref()
                    .is_some_and(|capture| capture.disposition.city == choice.city)
            })
        })
    }
}
