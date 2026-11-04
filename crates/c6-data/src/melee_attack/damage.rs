//! Admitted current combat rules for Unit/Unit damage-parameter assembly.
//! Returned continuations do not consume RNG or apply combat damage.
use crate::{
    resource_binding::ResourceAuthority, unit_initialization_definitions, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::InputIdentities, loaded_state::LoadedState,
    melee_attack::damage as kernel,
};

#[derive(Debug)]
pub enum Error {
    Binding,
    Definitions(unit_initialization_definitions::Error),
    Rule(&'static str),
}
pub struct Definitions {
    identities: InputIdentities,
    maximum_hit_points: i32,
    minimum_damage: i32,
    power_scaling: f32,
    base_damage: i32,
    power_dampener: i32,
    minimum_civilian_damage_percent: i32,
    max_extra_damage: i32,
}
fn value<'a>(rules: &'a Rules, name: &'static str) -> Result<&'a SqlValue, Error> {
    let table = rules
        .gameplay()
        .table("GlobalParameters")
        .ok_or(Error::Rule("GlobalParameters"))?;
    let mut rows = table.rows().filter(|r| r.text("Name").ok() == Some(name));
    let row = rows.next().ok_or(Error::Rule(name))?;
    if rows.next().is_some() {
        return Err(Error::Rule("duplicate combat global parameter"));
    }
    row.get("Value").map_err(|_| Error::Rule(name))
}
fn integer(rules: &Rules, name: &'static str) -> Result<i32, Error> {
    match value(rules, name)? {
        SqlValue::Integer(n) => Ok(*n as i32),
        SqlValue::Text(bytes) => std::str::from_utf8(bytes)
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or(Error::Rule(name)),
        _ => Err(Error::Rule(name)),
    }
}
fn scaling(rules: &Rules) -> Result<f32, Error> {
    let name = "COMBAT_POWER_SCALING";
    let value = match value(rules, name)? {
        SqlValue::Integer(n) => *n as f32,
        SqlValue::RealBits(bits) => f64::from_bits(*bits) as f32,
        SqlValue::Text(bytes) => std::str::from_utf8(bytes)
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or(Error::Rule(name))?,
        _ => return Err(Error::Rule(name)),
    };
    if !value.is_finite() {
        return Err(Error::Rule(name));
    }
    Ok(value)
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        resources: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if resources.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, resources)
            .map_err(Error::Definitions)?;
        Ok(Self {
            identities: state.binding().inputs.clone(),
            maximum_hit_points: integer(rules, "COMBAT_MAX_HIT_POINTS")?,
            minimum_damage: integer(rules, "COMBAT_MINIMUM_DAMAGE")?,
            power_scaling: scaling(rules)?,
            base_damage: integer(rules, "COMBAT_BASE_DAMAGE")?,
            power_dampener: integer(rules, "COMBAT_POWER_DAMPENER")?,
            minimum_civilian_damage_percent: integer(rules, "COMBAT_MIN_CIVILIAN_DAMAGE_PERCENT")?,
            max_extra_damage: integer(rules, "COMBAT_MAX_EXTRA_DAMAGE")?,
        })
    }
    pub fn max_extra_damage(&self) -> i32 {
        self.max_extra_damage
    }
}
struct Context<'a>(&'a Definitions);
impl kernel::Context for Context<'_> {
    type Error = Error;
    fn maximum_hit_points(&mut self) -> Result<i32, Error> {
        Ok(self.0.maximum_hit_points)
    }
    fn minimum_damage(&mut self) -> Result<i32, Error> {
        Ok(self.0.minimum_damage)
    }
    fn power_scaling(&mut self) -> Result<f32, Error> {
        Ok(self.0.power_scaling)
    }
    fn base_damage(&mut self) -> Result<i32, Error> {
        Ok(self.0.base_damage)
    }
    fn power_dampener(&mut self) -> Result<i32, Error> {
        Ok(self.0.power_dampener)
    }
    fn minimum_civilian_damage_percent(&mut self) -> Result<i32, Error> {
        Ok(self.0.minimum_civilian_damage_percent)
    }
}
pub fn prepare(
    state: &LoadedState,
    definitions: &Definitions,
    dealing: kernel::Strength,
    receiving: kernel::Strength,
    average: bool,
) -> Result<kernel::Next, Error> {
    if state.binding().inputs != definitions.identities {
        return Err(Error::Binding);
    }
    kernel::prepare(&mut Context(definitions), dealing, receiving, average)
}
