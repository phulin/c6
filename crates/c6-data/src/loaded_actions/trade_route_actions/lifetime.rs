//! Linux XP2 Trade.Manager.GetEarliestEndTurnForRoute2c0be40.
//! The current game era is read on every origin check, not frozen at initiation.
use super::Error;
use crate::{
    registry::linux_type_hash, unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

pub struct Definitions {
    binding: WireBinding,
    base: i32,
    speed_multiplier: i32,
    era_changes: Vec<Option<i32>>,
}
impl Definitions {
    /// Bind the actual game's speed type alongside the same rules identity.
    pub fn from_rules(state: &LoadedState, rules: &Rules, game_speed: u32) -> Result<Self, Error> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Requires("Linux XP2 trade lifetime rules"));
        }
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let base = parameters
            .rows()
            .find(|r| r.text("Name").ok() == Some("TRADE_ROUTE_TURN_DURATION_BASE"))
            .and_then(|r| r.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(Error::Missing("TRADE_ROUTE_TURN_DURATION_BASE"))?;
        let speeds = collection(rules, "GameSpeeds", "GameSpeedType")
            .map_err(|_| Error::Missing("GameSpeeds registry"))?;
        // Economic.AdjustForGameSpeed uses 100 when the configured type cannot
        // be found, as the other economic consumers do.
        let speed_multiplier = match speeds.find(game_speed) {
            None => 100,
            Some(entry) => {
                let row = rules
                    .gameplay()
                    .table("GameSpeeds")
                    .and_then(|t| t.row_by_id(entry.database_rowid))
                    .ok_or(Error::Missing("GameSpeeds row"))?;
                i32::try_from(row.integer("CostMultiplier").map_err(Error::Rules)?)
                    .map_err(|_| Error::Missing("CostMultiplier"))?
            }
        };
        let eras =
            collection(rules, "Eras", "EraType").map_err(|_| Error::Missing("Eras registry"))?;
        let xp = rules
            .gameplay()
            .table("Eras_XP2")
            .ok_or(Error::Missing("Eras_XP2"))?;
        let mut era_changes = Vec::new();
        for era in eras.entries() {
            let hash = linux_type_hash(&era.type_name);
            let change = match xp.rows().find(|r| {
                r.text("EraType")
                    .ok()
                    .is_some_and(|s| linux_type_hash(s.as_bytes()) == hash)
            }) {
                None => None,
                Some(row) => match row
                    .get("TradeRouteMinimumEndTurnChange")
                    .map_err(Error::Rules)?
                {
                    SqlValue::Null => None,
                    SqlValue::Integer(v) => Some(
                        i32::try_from(*v)
                            .map_err(|_| Error::Missing("TradeRouteMinimumEndTurnChange"))?,
                    ),
                    _ => return Err(Error::Missing("TradeRouteMinimumEndTurnChange")),
                },
            };
            era_changes.push(change);
        }
        Ok(Self {
            binding: state.binding().clone(),
            base,
            speed_multiplier,
            era_changes,
        })
    }
    pub fn earliest_end_turn(&self, state: &LoadedState, push_turn: i32) -> Result<i32, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Binding);
        }
        let era = state
            .game_eras()
            .map_err(Error::GameEras)?
            .current_era()
            .ok_or(Error::Missing("current game era"))?;
        let adjustment = usize::try_from(era)
            .ok()
            .and_then(|i| self.era_changes.get(i))
            .copied()
            .flatten();
        Ok(earliest(
            push_turn,
            self.base,
            adjustment,
            self.speed_multiplier,
        ))
    }
}
fn earliest(push_turn: i32, base: i32, era: Option<i32>, speed: i32) -> i32 {
    let adjust = |v| c6_core::research_acquisition::units::adjust_for_game_speed(v, speed);
    let mut end = push_turn.wrapping_add(adjust(base));
    if let Some(era) = era {
        end = end.wrapping_add(adjust(era));
    }
    if end > push_turn {
        end
    } else {
        push_turn.wrapping_add(1)
    }
}
#[cfg(test)]
mod tests {
    use super::earliest;
    #[test]
    fn era_change_is_scaled_separately_and_floor_wraps() {
        assert_eq!(earliest(10, 20, None, 100), 30);
        assert_eq!(earliest(10, 1, Some(1), 50), 11);
        assert_eq!(earliest(10, 20, Some(-30), 100), 11);
        assert_eq!(earliest(i32::MAX, 20, None, 100), i32::MIN);
    }
}
