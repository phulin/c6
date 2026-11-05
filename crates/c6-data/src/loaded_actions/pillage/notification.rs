//! Informational TilePillaged publication for the stock Linux headless domain.
//!
//! This notice introduces no player decision. The native base notification has
//! zero blocking and no auto activation; its OnAdd is empty. Native text, UI
//! occurrence identity and display expiry are presentation state. Added native
//! or script consumers must be implemented before using this scope.
use crate::{
    loaded_actions::stock_notifications::{Notice, Runtime},
    Rules, SqlValue,
};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

pub const TYPE_HASH: u32 = 0x0cfc_1e1c;

/// Requires the stock creator map and plain resources, with no added gameplay
/// consumers of notification reporting or host Event256. The stock translator
/// carries only NotificationID into host presentation delivery. This authority
/// is separate from whether a recipient is human or currently active.
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockPlainHeadlessNoAddedConsumers,
}
#[derive(Debug)]
pub enum Error {
    Binding,
    Rules(crate::Error),
    Missing(&'static str),
    Unsupported(&'static str),
    State(c6_core::loaded_state::Error),
    Notification(crate::loaded_actions::notifications::processing::Error),
}
pub struct Inputs {
    binding: WireBinding,
}
impl Inputs {
    /// Check the activated definition rather than assuming UI/decision flags
    /// from the notification's name. Expiry only controls informational display.
    pub fn from_rules(state: &LoadedState, rules: &Rules, _: Scope) -> Result<Self, Error> {
        let table = rules
            .gameplay()
            .table("Notifications")
            .ok_or(Error::Missing("Notifications"))?;
        let mut found = false;
        for row in table.rows() {
            if row.get("NotificationType").map_err(Error::Rules)?
                != &SqlValue::Text(b"NOTIFICATION_TILE_PILLAGED".to_vec())
            {
                continue;
            }
            if found {
                return Err(Error::Unsupported("duplicate TilePillaged definition"));
            }
            found = true;
            for (name, expected) in [
                ("AutoActivate", SqlValue::Integer(0)),
                ("AutoNotify", SqlValue::Integer(0)),
                ("ExpiresEndOfTurn", SqlValue::Integer(0)),
                ("ExpiresEndOfNextTurn", SqlValue::Integer(1)),
                ("SeverityType", SqlValue::Text(b"HIGH".to_vec())),
                ("SubType", SqlValue::Null),
            ] {
                if row.get(name).map_err(Error::Rules)? != &expected {
                    return Err(Error::Unsupported("TilePillaged definition flags"));
                }
            }
        }
        if !found {
            return Err(Error::Missing("TilePillaged definition"));
        }
        Ok(Self {
            binding: state.binding().clone(),
        })
    }
}

/// Called after the improvement flag/signal and before ImprovementChanged.
/// Read actual current plot/city ownership after the signal's synchronous effects.
/// Manager initialization, shutdown and human-recipient guards are shared with
/// the existing semantic notice publisher. No RNG draw or policy selection is
/// substituted for native presentation allocation or reporting.
pub fn publish<P>(
    state: &LoadedState,
    inputs: &Inputs,
    runtime: &mut Runtime<'_, P>,
    plot: usize,
    actor_owner: i32,
) -> Result<(), Error> {
    if state.binding() != &inputs.binding {
        return Err(Error::Binding);
    }
    let record = state
        .world()
        .map()
        .plots
        .get(plot)
        .ok_or(Error::Missing("TilePillaged plot"))?;
    let city = record
        .city_attributes
        .as_ref()
        .ok_or(Error::Missing("TilePillaged owning city"))?;
    let owner = i32::from(city.player);
    state.city(owner, city.owning_city).map_err(Error::State)?;
    let (x, y) = state
        .world()
        .map()
        .coordinates(plot as u32)
        .ok_or(Error::Missing("TilePillaged coordinates"))?;
    runtime
        .publish_notice(
            state,
            Notice::ImprovementPillaged {
                owner,
                actor_owner,
                location: [x, y],
                improvement: record.improvement_hash,
                city: city.owning_city,
            },
        )
        .map_err(Error::Notification)?;
    runtime
        .reached
        .push("stock TilePillaged informational notice");
    Ok(())
}
