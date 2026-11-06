//! Reached ordinary Keep subscribers; positive unimplemented branches fail.
use super::{error, Error, Stage};
use crate::loaded_actions::declare_war::{self, diplomacy::Reports};
use c6_core::{
    game_diplomatic_relations::war::GrievanceLogItem, loaded_state::LoadedState, unit::ComponentId,
};

pub struct Definitions {
    pub(super) binding: c6_core::core_player::initialization::WireBinding,
    capture_percent: i32,
    raze_percent: i32,
    city_percent: i32,
    pub(super) defensive_percent: i32,
    pub(super) ally_percent: i32,
    pub(super) friendship_percent: i32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &crate::Rules) -> Result<Self, String> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err("Keep Linux rules profile".into());
        }
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or("Keep parameters")?;
        let parameter = |name| {
            parameters
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .and_then(|r| r.get("Value").ok())
                .and_then(crate::game_parameters::sql_integer)
                .ok_or_else(|| format!("Keep parameter {name}"))
        };
        let actions = crate::unit_initialization_definitions::collection(
            rules,
            "DiplomaticActions",
            "DiplomaticActionType",
        )
        .map_err(|e| format!("{e:?}"))?;
        let capture_percent = if let Some(index) = actions.find_index(declare_war::SURPRISE_ACTION)
        {
            rules
                .gameplay()
                .table("DiplomaticActions")
                .and_then(|t| t.row_by_id(actions.entries()[index].database_rowid))
                .and_then(|r| r.get("CaptureWarmongerPercent").ok())
                .and_then(crate::game_parameters::sql_integer)
                .ok_or("Keep CaptureWarmongerPercent")?
        } else {
            0
        };
        let raze_percent = if let Some(index) = actions.find_index(declare_war::SURPRISE_ACTION) {
            rules
                .gameplay()
                .table("DiplomaticActions")
                .and_then(|t| t.row_by_id(actions.entries()[index].database_rowid))
                .and_then(|r| r.get("RazeWarmongerPercent").ok())
                .and_then(crate::game_parameters::sql_integer)
                .ok_or("Raze RazeWarmongerPercent")?
        } else {
            0
        };
        Ok(Self {
            binding: state.binding().clone(),
            capture_percent,
            raze_percent,
            city_percent: parameter("WARMONGER_CITY_PERCENT_OF_DOW")?,
            defensive_percent: parameter("SHARE_WAR_GRIEVANCES_DEFENSIVE_PERCENT")?,
            ally_percent: parameter("SHARE_WAR_GRIEVANCES_ALLY")?,
            friendship_percent: parameter("SHARE_WAR_GRIEVANCES_DECLARED_FRIENDS")?,
        })
    }
}
pub(super) fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or_else(|| error(Stage::CityOccupied, "Keep current Player"))
}
pub(super) fn percent(value: i32, factor: i32) -> i32 {
    value.wrapping_mul(factor) / 100
}
fn adjusted_size(value: i32, population: i32, average: i32) -> Result<i32, Error> {
    let difference = average.wrapping_sub(population);
    if difference <= 0 {
        return Ok(value);
    }
    let discount = difference
        .wrapping_mul(value)
        .checked_div(average)
        .ok_or_else(|| error(Stage::CityOccupied, "native city size division"))?;
    Ok(value.wrapping_sub(discount))
}
fn current_average(state: &LoadedState, alive_majors: &[i32]) -> Result<i32, Error> {
    let mut count = 0i32;
    let mut population = 0i32;
    for &owner in alive_majors {
        if !state
            .player_alive(owner)
            .map_err(|e| error(Stage::CityOccupied, e))?
        {
            return Err(error(
                Stage::CityOccupied,
                "current alive-major city average",
            ));
        }
        for city in &player(state, owner)?.cities.records.entries {
            count = count.wrapping_add(1);
            population = population.wrapping_add(city.value.header.population);
        }
    }
    if count <= 0 {
        Ok(0)
    } else {
        Ok(population / count)
    }
}
/// ComputeCityWarmongerPoints2aac7b0 for the fixture's actual SURPRISE_WAR.
/// Other war types fail rather than reusing this action definition's percent.
pub fn surprise_capture_points(
    state: &LoadedState,
    definitions: &Definitions,
    alive_majors: &[i32],
    city: ComponentId,
    previous_owner: i32,
) -> Result<i32, Error> {
    surprise_points(
        state,
        definitions,
        alive_majors,
        city,
        previous_owner,
        definitions.capture_percent,
    )
}
/// The Raze flag selects RazeWarmongerPercent before the same city/size kernel.
pub fn surprise_raze_points(
    state: &LoadedState,
    definitions: &Definitions,
    alive_majors: &[i32],
    city: ComponentId,
    previous_owner: i32,
) -> Result<i32, Error> {
    surprise_points(
        state,
        definitions,
        alive_majors,
        city,
        previous_owner,
        definitions.raze_percent,
    )
}
fn surprise_points(
    state: &LoadedState,
    definitions: &Definitions,
    alive_majors: &[i32],
    city: ComponentId,
    previous_owner: i32,
    action_percent: i32,
) -> Result<i32, Error> {
    if state.binding() != &definitions.binding {
        return Err(error(
            Stage::CityOccupied,
            "Keep grievance definition binding",
        ));
    }
    let owner = i32::from(city.owner);
    if state
        .game_relations()
        .map_err(|e| error(Stage::CityOccupied, e))?
        .war_type(owner, previous_owner)
        .map_err(|e| error(Stage::CityOccupied, e))?
        != declare_war::SURPRISE_WAR
    {
        return Err(error(
            Stage::CityOccupied,
            "capture grievances for another war type",
        ));
    }
    let population = state
        .city(owner, city.id)
        .map_err(|e| error(Stage::CityOccupied, e))?
        .header
        .population;
    adjusted_size(
        percent(action_percent, definitions.city_percent),
        population,
        current_average(state, alive_majors)?,
    )
}
pub(super) fn native_i32(value: f64) -> i32 {
    if !value.is_finite() || value < f64::from(i32::MIN) || value >= 2147483648.0 {
        i32::MIN
    } else {
        value as i32
    }
}
/// Actual Diplomacy.OnCityOccupied2aa20c0. The current callback ring determines
/// receiver order. Writes last-capture turn for EVERY receiver before HasMet.
#[allow(clippy::too_many_arguments)]
pub fn on_city_occupied<P, R: Reports<P>>(
    state: &mut LoadedState,
    backend: &mut P,
    definitions: &Definitions,
    alive_majors: &[i32],
    levels: &[(i32, u32)],
    receiver: i32,
    previous_owner: i32,
    city: ComponentId,
    reports: &mut R,
) -> Result<(), Error>
where
    R::Error: std::fmt::Debug,
{
    let turn = state.retained().core.current_turn;
    state
        .war_diplomacy_mut(receiver)
        .map_err(|e| error(Stage::CityOccupied, e))?
        .last_war_and_city_turns[7] = turn;
    let owner = i32::from(city.owner);
    // Player.Diplomacy.HasMet rejects self before GameRelations.HasMet.
    if receiver == owner
        || !state
            .game_relations()
            .map_err(|e| error(Stage::CityOccupied, e))?
            .has_met(receiver, owner)
            .map_err(|e| error(Stage::CityOccupied, e))?
    {
        return Ok(());
    }
    let major = crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
    if levels.iter().find(|r| r.0 == owner).map(|r| r.1) != Some(major) {
        return Ok(());
    }
    let score = state
        .game_relations()
        .map_err(|e| error(Stage::CityOccupied, e))?
        .grievance_score(owner, previous_owner)
        .map_err(|e| error(Stage::CityOccupied, e))?;
    let mut amount =
        surprise_capture_points(state, definitions, alive_majors, city, previous_owner)?;
    if receiver != previous_owner {
        if score > 0 {
            amount = percent(amount, definitions.defensive_percent);
        }
        let relations = state
            .game_relations()
            .map_err(|e| error(Stage::CityOccupied, e))?;
        amount = if relations
            .is_allied(receiver, previous_owner)
            .map_err(|e| error(Stage::CityOccupied, e))?
        {
            percent(amount, definitions.ally_percent)
        } else if relations
            .has_declared_friendship(receiver, previous_owner)
            .map_err(|e| error(Stage::CityOccupied, e))?
        {
            percent(amount, definitions.friendship_percent)
        } else {
            0
        };
    }
    let at = usize::try_from(receiver)
        .ok()
        .filter(|&i| i < 64)
        .ok_or_else(|| error(Stage::CityOccupied, "warmonger receiver"))?;
    state
        .city_mut(owner, city.id)
        .map_err(|e| error(Stage::CityOccupied, e))?
        .header
        .last_warmonger_penalty[at] = amount;
    if amount == 0 || levels.iter().find(|r| r.0 == receiver).map(|r| r.1) != Some(major) {
        return Ok(());
    }
    let name = state
        .city(owner, city.id)
        .map_err(|e| error(Stage::CityOccupied, e))?
        .header
        .name
        .clone();
    let modifiers = player(state, receiver)?
        .diplomacy
        .grievance_generation_modifier
        .wrapping_add(
            player(state, owner)?
                .diplomacy
                .grievance_generation_modifier,
        )
        .wrapping_add(100);
    let adjusted = native_i32(f64::from(modifiers.wrapping_mul(amount)) * 0.01);
    state
        .war_relations_mut()
        .map_err(|e| error(Stage::CityOccupied, e))?
        .change_grievance_and_tension(receiver, owner, adjusted, turn)
        .map_err(|e| error(Stage::CityOccupied, e))?;
    reports
        .grievances(state, backend, receiver, owner)
        .map_err(|e| error(Stage::CityOccupied, e))?;
    state
        .war_relations_mut()
        .map_err(|e| error(Stage::CityOccupied, e))?
        .append_grievance_log(GrievanceLogItem {
            holder: receiver,
            source: owner,
            turn,
            amount: adjusted,
            reason: b"LOC_DIPLO_GRIEVANCE_LOG_CITY_OCCUPIED".to_vec(),
            other_reason: name,
        })
        .map_err(|e| error(Stage::CityOccupied, e))
}

/// Espionage.OnCityCaptured2af16b0, with no active matching spy mission. A real
/// matching mission requires ClearOperations/AbortMission and stays unsupported.
pub fn espionage_no_active_mission(
    state: &LoadedState,
    definitions: &crate::unit_creation::Definitions,
    receiver: i32,
    city: ComponentId,
) -> Result<(), Error> {
    if !state
        .player_alive(receiver)
        .map_err(|e| error(Stage::CityOccupied, e))?
    {
        return Ok(());
    }
    let owner = i32::from(city.owner);
    if receiver != owner
        && !state
            .game_relations()
            .map_err(|e| error(Stage::CityOccupied, e))?
            .is_allied(receiver, owner)
            .map_err(|e| error(Stage::CityOccupied, e))?
    {
        return Ok(());
    }
    let player = state
        .world()
        .player(receiver)
        .map_err(|e| error(Stage::CityOccupied, e))?;
    for unit in player
        .unit_records()
        .map_err(|e| error(Stage::CityOccupied, e))?
    {
        let value = &unit.value;
        let Some(plot) = state
            .world()
            .map()
            .plot_at(value.k_location.x, value.k_location.y)
        else {
            continue;
        };
        if !plot
            .city_attributes
            .as_ref()
            .is_some_and(|a| i32::from(a.player) == owner && a.owning_city == city.id)
        {
            continue;
        }
        if definitions
            .is_spy(state, value)
            .map_err(|e| error(Stage::CityOccupied, e))?
            && value.k_espionage.e_operation != u32::MAX
        {
            return Err(error(
                Stage::CityOccupied,
                "active captured-city spy mission abort",
            ));
        }
    }
    Ok(())
}

/// ExpelForeignUnitsFromTerritory2aaa5e0(reason5), no placement branch. War and
/// open-border skips are read from the live canonical diplomatic owners.
pub fn expel_no_displacement(state: &LoadedState, owner: i32, alive: &[i32]) -> Result<(), Error> {
    if !player(state, owner)?.diplomacy.can_enforce_borders {
        return Ok(());
    }
    for &other in alive {
        if other == owner
            || !state
                .player_alive(other)
                .map_err(|e| error(Stage::Expulsion, e))?
        {
            continue;
        }
        let at = usize::try_from(owner)
            .ok()
            .filter(|&i| i < 64)
            .ok_or_else(|| error(Stage::Expulsion, "border owner"))?;
        if player(state, other)?.diplomacy.open_borders_from_count[at] != 0 {
            continue;
        }
        let relations = state
            .game_relations()
            .map_err(|e| error(Stage::Expulsion, e))?;
        if relations
            .is_allied(other, owner)
            .map_err(|e| error(Stage::Expulsion, e))?
            || relations
                .is_permanently_allied(other, owner)
                .map_err(|e| error(Stage::Expulsion, e))?
        {
            continue;
        }
        if state
            .game_relations()
            .map_err(|e| error(Stage::Expulsion, e))?
            .is_at_war(other, owner)
            .map_err(|e| error(Stage::Expulsion, e))?
        {
            continue;
        }
        declare_war::plain::expel_outside_borders(state, other, owner)
            .map_err(|e| error(Stage::Expulsion, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn city_size_discount_uses_wrapping_product_then_signed_truncation() {
        assert_eq!(adjusted_size(50, 1, 3).unwrap(), 17);
        assert_eq!(adjusted_size(50, 4, 3).unwrap(), 50);
        assert_eq!(adjusted_size(-50, 1, 3).unwrap(), -17);
        assert!(adjusted_size(1, -1, 0).is_err());
        assert_eq!(percent(i32::MAX, 200), -2);
    }
    #[test]
    fn native_grievance_conversion_preserves_indefinite_integer() {
        assert_eq!(native_i32(-1.99), -1);
        assert_eq!(native_i32(f64::NAN), i32::MIN);
        assert_eq!(native_i32(2147483648.0), i32::MIN);
    }
}
