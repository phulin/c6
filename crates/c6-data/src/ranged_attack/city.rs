//! Ordinary ranged Unit/city combat. The attacker never retaliates or advances.
//! Generation retains both original ordered game-sync draws, including reduced
//! inner damage while walls stand. Unsupported branches preserve reached RNG.
use super::{error, Error, RANGED};
use crate::{
    loaded_actions::city_range_attack as district, loaded_upkeep as upkeep, melee_attack as melee,
    unit_placement_state as placement, unit_sight_state as sight,
};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};
use melee::{backend, city, damage, experience, proposals, resolve};
type ProviderError<E> = <E as sight::Effects>::Error;
pub struct Resolved {
    pub members: city::Generated,
}

fn generate<B>(
    state: &mut LoadedState,
    backend: &mut B,
    defs: &city::Definitions,
    inputs: &melee::transition::Inputs<'_, '_, '_>,
    actor: UnitReference,
    defender: ComponentId,
) -> Result<city::Generated, Error>
where
    B: proposals::Effects
        + proposals::collection_modifiers::Services<Error = <B as proposals::Effects>::Error>,
    <B as proposals::Effects>::Error: std::fmt::Debug,
{
    let rules = inputs.generation;
    let unit = melee::actor(state, actor).map_err(|e| error("city actor", e))?;
    if state
        .unit_runtime(actor.player(), actor.id)
        .map_err(|e| error("city actor", e))?
        .domain
        != 2
        || crate::unit_movement_state::unit_is_embarked(unit)
    {
        return Err(error(
            "city scope",
            "ordinary unembarked land ranged attacker required",
        ));
    }
    let d = state
        .district(i32::from(defender.owner), defender.id)
        .map_err(|e| error("city target", e))?;
    if d.district_hash != crate::registry::linux_type_hash(b"DISTRICT_CITY_CENTER") {
        return Err(error("city scope", "city-center District required"));
    }
    let owner = district::district_owner(state, d).map_err(|e| error("city owner", e))?;
    for owner in [actor.player(), owner] {
        if rules.levels.iter().find(|r| r.0 == owner).map(|r| r.1) != Some(0x253718b0) {
            return Err(error("city scope", "ordinary major owners required"));
        }
    }
    let defending_player = state
        .retained()
        .players
        .iter()
        .find(|p| p.requested_owner == owner)
        .ok_or_else(|| error("city owner", "current Player.Cities"))?;
    if defending_player.cities.urban_defenses > 0 {
        return Err(error("city scope", "Urban Defenses continuation"));
    }
    let ranged = rules
        .formation
        .ranged(state, unit)
        .map_err(|e| error("city type", e))?;
    let bombard = rules
        .formation
        .bombard(state, unit)
        .map_err(|e| error("city type", e))?;
    if ranged <= 0 || ranged <= bombard {
        return Err(error("city scope", "ordinary RANGED combat required"));
    }
    let location = PlotCoord {
        x: d.location.0,
        y: d.location.1,
    };
    let inner_max = district::maximum_damage(state, d, &defs.district, rules.levels, false)
        .map_err(|e| error("city maximum", e))?;
    let outer_max = district::maximum_damage(state, d, &defs.district, rules.levels, true)
        .map_err(|e| error("city maximum", e))?;
    let initial_inner = d.garrison_damage;
    let initial_outer = d.outer_defenses_damage;
    let owning = district::district_city(state, d).map_err(|e| error("city owner", e))?;
    // Preserve unsupported special bypass branches explicitly; ordinary ranged
    // attacks use the native no-bypass parameter even if walls are exhausted.
    if owning.header.prevent_bypass_outer_defenses != 0
        || owning.header.prevent_melee_attack_outer_defenses != 0
    {
        return Err(error("city scope", "special wall prevention modifiers"));
    }
    let class = defs
        .promotion(state, actor)
        .map_err(|e| error("city bypass class", e))?;
    if city::adjacent_modifier(state, (location.x, location.y), actor.player(), class, true)
        .map_err(|e| error("city bypass", e))?
    {
        return Err(error(
            "city scope",
            "positive bypass outer defenses continuation",
        ));
    }
    let attacker_strength = rules
        .formation
        .ranged(state, unit)
        .map_err(|e| error("city strength", e))?
        .wrapping_shl(8);
    let (defender_strength, base_modifier) = melee::district_strength::base_defense(
        state,
        &defs.strength,
        &defs.district,
        rules.strength,
        rules.formation,
        rules.levels,
        defender,
    )
    .map_err(|e| error("city defense", e))?;
    let (id, _) = proposals::Context::new(state, backend)
        .propose_unit_district_with_type(actor, defender, location, RANGED)
        .map_err(|e| error("city proposal", e))?;
    let attack_modifier = melee::district_strength::attack_modifier_with_kind(
        state,
        rules.strength,
        rules.formation,
        rules.levels,
        rules.players,
        actor,
        defender,
        id,
        true,
        defs.ranged_penalty,
    )
    .map_err(|e| error("city attack strength", e))?;
    let defense_modifier = base_modifier.wrapping_add(
        melee::district_strength::defense_modifier_with_kind(
            state,
            &defs.district,
            rules.strength,
            rules.formation,
            rules.levels,
            actor,
            defender,
            id,
            true,
        )
        .map_err(|e| error("city defense strength", e))?,
    );
    proposals::collection_modifiers::forget_combat(state, backend, id)
        .map_err(|e| error("city proposal destruction", e))?;
    let attack = attacker_strength.wrapping_add(attack_modifier);
    let defense = defender_strength.wrapping_add(defense_modifier);
    let (inner, mut draws) = damage::unit_district_with_draws(
        state,
        rules.damage,
        actor,
        attack,
        defense,
        inner_max,
        initial_outer,
        outer_max,
        0x5e97d629,
        false,
    )
    .map_err(|e| error("city inner damage", e))?;
    let (outer, outer_draws) = damage::unit_district_with_draws(
        state,
        rules.damage,
        actor,
        attack,
        defense,
        inner_max,
        initial_outer,
        outer_max,
        0x6da56a3d,
        false,
    )
    .map_err(|e| error("city outer damage", e))?;
    draws.extend(outer_draws);
    let unit = melee::actor(state, actor).map_err(|e| error("city damage", e))?;
    let percentage = if unit.k_combat_modifers.b_ignore_ranged_vs_district_penalty {
        defs.outer_bombard_percent
    } else {
        defs.outer_ranged_percent
    };
    let outer = if outer > 0 {
        outer.wrapping_mul(percentage) / 100
    } else {
        outer
    };
    let attacker_max = rules
        .maximum_damage
        .maximum_damage(unit)
        .map_err(|e| error("city attacker maximum", e))?;
    let experience = if initial_inner < inner_max {
        experience::unit_district_award(
            state,
            inputs.experience,
            rules.formation,
            rules.levels,
            actor,
            owner,
            false,
        )
        .map_err(|e| error("city experience generation", e))?
    } else {
        0
    };
    Ok(city::Generated {
        attacker: actor,
        defender,
        location,
        attacker_strength_q8: attacker_strength,
        defender_strength_q8: defender_strength,
        attacker_modifier_q8: attack_modifier,
        defender_modifier_q8: defense_modifier,
        // GetDefenseType2be01e0 tests equality before the bypass query.
        defense_type: if initial_outer == outer_max {
            0x5e97d629
        } else {
            0x6da56a3d
        },
        attacker_damage: 0,
        defender_damage: inner,
        outer_damage: outer,
        attacker_final_damage: 0,
        defender_final_damage: initial_inner.wrapping_add(inner),
        outer_final_damage: initial_outer.wrapping_add(outer),
        attacker_maximum_damage: attacker_max,
        defender_maximum_damage: inner_max,
        outer_maximum_damage: outer_max,
        attacker_experience: experience,
        attacker_advances: false,
        game_sync_draws: draws,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute<E, R>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    backend: &mut backend::Backend<melee::transition::Reports<'_, E>, R>,
    inputs: &melee::transition::Inputs<'_, '_, '_>,
    bindings: &mut resolve::completion::stock::Bindings,
    actor: UnitReference,
    defender: ComponentId,
) -> Result<Resolved, Error>
where
    R: backend::Requirements,
    R::Error: std::fmt::Debug,
    E: city::resolve::Effects
        + placement::Effects
        + upkeep::Effects<Error = ProviderError<E>>
        + upkeep::OperationReports<Error = ProviderError<E>>
        + experience::Reports<Error = ProviderError<E>>
        + resolve::completion::stock::UnitDistrictEffects,
    ProviderError<E>: std::fmt::Debug,
    <E as resolve::completion::stock::Effects>::Error: std::fmt::Debug,
{
    let _ = sight;
    let defs = inputs
        .generation
        .city
        .ok_or_else(|| error("city rules", "current city combat definitions"))?;
    let members = generate(state, backend, defs, inputs, actor, defender)?;
    let effects = &mut *backend.reporting.0;
    if members.defender_final_damage >= members.defender_maximum_damage {
        return Err(error("city scope", "lethal garrison damage continuation"));
    }
    let unit = state
        .unit_mut(actor.player(), actor.id)
        .map_err(|e| error("city attacker complete", e))?;
    unit.i_fortify_turns = 0;
    unit.b_hidden_visibility_forfeited = true;
    upkeep::clear_unit_operations(state, actor.player(), actor.id, effects)
        .map_err(|e| error("city attacker clear", e))?;
    upkeep::complete_unit_movement(state, actor.player(), actor.id, effects)
        .map_err(|e| error("city attacker complete", e))?;
    if members.outer_damage > 0 && members.outer_maximum_damage > 0 {
        city::resolve::change_district_damage(
            state,
            effects,
            defender,
            members.outer_damage,
            members.outer_maximum_damage,
            true,
        )
        .map_err(|e| error("city outer damage", e))?;
    }
    city::resolve::change_district_damage(
        state,
        effects,
        defender,
        members.defender_damage,
        members.defender_maximum_damage,
        false,
    )
    .map_err(|e| error("city inner damage", e))?;
    effects
        .city_melee_publication(
            state,
            city::Publication::Statuses {
                attacker: actor,
                defender,
                attacker_damage: 0,
                defender_damage: members.defender_damage,
                status: inputs.death_report_status,
            },
        )
        .map_err(|e| error("city ranged status", e))?;
    experience::change(
        state,
        inputs.experience,
        inputs.generation.maximum_damage,
        inputs.generation.levels,
        actor,
        members.attacker_experience,
        effects,
    )
    .map_err(|e| error("city experience", e))?;
    let partners = experience::joint_war_partners(
        state,
        inputs.generation.levels,
        inputs.cached_majors,
        actor.player(),
    )
    .map_err(|e| error("city joint war", e))?;
    if !partners.owners.is_empty() {
        return Err(error(
            "city joint war",
            "positive District partner experience continuation",
        ));
    }
    upkeep::clear_unit_operations(state, actor.player(), actor.id, effects)
        .map_err(|e| error("city resolve clear", e))?;
    let heal = melee::actor(state, actor)
        .map_err(|e| error("city healing", e))?
        .k_health_modifiers
        .b_can_heal_after_action;
    state
        .unit_mut(actor.player(), actor.id)
        .map_err(|e| error("city healing", e))?
        .b_eligible_to_heal = heal;
    resolve::budget::consume_reference_attack(state, inputs.movement, effects, actor)
        .map_err(|e| error("city attack budget", e))?;
    city::weariness::apply(
        state,
        &defs.weariness,
        inputs.generation.maximum_damage,
        inputs.generation.levels,
        &members,
    )
    .map_err(|e| error("city war weariness", e))?;
    state
        .record_game_combat_death(false, || Ok(false))
        .map_err(|e| error("city combat statistics", e))?;
    resolve::completion::stock::unit_district_combat_occurred(
        state,
        bindings,
        inputs.callbacks,
        effects,
        &members,
        None,
    )
    .map_err(|e| error("city combat occurred", e))?;
    Ok(Resolved { members })
}
