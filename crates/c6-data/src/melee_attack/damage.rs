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
    World(c6_core::world::MissingInput),
    MissingUnit(c6_core::map::UnitReference),
    State(c6_core::loaded_state::Error),
    RetreatWhenKilled(c6_core::map::UnitReference),
    GameSync(c6_core::loaded_state::game_sync::AccessError),
    Math(c6_core::combat::DamageError),
    NativeArithmeticFault {
        draw: u16,
        fault: c6_core::combat::DamageError,
    },
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
    retreat_unit_types: Vec<u32>,
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
pub(super) fn integer(rules: &Rules, name: &'static str) -> Result<i32, Error> {
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
            retreat_unit_types: rules
                .gameplay()
                .table("UnitRetreats_XP1")
                .ok_or(Error::Rule("UnitRetreats_XP1"))?
                .rows()
                .map(|row| {
                    row.text("UnitType")
                        .map(|name| crate::registry::linux_type_hash(name.as_bytes()))
                        .map_err(|_| Error::Rule("UnitRetreats_XP1.UnitType"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
    /// Generic GenerateCombatResults' lethal-member pass. A matching retreat
    /// rule requires FindRetreatWhenKilledPlot (including its genuine draws)
    /// before flag/damage changes; it is never modeled as an ordinary death.
    pub fn retreat_when_killed_flag(
        &self,
        state: &LoadedState,
        reference: c6_core::map::UnitReference,
        final_damage: i32,
        maximum: i32,
    ) -> Result<bool, Error> {
        if final_damage < maximum {
            return Ok(false);
        }
        if state.binding().inputs != self.identities {
            return Err(Error::Binding);
        }
        let unit = state
            .world()
            .unit(reference)
            .map_err(Error::World)?
            .ok_or(Error::MissingUnit(reference))?
            .record;
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        if runtime.definition_index >= 0 && self.retreat_unit_types.contains(&unit.e_type_index) {
            return Err(Error::RetreatWhenKilled(reference));
        }
        Ok(false)
    }
    pub fn max_extra_damage(&self) -> i32 {
        self.max_extra_damage
    }
    pub fn maximum_hit_points(&self) -> i32 {
        self.maximum_hit_points
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
    defender: kernel::Strength,
    attacker: kernel::Strength,
    average: bool,
) -> Result<kernel::Next, Error> {
    if state.binding().inputs != definitions.identities {
        return Err(Error::Binding);
    }
    kernel::prepare(&mut Context(definitions), defender, attacker, average)
}
struct ExchangeContext<'a> {
    state: &'a mut LoadedState,
    definitions: &'a Definitions,
    defender: c6_core::map::UnitReference,
}
impl kernel::Context for ExchangeContext<'_> {
    type Error = Error;
    fn maximum_hit_points(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.maximum_hit_points)
    }
    fn minimum_damage(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.minimum_damage)
    }
    fn power_scaling(&mut self) -> Result<f32, Error> {
        Ok(self.definitions.power_scaling)
    }
    fn base_damage(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.base_damage)
    }
    fn power_dampener(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.power_dampener)
    }
    fn minimum_civilian_damage_percent(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.minimum_civilian_damage_percent)
    }
}
impl kernel::ExchangeContext for ExchangeContext<'_> {
    fn calculate_damage(
        &mut self,
        p: &c6_core::combat::DamageParameters,
        average: bool,
    ) -> Result<i32, Error> {
        if average {
            return c6_core::combat::calculate_average_damage(p, self.definitions.max_extra_damage)
                .map_err(Error::Math);
        }
        match self
            .state
            .calculate_game_sync_damage(p, self.definitions.max_extra_damage)
            .map_err(Error::GameSync)?
        {
            c6_core::combat_random::RandomDamageOutcome::Damage { amount, .. } => Ok(amount),
            c6_core::combat_random::RandomDamageOutcome::NativeArithmeticFaultAfterDraw {
                draw,
                fault,
            } => Err(Error::NativeArithmeticFault { draw, fault }),
        }
    }
    fn defender_embarked(&mut self) -> Result<bool, Error> {
        Ok(self
            .state
            .world()
            .unit(self.defender)
            .map_err(Error::World)?
            .ok_or(Error::MissingUnit(self.defender))?
            .record
            .b_embarked)
    }
    fn defender_can_fight_while_embarked(&mut self) -> Result<bool, Error> {
        Ok(self
            .state
            .world()
            .unit(self.defender)
            .map_err(Error::World)?
            .ok_or(Error::MissingUnit(self.defender))?
            .record
            .k_combat_modifers
            .b_fight_while_embarked)
    }
}
pub fn exchange(
    state: &mut LoadedState,
    definitions: &Definitions,
    defender: c6_core::map::UnitReference,
    attacker_strength: kernel::Strength,
    defender_strength: kernel::Strength,
    average: bool,
) -> Result<kernel::Exchange, Error> {
    if state.binding().inputs != definitions.identities {
        return Err(Error::Binding);
    }
    kernel::exchange(
        &mut ExchangeContext {
            state,
            definitions,
            defender,
        },
        attacker_strength,
        defender_strength,
        average,
    )
}
