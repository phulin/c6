//! Ordinary Train completion on the same canonical creation owner.
//! Queue/progress writes follow the synchronous creation and production signals.
use crate::{registry::linux_type_hash, unit_creation as creation, Rules, SqlValue};
use c6_core::{
    city_production::{Order, QueueEntry},
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    unit::{ComponentId, HashValue, PlotCoord},
};
#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    Creation(Box<creation::Error<E>>),
    Rules(crate::Error),
    Missing(&'static str),
    Effect(E),
}
/// Static definition facts; no captured CanTrain or completion answer is stored.
struct UnitDefinition {
    hash: u32,
    population_cost: Option<i32>,
    civilian: bool,
    unit_index: usize,
}
pub struct Definitions {
    inputs: InputIdentities,
    units: Vec<UnitDefinition>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &crate::resource_binding::ResourceAuthority<'_>,
    ) -> Result<Self, Error<()>> {
        if state.binding() != authority.destination_binding() {
            return Err(Error::Missing("Train rules binding"));
        }
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(|_| Error::Missing("admitted Train definitions"))?;
        let registry =
            crate::unit_initialization_definitions::collection(rules, "Units", "UnitType")
                .map_err(|_| Error::Missing("Train Unit definitions"))?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let mut units = Vec::new();
        for (index, entry) in registry.entries().iter().enumerate() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Train Unit row"))?;
            let population_cost = match row.get("PopulationCost").map_err(Error::Rules)? {
                SqlValue::Null => None,
                SqlValue::Integer(v) => Some(*v as i32),
                _ => return Err(Error::Missing("population cost type")),
            };
            units.push(UnitDefinition{hash:linux_type_hash(&entry.type_name),population_cost,civilian:matches!(row.get("FormationClass").map_err(Error::Rules)?,SqlValue::Text(v)if v==b"FORMATION_CLASS_CIVILIAN"),unit_index:index});
        }
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            units,
        })
    }
}
pub trait Effects: creation::FactoryEffects {
    fn production_unit_created(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        city: ComponentId,
    ) -> Result<(), Error<<Self as creation::Effects>::Error>>;
    fn player_production_unit_created(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        unit: ComponentId,
        city_id: u32,
    ) -> Result<(), Error<<Self as creation::Effects>::Error>>;
    fn production_completed_report(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        unit_index: i32,
        military_formation: i32,
    ) -> Result<(), Error<<Self as creation::Effects>::Error>>;
}
fn indexed(values: &[HashValue<i32>], hash: u32) -> i32 {
    values
        .iter()
        .rev()
        .find(|v| v.definition_hash == hash)
        .map_or(0, |v| v.value)
}
fn copies<E>(state: &LoadedState, owner: i32, city: u32, hash: u32) -> Result<i32, Error<E>> {
    let city = state.city(owner, city).map_err(Error::State)?;
    //2858990: a missing native city vector cell returns0 before Player.GetExtra.
    // Current initialized hash arrays have default0; the admitted definition order
    // is retained independently and this query does not synthesize a native array.
    let local = indexed(&city.build_queue.extra_unit_copy, hash);
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("copy Player"))?;
    Ok(local.wrapping_add(indexed(&player.units.extra_unit_copy, hash)))
}
/// Called by production::Effects::finish after current cost/progress have been
/// computed. `required_cost` is that current-build calculation, in whole units.
/// Factory-supported ordinary classes share this kernel; positive population/yield branches remain explicit.
pub fn finish_current<E: Effects>(
    factory: &mut creation::Context<'_, '_, '_, E>,
    definitions: &Definitions,
    entry: &QueueEntry,
    required_cost: i32,
    city: ComponentId,
) -> Result<bool, Error<<E as creation::Effects>::Error>> {
    if factory.state.binding().inputs != definitions.inputs {
        return Err(Error::Missing("Train definitions binding"));
    }
    let Order::Train {
        unit_hash,
        military_formation,
    } = entry.order
    else {
        return Ok(false);
    };
    let definition = definitions
        .units
        .iter()
        .find(|v| v.hash == unit_hash)
        .ok_or(Error::Missing("current Train Unit definition"))?;
    let owner = i32::from(city.owner);
    let value = factory.state.city(owner, city.id).map_err(Error::State)?;
    let progress = indexed(&value.build_queue.unit_progress, unit_hash);
    if progress < required_cost.wrapping_shl(8) {
        return Ok(false);
    }
    let military_formation = if military_formation == -1 {
        0
    } else {
        military_formation
    };
    let mut copy = 0_i32;
    if copies::<<E as creation::Effects>::Error>(factory.state, owner, city.id, unit_hash)? >= 0 {
        loop {
            if !definition.civilian
                && factory
                    .state
                    .city(owner, city.id)
                    .map_err(Error::State)?
                    .header
                    .yield_per_unit_complete
                    .iter()
                    .any(|v| v.value > 0)
            {
                return Err(Error::Missing("positive unit-completion yield grants"));
            }
            let current = factory.state.city(owner, city.id).map_err(Error::State)?;
            let location = PlotCoord {
                x: current.header.location_x,
                y: current.header.location_y,
            };
            let unit = factory
                .create_from_city(creation::Parameters {
                    city,
                    unit_hash,
                    military_formation,
                    location,
                })
                .map_err(|e| Error::Creation(Box::new(e)))?;
            if let Some(unit) = unit {
                factory
                    .effects
                    .production_unit_created(factory.state, unit, city)?;
                factory.effects.player_production_unit_created(
                    factory.state,
                    owner,
                    unit,
                    city.id,
                )?;
            }
            let old = copy;
            copy = copy.wrapping_add(1);
            if old
                >= copies::<<E as creation::Effects>::Error>(
                    factory.state,
                    owner,
                    city.id,
                    unit_hash,
                )?
            {
                break;
            }
        }
    }
    if definition.population_cost.is_some() {
        return Err(Error::Missing("Train population consumption"));
    }
    let value = factory.state.city(owner, city.id).map_err(Error::State)?;
    let delta = *value
        .population_change_on_creation
        .get(definition.unit_index)
        .ok_or(Error::Missing("current population-on-unit array"))?;
    if value.header.population.wrapping_add(delta) >= 2 {
        return Err(Error::Missing("Train ChangePopulation callbacks"));
    }
    let queue = &mut factory
        .state
        .city_mut(owner, city.id)
        .map_err(Error::State)?
        .build_queue;
    if queue.entries.first() != Some(entry) {
        return Err(Error::Missing("Train current queue changed"));
    }
    queue.entries.remove(0);
    let progress = queue
        .unit_progress
        .iter_mut()
        .rev()
        .find(|v| v.definition_hash == unit_hash)
        .ok_or(Error::Missing("Train progress cell"))?;
    progress.value = 0;
    queue.previous_production_type_hash = unit_hash;
    factory.effects.production_completed_report(
        factory.state,
        city,
        definition.unit_index as i32,
        military_formation,
    )?;
    Ok(true)
}
