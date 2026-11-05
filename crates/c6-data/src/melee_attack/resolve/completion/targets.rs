//! Stock AI.Targets combat bookkeeping over the retained current knowledge.
//! This does not select orders or invoke any native player policy.
use super::super::GeneratedMembers;
use crate::{
    registry::LinuxDefinitionRegistry, resource_binding::ResourceAuthority,
    unit_initialization_definitions::collection, unit_placement_state, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, map::UnitReference,
};
#[derive(Debug)]
pub enum Error {
    Binding,
    ArmyKnowledge(super::army_knowledge::Error),
    Missing(&'static str),
    Actor(crate::melee_attack::Error),
    State(c6_core::loaded_state::Error),
    District(c6_core::loaded_state::district_initialization::DistrictError),
    Knowledge(c6_core::loaded_state::game_statistics::AccessError),
    Maximum(crate::formation_movement_cost::Error),
}
pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    costs: Vec<i32>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(|_| Error::Binding)?;
        if authority.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        let units = collection(rules, "Units", "UnitType")
            .map_err(|_| Error::Missing("Unit definitions"))?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units table"))?;
        let costs = units
            .entries()
            .iter()
            .map(|entry| {
                let mut rows = table.rows().filter(|r| {
                    r.text("UnitType").ok().map(str::as_bytes) == Some(entry.type_name.as_slice())
                });
                let row = rows.next().ok_or(Error::Missing("Unit cost row"))?;
                if rows.next().is_some() {
                    return Err(Error::Missing("duplicate Unit cost"));
                }
                let value = row
                    .get("Cost")
                    .ok()
                    .and_then(|v| v.integer().ok())
                    .ok_or(Error::Missing("Unit cost"))?;
                i32::try_from(value).map_err(|_| Error::Missing("Unit cost range"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            units,
            costs,
        })
    }
    fn cost(&self, state: &LoadedState, unit: UnitReference) -> Result<i32, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Binding);
        }
        let unit = crate::melee_attack::actor(state, unit).map_err(Error::Actor)?;
        let index = self
            .units
            .find_index(unit.e_type_index)
            .ok_or(Error::Missing("current Unit definition"))?;
        if state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .definition_index
            != index as i32
        {
            return Err(Error::Binding);
        }
        self.costs
            .get(index)
            .copied()
            .ok_or(Error::Missing("current Unit cost"))
    }
}
fn death_value(cost: i32, level: i32) -> i32 {
    (cost >> 2)
        .wrapping_add((cost >> 4).wrapping_mul(level.wrapping_sub(1)))
        .wrapping_shl(8)
}
fn dead(
    state: &LoadedState,
    placement: &unit_placement_state::Definitions<'_, '_>,
    reference: UnitReference,
) -> Result<bool, Error> {
    let unit = crate::melee_attack::actor(state, reference).map_err(Error::Actor)?;
    Ok(unit.i_damage
        >= placement
            .movement
            .maximum_damage(unit)
            .map_err(Error::Maximum)?)
}
/// Source UnitKilled21ab1a0: only participant observers edit their existing
/// opponent-indexed WarKnowledge. Attacker death takes precedence over defender.
pub fn unit_killed(
    state: &mut LoadedState,
    defs: &Definitions,
    placement: &unit_placement_state::Definitions<'_, '_>,
    observer: i32,
    members: &GeneratedMembers,
) -> Result<(), Error> {
    let attacker = members.damage.attacker;
    let defender = members.damage.defender;
    let a = crate::melee_attack::actor(state, attacker)
        .map_err(Error::Actor)?
        .e_owner;
    let d = crate::melee_attack::actor(state, defender)
        .map_err(Error::Actor)?
        .e_owner;
    if a != observer && d != observer || a == -1 || d == -1 {
        return Ok(());
    }
    let victim = if dead(state, placement, attacker)? {
        attacker
    } else if dead(state, placement, defender)? {
        defender
    } else {
        return Ok(());
    };
    let victim_owner = crate::melee_attack::actor(state, victim)
        .map_err(Error::Actor)?
        .e_owner;
    let lost = victim_owner == observer;
    let opponent = if lost {
        if victim == attacker {
            d
        } else {
            a
        }
    } else {
        victim_owner
    };
    let cost = defs.cost(state, victim)?;
    let level = crate::melee_attack::actor(state, victim)
        .map_err(Error::Actor)?
        .k_experience
        .i_level;
    let value = death_value(cost, level);
    let current = state
        .combat_ai_targets_mut(observer)
        .map_err(Error::Knowledge)?;
    if current.player_id != observer || current.knowledge_base.owner != observer {
        return Err(Error::Binding);
    }
    let war = current
        .knowledge_base
        .war_knowledge
        .get_mut(usize::try_from(opponent).map_err(|_| Error::Missing("opponent index"))?)
        .ok_or(Error::Missing("current WarKnowledge index"))?;
    war.current_tally_q8 = if lost {
        war.current_tally_q8.wrapping_sub(value)
    } else {
        war.current_tally_q8.wrapping_add(value)
    };
    Ok(())
}
/// Unit/District branch of Targets::UnitKilled. Only the two participants
/// inspect their current CityKnowledge; an unknown city has no tally effect.
pub fn unit_district_combat(
    state: &mut LoadedState,
    defs: &Definitions,
    placement: &unit_placement_state::Definitions<'_, '_>,
    observer: i32,
    members: &crate::melee_attack::city::Generated,
) -> Result<(), Error> {
    if defs.binding != *state.binding() {
        return Err(Error::Binding);
    }
    let previous_owner = i32::from(members.defender.owner);
    if observer != members.attacker.player() && observer != previous_owner {
        return Ok(());
    }
    if dead(state, placement, members.attacker)? {
        return Err(Error::Missing("District combat attacker death bookkeeping"));
    }
    let city_id = state
        .district(previous_owner, members.defender.id)
        .map_err(Error::District)?
        .city_id;
    let current = state
        .combat_ai_targets_mut(observer)
        .map_err(Error::Knowledge)?;
    if current.player_id != observer || current.knowledge_base.owner != observer {
        return Err(Error::Binding);
    }
    if !current
        .knowledge_base
        .known_cities
        .iter()
        .any(|city| city.owner == previous_owner && city.id == city_id)
    {
        return Ok(());
    }
    Err(Error::Missing(
        "known CityValue and CityAttacked continuation",
    ))
}
/// Player.Signals+78 observes the current attacking Unit in the same canonical
/// ArmyKnowledge used by creation and visibility. Removed attacker locations
/// take CivKnowledge.UpdateUnit's source guard in the shared adapter.
pub fn observer_attacked(
    state: &mut LoadedState,
    defs: &Definitions,
    observer: i32,
    attacker: UnitReference,
) -> Result<(), Error> {
    if defs.binding != *state.binding() {
        return Err(Error::Binding);
    }
    let unit = crate::melee_attack::actor(state, attacker).map_err(Error::Actor)?;
    if !c6_core::geometry::plot_valid(unit.k_location.x, unit.k_location.y) {
        return Ok(());
    }
    super::army_knowledge::observe(state, &defs.units, observer, attacker)
        .map_err(Error::ArmyKnowledge)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn war_value_uses_arithmetic_shifts_before_wrapped_q8() {
        assert_eq!(death_value(40, 1), 2560);
        assert_eq!(death_value(40, 3), 3584);
        assert_eq!(death_value(-35, 2), -3072);
        assert_eq!(death_value(i32::MAX, i32::MAX), 256);
    }
}
