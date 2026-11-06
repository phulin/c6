//! Inner city melee and outer combat continuation over current canonical owners.
use super::{error, Error, Generated, Publication};
use crate::{
    loaded_upkeep as upkeep, unit_placement_state as placement, unit_sight_state as sight,
};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};
type ProviderError<E> = <E as sight::Effects>::Error;

pub trait Effects {
    fn city_melee_publication(
        &mut self,
        state: &mut LoadedState,
        publication: Publication,
    ) -> Result<(), String>;
}

#[must_use]
pub struct Resolved {
    pub members: Generated,
    pub captured: Option<super::capture::Captured>,
}

fn current_damage(state: &LoadedState, id: ComponentId, outer: bool) -> Result<i32, Error> {
    let district = state
        .district(i32::from(id.owner), id.id)
        .map_err(|e| error("District damage", e))?;
    Ok(if outer {
        district.outer_defenses_damage
    } else {
        district.garrison_damage
    })
}
pub(crate) fn change_district_damage<E: Effects>(
    state: &mut LoadedState,
    effects: &mut E,
    id: ComponentId,
    amount: i32,
    maximum: i32,
    outer: bool,
) -> Result<(), Error> {
    if maximum < 0 {
        return Err(error("District damage", "negative maximum"));
    }
    let previous = current_damage(state, id, outer)?;
    let current = previous.wrapping_add(amount).clamp(0, maximum);
    let district = state
        .district_mut(i32::from(id.owner), id.id)
        .map_err(|e| error("District damage", e))?;
    if outer {
        district.outer_defenses_damage = current;
    } else {
        district.garrison_damage = current;
    }
    if current != previous {
        effects
            .city_melee_publication(
                state,
                Publication::DistrictDamageChanged {
                    district: id,
                    defense_type: if outer { 0x6da56a3d } else { 0x5e97d629 },
                    current,
                    previous,
                },
            )
            .map_err(|e| error("District damage report", e))?;
    }
    // ChangeDamage writes the current turn after SetDamage and its subscribers,
    // whenever the requested positive delta survived the major-owner guard.
    if amount > 0 {
        let turn = state.retained().core.current_turn;
        state
            .district_mut(i32::from(id.owner), id.id)
            .map_err(|e| error("District last damage", e))?
            .last_turn_damaged = turn;
    }
    Ok(())
}

/// Owned inner continuation. No completed action is reported before all
/// capture, movement and outer CombatOccurred continuations succeed.
#[must_use]
pub struct Prepared {
    pub members: Generated,
    pub capture_required: bool,
    pub captured: Option<super::capture::Captured>,
}

/// Source-ordered attacker completion, damage and experience up to Capture.
#[allow(clippy::too_many_arguments)]
pub fn prepare<E>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    inputs: &crate::melee_attack::transition::Inputs<'_, '_, '_>,
    members: Generated,
) -> Result<Prepared, Error>
where
    E: Effects
        + placement::Effects
        + upkeep::Effects<Error = ProviderError<E>>
        + upkeep::OperationReports<Error = ProviderError<E>>
        + crate::melee_attack::experience::Reports<Error = ProviderError<E>>
        + crate::melee_attack::resolve::surviving::Effects
        + crate::melee_attack::resolve::completion::stock::UnitDistrictEffects,
    ProviderError<E>: std::fmt::Debug,
    <E as crate::melee_attack::resolve::completion::stock::Effects>::Error: std::fmt::Debug,
{
    let attacker = members.attacker;
    // AttackInto's successful-generation completion precedes ResolveCombat.
    state
        .unit_mut(attacker.player(), attacker.id)
        .map_err(|e| error("attacker completion", e))?
        .i_fortify_turns = 0;
    upkeep::clear_unit_operations(state, attacker.player(), attacker.id, effects)
        .map_err(|e| error("attacker completion", e))?;
    upkeep::complete_unit_movement(state, attacker.player(), attacker.id, effects)
        .map_err(|e| error("attacker completion", e))?;
    placement::change_damage(
        state,
        inputs.placement,
        inputs.queries,
        sight,
        effects,
        (attacker.player(), attacker.id),
        members.attacker_damage,
    )
    .map_err(|e| error("attacker damage", e))?;
    let unit =
        crate::melee_attack::actor(state, attacker).map_err(|e| error("attacker damage", e))?;
    if unit.i_damage
        >= inputs
            .generation
            .maximum_damage
            .maximum_damage(unit)
            .map_err(|e| error("attacker damage", e))?
    {
        return Err(error("attacker damage", "Unit death continuation"));
    }
    // ResolveMeleeUnitVsDistrictCombat applies positive outer damage before
    // garrison damage, using the precombat proposal even when this exhausts walls.
    if members.outer_damage > 0 && members.outer_maximum_damage > 0 {
        change_district_damage(
            state,
            effects,
            members.defender,
            members.outer_damage,
            members.outer_maximum_damage,
            true,
        )?;
    }
    change_district_damage(
        state,
        effects,
        members.defender,
        members.defender_damage,
        members.defender_maximum_damage,
        false,
    )?;
    effects
        .city_melee_publication(
            state,
            Publication::Statuses {
                attacker,
                defender: members.defender,
                attacker_damage: members.attacker_damage,
                defender_damage: members.defender_damage,
                status: inputs.death_report_status,
            },
        )
        .map_err(|e| error("city statuses", e))?;
    crate::melee_attack::experience::change(
        state,
        inputs.experience,
        inputs.generation.maximum_damage,
        inputs.generation.levels,
        attacker,
        members.attacker_experience,
        effects,
    )
    .map_err(|e| error("attacker experience", e))?;
    let partners = crate::melee_attack::experience::joint_war_partners(
        state,
        inputs.generation.levels,
        inputs.cached_majors,
        attacker.player(),
    )
    .map_err(|e| error("joint war experience", e))?;
    if !partners.owners.is_empty() {
        return Err(error(
            "joint war experience",
            "positive District defender award partners",
        ));
    }
    let capture_required =
        current_damage(state, members.defender, false)? >= members.defender_maximum_damage;
    Ok(Prepared {
        members,
        capture_required,
        captured: None,
    })
}

/// Continue the same inner resolver after the source Capture boundary.
#[allow(clippy::too_many_arguments)]
pub fn complete<E>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    definitions: &super::Definitions,
    inputs: &crate::melee_attack::transition::Inputs<'_, '_, '_>,
    bindings: &mut crate::melee_attack::resolve::completion::stock::Bindings,
    prepared: Prepared,
) -> Result<Resolved, Error>
where
    E: Effects
        + placement::Effects
        + upkeep::Effects<Error = ProviderError<E>>
        + upkeep::OperationReports<Error = ProviderError<E>>
        + crate::melee_attack::experience::Reports<Error = ProviderError<E>>
        + crate::melee_attack::resolve::surviving::Effects
        + crate::melee_attack::resolve::completion::stock::UnitDistrictEffects,
    ProviderError<E>: std::fmt::Debug,
    <E as crate::melee_attack::resolve::completion::stock::Effects>::Error: std::fmt::Debug,
{
    if prepared.capture_required && prepared.captured.is_none() {
        return Err(error("capture", "city capture continuation"));
    }
    let captured = prepared.captured;
    let members = prepared.members;
    let attacker = members.attacker;
    upkeep::clear_unit_operations(state, attacker.player(), attacker.id, effects)
        .map_err(|e| error("inner movement", e))?;
    let unit =
        crate::melee_attack::actor(state, attacker).map_err(|e| error("inner movement", e))?;
    let formation = state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(|e| error("inner movement", e))?
        .formation
        .ok_or_else(|| error("inner movement", "Formation"))?;
    let cost = effects
        .combat_movement_cost(state, sight, attacker, members.location)
        .map_err(|e| error("inner movement cost", e))?;
    upkeep::change_formation_moves(
        state,
        formation.owner,
        formation.requested_id,
        inputs.movement,
        effects,
        cost.wrapping_neg(),
    )
    .map_err(|e| error("inner movement cost", e))?;
    if captured.is_some() && members.attacker_advances {
        let (enemy, combat) = effects
            .visible_enemy_at(state, sight, attacker, members.location)
            .map_err(|e| error("capture advance", e))?;
        if !(enemy && combat)
            && !effects
                .enemy_defensible_district(state, attacker, members.location)
                .map_err(|e| error("capture advance", e))?
        {
            crate::melee_attack::resolve::surviving::advance_formation(
                state,
                inputs.placement,
                inputs.queries,
                sight,
                effects,
                formation.owner,
                formation.requested_id,
                members.location,
            )
            .map_err(|e| error("capture advance", e))?;
        }
    }
    crate::melee_attack::resolve::budget::consume_reference_attack(
        state,
        inputs.movement,
        effects,
        attacker,
    )
    .map_err(|e| error("outer attack budget", e))?;
    // AwardPostCombatEffects requires Unit/Unit versus, so this branch skips it.
    super::weariness::apply(
        state,
        &definitions.weariness,
        inputs.generation.maximum_damage,
        inputs.generation.levels,
        &members,
    )?;
    state
        .record_game_combat_death(false, || Ok(false))
        .map_err(|e| error("combat statistics", e))?;
    crate::melee_attack::resolve::completion::stock::unit_district_combat_occurred(
        state,
        bindings,
        inputs.callbacks,
        effects,
        &members,
        captured.as_ref().map(|c| &c.retired),
    )
    .map_err(|e| error("combat occurred", e))?;
    Ok(Resolved { members, captured })
}

/// Supported surviving attacker/city path. A reached destruction/capture or
/// attacker death is retained as an explicit continuation error.
#[allow(clippy::too_many_arguments)]
pub fn apply<E>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    definitions: &super::Definitions,
    inputs: &crate::melee_attack::transition::Inputs<'_, '_, '_>,
    bindings: &mut crate::melee_attack::resolve::completion::stock::Bindings,
    members: Generated,
) -> Result<Resolved, Error>
where
    E: Effects
        + placement::Effects
        + upkeep::Effects<Error = ProviderError<E>>
        + upkeep::OperationReports<Error = ProviderError<E>>
        + crate::melee_attack::experience::Reports<Error = ProviderError<E>>
        + crate::melee_attack::resolve::surviving::Effects
        + crate::melee_attack::resolve::completion::stock::UnitDistrictEffects,
    ProviderError<E>: std::fmt::Debug,
    <E as crate::melee_attack::resolve::completion::stock::Effects>::Error: std::fmt::Debug,
{
    let prepared = prepare(state, sight, effects, inputs, members)?;
    complete(
        state,
        sight,
        effects,
        definitions,
        inputs,
        bindings,
        prepared,
    )
}
