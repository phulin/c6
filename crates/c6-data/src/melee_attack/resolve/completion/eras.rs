//! Exact ordinary Eras combat-credit guards. Nonzero reached credits require
//! ChangeEraScore rather than substituting a major-player shortcut.
use super::super::GeneratedMembers;
use crate::unit_placement_state;
use c6_core::{loaded_state::LoadedState, map::UnitReference};
#[derive(Debug)]
pub enum Error {
    Actor(crate::melee_attack::Error),
    State(c6_core::loaded_state::Error),
    Eras(c6_core::loaded_state::game_eras::AccessError),
    Maximum(crate::formation_movement_cost::Error),
    RequiresEraScore {
        owner: i32,
        amount: i32,
        reason: i32,
    },
    GiantDeathRobotDefinition,
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
pub fn on_combat(
    state: &LoadedState,
    placement: &unit_placement_state::Definitions<'_, '_>,
    barbarian_owner: i32,
    members: &GeneratedMembers,
) -> Result<(), Error> {
    for (victim, winner) in [
        (members.damage.attacker, members.damage.defender),
        (members.damage.defender, members.damage.attacker),
    ] {
        if !dead(state, placement, victim)? {
            continue;
        }
        let victim = crate::melee_attack::actor(state, victim).map_err(Error::Actor)?;
        if victim.e_owner == barbarian_owner {
            continue;
        }
        let owner = crate::melee_attack::actor(state, winner)
            .map_err(Error::Actor)?
            .e_owner;
        let Some(bonus) = state.game_eras_combat_bonuses(owner).map_err(Error::Eras)? else {
            continue;
        };
        if bonus.defeated_corps != 0 && victim.e_military_formation == 1 {
            return Err(Error::RequiresEraScore {
                owner,
                amount: bonus.defeated_corps,
                reason: 5,
            });
        }
        if bonus.defeated_army != 0 && victim.e_military_formation == 2 {
            return Err(Error::RequiresEraScore {
                owner,
                amount: bonus.defeated_army,
                reason: 6,
            });
        }
        if bonus.defeated_naval != 0
            && state
                .unit_runtime(victim.e_owner, victim.id)
                .map_err(Error::State)?
                .domain
                == 0
        {
            return Err(Error::RequiresEraScore {
                owner,
                amount: bonus.defeated_naval,
                reason: 10,
            });
        }
        if bonus.defeated_giant_death_robot != 0 {
            return Err(Error::GiantDeathRobotDefinition);
        }
    }
    Ok(())
}
