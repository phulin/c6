//! Linux XP2 Game.Eras.DoTurn2996cd0 and IsReadyForTransition2997890.
//! Countdown/readiness use the canonical current manager and player eras.
//! Reached initialization and era-transition continuations remain explicit.
use crate::{
    registry::linux_type_hash, unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, diplomacy_meeting::MAJOR_LEVEL,
    game_capabilities::Membership, loaded_state::LoadedState,
};

pub const CAPABILITY: u32 = 0xf10572be;
#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    Requires(&'static str),
    State(c6_core::loaded_state::game_eras::AccessError),
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round eras: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Era {
    hash: u32,
    chronology: i32,
    minimum: Option<i32>,
    maximum: Option<i32>,
}
pub struct Definitions {
    binding: WireBinding,
    eras: Vec<Era>,
    speeds: Vec<(u32, i32)>,
    countdown: i32,
    last: i32,
}
fn int(row: crate::Row<'_>, name: &'static str) -> Result<i32, Error> {
    i32::try_from(row.integer(name).map_err(Error::Rules)?).map_err(|_| Error::Missing(name))
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let registry = collection(rules, "Eras", "EraType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Eras")
            .ok_or(Error::Missing("Eras"))?;
        let xp = rules
            .gameplay()
            .table("Eras_XP1")
            .ok_or(Error::Missing("Eras_XP1"))?;
        let mut eras = Vec::new();
        let mut last = -1;
        let mut highest = i32::MIN;
        for (index, entry) in registry.entries().iter().enumerate() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Era row"))?;
            let hash = linux_type_hash(&entry.type_name);
            let extension = xp.rows().find(|r| {
                r.text("EraType")
                    .ok()
                    .is_some_and(|s| linux_type_hash(s.as_bytes()) == hash)
            });
            let optional = |name| -> Result<Option<i32>, Error> {
                let Some(row) = extension else {
                    return Ok(None);
                };
                match row.get(name).map_err(Error::Rules)? {
                    SqlValue::Null => Ok(None),
                    _ => int(row, name).map(Some),
                }
            };
            let chronology = int(row, "ChronologyIndex")?;
            if chronology > highest {
                highest = chronology;
                last = index as i32;
            }
            eras.push(Era {
                hash,
                chronology,
                minimum: optional("GameEraMinimumTurns")?,
                maximum: optional("GameEraMaximumTurns")?,
            });
        }
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let countdown = parameters
            .rows()
            .find(|r| r.text("Name").ok() == Some("NEXT_ERA_TURN_COUNTDOWN"))
            .and_then(|r| r.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(Error::Missing("NEXT_ERA_TURN_COUNTDOWN"))?;
        let speeds = rules
            .gameplay()
            .table("GameSpeeds")
            .ok_or(Error::Missing("GameSpeeds"))?
            .rows()
            .map(|r| {
                Ok((
                    linux_type_hash(r.text("GameSpeedType").map_err(Error::Rules)?.as_bytes()),
                    int(r, "CostMultiplier")?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            eras,
            speeds,
            countdown,
            last,
        })
    }
}
#[derive(Clone, Copy)]
pub struct Scheduling {
    pub game_speed_hash: u32,
    pub game_end_turn: i32,
}
/// Ordered a47d34e3 report subscribers on the caller's same stock runtime.
pub trait Effects {
    fn countdown_started(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        countdown: i32,
    ) -> Result<(), Error>;
}
fn scale(turns: i32, percent: i32) -> i32 {
    let q8 = i64::from(turns.wrapping_shl(8)) * i64::from(percent.wrapping_shl(8)) / 25_600;
    (q8.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32) >> 8
}
fn majors(
    state: &LoadedState,
    players: &crate::unit_placement_services::Runtime,
) -> Result<Vec<i32>, Error> {
    if players.binding != *state.binding() {
        return Err(Error::Missing("Eras Player.Manager binding"));
    }
    let (list, count) = players
        .alive_begin::<()>()
        .map_err(|_| Error::Missing("Eras GetAlive"))?;
    let mut result = Vec::new();
    for index in 0..count {
        let owner = players
            .alive_at::<()>(list, index)
            .map_err(|_| Error::Missing("Eras alive order"))?;
        let level = players
            .player::<()>(owner)
            .map_err(|_| Error::Missing("Eras player level"))?
            .civilization_level
            .ok_or(Error::Missing("Eras player level"))?;
        if level == MAJOR_LEVEL {
            result.push(owner);
        }
    }
    Ok(result)
}
fn threshold(start: i32, duration: Option<i32>, percent: i32, fallback: i32) -> i32 {
    if start < 0 {
        return fallback;
    }
    duration.map_or(fallback, |v| start.wrapping_add(scale(v, percent)))
}
pub fn is_ready(
    state: &LoadedState,
    defs: &Definitions,
    players: &crate::unit_placement_services::Runtime,
    scheduling: Scheduling,
) -> Result<bool, Error> {
    if defs.binding != *state.binding() {
        return Err(Error::Missing("Eras definition binding"));
    }
    let runtime = state.game_eras().map_err(Error::State)?;
    let current = runtime
        .current_era()
        .ok_or(Error::Missing("Eras current era"))?;
    let start = runtime
        .era_start_turn()
        .ok_or(Error::Missing("Eras start turn"))?;
    let era = usize::try_from(current).ok().and_then(|i| defs.eras.get(i));
    let percent = defs
        .speeds
        .iter()
        .find(|r| r.0 == scheduling.game_speed_hash)
        .map(|r| r.1)
        .ok_or(Error::Missing("Eras game speed"))?;
    let minimum = threshold(start, era.and_then(|e| e.minimum), percent, 0);
    let turn = state.retained().core.current_turn;
    if turn < minimum.wrapping_sub(defs.countdown) {
        return Ok(false);
    }
    let maximum = threshold(
        start,
        era.and_then(|e| e.maximum),
        percent,
        scheduling.game_end_turn,
    );
    if turn >= maximum.wrapping_sub(defs.countdown) {
        return Ok(true);
    }
    let major_order = majors(state, players)?;
    let mut advanced = 0i32;
    if current != -1 {
        for &owner in &major_order {
            let player = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or(Error::Missing("Eras current player"))?;
            let index = defs
                .eras
                .iter()
                .position(|e| e.hash == player.eras.era_hash)
                .map_or(-1, |i| i as i32);
            if index == current {
                continue;
            }
            // GetMaxEra returns -1 if either definition is invalid, otherwise
            // retains its first argument on a chronology tie.
            let maximum = match (
                era,
                usize::try_from(index).ok().and_then(|i| defs.eras.get(i)),
            ) {
                (Some(a), Some(b)) => {
                    if b.chronology > a.chronology {
                        index
                    } else {
                        current
                    }
                }
                _ => -1,
            };
            if maximum == index {
                advanced = advanced.wrapping_add(1);
            }
        }
    }
    Ok(advanced >= (major_order.len() as i32).wrapping_sub(advanced))
}
pub fn run<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    players: &crate::unit_placement_services::Runtime,
    capabilities: &Membership,
    scheduling: Scheduling,
    effects: &mut E,
) -> Result<(), Error> {
    if !capabilities.has(CAPABILITY) {
        return Ok(());
    }
    if defs.binding != *state.binding() {
        return Err(Error::Missing("Eras definition binding"));
    }
    let runtime = state.game_eras().map_err(Error::State)?;
    let current = runtime
        .current_era()
        .ok_or(Error::Missing("Eras current era"))?;
    let world = state.game_eras_world_turn().map_err(Error::State)?;
    if current == -1 || !world.starting_thresholds_calculated {
        return Err(Error::Requires(
            "Eras initial threshold recalculation/SetCurrentEra",
        ));
    }
    if current == defs.last {
        return Ok(());
    }
    if world.next_era_countdown == -1 && is_ready(state, defs, players, scheduling)? {
        state
            .set_game_eras_countdown(defs.countdown)
            .map_err(Error::State)?;
        for owner in majors(state, players)? {
            if owner != -1 {
                effects.countdown_started(state, owner, defs.countdown)?;
            }
        }
    }
    let countdown = state
        .game_eras_world_turn()
        .map_err(Error::State)?
        .next_era_countdown;
    if countdown >= 0 {
        state
            .set_game_eras_countdown(countdown.wrapping_sub(1))
            .map_err(Error::State)?;
        if countdown == 0 {
            return Err(Error::Requires(
                "Eras next-era transition/commemoration decisions",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_end_turn_defaults_and_speed_rounding() {
        assert_eq!(threshold(-1, Some(40), 100, 0), 0);
        assert_eq!(threshold(-1, Some(40), 100, 500), 500);
        assert_eq!(threshold(7, None, 100, 500), 500);
        assert_eq!(threshold(7, Some(41), 50, 0), 27);
        assert_eq!(threshold(7, Some(-1), 50, 0), 6);
    }
}
