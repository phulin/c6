//! GlobalAI combat subscriber20fc870: real retained MayhemTracker state,
//! including separate combat/death binary32 additions. Logging reads static
//! names/turn/scalars only; its file formatting has no gameplay/RNG effect.
use super::super::GeneratedMembers;
use crate::{resource_binding::ResourceAuthority, unit_placement_state, Rules, SqlValue};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error {
    District(c6_core::loaded_state::district_initialization::DistrictError),
    DistrictDeathContinuation,
    Binding,
    Parameter(&'static str),
    Actor(crate::melee_attack::Error),
    Maximum(crate::formation_movement_cost::Error),
    State(c6_core::loaded_state::game_statistics::AccessError),
}
pub struct Definitions {
    binding: WireBinding,
    combat: f32,
    combat_barbarians: f32,
    death: f32,
    death_barbarians: f32,
    city_death: f32,
    city_death_barbarians: f32,
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
        let table = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Parameter("GlobalParameters"))?;
        let parameter = |name: &'static str| -> Result<f32, Error> {
            let mut rows = table
                .rows()
                .filter(|row| row.text("Name").ok() == Some(name));
            let row = rows.next().ok_or(Error::Parameter(name))?;
            if rows.next().is_some() {
                return Err(Error::Parameter(name));
            }
            let value = match row.get("Value").map_err(|_| Error::Parameter(name))? {
                SqlValue::Integer(n) => *n as f64,
                SqlValue::RealBits(bits) => f64::from_bits(*bits),
                SqlValue::Text(bytes) => std::str::from_utf8(bytes)
                    .map_err(|_| Error::Parameter(name))?
                    .parse::<f64>()
                    .map_err(|_| Error::Parameter(name))?,
                _ => return Err(Error::Parameter(name)),
            };
            Ok(value as f32)
        };
        Ok(Self {
            binding: state.binding().clone(),
            combat: parameter("MAYHEM_UNIT_COMBAT")?,
            combat_barbarians: parameter("MAYHEM_UNIT_COMBAT_WITH_BARBARIANS")?,
            death: parameter("MAYHEM_UNIT_DEATH")?,
            death_barbarians: parameter("MAYHEM_UNIT_DEATH_WITH_BARBARIANS")?,
            city_death: parameter("MAYHEM_CITY_DEATH")?,
            city_death_barbarians: parameter("MAYHEM_CITY_DEATH_WITH_BARBARIANS")?,
        })
    }
}
/// `barbarian_owner` is the current PlayerManager cached role ID, not an assumed
/// fixed slot or a captured predicate. Ordinary unit/unit Results only.
pub fn on_combat(
    state: &mut LoadedState,
    definitions: &Definitions,
    placement: &unit_placement_state::Definitions<'_, '_>,
    barbarian_owner: i32,
    members: &GeneratedMembers,
) -> Result<(), Error> {
    if definitions.binding != *state.binding() {
        return Err(Error::Binding);
    }
    let attacker =
        crate::melee_attack::actor(state, members.damage.attacker).map_err(Error::Actor)?;
    let attacker_owner = attacker.e_owner;
    let defender =
        crate::melee_attack::actor(state, members.damage.defender).map_err(Error::Actor)?;
    let defender_owner = defender.e_owner;
    let barbarians = attacker_owner == barbarian_owner || defender_owner == barbarian_owner;
    state
        .add_global_mayhem(if barbarians {
            definitions.combat_barbarians
        } else {
            definitions.combat
        })
        .map_err(Error::State)?;
    // LogCombatEvent checks defender death before attacker death. LogDeath
    // rereads both current Unit owners after the first contribution.
    let defender =
        crate::melee_attack::actor(state, members.damage.defender).map_err(Error::Actor)?;
    let dead_defender = defender.i_damage
        >= placement
            .movement
            .maximum_damage(defender)
            .map_err(Error::Maximum)?;
    let dead = if dead_defender {
        true
    } else {
        let attacker =
            crate::melee_attack::actor(state, members.damage.attacker).map_err(Error::Actor)?;
        attacker.i_damage
            >= placement
                .movement
                .maximum_damage(attacker)
                .map_err(Error::Maximum)?
    };
    if dead {
        let winner = if dead_defender {
            members.damage.attacker
        } else {
            members.damage.defender
        };
        let loser = if dead_defender {
            members.damage.defender
        } else {
            members.damage.attacker
        };
        let winner_owner = crate::melee_attack::actor(state, winner)
            .map_err(Error::Actor)?
            .e_owner;
        let barbarians = winner_owner == barbarian_owner
            || crate::melee_attack::actor(state, loser)
                .map_err(Error::Actor)?
                .e_owner
                == barbarian_owner;
        state
            .add_global_mayhem(if barbarians {
                definitions.death_barbarians
            } else {
                definitions.death
            })
            .map_err(Error::State)?;
    }
    Ok(())
}

/// Native Results overload uses the real District owner and checks Unit death
/// before District.HasMaxDamage. Death continuations remain explicit.
pub fn on_district_combat(
    state: &mut LoadedState,
    definitions: &Definitions,
    placement: &unit_placement_state::Definitions<'_, '_>,
    barbarian_owner: i32,
    members: &crate::city_strike::Generated,
) -> Result<(), Error> {
    if definitions.binding != *state.binding() {
        return Err(Error::Binding);
    }
    let defender = crate::melee_attack::actor(state, members.defender).map_err(Error::Actor)?;
    let barbarian =
        i32::from(members.attacker.owner) == barbarian_owner || defender.e_owner == barbarian_owner;
    state
        .add_global_mayhem(if barbarian {
            definitions.combat_barbarians
        } else {
            definitions.combat
        })
        .map_err(Error::State)?;
    let defender = crate::melee_attack::actor(state, members.defender).map_err(Error::Actor)?;
    if defender.i_damage
        >= placement
            .movement
            .maximum_damage(defender)
            .map_err(Error::Maximum)?
    {
        return Err(Error::DistrictDeathContinuation);
    }
    let district = state
        .district(i32::from(members.attacker.owner), members.attacker.id)
        .map_err(Error::District)?;
    if district.garrison_damage >= members.attacker_maximum_damage {
        return Err(Error::DistrictDeathContinuation);
    }
    Ok(())
}

/// Unit/District Results use the same real mayhem owner and current HP reads.
pub fn on_unit_district_combat(
    state: &mut LoadedState,
    definitions: &Definitions,
    placement: &unit_placement_state::Definitions<'_, '_>,
    barbarian_owner: i32,
    members: &crate::melee_attack::city::Generated,
    retired: Option<&crate::city_capture::retired_district::RetiredDistrictScalars>,
) -> Result<(), Error> {
    if definitions.binding != *state.binding() {
        return Err(Error::Binding);
    }
    let attacker = crate::melee_attack::actor(state, members.attacker).map_err(Error::Actor)?;
    let barbarians =
        attacker.e_owner == barbarian_owner || i32::from(members.defender.owner) == barbarian_owner;
    state
        .add_global_mayhem(if barbarians {
            definitions.combat_barbarians
        } else {
            definitions.combat
        })
        .map_err(Error::State)?;
    let attacker = crate::melee_attack::actor(state, members.attacker).map_err(Error::Actor)?;
    if attacker.i_damage
        >= placement
            .movement
            .maximum_damage(attacker)
            .map_err(Error::Maximum)?
    {
        return Err(Error::DistrictDeathContinuation);
    }
    let damage = match retired {
        Some(view) => view.garrison_damage,
        None => {
            state
                .district(i32::from(members.defender.owner), members.defender.id)
                .map_err(Error::District)?
                .garrison_damage
        }
    };
    if damage >= members.defender_maximum_damage {
        // LogDeath(Unit,District,Player) uses the city-center definition and
        // the attacking Unit's IsBarbarian branch; it consumes no RNG.
        state
            .add_global_mayhem(if attacker.e_owner == barbarian_owner {
                definitions.city_death_barbarians
            } else {
                definitions.city_death
            })
            .map_err(Error::State)?;
    }
    Ok(())
}
