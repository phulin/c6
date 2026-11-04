//! Resources.DoTurnDeactivate2b70290: current accumulated resource caps.
use crate::{
    registry::linux_type_hash, unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::loaded_state::LoadedState;
#[derive(Debug)]
pub enum Error<E> {
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    State(c6_core::loaded_state::Error),
    Missing(&'static str),
    Effect(E),
}
struct Resource {
    hash: u32,
    cap: Option<i32>,
}
pub struct Definitions {
    ordered: Vec<Resource>,
}
fn integer<E>(v: &SqlValue) -> Result<i32, Error<E>> {
    match v {
        SqlValue::Integer(v) => {
            i32::try_from(*v).map_err(|_| Error::Missing("resource integer range"))
        }
        _ => Err(Error::Missing("resource integer")),
    }
}
/// Economic.AdjustForGameSpeed(int): wrap the Q8 constructors, use signed
/// division, clamp Q8 storage, then arithmetic-shift AsInt (2be38b0/2b9c990).
fn scale(cap: i32, percent: i32) -> i32 {
    let q8 = i64::from(cap.wrapping_shl(8)) * i64::from(percent.wrapping_shl(8)) / 25_600;
    (q8.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32) >> 8
}
impl Definitions {
    /// Native resource order is an explicit immutable definition input. The
    /// current initialized ResourceEntry hash vector preserves that same order.
    pub fn from_rules<E>(
        rules: &Rules,
        native_order: &[u32],
        speed_hash: u32,
    ) -> Result<Self, Error<E>> {
        let resources =
            collection(rules, "Resources", "ResourceType").map_err(Error::Definitions)?;
        if resources.entries().len() != native_order.len() {
            return Err(Error::Missing("complete native resource order"));
        }
        let speeds =
            collection(rules, "GameSpeeds", "GameSpeedType").map_err(Error::Definitions)?;
        let percent = if let Some(speed) = speeds.find(speed_hash) {
            let row = rules
                .gameplay()
                .table("GameSpeeds")
                .and_then(|t| t.row_by_id(speed.database_rowid))
                .ok_or(Error::Missing("current game speed"))?;
            integer(row.get("CostMultiplier").map_err(Error::Rules)?)?
        } else {
            100
        };
        let consumption = rules
            .gameplay()
            .table("Resource_Consumption")
            .ok_or(Error::Missing("Resource_Consumption"))?;
        let mut ordered = Vec::with_capacity(native_order.len());
        for (index, &hash) in native_order.iter().enumerate() {
            if native_order[..index].contains(&hash) || resources.find(hash).is_none() {
                return Err(Error::Missing("native resource identity"));
            }
            let mut cap = None;
            for row in consumption.rows() {
                if linux_type_hash(row.text("ResourceType").map_err(Error::Rules)?.as_bytes())
                    == hash
                {
                    cap = if integer(row.get("Accumulate").map_err(Error::Rules)?)? != 0 {
                        Some(scale(
                            integer(row.get("StockpileCap").map_err(Error::Rules)?)?,
                            percent,
                        ))
                    } else {
                        None
                    };
                }
            }
            ordered.push(Resource { hash, cap });
        }
        Ok(Self { ordered })
    }
}
pub trait Effects {
    type Error;
    fn allocate_luxuries(&mut self, state: &mut LoadedState, owner: i32)
        -> Result<(), Self::Error>;
    fn resource_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        index: i32,
        ownership: i32,
        delta: i32,
    ) -> Result<(), Self::Error>;
    fn report_resource(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        index: i32,
        event: u32,
    ) -> Result<(), Self::Error>;
}
pub fn deactivate<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    definitions: &Definitions,
    effects: &mut E,
) -> Result<usize, Error<E::Error>> {
    let mut changed = 0;
    for (index, definition) in definitions.ordered.iter().enumerate() {
        let Some(cap) = definition.cap else {
            continue;
        };
        let resources = &state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("resource player"))?
            .resources;
        let stored = resources
            .resource_ownership
            .iter()
            .find(|e| e.resource_hash == definition.hash)
            .and_then(|e| e.value.first())
            .copied()
            .ok_or(Error::Missing("owned resource amount"))?;
        let reserved = resources
            .reserved_resources
            .iter()
            .find(|e| e.resource_hash == definition.hash)
            .ok_or(Error::Missing("reserved resource amount"))?
            .value;
        let amount = if reserved > 0 {
            stored.wrapping_sub(reserved)
        } else {
            stored
        };
        if amount <= 0 {
            continue;
        }
        let modifier = resources
            .resource_stockpile_cap_modifier
            .iter()
            .find(|e| e.resource_hash == definition.hash)
            .ok_or(Error::Missing("resource cap modifier"))?
            .value;
        let delta = cap.wrapping_add(modifier).wrapping_sub(amount);
        if delta >= 0 {
            continue;
        }
        let values = state.player_resources_mut(owner).map_err(Error::State)?;
        let value = values
            .resource_ownership
            .iter_mut()
            .find(|e| e.resource_hash == definition.hash)
            .and_then(|e| e.value.first_mut())
            .ok_or(Error::Missing("current owned resource amount"))?;
        *value = value.wrapping_add(delta).max(0);
        // ChangeResourceAmount stores BEFORE allocation/signal/report. Errors
        // preserve this prefix; callers must not retry the same turn phase.
        effects
            .allocate_luxuries(state, owner)
            .map_err(Error::Effect)?;
        effects
            .resource_changed(state, owner, index as i32, 0, delta)
            .map_err(Error::Effect)?;
        effects
            .report_resource(state, owner, index as i32, 0x1153df52)
            .map_err(Error::Effect)?;
        changed += 1;
    }
    Ok(changed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speed_scaling_preserves_fixed_point_wrap_clamp_and_negative_rounding() {
        assert_eq!(scale(50, 150), 75);
        assert_eq!(scale(-1, 50), -1);
        assert_eq!(scale(i32::MAX, 100), -1);
        assert_eq!(scale(1_000_000, 1_000_000), i32::MAX >> 8);
    }
}
