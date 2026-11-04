//! Source-derived Linux acquisition unit costs and formation tech updates.
//! Definitions, current native type indices and economic inputs must belong to
//! the admitted runtime. Native differential acceptance is still required.
use crate::{
    formation_upkeep::{self, FormationUpkeepContext, FormationUpkeepError},
    loaded_components::RetainedUnits,
    player_prerequisites::IndexedBools,
    unit::{HashValue, Unit},
    unit_initialization::{FormationReference, UnitRuntime},
    world::{Player, World},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Definition {
    pub hash: u32,
    pub cost: i32,
    pub progression_model_hash: u32,
    pub progression_param: i32,
    pub strategic_resource_present: bool,
    /// Actual matching Unit_XP2 definition, or captured absent reference.
    pub resource_cost: Option<i32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Economic {
    pub speed_cost_multiplier: i32,
    pub game_cost_escalation: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResearchCompletion {
    pub technologies: i32,
    pub civics: i32,
}
impl ResearchCompletion {
    fn maximum(self) -> i32 {
        self.technologies.max(self.civics)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    Input(E),
    World(crate::world::MissingInput),
    MissingArrayValue(u32),
    ResearchIndex(i32),
}

pub struct CostArrays<'a> {
    pub production: &'a mut [HashValue<i32>],
    pub resource: &'a mut [HashValue<i32>],
}
impl<'a> From<&'a mut RetainedUnits> for CostArrays<'a> {
    fn from(units: &'a mut RetainedUnits) -> Self {
        Self {
            production: &mut units.current_unit_cost,
            resource: &mut units.current_unit_resource_cost,
        }
    }
}

/// Native NameToCostProgressionModel. Unrecognized hashes remain -1.
pub fn progression_model(hash: u32) -> i32 {
    match hash {
        0x33b0_05a1 => 0,
        0x1289_7359 => 1,
        0xc335_9851 => 2,
        0x2032_5b0c => 3,
        0xe3b0_2906 => 4,
        0xf594_ad38 => 5,
        0xe8a4_b88d => 6,
        0x582c_dfd9 => 7,
        0xb074_6159 => 8,
        _ => -1,
    }
}

/// Q8 constructor wraps before the clamped signed-64 percentage operation;
/// AsInt uses arithmetic shift, including negative fractional results.
pub fn adjust_for_game_speed(value: i32, multiplier: i32) -> i32 {
    let value = value.wrapping_shl(8) as i64;
    let multiplier = multiplier.wrapping_shl(8) as i64;
    ((value * multiplier / 25_600).clamp(i32::MIN as i64, i32::MAX as i64) as i32) >> 8
}

fn percentage_cost(base: i32, parameter: i32, percent: i32) -> i32 {
    (parameter.wrapping_mul(base) / 100)
        .wrapping_sub(base)
        .wrapping_mul(percent)
        .wrapping_div(100)
        .wrapping_add(base)
}
fn cvttsd2si(value: f64) -> i32 {
    if !value.is_finite() || value < i32::MIN as f64 || value >= 2_147_483_648.0 {
        i32::MIN
    } else {
        value as i32
    }
}

/// RecomputeCost invokes escalation only for models 0,1,2,6. Other recognized
/// models retain their base cost here, despite having Economic implementations.
pub fn cost(
    definition: Definition,
    constructed: i32,
    current_units: i32,
    research: ResearchCompletion,
    economic: Economic,
) -> i32 {
    let base = definition.cost;
    let parameter = definition.progression_param;
    let escalated = match progression_model(definition.progression_model_hash) {
        0 => constructed.wrapping_mul(parameter).wrapping_add(base),
        1 => percentage_cost(base, parameter, research.maximum()),
        2 => {
            // Target calls Linux libm exp on this exact double expression,
            // then cvttsd2si (Rust's saturating float cast alone differs).
            let exponential =
                cvttsd2si(((constructed as f64) * (parameter as f64 / 100.0)).exp() * base as f64);
            if constructed > 0 {
                percentage_cost(
                    exponential,
                    economic.game_cost_escalation,
                    research.maximum(),
                )
            } else {
                exponential
            }
        }
        6 => current_units.wrapping_mul(parameter).wrapping_add(base),
        _ => base,
    };
    adjust_for_game_speed(escalated, economic.speed_cost_multiplier)
}

pub fn resource_cost(definition: Definition, economic: Economic) -> i32 {
    if !definition.strategic_resource_present {
        return 0;
    }
    let Some(base) = definition.resource_cost else {
        return 0;
    };
    let adjusted = adjust_for_game_speed(base, economic.speed_cost_multiplier);
    if base > 0 {
        adjusted.max(1)
    } else {
        adjusted
    }
}

fn value<T: Copy>(pairs: &[HashValue<T>], hash: u32) -> Option<T> {
    pairs
        .iter()
        .rev()
        .find(|p| p.definition_hash == hash)
        .map(|p| p.value)
}

/// Complete graph iteration and GetNumTechs/GetNumCivics are separate inputs.
/// Counts follow iteration including duplicates, then native signed arithmetic.
pub fn percent_complete(
    acquired: &IndexedBools,
    definition_hashes: &[u32],
    iteration: &[i32],
    number: u32,
) -> Result<i32, Error<std::convert::Infallible>> {
    if number == 0 {
        return Ok(100);
    }
    let mut count = 0u32;
    for &index in iteration {
        let hash = *definition_hashes
            .get(index as u32 as usize)
            .ok_or(Error::ResearchIndex(index))?;
        count = count.wrapping_add(u32::from(
            value(&acquired.pairs, hash).ok_or(Error::MissingArrayValue(hash))?,
        ));
    }
    if count > number {
        return Ok(100);
    }
    Ok(((count.wrapping_mul(100) as i32 as i64) / (number as i32 as i64)) as i32)
}

/// Direct writes to retained Player.Units arrays. Runtime native unit types are
/// read only for model6; they are not inferred from the Unit's saved hash.
/// All lookups are checked before writes; these source-closed getters draw no RNG.
pub fn recompute_all_costs<E>(
    arrays: CostArrays<'_>,
    constructed: &[HashValue<i32>],
    owner: &Player,
    definitions: &[Definition],
    research: ResearchCompletion,
    economic: Economic,
    mut native_type: impl FnMut(&Unit) -> Result<i32, E>,
) -> Result<(), Error<E>> {
    let mut updates = Vec::with_capacity(definitions.len());
    for (index, &definition) in definitions.iter().enumerate() {
        let model = progression_model(definition.progression_model_hash);
        let constructed_count = if matches!(model, 0 | 2) {
            value(constructed, definition.hash).ok_or(Error::MissingArrayValue(definition.hash))?
        } else {
            0
        };
        let mut current = 0i32;
        if model == 6 {
            // GetNumUnitsOfType(type, NO_FORMATION=0, false, false): exclude
            // delayed death and levies, accept unformed -1 as well as 0.
            for slot in owner.unit_records().map_err(Error::World)? {
                let unit = &slot.value;
                if native_type(unit).map_err(Error::Input)? == index as i32
                    && !unit.b_delayed_death
                    && !unit.b_is_levied
                    && matches!(unit.e_military_formation, -1 | 0)
                {
                    current = current.wrapping_add(1);
                }
            }
        }
        let cost_slot = arrays
            .production
            .iter()
            .rposition(|p| p.definition_hash == definition.hash)
            .ok_or(Error::MissingArrayValue(definition.hash))?;
        let resource_slot = arrays
            .resource
            .iter()
            .rposition(|p| p.definition_hash == definition.hash)
            .ok_or(Error::MissingArrayValue(definition.hash))?;
        updates.push((
            cost_slot,
            cost(definition, constructed_count, current, research, economic),
            resource_slot,
            resource_cost(definition, economic),
        ));
    }
    for (cost_slot, cost, resource_slot, resource) in updates {
        arrays.production[cost_slot].value = cost;
        arrays.resource[resource_slot].value = resource;
    }
    Ok(())
}

pub trait TechUpdatesContext: FormationUpkeepContext {
    fn world(&self) -> &World;
    fn runtime(&self, owner: i32, id: u32) -> Result<&UnitRuntime, Self::Error>;
    /// Select the authoritative Formation for the existing FormationUpkeepContext.
    /// This does not execute an update; update_attributes is called directly.
    fn select_formation(&mut self, reference: FormationReference) -> Result<(), Self::Error>;
    fn preflight_tech_updates(&self, owner: i32) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum TechUpdateError<E> {
    Context(E),
    World(crate::world::MissingInput),
    Formation(FormationUpkeepError<E>),
}

/// DoTechUpdates resolves each live unit again, skips delayed death/null
/// formations and executes existing UpdateAttributes. Shared formations are
/// updated once per unit visit, preserving duplicate reports and RNG effects.
pub fn do_tech_updates<C: TechUpdatesContext>(
    c: &mut C,
    owner: i32,
) -> Result<(), TechUpdateError<C::Error>> {
    c.preflight_tech_updates(owner)
        .map_err(TechUpdateError::Context)?;
    let mut index = 0;
    loop {
        let player = c.world().player(owner).map_err(TechUpdateError::World)?;
        let slots = player.unit_records().map_err(TechUpdateError::World)?;
        let Some(slot) = slots.get(index) else {
            return Ok(());
        };
        if !slot.value.b_delayed_death {
            let id = slot.value.id;
            if let Some(found) = player.unit(id).map_err(TechUpdateError::World)? {
                let reference = c
                    .runtime(owner, found.resolved_id)
                    .map_err(TechUpdateError::Context)?
                    .formation;
                if let Some(reference) = reference {
                    c.select_formation(reference)
                        .map_err(TechUpdateError::Context)?;
                    formation_upkeep::update_attributes(c).map_err(TechUpdateError::Formation)?;
                }
            }
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RESEARCH: ResearchCompletion = ResearchCompletion {
        technologies: 25,
        civics: 50,
    };
    const ECONOMIC: Economic = Economic {
        speed_cost_multiplier: 100,
        game_cost_escalation: 1000,
    };
    fn definition(model: u32) -> Definition {
        Definition {
            hash: 19,
            cost: 100,
            progression_model_hash: model,
            progression_param: 20,
            strategic_resource_present: true,
            resource_cost: Some(5),
        }
    }
    #[test]
    fn reached_models_match_native_switch_and_integer_order() {
        assert_eq!(cost(definition(0x33b0_05a1), 3, 9, RESEARCH, ECONOMIC), 160);
        assert_eq!(cost(definition(0x1289_7359), 3, 9, RESEARCH, ECONOMIC), 60);
        assert_eq!(cost(definition(0xe8a4_b88d), 3, 9, RESEARCH, ECONOMIC), 280);
        // Model3 is recognized but RecomputeCost never calls escalation for it.
        assert_eq!(cost(definition(0x2032_5b0c), 3, 9, RESEARCH, ECONOMIC), 100);
        assert_eq!(cost(definition(0), 3, 9, RESEARCH, ECONOMIC), 100);
        assert_eq!(cost(definition(0xc335_9851), 0, 9, RESEARCH, ECONOMIC), 100);
        // exp(0.6)*100 truncates to182, then research rescales to1001.
        assert_eq!(
            cost(definition(0xc335_9851), 3, 9, RESEARCH, ECONOMIC),
            1001
        );
        assert_eq!(cvttsd2si(f64::NAN), i32::MIN);
        assert_eq!(cvttsd2si(2_147_483_648.0), i32::MIN);
    }
    #[test]
    fn speed_q8_wrapping_clamping_and_resource_minimum() {
        assert_eq!(adjust_for_game_speed(3, 50), 1);
        assert_eq!(adjust_for_game_speed(-3, 50), -2);
        assert_eq!(adjust_for_game_speed(i32::MAX, 100), -1);
        assert_eq!(adjust_for_game_speed(8_000_000, 200), 8_388_607);
        let economic = Economic {
            speed_cost_multiplier: 1,
            ..ECONOMIC
        };
        assert_eq!(resource_cost(definition(0), economic), 1);
        assert_eq!(
            resource_cost(
                Definition {
                    resource_cost: Some(-3),
                    ..definition(0)
                },
                Economic {
                    speed_cost_multiplier: 50,
                    ..ECONOMIC
                }
            ),
            -2
        );
        assert_eq!(
            resource_cost(
                Definition {
                    strategic_resource_present: false,
                    ..definition(0)
                },
                ECONOMIC
            ),
            0
        );
        assert_eq!(
            resource_cost(
                Definition {
                    resource_cost: None,
                    ..definition(0)
                },
                ECONOMIC
            ),
            0
        );
    }
    #[test]
    fn graph_iteration_and_denominator_are_independent() {
        let acquired = IndexedBools {
            pairs: vec![
                HashValue {
                    definition_hash: 9,
                    value: false,
                },
                HashValue {
                    definition_hash: 9,
                    value: true,
                },
            ],
        };
        assert_eq!(percent_complete(&acquired, &[9], &[0, 0], 4), Ok(50));
        assert_eq!(percent_complete(&acquired, &[9], &[0, 0], 1), Ok(100));
        assert_eq!(percent_complete(&acquired, &[], &[], 0), Ok(100));
        assert!(matches!(
            percent_complete(&acquired, &[7], &[0], 1),
            Err(Error::MissingArrayValue(7))
        ));
    }
    #[test]
    fn costs_write_real_arrays_and_missing_input_does_not_mutate() {
        let owner = Player::new(0, None, None).unwrap();
        let mut production = vec![HashValue {
            definition_hash: 19,
            value: -1,
        }];
        let mut resource = vec![HashValue {
            definition_hash: 19,
            value: -2,
        }];
        let constructed = vec![HashValue {
            definition_hash: 19,
            value: 3,
        }];
        recompute_all_costs(
            CostArrays {
                production: &mut production,
                resource: &mut resource,
            },
            &constructed,
            &owner,
            &[definition(0x33b0_05a1)],
            RESEARCH,
            ECONOMIC,
            |_| -> Result<i32, ()> { panic!("constructed model never reads unit instances") },
        )
        .unwrap();
        assert_eq!((production[0].value, resource[0].value), (160, 5));
        resource.clear();
        assert_eq!(
            recompute_all_costs(
                CostArrays {
                    production: &mut production,
                    resource: &mut resource
                },
                &constructed,
                &owner,
                &[definition(0x33b0_05a1)],
                RESEARCH,
                ECONOMIC,
                |_| Ok::<_, ()>(0)
            ),
            Err(Error::MissingArrayValue(19))
        );
        assert_eq!(production[0].value, 160);
    }
}
