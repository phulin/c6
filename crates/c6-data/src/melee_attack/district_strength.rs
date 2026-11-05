//! Current ordinary Unit/District melee strengths. City defenses remain actual
//! District participants; unsupported owner/outer-defense branches are errors.
use super::strength as shared;
use crate::{loaded_actions::city_range_attack as city, registry::LinuxDefinitionRegistry, Rules};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};

#[derive(Debug)]
pub struct Error(pub String);
fn error(e: impl std::fmt::Debug) -> Error {
    Error(format!("{e:?}"))
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unit/District strength: {}", self.0)
    }
}
impl std::error::Error for Error {}

pub struct Definitions {
    binding: WireBinding,
    districts: LinuxDefinitionRegistry,
    district_city_strength: Vec<i32>,
    militia: i32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let districts =
            crate::unit_initialization_definitions::collection(rules, "Districts", "DistrictType")
                .map_err(error)?;
        let table = rules
            .gameplay()
            .table("Districts")
            .ok_or_else(|| error("Districts"))?;
        let district_city_strength = districts
            .entries()
            .iter()
            .map(|e| {
                let row = table
                    .row_by_id(e.database_rowid)
                    .ok_or_else(|| error("District row"))?;
                row.integer("CityStrengthModifier")
                    .map(|n| n as i32)
                    .map_err(error)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            districts,
            district_city_strength,
            militia: super::damage::integer(rules, "COMBAT_GARRISON_MILITIA_MODIFIER")
                .map_err(error)?,
        })
    }
    fn check(&self, state: &LoadedState) -> Result<(), Error> {
        if state.binding() != &self.binding {
            return Err(error("rules identity"));
        }
        Ok(())
    }
}
fn major(levels: &[(i32, u32)], owner: i32) -> Result<(), Error> {
    let mut rows = levels.iter().filter(|r| r.0 == owner);
    if rows.next().map(|r| r.1) != Some(0x253718b0) || rows.next().is_some() {
        return Err(error("ordinary major owner required"));
    }
    Ok(())
}
fn proposal(state: &LoadedState, id: u32, attack: bool) -> Result<i32, Error> {
    let proposals = state.combat_proposals().map_err(error)?;
    let Some((_, handle)) = proposals.active().iter().find(|r| r.0 == id) else {
        return Ok(0);
    };
    let proposal = proposals.proposal(*handle).map_err(error)?;
    Ok(if attack {
        proposal.attacker_strength_modifier
    } else {
        proposal.defender_strength_modifier
    }
    .wrapping_shl(8))
}

/// GetBaseDefenseStrength then GetBaseDefenseStrengthModifiers, with no preview
/// text. Wall bonuses require the separate outer-defense profile.
#[allow(clippy::too_many_arguments)]
pub fn base_defense(
    state: &LoadedState,
    definitions: &Definitions,
    city_definitions: &city::Definitions,
    strength: &shared::Definitions,
    formation: &crate::formation_rules::Definitions,
    levels: &[(i32, u32)],
    defender: ComponentId,
) -> Result<(i32, i32), Error> {
    definitions.check(state)?;
    let district = state
        .district(i32::from(defender.owner), defender.id)
        .map_err(error)?;
    let owner = city::district_owner(state, district).map_err(error)?;
    major(levels, owner)?;
    let maximum =
        city::maximum_damage(state, district, city_definitions, levels, false).map_err(error)?;
    if maximum <= 0 {
        return Ok((0, 0));
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or_else(|| error("District owner"))?;
    let base = player
        .stats
        .max_melee_strength_trained
        .wrapping_sub(definitions.militia)
        .wrapping_shl(8);
    let mut modifiers = 0i32;
    if let Some(garrison) =
        city::garrison(state, district, city_definitions, formation).map_err(error)?
    {
        let combat = formation
            .combat(state, garrison)
            .map_err(error)?
            .wrapping_shl(8);
        let wounded = strength
            .unit_reduction_for_damage(
                state,
                garrison.i_damage,
                garrison
                    .k_combat_modifers
                    .i_strength_reduction_for_damage_modifier,
            )
            .map_err(error)?;
        let current = combat.wrapping_sub(wounded);
        if current > base {
            modifiers = current.wrapping_sub(base);
        }
    }
    if city::maximum_damage(state, district, city_definitions, levels, true).map_err(error)? > 0 {
        return Err(error("wall defense strength bonuses"));
    }
    let owning = city::district_city(state, district).map_err(error)?;
    let mut districts = 0i32;
    for &id in &owning.districts.district_ids {
        let Some(d) = player
            .districts
            .records
            .entries
            .iter()
            .find(|e| e.value.id == id)
            .map(|e| &e.value)
        else {
            continue;
        };
        if d.is_complete && !d.pillaged {
            if let Some(index) = definitions.districts.find_index(d.district_hash) {
                districts = districts.wrapping_add(definitions.district_city_strength[index]);
            }
        }
    }
    if districts > 0 {
        modifiers = modifiers.wrapping_add(districts.wrapping_shl(8));
    }
    modifiers = modifiers.wrapping_add(
        owning
            .combat
            .inner_defense_strength_modifier
            .wrapping_shl(8),
    );
    Ok((base, modifiers))
}

#[allow(clippy::too_many_arguments)]
pub fn attack_modifier(
    state: &LoadedState,
    strength: &shared::Definitions,
    formation: &crate::formation_rules::Definitions,
    levels: &[(i32, u32)],
    players: &shared::CurrentPlayerCombat<'_>,
    attacker: UnitReference,
    defender: ComponentId,
    proposal_id: u32,
) -> Result<i32, Error> {
    let district = state
        .district(i32::from(defender.owner), defender.id)
        .map_err(error)?;
    let at = PlotCoord {
        x: district.location.0,
        y: district.location.1,
    };
    let owner = city::district_owner(state, district).map_err(error)?;
    let mut opening = shared::opening_terms(
        state,
        strength,
        levels,
        attacker,
        c6_core::melee_attack::strength::Side::Attack,
    )
    .map_err(error)?;
    opening =
        opening.wrapping_add(shared::amphibious_penalty(state, attacker, Some(at)).map_err(error)?);
    let mut bonuses =
        shared::versus_opponent_bonus(state, levels, attacker, owner).map_err(error)?;
    bonuses = bonuses.wrapping_add(shared::territory_bonus(state, attacker, at).map_err(error)?);
    bonuses =
        bonuses.wrapping_add(shared::miscellaneous_attack_bonus(state, attacker).map_err(error)?);
    bonuses = bonuses
        .wrapping_add(shared::holy_city_bonus(state, strength, players, attacker).map_err(error)?);
    bonuses =
        bonuses.wrapping_add(shared::neighbor_bonus(state, formation, attacker).map_err(error)?);
    bonuses = bonuses
        .wrapping_add(shared::military_policy_bonus(state, strength, attacker).map_err(error)?);
    bonuses = bonuses.wrapping_add(shared::diplomatic_bonus(state, attacker).map_err(error)?);
    bonuses = bonuses
        .wrapping_add(shared::espionage_bonus(state, levels, attacker, owner).map_err(error)?);
    bonuses = bonuses
        .wrapping_add(shared::adjacent_levy_bonus(state, formation, attacker).map_err(error)?);
    bonuses = bonuses.wrapping_add(shared::allies_bonus(state, players, attacker).map_err(error)?);
    bonuses = bonuses.wrapping_add(shared::unused_movement_bonus(state, attacker).map_err(error)?);
    bonuses = bonuses
        .wrapping_add(shared::friendly_city_bonus(state, players, attacker, at).map_err(error)?);
    let unit = super::actor(state, attacker).map_err(error)?;
    if unit.k_religion.i_religious_strength <= 0
        && unit.k_combat_modifers.i_combat_bonus_against_district > 0
    {
        bonuses = bonuses.wrapping_add(
            unit.k_combat_modifers
                .i_combat_bonus_against_district
                .wrapping_shl(8),
        );
    }
    bonuses = bonuses
        .wrapping_add(shared::specific_religion_bonus(state, strength, attacker).map_err(error)?);
    Ok(bonuses
        .wrapping_add(proposal(state, proposal_id, true)?)
        .wrapping_add(opening))
}

#[allow(clippy::too_many_arguments)]
pub fn defense_modifier(
    state: &LoadedState,
    city_definitions: &city::Definitions,
    strength: &shared::Definitions,
    formation: &crate::formation_rules::Definitions,
    levels: &[(i32, u32)],
    attacker: UnitReference,
    defender: ComponentId,
    proposal_id: u32,
) -> Result<i32, Error> {
    let district = state
        .district(i32::from(defender.owner), defender.id)
        .map_err(error)?;
    let maximum =
        city::maximum_damage(state, district, city_definitions, levels, false).map_err(error)?;
    if maximum == 0 {
        return Ok(0);
    }
    let percent = district
        .garrison_damage
        .wrapping_mul(100)
        .checked_div(maximum)
        .ok_or_else(|| error("District wound division"))?;
    let coefficient = city::garrison(state, district, city_definitions, formation)
        .map_err(error)?
        .map_or(0, |u| {
            u.k_combat_modifers.i_strength_reduction_for_damage_modifier
        });
    let wounded = strength
        .district_reduction_for_damage(state, percent, coefficient)
        .map_err(error)?;
    let at = PlotCoord {
        x: district.location.0,
        y: district.location.1,
    };
    let terrain = shared::terrain_bonus(
        state,
        strength,
        at,
        super::actor(state, attacker).map_err(error)?.k_location,
        attacker,
    )
    .map_err(error)?;
    let _ = city::district_owner(state, district).map_err(error)?;
    Ok(terrain
        .wrapping_add(proposal(state, proposal_id, false)?)
        .wrapping_sub(wounded))
}
