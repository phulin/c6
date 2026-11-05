//! Typed state retained across Linux City.Manager.Transfer's destroy/create.
//!
//! Transfer creates a new City and District identity. These helpers never move
//! an old City record between owners, copy constructor-derived modifiers, or
//! claim that snapshots alone execute the required gameplay continuations.
use crate::{
    city_initialization::Parameters,
    city_production::Buildings,
    city_services::{CulturalIdentity, Religion},
    loaded_state::{Error, LoadedState},
    unit::{variants::TypedVariantMap, ComponentId, PlotCoord},
};

pub const TRANSFER_COMBAT: u32 = 0xba0d_34de;

/// Values explicitly retained before Player.Cities.Destroy(city, true).
/// Districts/improvements have their own recreation lifetimes and are retained
/// by the data-layer transfer owner alongside this snapshot.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub old_city: ComponentId,
    pub location: PlotCoord,
    pub name: Vec<u8>,
    pub original_owner: i32,
    pub original_capital: bool,
    pub founded_turn: i32,
    pub population: i32,
    pub buildings: Buildings,
    pub religion: Religion,
    pub cultural_identity: CulturalIdentity,
    pub properties: TypedVariantMap,
    pub purchased_plots: Vec<u32>,
}
impl Snapshot {
    /// Native GetPurchasedPlots traverses descending map plot indexes. Keep
    /// full City identity in the predicate; a recycled low slot is insufficient.
    pub fn capture(state: &LoadedState, city: ComponentId) -> Result<Self, Error> {
        let value = state.city(i32::from(city.owner), city.id)?;
        if city.component_type != 2 {
            return Err(Error::Binding);
        }
        let purchased_plots = state
            .world()
            .map()
            .plots
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, plot)| {
                plot.city_attributes.as_ref().is_some_and(|attrs| {
                    i32::from(attrs.player) == i32::from(city.owner) && attrs.owning_city == city.id
                })
            })
            .map(|(index, _)| index as u32)
            .collect();
        Ok(Self {
            old_city: city,
            location: PlotCoord {
                x: value.header.location_x,
                y: value.header.location_y,
            },
            name: value.header.name.clone(),
            original_owner: value.header.original_owner,
            original_capital: value.header.original_capital,
            founded_turn: value.turn_city_founded,
            population: value.header.population,
            buildings: value.buildings.clone(),
            religion: value.religion.clone(),
            cultural_identity: value.cultural_identity.clone(),
            properties: value.properties.clone(),
            purchased_plots,
        })
    }
    pub fn initialization_parameters(&self) -> Parameters {
        Parameters {
            original_owner: self.original_owner,
            transfer_type: TRANSFER_COMBAT as i32,
            original_capital: self.original_capital,
            founded_turn: self.founded_turn,
        }
    }
}

/// City.Buildings.CopyBuildings2864880 moves these seven members only. Yield
/// modifiers, feature yields and other effects belong to the fresh city's
/// current callbacks and must not be replaced with the old owner's values.
pub fn copy_buildings(destination: &mut Buildings, source: Buildings) {
    destination.building_locations = source.building_locations;
    destination.pillaged = source.pillaged;
    destination.era_created = source.era_created;
    destination.num_units_supported = source.num_units_supported;
    destination.great_work_buildings = source.great_work_buildings;
    destination.extra_great_works = source.extra_great_works;
    destination.building_name_overrides = source.building_name_overrides;
}

/// The ordinary combat-transfer damage assignment uses wrapping native int
/// multiplication followed by truncating signed division; it is not a combat
/// ChangeDamage call and therefore does not stamp last_turn_damaged.
pub fn captured_center_damage(maximum: i32, percentage: i32) -> i32 {
    maximum.wrapping_mul(percentage) / 100
}

/// Player.Cities.CityPopulationAfterConquest2a5a120. A nonzero override
/// selects wrapping integer percentage arithmetic; otherwise the native f32
/// loss parameter is subtracted from 1.0 before the truncating SSE conversion.
pub fn population_after_conquest(population: i32, percentage_override: i32, loss: f32) -> i32 {
    if percentage_override != 0 {
        let product = population.wrapping_mul(percentage_override);
        if product < 200 {
            1
        } else {
            ((product as u32) / 100) as i32
        }
    } else {
        let value = (1.0_f32 - loss) * population as f32;
        let converted = if !(-2147483648.0_f32..2147483648.0_f32).contains(&value) {
            i32::MIN
        } else {
            value as i32
        };
        if converted < 2 {
            1
        } else {
            converted
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn building_reconstruction_preserves_fresh_owner_modifier_state() {
        let definitions = crate::city_initialization::Definitions {
            yields: &[10],
            units: &[],
            buildings: &[20],
            districts: &[],
            projects: &[],
            terrains: &[],
            resources: &[],
            features: &[],
            production_yield: 0,
            loyalty_start: 100,
            building_support: &[],
        };
        let mut old = crate::city_initialization::constructor::construct(
            &definitions,
            1,
            65536,
            crate::city_initialization::Parameters::default(),
        )
        .unwrap()
        .buildings;
        let mut new = crate::city_initialization::constructor::construct(
            &definitions,
            0,
            65537,
            crate::city_initialization::Parameters::default(),
        )
        .unwrap()
        .buildings;
        old.building_locations[0].value = 7;
        old.pillaged[0].value = true;
        old.num_units_supported[0].value = 3;
        old.yield_modifiers
            .push(crate::city_production::BuildingYieldModifier {
                building_hash: 20,
                yield_hash: 10,
                amount: 99,
            });
        new.yield_modifiers
            .push(crate::city_production::BuildingYieldModifier {
                building_hash: 20,
                yield_hash: 10,
                amount: 8,
            });
        super::copy_buildings(&mut new, old);
        assert_eq!(new.building_locations[0].value, 7);
        assert!(new.pillaged[0].value);
        assert_eq!(new.num_units_supported[0].value, 3);
        assert_eq!(new.yield_modifiers[0].amount, 8);
    }
    #[test]
    fn conquest_population_retains_native_minimum_and_f32_truncation() {
        assert_eq!(super::population_after_conquest(1, 0, 0.25), 1);
        assert_eq!(super::population_after_conquest(7, 0, 0.25), 5);
        assert_eq!(super::population_after_conquest(7, 50, 0.99), 3);
        assert_eq!(super::population_after_conquest(i32::MAX, 2, 0.0), 1);
        assert_eq!(super::population_after_conquest(100, 0, f32::NAN), 1);
    }
    #[test]
    fn captured_center_damage_uses_native_wrapping_and_signed_truncation() {
        assert_eq!(super::captured_center_damage(200, 50), 100);
        assert_eq!(super::captured_center_damage(201, 50), 100);
        assert_eq!(super::captured_center_damage(-201, 50), -100);
        assert_eq!(super::captured_center_damage(i32::MAX, 100), -1);
    }
}
