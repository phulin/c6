//! Current canonical damage/fuel contributions to Linux melee strength.
//! Situational bonuses and proposal modifier reads remain mandatory next stages.
use crate::{
    game_parameters::{IntegerParameter, Parameters},
    Rules, SqlValue,
};
use c6_core::{
    loaded_state::LoadedState, map::UnitReference, melee_attack::strength as kernel, unit::Unit,
};
#[derive(Debug)]
pub enum Error {
    Actor(super::Error),
    Parameters(crate::game_parameters::Error),
    Missing(&'static str),
    OwnerLevel(i32),
}
pub struct Definitions {
    max_hit_points: i32,
    wounded_multiplier: i32,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let parameters = Parameters::from_rules(rules).map_err(Error::Parameters)?;
        let max_hit_points = parameters
            .integer(IntegerParameter::CombatMaxHitPoints)
            .map_err(Error::Parameters)?;
        let rows = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let mut values = rows
            .rows()
            .filter(|r| r.text("Name").ok() == Some("COMBAT_WOUNDED_DAMAGE_MULTIPLIER"));
        let row = values
            .next()
            .ok_or(Error::Missing("COMBAT_WOUNDED_DAMAGE_MULTIPLIER"))?;
        if values.next().is_some() {
            return Err(Error::Missing("duplicate COMBAT_WOUNDED_DAMAGE_MULTIPLIER"));
        }
        let wounded_multiplier = match row
            .get("Value")
            .map_err(|_| Error::Missing("wounded multiplier Value"))?
        {
            SqlValue::Integer(n) => *n as i32,
            SqlValue::Text(s) => std::str::from_utf8(s)
                .map_err(|_| Error::Missing("wounded multiplier UTF8"))?
                .parse::<i32>()
                .map_err(|_| Error::Missing("wounded multiplier integer"))?,
            _ => return Err(Error::Missing("wounded multiplier integer")),
        };
        Ok(Self {
            max_hit_points,
            wounded_multiplier,
        })
    }
}
struct Current<'a> {
    unit: &'a Unit,
    definitions: &'a Definitions,
    levels: &'a [(i32, u32)],
}
impl kernel::Context for Current<'_> {
    type Error = Error;
    fn no_damage_reduction(&mut self) -> Result<bool, Error> {
        Ok(self.unit.k_combat_modifers.b_no_reduction_for_damage)
    }
    fn damage(&mut self) -> Result<i32, Error> {
        Ok(self.unit.i_damage)
    }
    fn max_hit_points(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.max_hit_points)
    }
    fn damage_reduction_modifier(&mut self) -> Result<i32, Error> {
        Ok(self
            .unit
            .k_combat_modifers
            .i_strength_reduction_for_damage_modifier)
    }
    fn wounded_multiplier(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.wounded_multiplier)
    }
    fn owner_is_major(&mut self) -> Result<bool, Error> {
        let mut found = self
            .levels
            .iter()
            .filter(|(owner, _)| *owner == self.unit.e_owner);
        let level = found.next().ok_or(Error::OwnerLevel(self.unit.e_owner))?.1;
        if found.next().is_some() {
            return Err(Error::OwnerLevel(self.unit.e_owner));
        }
        Ok(level == 0x253718b0)
    }
    fn fuel_reduction(&mut self) -> Result<i32, Error> {
        Ok(self
            .unit
            .k_combat_modifers
            .i_strength_reduction_insufficient_fuel)
    }
}
/// Reads the current World Unit, never the retained initial actor. This is only
/// the opening contribution; callers must not use it as a complete modifier.
pub fn opening_terms(
    state: &LoadedState,
    definitions: &Definitions,
    levels: &[(i32, u32)],
    unit: UnitReference,
    side: kernel::Side,
) -> Result<i32, kernel::Error<Error>> {
    let unit = super::actor(state, unit).map_err(|e| kernel::Error::Query(Error::Actor(e)))?;
    kernel::opening_terms(
        &mut Current {
            unit,
            definitions,
            levels,
        },
        side,
    )
}
