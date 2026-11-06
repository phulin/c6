//! Source-ordered ordinary land Unit melee against a city-center District.
//! Generation is a typed unfinished stage. It never reports an applied action.
use super::{damage, district_strength, proposals};
use crate::{loaded_actions::city_range_attack as district, Rules};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};
pub mod capture;
pub mod resolve;
pub(crate) mod transition;
pub mod weariness;

#[derive(Clone, Debug)]
pub struct ReportScope {
    binding: c6_core::core_player::initialization::WireBinding,
    witness: c6_core::loaded_state::ObservedSnapshotWitness,
}
impl ReportScope {
    /// The caller qualifies the actual DistrictDamageChanged, combat status
    /// and XP report routes for this same observed state origin.
    pub fn from_qualified_origin(state: &LoadedState) -> Option<Self> {
        Some(Self {
            binding: state.binding().clone(),
            witness: state.observed_snapshot()?.clone(),
        })
    }
    pub fn matches(&self, state: &LoadedState) -> bool {
        state.binding() == &self.binding && state.observed_snapshot() == Some(&self.witness)
    }
}
#[derive(Clone, Debug)]
pub enum Publication {
    DistrictDamageChanged {
        district: ComponentId,
        defense_type: u32,
        current: i32,
        previous: i32,
    },
    Statuses {
        attacker: UnitReference,
        defender: ComponentId,
        attacker_damage: i32,
        defender_damage: i32,
        status: u32,
    },
}

#[derive(Debug)]
pub struct Error {
    pub stage: &'static str,
    pub detail: String,
}
pub(crate) fn error(stage: &'static str, detail: impl std::fmt::Debug) -> Error {
    Error {
        stage,
        detail: format!("{detail:?}"),
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "city melee {}: {}", self.stage, self.detail)
    }
}
impl std::error::Error for Error {}
pub struct Definitions {
    pub district: district::Definitions,
    pub strength: district_strength::Definitions,
    pub weariness: weariness::Definitions,
    promotion_classes: Vec<(u32, u32)>,
    outer_melee_percent: i32,
    pub(crate) outer_ranged_percent: i32,
    pub(crate) ranged_penalty: i32,
    pub(crate) outer_bombard_percent: i32,
    outer_enabled_percent: i32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let units = rules
            .gameplay()
            .table("Units")
            .ok_or_else(|| error("rules", "Units"))?;
        let promotion_classes = units
            .rows()
            .map(|r| {
                let hash = crate::registry::linux_type_hash(
                    r.text("UnitType")
                        .map_err(|e| error("rules", e))?
                        .as_bytes(),
                );
                let class = match r.get("PromotionClass").map_err(|e| error("rules", e))? {
                    crate::SqlValue::Null => u32::MAX,
                    crate::SqlValue::Text(name) => crate::registry::linux_type_hash(name),
                    _ => return Err(error("rules", "PromotionClass")),
                };
                Ok((hash, class))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            weariness: weariness::Definitions::from_rules(state, rules)?,
            district: district::Definitions::from_current_rules(state, rules)
                .map_err(|e| error("rules", e))?,
            strength: district_strength::Definitions::from_rules(state, rules)
                .map_err(|e| error("rules", e))?,
            promotion_classes,
            outer_ranged_percent: damage::integer(rules, "COMBAT_DEFENSE_DAMAGE_PERCENT_RANGED")
                .map_err(|e| error("rules", e))?,
            ranged_penalty: damage::integer(rules, "COMBAT_RANGED_VS_DISTRICT_STRENGTH_MODIFIER")
                .map_err(|e| error("rules", e))?,
            outer_bombard_percent: damage::integer(rules, "COMBAT_DEFENSE_DAMAGE_PERCENT_BOMBARD")
                .map_err(|e| error("rules", e))?,
            outer_melee_percent: damage::integer(rules, "COMBAT_DEFENSE_DAMAGE_PERCENT_MELEE")
                .map_err(|e| error("rules", e))?,
            outer_enabled_percent: damage::integer(rules, "COMBAT_DEFENSE_DAMAGE_PERCENT_BOMBARD")
                .map_err(|e| error("rules", e))?,
        })
    }
    pub(crate) fn promotion(&self, state: &LoadedState, unit: UnitReference) -> Result<u32, Error> {
        let unit = super::actor(state, unit).map_err(|e| error("promotion class", e))?;
        self.promotion_classes
            .iter()
            .find(|r| r.0 == unit.e_type_index)
            .map(|r| r.1)
            .ok_or_else(|| error("promotion class", "current Unit definition"))
    }
}

/// Real gameplay members for the Unit and District. An optional replacement
/// identity from capture must be retained separately from this precombat ID.
#[derive(Debug)]
#[must_use = "numeric city melee requires attacker completion and full ResolveCombat"]
pub struct Generated {
    pub attacker: UnitReference,
    pub defender: ComponentId,
    pub location: PlotCoord,
    pub attacker_strength_q8: i32,
    pub defender_strength_q8: i32,
    pub attacker_modifier_q8: i32,
    pub defender_modifier_q8: i32,
    pub defense_type: u32,
    pub attacker_damage: i32,
    pub defender_damage: i32,
    pub outer_damage: i32,
    pub attacker_final_damage: i32,
    pub defender_final_damage: i32,
    pub outer_final_damage: i32,
    pub attacker_maximum_damage: i32,
    pub defender_maximum_damage: i32,
    pub outer_maximum_damage: i32,
    pub attacker_experience: i32,
    pub attacker_advances: bool,
    pub game_sync_draws: Vec<damage::RandomDraw>,
}
pub(crate) fn adjacent_modifier(
    state: &LoadedState,
    at: (i32, i32),
    owner: i32,
    class: u32,
    bypass: bool,
) -> Result<bool, Error> {
    let map = state.world().map();
    for direction in 0..6 {
        let Some((x, y)) = district::adjacent_location(state, at, direction) else {
            continue;
        };
        let index = (y as u32)
            .wrapping_mul(map.units.width)
            .wrapping_add(x as u32);
        for layer in &map.units.layers {
            for &id in state
                .world()
                .plot_units(layer.layer_id, index as usize)
                .map_err(|e| error("adjacent units", e))?
            {
                let Some(unit) = state
                    .world()
                    .unit(id)
                    .map_err(|e| error("adjacent units", e))?
                else {
                    continue;
                };
                let unit = unit.record;
                if unit.b_delayed_death || unit.e_owner != owner {
                    continue;
                }
                let (all, classes) = if bypass {
                    (
                        unit.k_combat_modifers.b_bypass_walls,
                        &unit.ai_bypass_walls_promotion_class,
                    )
                } else {
                    (
                        unit.k_combat_modifers.b_enable_wall_attack,
                        &unit.ai_enable_wall_attack_promotion_class,
                    )
                };
                if all
                    || classes
                        .iter()
                        .any(|r| r.definition_hash == class && r.value != 0)
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// GenerateMeleeUnitVsDistrictCombatResults28d0110. Retaliation, inner and
/// outer damage calls run in that order, including the third no-wall draw.
#[allow(clippy::too_many_arguments)]
pub fn generate<B>(
    state: &mut LoadedState,
    backend: &mut B,
    definitions: &Definitions,
    rules: &super::execution::GenerateRules<'_>,
    experience: &super::experience::Definitions,
    attacker: UnitReference,
    defender: ComponentId,
) -> Result<Generated, Error>
where
    B: proposals::Effects
        + proposals::collection_modifiers::Services<Error = <B as proposals::Effects>::Error>,
    <B as proposals::Effects>::Error: std::fmt::Debug,
{
    let unit = super::actor(state, attacker).map_err(|e| error("actor", e))?;
    if rules
        .levels
        .iter()
        .find(|r| r.0 == unit.e_owner)
        .map(|r| r.1)
        != Some(0x253718b0)
    {
        return Err(error("scope", "major attacking owner required"));
    }
    if state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(|e| error("actor", e))?
        .domain
        != 2
        || crate::unit_movement_state::unit_is_embarked(unit)
    {
        return Err(error("scope", "ordinary unembarked land melee required"));
    }
    let d = state
        .district(i32::from(defender.owner), defender.id)
        .map_err(|e| error("defender", e))?;
    if d.district_hash != crate::registry::linux_type_hash(b"DISTRICT_CITY_CENTER") {
        return Err(error("scope", "city-center District required"));
    }
    let owner = district::district_owner(state, d).map_err(|e| error("defender", e))?;
    if !state
        .game_relations()
        .map_err(|e| error("relations", e))?
        .is_at_war(unit.e_owner, owner)
        .map_err(|e| error("relations", e))?
    {
        return Err(error("scope", "current enemy District required"));
    }
    let location = PlotCoord {
        x: d.location.0,
        y: d.location.1,
    };
    let outer_max = district::maximum_damage(state, d, &definitions.district, rules.levels, true)
        .map_err(|e| error("maximum", e))?;
    let defender_max =
        district::maximum_damage(state, d, &definitions.district, rules.levels, false)
            .map_err(|e| error("maximum", e))?;
    let initial_defender_damage = d.garrison_damage;
    let initial_outer_damage = d.outer_defenses_damage;
    if outer_max == 0 && initial_outer_damage != 0 {
        return Err(error("scope", "nonzero absent outer defenses damage"));
    }
    let owning = district::district_city(state, d).map_err(|e| error("defender", e))?;
    let prevent_bypass = owning.header.prevent_bypass_outer_defenses > 0;
    let prevent_outer = owning.header.prevent_melee_attack_outer_defenses > 0;
    if outer_max > 0 && (prevent_bypass || prevent_outer) {
        return Err(error("scope", "special wall prevention modifiers"));
    }
    let attacker_strength = rules
        .formation
        .combat(state, unit)
        .map_err(|e| error("base attack", e))?
        .wrapping_shl(8);
    let (defender_strength, base_modifier) = district_strength::base_defense(
        state,
        &definitions.strength,
        &definitions.district,
        rules.strength,
        rules.formation,
        rules.levels,
        defender,
    )
    .map_err(|e| error("base defense", e))?;
    let (id, _) = proposals::Context::new(state, backend)
        .propose_unit_district(attacker, defender, location)
        .map_err(|e| error("proposal", e))?;
    let attack_modifier = district_strength::attack_modifier(
        state,
        rules.strength,
        rules.formation,
        rules.levels,
        rules.players,
        attacker,
        defender,
        id,
    )
    .map_err(|e| error("attack modifier", e))?;
    let defense_modifier = base_modifier.wrapping_add(
        district_strength::defense_modifier(
            state,
            &definitions.district,
            rules.strength,
            rules.formation,
            rules.levels,
            attacker,
            defender,
            id,
        )
        .map_err(|e| error("defense modifier", e))?,
    );
    proposals::collection_modifiers::forget_combat(state, backend, id)
        .map_err(|e| error("proposal destruction", e))?;
    let attack = attacker_strength.wrapping_add(attack_modifier);
    let defense = defender_strength.wrapping_add(defense_modifier);
    let (mut attacker_damage, mut draws) =
        damage::district_unit_with_draws(state, rules.damage, attacker, defense, attack)
            .map_err(|e| error("retaliation", e))?;
    let class = definitions.promotion(state, attacker)?;
    let bypass = !prevent_bypass
        && adjacent_modifier(
            state,
            (location.x, location.y),
            attacker.player(),
            class,
            true,
        )?;
    let (mut defender_damage, inner_draws) = damage::unit_district_with_draws(
        state,
        rules.damage,
        attacker,
        attack,
        defense,
        defender_max,
        initial_outer_damage,
        outer_max,
        0x5e97d629,
        bypass,
    )
    .map_err(|e| error("garrison damage", e))?;
    draws.extend(inner_draws);
    let (mut outer_damage, outer_draws) = damage::unit_district_with_draws(
        state,
        rules.damage,
        attacker,
        attack,
        defense,
        defender_max,
        initial_outer_damage,
        outer_max,
        0x6da56a3d,
        bypass,
    )
    .map_err(|e| error("outer damage", e))?;
    draws.extend(outer_draws);
    if outer_damage > 0 {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == attacker.player())
            .ok_or_else(|| error("outer scaling", "attacker owner"))?;
        let enabled_class = player
            .units
            .enable_wall_attack_whole_game_promotion_class
            .iter()
            .find(|r| r.definition_hash == class)
            .map_or(0, |r| r.value)
            != 0;
        let religion = &player.religion;
        if religion.enable_wall_attack_whole_game_same_religion != 0
            || religion
                .enable_wall_attack_whole_game_same_religion_promotion_class
                .iter()
                .any(|r| r.definition_hash == class && r.value != 0)
        {
            return Err(error(
                "outer scaling",
                "same-religion wall attack continuation",
            ));
        }
        let enabled = enabled_class
            || (!prevent_outer
                && adjacent_modifier(
                    state,
                    (location.x, location.y),
                    attacker.player(),
                    class,
                    false,
                )?);
        let percentage = if enabled {
            definitions.outer_enabled_percent
        } else if prevent_outer {
            0
        } else {
            definitions.outer_melee_percent
        };
        outer_damage = percentage.wrapping_mul(outer_damage) / 100;
    }
    let unit = super::actor(state, attacker).map_err(|e| error("final damage", e))?;
    let attacker_max = rules
        .maximum_damage
        .maximum_damage(unit)
        .map_err(|e| error("maximum", e))?;
    let initial_attacker_damage = unit.i_damage;
    let mut attacker_final = initial_attacker_damage.wrapping_add(attacker_damage);
    let mut defender_final = initial_defender_damage.wrapping_add(defender_damage);
    if defender_final >= defender_max && attacker_final >= attacker_max {
        if defender_final.wrapping_sub(defender_max) < attacker_final.wrapping_sub(attacker_max) {
            defender_damage = defender_max
                .wrapping_sub(initial_defender_damage)
                .wrapping_sub(1);
            defender_final = initial_defender_damage.wrapping_add(defender_damage);
        } else {
            attacker_damage = attacker_max
                .wrapping_sub(initial_attacker_damage)
                .wrapping_sub(1);
            attacker_final = initial_attacker_damage.wrapping_add(attacker_damage);
        }
    }
    let attacker_experience = super::experience::unit_district_award(
        state,
        experience,
        rules.formation,
        rules.levels,
        attacker,
        owner,
        defender_final >= defender_max,
    )
    .map_err(|e| error("experience", e))?;
    let unit = super::actor(state, attacker).map_err(|e| error("advance", e))?;
    let melee = rules
        .formation
        .combat(state, unit)
        .map_err(|e| error("advance", e))?;
    let advances = defender_final >= defender_max
        && melee
            >= rules
                .formation
                .ranged(state, unit)
                .map_err(|e| error("advance", e))?
        && melee
            >= rules
                .formation
                .bombard(state, unit)
                .map_err(|e| error("advance", e))?;
    if rules
        .damage
        .retreat_when_killed_flag(state, attacker, attacker_final, attacker_max)
        .map_err(|e| error("retreat", e))?
    {
        return Err(error("retreat", "attacker retreat continuation"));
    }
    Ok(Generated {
        attacker,
        defender,
        location,
        attacker_strength_q8: attacker_strength,
        defender_strength_q8: defender_strength,
        attacker_modifier_q8: attack_modifier,
        defender_modifier_q8: defense_modifier,
        defense_type: if initial_outer_damage == outer_max || bypass {
            0x5e97d629
        } else {
            0x6da56a3d
        },
        attacker_damage,
        defender_damage,
        outer_damage,
        attacker_final_damage: attacker_final,
        defender_final_damage: defender_final,
        outer_final_damage: initial_outer_damage.wrapping_add(outer_damage),
        attacker_maximum_damage: attacker_max,
        defender_maximum_damage: defender_max,
        outer_maximum_damage: outer_max,
        attacker_experience,
        attacker_advances: advances,
        game_sync_draws: draws,
    })
}
