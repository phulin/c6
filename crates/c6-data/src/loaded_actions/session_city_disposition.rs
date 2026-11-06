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
        self.context
            .notifications
            .backend
            .combat()
            .ok()?
            .city_melees
            .iter()
            .find_map(|row| row.captured.as_ref().map(|capture| &capture.disposition))
    }
}
