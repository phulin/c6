//! Current combat Boost.Manager consumers. Filters read canonical participants;
//! positive continuations use the existing complete research manager path.
use super::super::GeneratedMembers;
use crate::{
    formation_movement_cost,
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    research,
    resource_binding::ResourceAuthority,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, map::UnitReference,
    relic_creation::boost as kernel,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Check {
    KillWith,
    KillSpecificUnit,
    NumberBarbarianKills,
}
#[derive(Clone, Copy, Debug)]
pub struct Callback {
    pub check: Check,
    pub boost_row_id: i32,
}
struct Definition {
    row_id: i32,
    boost_index: usize,
    check: Option<Check>,
    unit: Option<i32>,
    count: i32,
}
pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    replacements: Vec<(i32, i32)>,
    boosts: Vec<Definition>,
}
#[derive(Debug)]
pub enum DefinitionError {
    Binding,
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    Missing(&'static str),
}
impl std::fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "combat boost definitions: {self:?}")
    }
}
impl std::error::Error for DefinitionError {}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, DefinitionError> {
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(DefinitionError::Definitions)?;
        if state.binding() != bound.destination_binding() {
            return Err(DefinitionError::Binding);
        }
        let units = crate::unit_initialization_definitions::collection(rules, "Units", "UnitType")
            .map_err(DefinitionError::Definitions)?;
        let mut replacements = Vec::new();
        for row in rules
            .gameplay()
            .table("UnitReplaces")
            .ok_or(DefinitionError::Missing("UnitReplaces"))?
            .rows()
        {
            let index = |column| -> Result<Option<i32>, DefinitionError> {
                Ok(units
                    .find_index(linux_type_hash(
                        row.text(column).map_err(DefinitionError::Rules)?.as_bytes(),
                    ))
                    .map(|i| i as i32))
            };
            if let (Some(replacement), Some(base)) =
                (index("CivUniqueUnitType")?, index("ReplacesUnitType")?)
            {
                replacements.push((base, replacement));
            }
        }
        let mut boosts = Vec::new();
        for (boost_index, row) in rules
            .gameplay()
            .table("Boosts")
            .ok_or(DefinitionError::Missing("Boosts"))?
            .rows()
            .enumerate()
        {
            let check = match row.text("BoostClass").map_err(DefinitionError::Rules)? {
                "BOOST_TRIGGER_KILL_WITH" => Some(Check::KillWith),
                "BOOST_TRIGGER_KILL_SPECIFIC_UNIT" => Some(Check::KillSpecificUnit),
                "BOOST_TRIGGER_NUM_BARBS_KILLED" => Some(Check::NumberBarbarianKills),
                _ => None,
            };
            let unit = match row.get("Unit1Type").map_err(DefinitionError::Rules)? {
                SqlValue::Null => None,
                SqlValue::Text(name) => units.find_index(linux_type_hash(name)).map(|i| i as i32),
                _ => return Err(DefinitionError::Missing("Unit1Type")),
            };
            let int = |column| -> Result<i32, DefinitionError> {
                row.integer(column)
                    .map_err(DefinitionError::Rules)?
                    .try_into()
                    .map_err(|_| DefinitionError::Missing(column))
            };
            boosts.push(Definition {
                row_id: int("BoostID")?,
                boost_index,
                check,
                unit,
                count: int("NumItems")?,
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            units,
            replacements,
            boosts,
        })
    }
    fn same_or_replacement(&self, a: i32, b: i32) -> bool {
        same_or_replacement(&self.replacements, a, b)
    }
}
fn same_or_replacement(pairs: &[(i32, i32)], a: i32, b: i32) -> bool {
    a == b
        || pairs.iter().any(|&(base, replacement)| {
            (base == a && replacement == b) || (base == b && replacement == a)
        })
}
pub struct Inputs<'a, 'definitions> {
    pub maximum_damage: &'a formation_movement_cost::Definitions<'definitions>,
    pub civilization_levels: &'a [(i32, u32)],
}
/// A mandatory positive continuation. A missing research runtime must return an
/// error; filtering a callback never constructs/reimports a second owner.
pub trait Trigger {
    type Error;
    fn trigger(
        &mut self,
        state: &mut LoadedState,
        player: i32,
        boost_index: usize,
    ) -> Result<(), Self::Error>;
}
/// Borrow the caller's persistent research/Script/notification backend.
pub struct ResearchTrigger<'a, E> {
    pub definitions: &'a research::Definitions,
    pub effects: &'a mut E,
}
impl<E: research::Effects> Trigger for ResearchTrigger<'_, E> {
    type Error = kernel::Error<research::Error<E::Error>>;
    fn trigger(
        &mut self,
        state: &mut LoadedState,
        player: i32,
        boost_index: usize,
    ) -> Result<(), Self::Error> {
        kernel::trigger_boost(
            &mut research::Context {
                state,
                definitions: self.definitions,
                effects: self.effects,
            },
            Some(player),
            boost_index,
        )
    }
}
#[derive(Debug)]
pub enum Error<E> {
    Binding,
    Definition,
    Actor(crate::melee_attack::Error),
    Maximum(formation_movement_cost::Error),
    MissingLevel(i32),
    State(c6_core::loaded_state::Error),
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "combat boosts: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
fn unit(
    state: &LoadedState,
    id: UnitReference,
) -> Result<&c6_core::unit::Unit, crate::melee_attack::Error> {
    crate::melee_attack::actor(state, id)
}
fn dead<E>(
    state: &LoadedState,
    inputs: &Inputs<'_, '_>,
    id: UnitReference,
) -> Result<bool, Error<E>> {
    let u = unit(state, id).map_err(Error::Actor)?;
    Ok(u.i_damage
        >= inputs
            .maximum_damage
            .maximum_damage(u)
            .map_err(Error::Maximum)?)
}
fn barbarian<E>(
    state: &LoadedState,
    inputs: &Inputs<'_, '_>,
    id: UnitReference,
) -> Result<bool, Error<E>> {
    let owner = unit(state, id).map_err(Error::Actor)?.e_owner;
    inputs
        .civilization_levels
        .iter()
        .find(|r| r.0 == owner)
        .map(|r| r.1 == 0xd959ec24)
        .ok_or(Error::MissingLevel(owner))
}
fn matches<E>(
    state: &LoadedState,
    defs: &Definitions,
    id: UnitReference,
    target: i32,
) -> Result<bool, Error<E>> {
    let kind = defs
        .units
        .find_index(unit(state, id).map_err(Error::Actor)?.e_type_index)
        .map_or(-1, |i| i as i32);
    Ok(defs.same_or_replacement(kind, target))
}
/// Invoke per actual signal listener, in caller-preserved order. The ordinary
/// GeneratedMembers domain is unit/unit; district combat uses a separate path.
pub fn on_combat<T: Trigger>(
    state: &mut LoadedState,
    effects: &mut T,
    defs: &Definitions,
    inputs: Inputs<'_, '_>,
    callback: Callback,
    members: &GeneratedMembers,
) -> Result<(), Error<T::Error>> {
    if state.binding() != &defs.binding {
        return Err(Error::Binding);
    }
    let definition = defs
        .boosts
        .iter()
        .find(|r| r.row_id == callback.boost_row_id && r.check == Some(callback.check))
        .ok_or(Error::Definition)?;
    let attacker = members.damage.attacker;
    let defender = members.damage.defender;
    match callback.check {
        Check::KillSpecificUnit => {
            let Some(target) = definition.unit else {
                return Ok(());
            };
            let owner = unit(state, attacker).map_err(Error::Actor)?.e_owner;
            if dead(state, &inputs, defender)? && matches(state, defs, defender, target)? {
                effects
                    .trigger(state, owner, definition.boost_index)
                    .map_err(Error::Effect)?;
            }
        }
        Check::KillWith => {
            let Some(target) = definition.unit else {
                return Ok(());
            };
            if matches(state, defs, attacker, target)? {
                let owner = unit(state, attacker).map_err(Error::Actor)?.e_owner;
                if dead(state, &inputs, defender)? {
                    return effects
                        .trigger(state, owner, definition.boost_index)
                        .map_err(Error::Effect);
                }
            }
            if matches(state, defs, defender, target)? {
                let owner = unit(state, defender).map_err(Error::Actor)?.e_owner;
                if dead(state, &inputs, attacker)? {
                    effects
                        .trigger(state, owner, definition.boost_index)
                        .map_err(Error::Effect)?;
                }
            }
        }
        Check::NumberBarbarianKills => {
            if dead(state, &inputs, attacker)? && barbarian(state, &inputs, attacker)? {
                let owner = unit(state, defender).map_err(Error::Actor)?.e_owner;
                if state
                    .change_barbarians_killed(owner, 1)
                    .map_err(Error::State)?
                    >= definition.count
                {
                    effects
                        .trigger(state, owner, definition.boost_index)
                        .map_err(Error::Effect)?;
                }
            }
            // Native rereads CURRENT defender death after the first trigger callbacks.
            if dead(state, &inputs, defender)? && barbarian(state, &inputs, defender)? {
                let owner = unit(state, attacker).map_err(Error::Actor)?.e_owner;
                if state
                    .change_barbarians_killed(owner, 1)
                    .map_err(Error::State)?
                    >= definition.count
                {
                    effects
                        .trigger(state, owner, definition.boost_index)
                        .map_err(Error::Effect)?;
                }
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_is_symmetric_direct_but_not_transitive_or_sibling_equivalence() {
        let pairs = [(1, 2), (1, 3), (2, 4)];
        assert!(same_or_replacement(&pairs, 1, 1));
        assert!(same_or_replacement(&pairs, 1, 2));
        assert!(same_or_replacement(&pairs, 2, 1));
        assert!(!same_or_replacement(&pairs, 2, 3));
        assert!(!same_or_replacement(&pairs, 1, 4));
    }
}
