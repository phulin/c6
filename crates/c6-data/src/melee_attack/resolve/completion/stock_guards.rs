//! Current stock guards for ordinary Unit/Unit combat subscribers.
//! Positive Emergency and unsupported tribe death branches remain explicit.
use super::super::GeneratedMembers;
use crate::{melee_attack::actor, unit_placement_state};
use c6_core::{loaded_state::LoadedState, map::UnitReference};

#[derive(Debug)]
pub enum Error {
    Actor(crate::melee_attack::Error),
    Maximum(crate::formation_movement_cost::Error),
    EmergencyKilledEnemy,
    BarbarianTribeUnitDied { tribe: i32, victim: UnitReference },
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
}

pub struct Definitions {
    scout_lost: i32,
}
impl Definitions {
    pub fn from_rules(rules: &crate::Rules) -> Result<Self, Error> {
        let table = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let mut rows = table
            .rows()
            .filter(|r| r.text("Name").ok() == Some("BARBARIAN_BOLDNESS_PER_SCOUT_LOST"));
        let row = rows
            .next()
            .ok_or(Error::Missing("BARBARIAN_BOLDNESS_PER_SCOUT_LOST"))?;
        if rows.next().is_some() {
            return Err(Error::Missing("duplicate scout lost parameter"));
        }
        let value = row
            .get("Value")
            .map_err(|_| Error::Missing("scout lost value"))?;
        let scout_lost = match value {
            crate::SqlValue::Integer(n) => i32::try_from(*n).ok(),
            crate::SqlValue::Text(b) => std::str::from_utf8(b).ok().and_then(|s| s.parse().ok()),
            _ => None,
        }
        .ok_or(Error::Missing("scout lost integer"))?;
        Ok(Self { scout_lost })
    }
}

/// ScoreOnAttackedEnemyCity first requires a District defender. This typed
/// generator has two Unit members and cannot satisfy that interface test.
pub fn emergency_attacked_city(_members: &GeneratedMembers) {}

/// An explicit current empty Emergency manager, retained by the callback owner.
/// Positive instance creation/scoring remains unsupported.
pub struct EmptyEmergencyScoring;

/// ScoreOnKilledEnemy uses the defender's delayed-death byte, not IsDead.
/// ChangePlayerScore first iterates the current Emergency list. An explicitly
/// empty manager returns before any score mutation, subscriber or RNG draw.
pub fn emergency_killed_enemy(
    state: &LoadedState,
    members: &GeneratedMembers,
    emergency_scoring: Option<&EmptyEmergencyScoring>,
) -> Result<(), Error> {
    if actor(state, members.damage.defender)
        .map_err(Error::Actor)?
        .b_delayed_death
        && emergency_scoring.is_none()
    {
        return Err(Error::EmergencyKilledEnemy);
    }
    Ok(())
}

/// FreeCities.OnCombatOccurred additionally requires a District member and
/// its City. A Unit/Unit result has no District even when the owner matches.
pub fn free_cities_unit_combat(_members: &GeneratedMembers) {}

fn dead(
    state: &LoadedState,
    definitions: &unit_placement_state::Definitions<'_, '_>,
    reference: UnitReference,
) -> Result<bool, Error> {
    let unit = actor(state, reference).map_err(Error::Actor)?;
    Ok(unit.i_damage
        >= definitions
            .movement
            .maximum_damage(unit)
            .map_err(Error::Maximum)?)
}

fn tribe_and_victim(
    attacker_owner: i32,
    defender_owner: i32,
    barbarian_owner: i32,
    defender_dead: bool,
    attacker_dead: impl FnOnce() -> bool,
) -> Option<(bool, bool)> {
    if attacker_owner != barbarian_owner && defender_owner != barbarian_owner {
        return None;
    }
    if defender_dead {
        // (tribe belongs to attacker, victim is attacker)
        Some((attacker_owner == barbarian_owner, false))
    } else if attacker_dead() {
        Some((attacker_owner == barbarian_owner, true))
    } else {
        None
    }
}

/// Manager.OnCombatOccurred checks defender death first and uses the current
/// barbarian participant's tribe index. Original-created unassigned Units have
/// index -1; a real tribe requires its OnUnitDied continuation.
pub fn barbarian(
    state: &mut LoadedState,
    definitions: &unit_placement_state::Definitions<'_, '_>,
    barbarian_owner: i32,
    parameters: &Definitions,
    members: &GeneratedMembers,
) -> Result<(), Error> {
    let a = members.damage.attacker;
    let d = members.damage.defender;
    let ao = actor(state, a).map_err(Error::Actor)?.e_owner;
    let do_ = actor(state, d).map_err(Error::Actor)?.e_owner;
    if ao != barbarian_owner && do_ != barbarian_owner {
        return Ok(());
    }
    let defender_dead = dead(state, definitions, d)?;
    let attacker_dead = if defender_dead {
        false
    } else {
        dead(state, definitions, a)?
    };
    let Some((attacker_tribe, attacker_victim)) =
        tribe_and_victim(ao, do_, barbarian_owner, defender_dead, || attacker_dead)
    else {
        return Ok(());
    };
    let tribe_unit = if attacker_tribe { a } else { d };
    let tribe = actor(state, tribe_unit)
        .map_err(Error::Actor)?
        .i_barbarian_tribe_index;
    if tribe != -1 {
        let victim = if attacker_victim { a } else { d };
        let unit = actor(state, victim).map_err(Error::Actor)?;
        if unit.e_owner != barbarian_owner || unit.e_ai_type != 2 {
            return Err(Error::BarbarianTribeUnitDied { tribe, victim });
        }
        let instance = state
            .barbarian_manager_mut()
            .map_err(Error::State)?
            .instances
            .iter_mut()
            .find(|t| t.tribe_index == tribe)
            .ok_or(Error::Missing("combat tribe instance"))?;
        instance.cur_num_scouts = instance.cur_num_scouts.wrapping_sub(1);
        instance.turns_since_scout_death = 0;
        instance.boldness = instance.boldness.wrapping_add(parameters.scout_lost);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn barbarian_death_prefers_defender_and_selects_barbarian_tribe() {
        assert_eq!(
            tribe_and_victim(0, 63, 63, true, || panic!()),
            Some((false, false))
        );
        assert_eq!(
            tribe_and_victim(0, 63, 63, false, || true),
            Some((false, true))
        );
        assert_eq!(
            tribe_and_victim(63, 0, 63, false, || true),
            Some((true, true))
        );
        assert_eq!(tribe_and_victim(0, 1, 63, true, || panic!()), None);
    }
}
