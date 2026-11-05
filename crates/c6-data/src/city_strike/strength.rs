//! Source District attack and Unit-versus-District defense modifiers.
use crate::{
    formation_rules,
    loaded_actions::city_range_attack as city,
    melee_attack::{self as melee, strength as shared},
};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::ComponentId};

#[derive(Debug)]
pub enum Error {
    District(c6_core::loaded_state::district_initialization::DistrictError),
    City(crate::loaded_actions::Error),
    Shared(shared::Error),
    Actor(melee::Error),
    ProposalAccess(c6_core::loaded_state::combat_proposals::AccessError),
    Proposal(c6_core::melee_attack::proposals::registry::Error),
    Arithmetic,
    MissingLevel(i32),
}
fn proposal_modifier(state: &LoadedState, id: u32, attack: bool) -> Result<i32, Error> {
    let registry = state.combat_proposals().map_err(Error::ProposalAccess)?;
    let Some((_, handle)) = registry.active().iter().find(|row| row.0 == id) else {
        return Ok(0);
    };
    let proposal = registry.proposal(*handle).map_err(Error::Proposal)?;
    Ok(if attack {
        proposal.attacker_strength_modifier
    } else {
        proposal.defender_strength_modifier
    })
}
fn percent(damage: i32, maximum: i32) -> Result<i32, Error> {
    damage
        .wrapping_mul(100)
        .checked_div(maximum)
        .ok_or(Error::Arithmetic)
}
/// Native district attack modifier uses outer-defense damage when outer HP is
/// positive and garrison damage otherwise, before querying the actual garrison.
#[allow(clippy::too_many_arguments)]
pub fn attack_modifier_q8(
    state: &LoadedState,
    definitions: &city::Definitions,
    strength: &shared::Definitions,
    formation: &formation_rules::Definitions,
    levels: &[(i32, u32)],
    attacker: ComponentId,
    proposal_id: u32,
) -> Result<i32, Error> {
    let district = state
        .district(i32::from(attacker.owner), attacker.id)
        .map_err(Error::District)?;
    let outer =
        city::maximum_damage(state, district, definitions, levels, true).map_err(Error::City)? > 0;
    let damage = if outer {
        district.outer_defenses_damage
    } else {
        district.garrison_damage
    };
    let maximum =
        city::maximum_damage(state, district, definitions, levels, outer).map_err(Error::City)?;
    let fraction = percent(damage, maximum)?;
    let coefficient = city::garrison(state, district, definitions, formation)
        .map_err(Error::City)?
        .map_or(0, |unit| {
            unit.k_combat_modifers
                .i_strength_reduction_for_damage_modifier
        });
    let reduction = strength
        .district_reduction_for_damage(state, fraction, coefficient)
        .map_err(Error::Shared)?;
    let owning_city = city::district_city(state, district).map_err(Error::City)?;
    let mut modifier = owning_city
        .combat
        .ranged_strike_strength_modifier
        .wrapping_shl(8);
    // Native queries GetOwner even when there is no active proposal.
    let _ = city::district_owner(state, district).map_err(Error::City)?;
    modifier = modifier.wrapping_add(proposal_modifier(state, proposal_id, true)?.wrapping_shl(8));
    Ok(modifier.wrapping_sub(reduction))
}
/// Native Unit-versus-District defense has its own opening: it uses the wounded
/// DISTRICT multiplier, ignores the Unit no-damage-reduction flag, and has no
/// ordinary Unit/Unit formation-class or combat-strength early return.
#[allow(clippy::too_many_arguments)]
pub fn defense_modifier_q8(
    state: &LoadedState,
    definitions: &shared::Definitions,
    formation: &formation_rules::Definitions,
    levels: &[(i32, u32)],
    players: &shared::CurrentPlayerCombat<'_>,
    attacker: ComponentId,
    defender: UnitReference,
    proposal_id: u32,
) -> Result<i32, Error> {
    let maximum = definitions
        .district_maximum_hit_points(state)
        .map_err(Error::Shared)?;
    if maximum == 0 {
        return Ok(0);
    }
    let unit = melee::actor(state, defender).map_err(Error::Actor)?;
    let fraction = percent(unit.i_damage, maximum)?;
    let mut opening = 0_i32.wrapping_sub(
        definitions
            .district_reduction_for_damage(
                state,
                fraction,
                unit.k_combat_modifers
                    .i_strength_reduction_for_damage_modifier,
            )
            .map_err(Error::Shared)?,
    );
    let level = levels
        .iter()
        .find(|row| row.0 == unit.e_owner)
        .ok_or(Error::MissingLevel(unit.e_owner))?
        .1;
    if level == 0x253718b0
        && unit
            .k_combat_modifers
            .i_strength_reduction_insufficient_fuel
            > 0
    {
        opening = opening.wrapping_sub(
            unit.k_combat_modifers
                .i_strength_reduction_insufficient_fuel
                .wrapping_shl(8),
        );
    }
    if crate::unit_movement_state::unit_is_embarked(unit)
        && !unit.k_combat_modifers.b_fight_while_embarked
    {
        return Ok(opening);
    }
    let district = state
        .district(i32::from(attacker.owner), attacker.id)
        .map_err(Error::District)?;
    let owner = city::district_owner(state, district).map_err(Error::City)?;
    let mut bonuses =
        shared::versus_opponent_bonus(state, levels, defender, owner).map_err(Error::Shared)?;
    bonuses = bonuses.wrapping_add(
        shared::territory_bonus(state, defender, unit.k_location).map_err(Error::Shared)?,
    );
    bonuses = bonuses.wrapping_add(
        shared::terrain_bonus_with_kind(
            state,
            definitions,
            unit.k_location,
            c6_core::unit::PlotCoord {
                x: district.location.0,
                y: district.location.1,
            },
            defender,
            true,
        )
        .map_err(Error::Shared)?,
    );
    bonuses = bonuses.wrapping_add(
        shared::fortification_bonus(state, definitions, defender).map_err(Error::Shared)?,
    );
    // Ranged GetSupportBonus still queries actual Player.HasCombatAdjacency.
    let _ = players
        .has_adjacency(state, unit.e_owner)
        .map_err(Error::Shared)?;
    bonuses = bonuses.wrapping_add(
        shared::holy_city_bonus(state, definitions, players, defender).map_err(Error::Shared)?,
    );
    bonuses = bonuses
        .wrapping_add(shared::neighbor_bonus(state, formation, defender).map_err(Error::Shared)?);
    bonuses = bonuses.wrapping_add(
        shared::military_policy_bonus(state, definitions, defender).map_err(Error::Shared)?,
    );
    bonuses =
        bonuses.wrapping_add(shared::diplomatic_bonus(state, defender).map_err(Error::Shared)?);
    let owner = city::district_owner(state, district).map_err(Error::City)?;
    bonuses = bonuses.wrapping_add(
        shared::espionage_bonus(state, levels, defender, owner).map_err(Error::Shared)?,
    );
    bonuses = bonuses.wrapping_add(
        shared::adjacent_levy_bonus(state, formation, defender).map_err(Error::Shared)?,
    );
    bonuses = bonuses
        .wrapping_add(shared::allies_bonus(state, players, defender).map_err(Error::Shared)?);
    bonuses = bonuses
        .wrapping_add(shared::unused_movement_bonus(state, defender).map_err(Error::Shared)?);
    bonuses = bonuses.wrapping_add(
        shared::friendly_city_bonus(state, players, defender, unit.k_location)
            .map_err(Error::Shared)?,
    );
    bonuses = bonuses.wrapping_add(
        shared::specific_religion_bonus(state, definitions, defender).map_err(Error::Shared)?,
    );
    bonuses = bonuses.wrapping_add(proposal_modifier(state, proposal_id, false)?.wrapping_shl(8));
    Ok(bonuses.wrapping_add(opening))
}
