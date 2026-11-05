//! Current-state Linux District/Unit ranged combat stages.
//!
//! District participants, rules and RNG remain canonical. Generation stages are
//! explicit continuations; their outputs do not claim a completed City command.
pub mod completion;
pub mod reports;
pub mod strength;
use crate::{loaded_actions::city_range_attack as city, melee_attack as melee};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};

#[derive(Debug)]
pub struct Error {
    pub stage: &'static str,
    pub detail: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "city strike {}: {}", self.stage, self.detail)
    }
}
impl std::error::Error for Error {}
pub(crate) fn error(stage: &'static str, detail: impl std::fmt::Debug) -> Error {
    Error {
        stage,
        detail: format!("{detail:?}"),
    }
}
/// Complete generated numeric members, before ResolveCombat and command reports.
/// The attacker identity names the actual District; there is no attacking Unit.
#[derive(Clone, Debug)]
#[must_use = "generated members still require damage, experience and callback resolution"]
pub struct Generated {
    pub attacker: ComponentId,
    pub defender: UnitReference,
    pub location: PlotCoord,
    pub attacker_strength_q8: i32,
    pub defender_strength_q8: i32,
    pub attacker_modifier_q8: i32,
    pub defender_modifier_q8: i32,
    pub damage: i32,
    pub defender_final_damage: i32,
    pub defender_maximum_damage: i32,
    pub attacker_maximum_damage: i32,
    pub attacker_outer_maximum_damage: i32,
    pub defender_experience: i32,
    pub game_sync_draws: Vec<melee::damage::RandomDraw>,
}
/// Genuine District/Unit generation on the current proposal and RNG owners.
/// The caller owns command legality, actual target selection and outer Resolve.
#[allow(clippy::too_many_arguments)]
pub fn generate<B>(
    state: &mut LoadedState,
    backend: &mut B,
    definitions: &city::Definitions,
    inputs: &melee::transition::Inputs<'_, '_, '_>,
    attacker: ComponentId,
    defender: UnitReference,
    location: PlotCoord,
) -> Result<Generated, Error>
where
    B: melee::proposals::Effects
        + melee::proposals::collection_modifiers::Services<
            Error = <B as melee::proposals::Effects>::Error,
        >,
    <B as melee::proposals::Effects>::Error: std::fmt::Debug,
{
    if attacker.component_type != 3 {
        return Err(error("attacker", "District component type required"));
    }
    let rules = inputs.generation;
    let district = state
        .district(i32::from(attacker.owner), attacker.id)
        .map_err(|e| error("attacker", e))?;
    let attack = city::attack_strength_current_q8(state, district, definitions, rules.levels)
        .map_err(|e| error("base attack", e))?;
    let defense = rules
        .formation
        .combat(
            state,
            melee::actor(state, defender).map_err(|e| error("defender", e))?,
        )
        .map_err(|e| error("base defense", e))?
        .wrapping_shl(8);
    let (id, _) = melee::proposals::Context::new(state, backend)
        .propose_district_unit(attacker, defender, location)
        .map_err(|e| error("proposal", e))?;
    let attack_modifier = strength::attack_modifier_q8(
        state,
        definitions,
        rules.strength,
        rules.formation,
        rules.levels,
        attacker,
        id,
    )
    .map_err(|e| error("attack modifier", e))?;
    let defense_modifier = strength::defense_modifier_q8(
        state,
        rules.strength,
        rules.formation,
        rules.levels,
        rules.players,
        attacker,
        defender,
        id,
    )
    .map_err(|e| error("defense modifier", e))?;
    // District generation destroys its temporary proposal BEFORE damage/RNG.
    melee::proposals::collection_modifiers::forget_combat(state, backend, id)
        .map_err(|e| error("proposal destruction", e))?;
    let (damage, draws) = melee::damage::district_unit_with_draws(
        state,
        rules.damage,
        defender,
        attack.wrapping_add(attack_modifier),
        defense.wrapping_add(defense_modifier),
    )
    .map_err(|e| error("damage generation", e))?;
    let final_damage = melee::actor(state, defender)
        .map_err(|e| error("final damage", e))?
        .i_damage
        .wrapping_add(damage);
    let district = state
        .district(i32::from(attacker.owner), attacker.id)
        .map_err(|e| error("attacker", e))?;
    let attacker_max = city::maximum_damage(state, district, definitions, rules.levels, false)
        .map_err(|e| error("garrison HP", e))?;
    let outer_max = city::maximum_damage(state, district, definitions, rules.levels, true)
        .map_err(|e| error("outer HP", e))?;
    let defender_max = rules
        .maximum_damage
        .maximum_damage(melee::actor(state, defender).map_err(|e| error("defender", e))?)
        .map_err(|e| error("maximum", e))?;
    let experience = melee::experience::district_defender_award(
        state,
        inputs.experience,
        rules.formation,
        defender,
    )
    .map_err(|e| error("experience generation", e))?;
    if rules
        .damage
        .retreat_when_killed_flag(state, defender, final_damage, defender_max)
        .map_err(|e| error("retreat generation", e))?
    {
        return Err(error("retreat generation", "retreat continuation required"));
    }
    Ok(Generated {
        attacker,
        defender,
        location,
        attacker_strength_q8: attack,
        defender_strength_q8: defense,
        attacker_modifier_q8: attack_modifier,
        defender_modifier_q8: defense_modifier,
        damage,
        defender_final_damage: final_damage,
        defender_maximum_damage: defender_max,
        attacker_maximum_damage: attacker_max,
        attacker_outer_maximum_damage: outer_max,
        defender_experience: experience,
        game_sync_draws: draws,
    })
}

/// One explicit ordinary City command outcome, after all supported aftermath.
#[derive(Debug)]
pub enum Outcome {
    Rejected,
    Applied(Generated),
}
