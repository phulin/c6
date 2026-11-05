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
/// Actual canonical GameSync samples retained in source execution order.
/// These are observations of the consumed stream, never replay inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RandomDraw {
    pub bound: u16,
    pub value: u16,
    pub before: c6_core::rng::native_serialization::SerializedRandomRecord,
    pub after: c6_core::rng::native_serialization::SerializedRandomRecord,
}
struct ExchangeContext<'a> {
    state: &'a mut LoadedState,
    definitions: &'a Definitions,
    defender: c6_core::map::UnitReference,
    draws: &'a mut Vec<RandomDraw>,
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
        let before = *self.state.game_sync().map_err(Error::GameSync)?.record;
        let outcome = self
            .state
            .calculate_game_sync_damage(p, self.definitions.max_extra_damage)
            .map_err(Error::GameSync)?;
        let after = *self.state.game_sync().map_err(Error::GameSync)?.record;
        let draw = match &outcome {
            c6_core::combat_random::RandomDamageOutcome::Damage { draw, .. }
            | c6_core::combat_random::RandomDamageOutcome::NativeArithmeticFaultAfterDraw {
                draw,
                ..
            } => *draw,
        };
        self.draws.push(RandomDraw {
            bound: self.definitions.max_extra_damage as u16,
            value: draw,
            before,
            after,
        });
        match outcome {
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
    exchange_with_draws(
        state,
        definitions,
        defender,
        attacker_strength,
        defender_strength,
        average,
    )
    .map(|(exchange, _)| exchange)
}
/// Same exchange, retaining the actual consumed samples for comparison/output.
pub fn exchange_with_draws(
    state: &mut LoadedState,
    definitions: &Definitions,
    defender: c6_core::map::UnitReference,
    attacker_strength: kernel::Strength,
    defender_strength: kernel::Strength,
    average: bool,
) -> Result<(kernel::Exchange, Vec<RandomDraw>), Error> {
    if state.binding().inputs != definitions.identities {
        return Err(Error::Binding);
    }
    let mut draws = Vec::new();
    let result = kernel::exchange(
        &mut ExchangeContext {
            state,
            definitions,
            defender,
            draws: &mut draws,
        },
        attacker_strength,
        defender_strength,
        average,
    )?;
    Ok((result, draws))
}

/// One original CalculateUnitVsUnitDamage call. Ranged combat never executes
/// the melee exchange's retaliation call or embarked-defender override.
pub(crate) fn single_with_draws(
    state: &mut LoadedState,
    definitions: &Definitions,
    defender: c6_core::map::UnitReference,
    attacker_strength: kernel::Strength,
    defender_strength: kernel::Strength,
) -> Result<(i32, Vec<RandomDraw>), Error> {
    use kernel::ExchangeContext as _;
    let pending = prepare(
        state,
        definitions,
        defender_strength,
        attacker_strength,
        false,
    )?;
    let mut draws = Vec::new();
    let mut context = ExchangeContext {
        state,
        definitions,
        defender,
        draws: &mut draws,
    };
    let amount = match pending {
        kernel::Next::FixedDamage(amount) => amount,
        kernel::Next::RequiresCalculateDamage {
            parameters,
            average,
        } => context.calculate_damage(&parameters, average)?,
    };
    Ok((amount, draws))
}

/// One genuine District/Unit damage call. Shares the canonical GameSync owner
/// and draw records with other combat; no Unit/Unit civilian shortcut applies.
pub fn district_unit_with_draws(
    state: &mut LoadedState,
    definitions: &Definitions,
    defender: c6_core::map::UnitReference,
    attacker_raw: i32,
    defender_raw: i32,
) -> Result<(i32, Vec<RandomDraw>), Error> {
    use kernel::ExchangeContext as _;
    if state.binding().inputs != definitions.identities {
        return Err(Error::Binding);
    }
    let pending = kernel::prepare_district_unit(
        &mut Context(definitions),
        defender_raw,
        attacker_raw,
        false,
    )?;
    let mut draws = Vec::new();
    let mut context = ExchangeContext {
        state,
        definitions,
        defender,
        draws: &mut draws,
    };
    let kernel::Next::RequiresCalculateDamage {
        parameters,
        average,
    } = pending
    else {
        unreachable!("District/Unit always uses the damage kernel")
    };
    let amount = context.calculate_damage(&parameters, average)?;
    Ok((amount, draws))
}

/// Genuine Unit/District damage parameter call, before the outer-defense
/// percentage adjustment. Both the garrison and outer calls consume a draw.
#[allow(clippy::too_many_arguments)]
pub(crate) fn unit_district_with_draws(
    state: &mut LoadedState,
    definitions: &Definitions,
    attacker: c6_core::map::UnitReference,
    attacker_raw: i32,
    defender_raw: i32,
    maximum: i32,
    outer_damage: i32,
    outer_maximum: i32,
    defense_type: u32,
    bypass: bool,
) -> Result<(i32, Vec<RandomDraw>), Error> {
    use kernel::ExchangeContext as _;
    if state.binding().inputs != definitions.identities {
        return Err(Error::Binding);
    }
    let parameters = c6_core::combat::DamageParameters {
        attacker_strength_raw: attacker_raw,
        defender_strength_raw: defender_raw,
        defender_max_hit_points: maximum,
        outer_defense_damage: outer_damage,
        outer_defense_max_hit_points: outer_maximum,
        defense_type,
        power_scaling_raw: c6_core::movement::MovementPoints::route_float_bits(
            definitions.power_scaling.to_bits(),
        )
        .0,
        minimum_damage: definitions.minimum_damage,
        base_damage: definitions.base_damage,
        power_dampener: definitions.power_dampener,
        max_extra_damage: 0,
        bypass_defenses: bypass,
    };
    let mut draws = Vec::new();
    let amount = ExchangeContext {
        state,
        definitions,
        defender: attacker,
        draws: &mut draws,
    }
    .calculate_damage(&parameters, false)?;
    Ok((amount, draws))
}
