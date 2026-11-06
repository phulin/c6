//! Current victory requirement Tests. SimpleRequirement owns active/state and
//! Changed delivery; these routines return its native four-value convention.
use crate::{
    loaded_actions::stock_notifications::scoring, resource_binding::ResourceAuthority, Rules,
};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
use std::collections::BTreeMap;
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Scoring(scoring::Error),
    Resources(crate::resource_binding::Error),
    Rules(crate::Error),
    Missing(&'static str),
    UnitDamage(crate::formation_movement_cost::Error),
}
#[derive(Clone, Copy, Debug)]
pub enum Object {
    Team(i32),
    Player(i32),
    /// Source-qualified absence of the requested interfaces.
    NoInterface,
    Unavailable,
}
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    GameTurnMaxReached,
    TeamHasHighestScore,
    TouristsExceedStaycationers,
    TeamDiplomaticVictory { initialized_team: i32 },
}
#[derive(Clone, Copy, Debug)]
pub struct Receiver {
    pub kind: Kind,
    pub context: Object,
    pub subject: Object,
}
/// Unsaved current inputs retain actual native iteration order. None is a
/// missing map result, distinct from a present vector containing no players.
pub struct Inputs {
    pub binding: WireBinding,
    pub alive_majors: Vec<i32>,
    pub team_players: Vec<(i32, Option<Vec<Option<i32>>>)>,
    pub civilization_levels: Vec<(i32, u32)>,
    pub game_start_turn: Option<i32>,
    pub turn_limit_type: Option<i32>,
}
pub struct Definitions {
    diplomatic_points_required: i32,
}
impl Definitions {
    pub fn from_rules(rules: &Rules, authority: &ResourceAuthority<'_>) -> Result<Self, Error> {
        authority.validate_rules(rules).map_err(Error::Resources)?;
        let table = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let mut threshold = None;
        for row in table.rows() {
            if row.text("Name").map_err(Error::Rules)? == "DIPLOMATIC_VICTORY_POINTS_REQUIRED" {
                threshold =
                    crate::game_parameters::sql_integer(row.get("Value").map_err(Error::Rules)?);
                break;
            }
        }
        Ok(Self {
            diplomatic_points_required: threshold
                .ok_or(Error::Missing("diplomatic victory threshold"))?,
        })
    }
}
fn as_player(object: Object) -> Result<Option<i32>, Error> {
    match object {
        Object::Player(owner) => Ok(Some(owner)),
        Object::Team(_) | Object::NoInterface => Ok(None),
        Object::Unavailable => Err(Error::Missing("current requirement object IPlayer")),
    }
}
fn as_team(object: Object) -> Result<Option<i32>, Error> {
    match object {
        Object::Team(team) => Ok(Some(team)),
        Object::NoInterface => Ok(None),
        _ => Err(Error::Missing("current requirement object IGame/ITeam")),
    }
}
fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current victory Player"))
}
/// PlayerDefaultDefeat237d810. The initialized owner and flags belong to the
/// same Simple requirement; city membership and delayed death are read live.
/// The compatibility entry leaves unit type and damage inputs unavailable.
pub fn test_default_defeat(
    state: &LoadedState,
    initialized_owner: i32,
    subject: Object,
    no_units_raw: u8,
    no_cities_raw: u8,
) -> Result<i32, Error> {
    test_default_defeat_current(
        state,
        initialized_owner,
        subject,
        no_units_raw,
        no_cities_raw,
        None,
    )
}
/// Current native unit search with its initialized definition index and live
/// dynamic damage properties. u32::MAX is the native wildcard type.
pub fn test_default_defeat_with_units(
    state: &LoadedState,
    binding: (i32, Object, u8, u8),
    unit_type: u32,
    movement: &crate::formation_movement_cost::Definitions<'_>,
) -> Result<i32, Error> {
    test_default_defeat_current(
        state,
        binding.0,
        binding.1,
        binding.2,
        binding.3,
        Some((unit_type as i32, movement)),
    )
}
fn test_default_defeat_current(
    state: &LoadedState,
    initialized_owner: i32,
    subject: Object,
    no_units_raw: u8,
    no_cities_raw: u8,
    units: Option<(i32, &crate::formation_movement_cost::Definitions<'_>)>,
) -> Result<i32, Error> {
    if initialized_owner == -1 {
        return Ok(3);
    }
    let Some(owner) = as_player(subject)? else {
        return Ok(1);
    };
    let player = player(state, owner)?;
    if no_cities_raw == 0 {
        if let Some(result) = default_defeat_city_result(
            player.cities.peak_cities,
            player
                .cities
                .records
                .entries
                .iter()
                .map(|entry| entry.value.header.delayed_death),
        ) {
            return Ok(result);
        }
    }
    if no_units_raw != 0 || player.units.peak_units == 0 {
        return Ok(1);
    }
    let (unit_type, movement) =
        units.ok_or(Error::Missing("DefaultDefeat current Unit predicate"))?;
    let records = state
        .world()
        .player(owner)
        .map_err(|_| Error::Missing("DefaultDefeat current Unit owner"))?
        .unit_records()
        .map_err(|_| Error::Missing("DefaultDefeat current Unit roster"))?;
    for entry in records {
        let unit = &entry.value;
        // Native IsDead precedes delayed death and GetType, including errors
        // from unavailable current dynamic-property conversions.
        let maximum = movement.maximum_damage(unit).map_err(Error::UnitDamage)?;
        if default_defeat_unit_survives(
            unit.i_damage,
            maximum,
            unit.b_delayed_death,
            unit_type,
            || {
                Ok(state
                    .unit_runtime(owner, unit.id)
                    .map_err(Error::State)?
                    .definition_index)
            },
        )? {
            return Ok(1);
        }
    }
    Ok(2)
}
fn default_defeat_unit_survives<E>(
    damage: i32,
    maximum: i32,
    delayed_death: bool,
    unit_type: i32,
    current_type: impl FnOnce() -> Result<i32, E>,
) -> Result<bool, E> {
    if damage >= maximum || delayed_death {
        return Ok(false);
    }
    Ok(unit_type == -1 || current_type()? == unit_type)
}
fn default_defeat_city_result(
    peak_cities: u32,
    delayed_death: impl IntoIterator<Item = bool>,
) -> Option<i32> {
    (peak_cities != 0).then(|| {
        if delayed_death.into_iter().any(|dead| !dead) {
            1
        } else {
            2
        }
    })
}
pub fn test(
    state: &LoadedState,
    definitions: &Definitions,
    inputs: &Inputs,
    scoring: &scoring::Runtime,
    score_definitions: &scoring::Definitions,
    r: Receiver,
) -> Result<i32, Error> {
    if inputs.binding != *state.binding() {
        return Err(Error::Missing("victory inputs binding"));
    }
    match r.kind {
        Kind::GameTurnMaxReached => {
            if as_team(r.subject)?.is_none() {
                return Ok(1);
            }
            let start = inputs
                .game_start_turn
                .ok_or(Error::Missing("current GameStartTurn inputs"))?;
            let max = state.retained().core.max_turns;
            let limit = inputs
                .turn_limit_type
                .ok_or(Error::Missing("current TurnLimitType"))?;
            Ok(turn_limit_result(
                state.retained().core.current_turn,
                game_end_turn(start, max, limit),
            ))
        }
        Kind::TeamHasHighestScore => {
            let Some(team) = as_team(r.subject)? else {
                return Ok(1);
            };
            highest_score(
                &inputs.alive_majors,
                team,
                &score_definitions.tie_breakers,
                |owner| state.player_team(owner).map_err(Error::State),
                |owner, raw| match raw {
                    None => scoring
                        .get_score(state, score_definitions, owner)
                        .map_err(Error::Scoring),
                    Some(raw) => scoring
                        .raw_score_with_definitions(state, score_definitions, owner, raw)
                        .map_err(Error::Scoring),
                },
            )
        }
        Kind::TouristsExceedStaycationers => {
            // Both interfaces are queried before the absence guard.
            let context = as_player(r.context)?;
            let subject = as_player(r.subject)?;
            let (Some(context), Some(subject)) = (context, subject) else {
                return Ok(1);
            };
            let level = inputs
                .civilization_levels
                .iter()
                .find(|p| p.0 == subject)
                .ok_or(Error::Missing("current victory civilization level"))?
                .1;
            if level != 0x253718b0 || context == subject {
                return Ok(0);
            }
            let mut total = 0i32;
            for &other in &inputs.alive_majors {
                if other != context
                    && state
                        .game_relations()
                        .map_err(Error::State)?
                        .has_met(context, other)
                        .map_err(Error::Relations)?
                {
                    let value = player(state, context)?
                        .culture
                        .tourists_from
                        .get(other as u32 as usize)
                        .ok_or(Error::Missing("current tourists-from cell"))?;
                    total = total.wrapping_add(*value);
                }
            }
            Ok(i32::from(
                total <= player(state, subject)?.culture.staycationers,
            ))
        }
        Kind::TeamDiplomaticVictory { initialized_team } => {
            if initialized_team == -1 {
                return Ok(3);
            }
            let members = inputs
                .team_players
                .iter()
                .find(|p| p.0 == initialized_team)
                .ok_or(Error::Missing("current GetTeamPlayers lookup"))?;
            let Some(members) = &members.1 else {
                return Ok(1);
            };
            for owner in members.iter().flatten() {
                if !state.player_alive(*owner).map_err(Error::State)? {
                    continue;
                }
                let v = &player(state, *owner)?.stats.diplomatic_victory_points;
                if tracked_value(v.value_q8, v.min_q8, v.max_q8) >> 8
                    >= definitions.diplomatic_points_required
                {
                    return Ok(2);
                }
            }
            Ok(1)
        }
    }
}
fn game_end_turn(start: i32, max: i32, limit: i32) -> i32 {
    match limit as u32 {
        u32::MAX | 0x809925de => 0,
        0xdef9f0af => max.wrapping_add(1),
        _ => start.wrapping_add(max),
    }
}
fn turn_limit_result(current: i32, end: i32) -> i32 {
    if end <= 0 {
        3
    } else if current >= end {
        2
    } else {
        1
    }
}
fn tracked_value(value: i32, min: Option<i32>, max: Option<i32>) -> i32 {
    let value = max.map_or(value, |m| value.min(m));
    min.map_or(value, |m| value.max(m))
}
fn keep_highest(scores: &mut Vec<(i32, i32)>) {
    if let Some(maximum) = scores.iter().map(|p| p.1).max() {
        scores.retain(|p| p.1 == maximum);
    }
}
fn highest_score(
    majors: &[i32],
    subject: i32,
    lines: &[(i32, scoring::Raw)],
    mut team: impl FnMut(i32) -> Result<i32, Error>,
    mut score: impl FnMut(i32, Option<scoring::Raw>) -> Result<i32, Error>,
) -> Result<i32, Error> {
    let mut found = false;
    for &owner in majors {
        if team(owner)? == subject {
            found = true;
            break;
        }
    }
    if !found {
        return Ok(1);
    }
    let mut totals = BTreeMap::<i32, i32>::new();
    for &owner in majors {
        totals.entry(team(owner)?).or_default();
    }
    for &owner in majors {
        let raw = score(owner, None)?.wrapping_shl(8);
        let entry = totals
            .get_mut(&team(owner)?)
            .expect("initialized current team");
        *entry = entry.wrapping_add(raw);
    }
    let mut remaining: Vec<_> = totals.into_iter().collect();
    keep_highest(&mut remaining);
    if !remaining.iter().any(|p| p.0 == subject) {
        return Ok(1);
    }
    if remaining.len() == 1 {
        return Ok(0);
    }
    if lines.len() > 32 {
        return Err(Error::Missing("native large tie-break sort"));
    }
    let mut lines = lines.to_vec();
    // Stable strict-priority insertion order, matching the native <=32 branch.
    lines.sort_by_key(|line| std::cmp::Reverse(line.0));
    for (_, kind) in lines {
        for entry in &mut remaining {
            entry.1 = 0;
            for &owner in majors {
                // Target lambda tests the subject team, not entry.0.
                if team(owner)? == subject {
                    entry.1 = entry
                        .1
                        .wrapping_add(score(owner, Some(kind))?.wrapping_shl(8));
                }
            }
        }
        keep_highest(&mut remaining);
        if !remaining.iter().any(|p| p.0 == subject) {
            return Ok(1);
        }
        if remaining.len() == 1 {
            return Ok(0);
        }
    }
    Ok(i32::from(
        remaining.iter().map(|p| p.0).min() != Some(subject),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_defeat_live_city_precedes_units_and_empty_peak_falls_through() {
        assert_eq!(default_defeat_city_result(1, [false]), Some(1));
        assert_eq!(default_defeat_city_result(2, [true, false]), Some(1));
        assert_eq!(default_defeat_city_result(2, [true]), Some(2));
        assert_eq!(default_defeat_city_result(1, []), Some(2));
        let unread = std::iter::from_fn(|| panic!("zero peak must skip city iteration"));
        assert_eq!(default_defeat_city_result(0, unread), None);
    }
    #[test]
    fn default_defeat_units_ignore_dead_delayed_and_wrong_types() {
        let unavailable = || Err::<i32, _>("type unavailable");
        assert_eq!(
            default_defeat_unit_survives(100, 100, false, 0, unavailable),
            Ok(false)
        );
        assert_eq!(
            default_defeat_unit_survives(0, 100, true, 0, unavailable),
            Ok(false)
        );
        assert_eq!(
            default_defeat_unit_survives(99, 100, false, -1, unavailable),
            Ok(true)
        );
        assert_eq!(
            default_defeat_unit_survives(0, 100, false, 0, || Ok::<_, &str>(21)),
            Ok(false)
        );
        assert_eq!(
            default_defeat_unit_survives(0, 100, false, 0, || Ok::<_, &str>(0)),
            Ok(true)
        );
        assert_eq!(
            default_defeat_unit_survives(99, 100, false, 0, unavailable),
            Err("type unavailable")
        );
        // Current dynamic maximums may differ from the global rule, including
        // nonpositive signed values; IsDead still uses the signed comparison.
        assert_eq!(
            default_defeat_unit_survives(20, 20, false, -1, unavailable),
            Ok(false)
        );
        assert_eq!(
            default_defeat_unit_survives(0, -1, false, -1, unavailable),
            Ok(false)
        );
    }
    #[test]
    fn turn_limit_modes_and_wrapping() {
        assert_eq!(game_end_turn(50, 100, -1), 0);
        assert_eq!(game_end_turn(50, 100, 0xdef9f0afu32 as i32), 101);
        assert_eq!(game_end_turn(i32::MAX, 1, 42), i32::MIN);
        assert_eq!(turn_limit_result(9, 10), 1);
        assert_eq!(turn_limit_result(10, 10), 2);
        assert_eq!(turn_limit_result(10, 0), 3);
    }
    #[test]
    fn diplomatic_clamps_upper_then_lower_and_sar() {
        assert_eq!(tracked_value(100, Some(200), Some(50)), 200);
        assert_eq!(tracked_value(-1, None, None) >> 8, -1);
    }
    #[test]
    fn tied_raw_queries_repeat_subject_team_and_choose_smallest_team() {
        let mut calls = Vec::new();
        let result = highest_score(
            &[4, 8],
            7,
            &[(1, scoring::Raw::Cities)],
            |owner| Ok::<_, Error>(if owner == 4 { -2 } else { 7 }),
            |owner, raw| {
                calls.push((owner, raw));
                Ok(0)
            },
        )
        .unwrap();
        assert_eq!(result, 1);
        assert_eq!(
            calls,
            [
                (4, None),
                (8, None),
                (8, Some(scoring::Raw::Cities)),
                (8, Some(scoring::Raw::Cities))
            ]
        );
    }
    #[test]
    fn q8_wrapping_can_change_highest_team() {
        assert_eq!(
            highest_score(&[1, 2], 1, &[], Ok::<_, Error>, |p, _| Ok(if p == 1 {
                0x00800000
            } else {
                0
            }))
            .unwrap(),
            1
        );
    }
}
