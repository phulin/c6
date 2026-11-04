//! Constructor and Initialize state assignments for an ordinary fresh City.
//! Mutable callbacks (signal registration and CityBuild creation) are joined by
//! the parent lifecycle; this function returns its canonical pre-CityBuild state.
use super::Error;
use crate::unit::{
    variants::{TypedVariantMap, VariantMap},
    HashValue,
};
use crate::{city::City, city_citizens, city_economy, city_production, city_services, city_wire};

/// Current admitted native collection order; no captured City/template input.
pub struct Definitions<'a> {
    pub yields: &'a [u32],
    pub units: &'a [u32],
    pub buildings: &'a [u32],
    pub districts: &'a [u32],
    pub projects: &'a [u32],
    pub terrains: &'a [u32],
    pub resources: &'a [u32],
    pub features: &'a [u32],
    pub production_yield: i32,
    pub loyalty_start: i32,
    /// Native Unit_BuildingPrereq iteration order. NULL references are skipped.
    pub building_support: &'a [BuildingSupport],
}
#[derive(Clone, Copy, Debug)]
pub struct BuildingSupport {
    pub building_index: Option<usize>,
    pub num_supported: i16,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub original_owner: i32,
    pub transfer_type: i32,
    pub original_capital: bool,
    pub founded_turn: i32,
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            original_owner: -1,
            transfer_type: -1,
            original_capital: false,
            founded_turn: -1,
        }
    }
}
fn pairs<T: Copy>(hashes: &[u32], value: T) -> Vec<HashValue<T>> {
    hashes
        .iter()
        .map(|&definition_hash| HashValue {
            definition_hash,
            value,
        })
        .collect()
}

/// City.Instance constructor2892010 and Initialize2894390 through CityBuild.
/// Components use their actual constructor/Reset defaults, definition-sized
/// arrays, and Unit_BuildingPrereq overwrites (Buildings.Reset28631a0).
pub fn construct(
    definitions: &Definitions<'_>,
    owner: i32,
    id: u32,
    parameters: Parameters,
) -> Result<City, Error> {
    if !(0..64).contains(&owner)
        || [
            definitions.yields,
            definitions.units,
            definitions.buildings,
            definitions.districts,
            definitions.projects,
            definitions.terrains,
            definitions.resources,
            definitions.features,
        ]
        .iter()
        .any(|v| v.len() > 65536)
    {
        return Err(Error::Allocator);
    }
    let mut supported = pairs(definitions.buildings, -1i16);
    for row in definitions.building_support {
        if row.num_supported != -1 {
            if let Some(index) = row.building_index {
                supported.get_mut(index).ok_or(Error::Allocator)?.value = row.num_supported;
            }
        }
    }
    let header = city_wire::Header {
        wire_version: 51,
        deprecated_yield_q8: Vec::new(),
        id,
        location_x: -9999,
        location_y: -9999,
        owner,
        original_owner: if parameters.original_owner == -1 {
            owner
        } else {
            parameters.original_owner
        },
        owner_before_occupation: owner,
        liberator: -1,
        city_ai_id: u32::MAX,
        population: 0,
        capital: false,
        original_capital: parameters.original_capital,
        last_transfer_type: parameters.transfer_type,
        just_conquered_from: -1,
        just_rebelled_from: -1,
        last_barbarian_invasion_turn: -1,
        occupied: false,
        last_warmonger_penalty: [0; 64],
        production_unit_changes: 0,
        production_building_changes: 0,
        production_district_changes: 0,
        wonder_envoy: 0,
        delayed_death: false,
        transferring: false,
        yield_modifiers: pairs(definitions.yields, 0),
        follower_yield_modifiers: pairs(definitions.yields, 0),
        yield_modifiers_per_governor_title_q8: pairs(definitions.yields, 0),
        yield_changes: pairs(definitions.yields, 0),
        yield_per_district: pairs(definitions.yields, 0),
        extra_yield_per_population_q8: pairs(definitions.yields, 0),
        allow_incoming_regional_stacking_counts: pairs(definitions.yields, 0),
        yield_per_unit_complete: pairs(definitions.yields, 0),
        yield_per_building_complete: pairs(definitions.yields, 0),
        yield_per_wonder_complete: pairs(definitions.yields, 0),
        yield_per_terrain_class_cities: pairs(definitions.yields, 0),
        project_yield_q8: pairs(definitions.yields, 0),
        yield_modifier_from_faith_city: pairs(definitions.yields, 0),
        happiness_modifiers: Vec::new(),
        terrain_modifiers: Vec::new(),
        improvement_valid_terrain: Vec::new(),
        terrain_class_yield_improvement_modifiers: Vec::new(),
        project_yield_project_hash: u32::MAX,
        housing_per_district: 0,
        dynamic_bonuses: Vec::new(),
        yield_per_terrain_type: Vec::new(),
        max_unit_level: 2,
        unit_experience_modifier_q8: 0,
        attacks_per_turn: 1,
        great_person_point_bonus_percent: 0,
        settler_consume_pop: true,
        yield_per_flood: pairs(definitions.yields, 0),
        espionage_defense_bonus: 0,
        project_production: false,
        prevent_bypass_outer_defenses: 0,
        prevent_melee_attack_outer_defenses: 0,
        name: Vec::new(),
        attributes: VariantMap::default(),
    };
    let citizens = city_citizens::Citizens {
        working_plot: vec![0; 37],
        forced_working_plot: vec![0; 37],
        favored_yields: pairs(definitions.yields, false),
        disfavored_yields: pairs(definitions.yields, false),
        num_unassigned_citizens: 0,
        num_citizens_working_plots: 0,
        force_update_plot_appeal: false,
    };
    let culture = city_economy::Culture {
        wire_version: city_economy::CULTURE_VERSION,
        culture_raw_q8: 0,
        plots_acquired: 0,
        border_expansion_modifier: 0,
        min_era_enhanced_tourism_hash: u32::MAX,
        enhanced_tourism_modifier: 0,
        improvement_tourism_modifier: 0,
        national_park_tourism_modifier: 0,
        tourism_from_green_energy_modifier: 0,
        tourism_modifiers: Vec::new(),
        great_work_yield_modifiers: Vec::new(),
        expended_great_person_tiles: 0,
        plots_acquired_from_culture: 0,
        next_buyable_plot: -1,
        tourism_per_feature: 0,
        loyalty_from_great_works_citizen_modifier: 0,
        no_culture_border_expansion: false,
    };
    let gold = city_economy::Gold {
        wire_version: city_economy::GOLD_VERSION,
        plot_cost_modifier: 0,
        unit_cost_modifier: pairs(definitions.units, 0),
        building_cost_modifier: pairs(definitions.buildings, 0),
        district_cost_modifier: pairs(definitions.districts, 0),
        plot_cost_terrain_modifier: pairs(definitions.terrains, 0),
        siphoned_gold: 0,
        deprecated_building_faith_purchase: Vec::new(),
        deprecated_unit_faith_purchase: Vec::new(),
        building_faith_purchase_enabled: pairs(definitions.buildings, 0),
        unit_faith_purchase_enabled: pairs(definitions.units, 0),
        can_purchase_districts: false,
        gold_from_citizens: 0,
    };
    let growth = city_economy::Growth {
        wire_version: city_economy::GROWTH_VERSION,
        food_raw_q8: 0,
        other_growth_modifier_raw_q8: 0,
        housing_from_water_modifier: 0,
        housing_from_buildings_modifier: 0,
        housing_from_improvements_modifier: 0,
        housing_from_districts_modifier: 0,
        housing_from_civics_modifier: 0,
        amenities_from_entertainment_modifier: 0,
        amenities_from_civics_modifier: 0,
        extra_appeal: 0,
        universal_fresh_water_housing_bonus: false,
        housing_from_great_people_modifier: 0,
        housing_from_starting_era_modifier: 0,
        housing_from_great_works_modifier: 0,
        amenities_from_great_people_modifier: 0,
        amenities_from_religion_modifier: 0,
        amenities_from_national_parks_modifier: 0,
        amenities_from_starting_era_modifier: 0,
        amenities_from_improvements_modifier: 0,
        turn_last_notified_starvation: i32::MIN,
        turn_last_notified_food_focus: i32::MIN,
        turn_last_notified_need_housing: i32::MIN,
        turn_last_notified_need_amenities: i32::MIN,
        amenities_from_city_states_modifier: 0,
        amenities_from_governors_modifier: 0,
        amenities_from_districts_modifier: 0,
        amenities_from_natural_wonders_modifier: 0,
        amenities_from_traits_modifier: 0,
        no_fresh_water_housing: 0,
        add_amenity_per_adjacent_luxury: 0,
        amenities_from_religion_followers: 0,
        amenities_from_religion_minimum_followers: 0,
        feature_appeal_modifiers: Vec::new(),
    };
    let religion = city_services::Religion {
        majority_religion_hash: u32::MAX,
        religion_status: Vec::new(),
        next_religion_hash: u32::MAX,
        turns_to_next_religion: 0,
        pressure_modifier: 0,
        religious_combat_bonus: 0,
        extra_religious_heal: 0,
        religion_ignore_pressure: false,
        religion_ignore_combat: false,
        num_extra_promotions: 0,
        previous_real_religion_hash: u32::MAX,
    };
    let resources = city_services::Resources {
        wire_version: city_services::RESOURCES_VERSION,
        city_id: city_services::CityId {
            owner: owner as i16,
            component_type: 2,
            id,
        },
        harvest_bonus_modifier: 0,
        resource_yield_by_count: pairs(definitions.yields, 0),
        yield_by_number_of_resources: pairs(definitions.yields, 0),
        extra_accumulation_modifier: 0,
        extra_accumulation_for_strategic_diversity_modifier: 0,
        extra_amenity_modifier: 0,
        extra_accumulation_terrain_modifier: pairs(definitions.terrains, 0),
        extra_accumulation_specific_resource_modifier: pairs(definitions.resources, 0),
    };
    let build_queue = city_production::BuildQueue {
        wire_version: city_production::BuildQueue::VERSION,
        entries: Vec::new(),
        yield_type: definitions.production_yield,
        wonder_production_multiplier: 256,
        river_district_production_multiplier: 256,
        river_wonder_production_multiplier: 256,
        building_production_modifier: 256,
        building_non_wonder_production_modifier: 256,
        city_unit_production_modifier: 256,
        district_production_modifier: 256,
        adjacent_city_river_building_production_multiplier: 256,
        adjacent_city_river_district_production_multiplier: 256,
        district_adjacent_natural_wonder_production_modifier: pairs(definitions.features, 256),
        wonder_adjacent_natural_wonder_production_modifier: pairs(definitions.features, 256),
        building_progress: pairs(definitions.buildings, 0),
        construction_plot: pairs(definitions.buildings, -1),
        unit_progress: pairs(definitions.units, 0),
        district_progress: pairs(definitions.districts, 0),
        project_progress: pairs(definitions.projects, 0),
        zoning_plot: pairs(definitions.districts, -1),
        building_production_multiplier: pairs(definitions.buildings, 256),
        unit_production_multiplier: pairs(definitions.units, 0),
        district_production_multiplier: pairs(definitions.districts, 256),
        extra_unit_copy: pairs(definitions.units, 0),
        features_valid_wonders: pairs(definitions.features, false),
        production_overflow: 0,
        production_salvage: 0,
        feature_overrides: Vec::new(),
        corps_army_discount: pairs(definitions.units, 0),
        project_production_multiplier: pairs(definitions.projects, 256),
        previous_production_type_hash: 0,
        ignore_strategic_resource_requirements: false,
        strategic_resource_requirement_modifier: 0,
        district_building_production_modifiers: Vec::new(),
    };
    let buildings = city_production::Buildings {
        wire_version: city_production::Buildings::VERSION,
        building_locations: pairs(definitions.buildings, -1),
        pillaged: pairs(definitions.buildings, false),
        era_created: pairs(definitions.buildings, -1),
        num_units_supported: supported,
        yield_modifiers: Vec::new(),
        yield_changes: Vec::new(),
        yield_changes_city_states: Vec::new(),
        wonder_yield_changes: Vec::new(),
        great_work_buildings: Vec::new(),
        extra_great_works: Vec::new(),
        yield_change_from_features: Vec::new(),
        building_name_overrides: Vec::new(),
    };
    let combat = city_services::Combat {
        wire_version: city_services::COMBAT_VERSION,
        outer_defense_strength_modifier: 0,
        inner_defense_strength_modifier: 0,
        ranged_strike_strength_modifier: 0,
        siege_protection: false,
        anti_air_defense_bonus: 0,
        city_combat_bonus: 0,
        city_friendly_combat_bonus: 0,
        attack_range_modifier: 0,
        hit_points_modifier: 0,
    };
    let trade = city_services::Trade {
        wire_version: city_services::TRADE_VERSION,
        constructed_trading_posts: vec![false; 64],
        num_foreign_routes_passing_through: 0,
        flat_yield_bonus_to_others_for_domestic: pairs(definitions.yields, 0),
        flat_yield_bonus_to_others: pairs(definitions.yields, 0),
        flat_yield_bonus_from_others: pairs(definitions.yields, 0),
        yield_per_local_bonus_resource_for_domestic: pairs(definitions.yields, 0),
        yield_per_local_bonus_resource_for_international: pairs(definitions.yields, 0),
        yield_per_destination_luxury_resource_for_international: pairs(definitions.yields, 0),
        yield_per_destination_strategic_resource_for_domestic: pairs(definitions.yields, 0),
        yield_per_destination_strategic_resource_for_international: pairs(definitions.yields, 0),
        flat_yield_bonus_for_domestic: pairs(definitions.yields, 0),
        flat_yield_bonus_for_international: pairs(definitions.yields, 0),
        yield_from_foreign_routes_passing_through: pairs(definitions.yields, 0),
        yield_per_local_luxury_resource_for_international: pairs(definitions.yields, 0),
        yield_per_local_strategic_resource_for_international: pairs(definitions.yields, 0),
        yield_per_major_trade_partner: pairs(definitions.yields, 0),
    };
    let cultural_identity = city_services::CulturalIdentity {
        loyalty_raw_q8: definitions.loyalty_start.wrapping_shl(8),
        last_loyalty_change_raw_q8: 0,
        identities_present: Vec::new(),
        conversion_outcome: None,
        turns_to_conversion: None,
        transfer_to: -1,
        identity_pressure_from_ages_raw_q8: 0,
        identity_pressure_from_governor_on_domestic_raw_q8: 0,
        identity_pressure_from_governor_on_foreign_raw_q8: 0,
        identity_pressure_from_modifiers_raw_q8: 0,
        turn_last_notified_losing_cultural_identity: i32::MIN,
        martial_law_target: -1,
        always_loyal_flag_count: 0,
        identity_per_turn_from_modifiers_raw_q8: 0,
        loyalty_level_hash: u32::MAX,
        extra_identity_per_turn_per_citizen_raw_q8: 0,
    };
    let power = city_services::Power {
        wire_version: city_services::POWER_VERSION,
        free_power_raw_q8: 0,
        temporary_power_raw_q8: 0,
        required_power_total_raw_q8: 0,
        required_power_from_modifiers_raw_q8: 0,
        ever_generated_power: false,
        turn_last_notified_need_power: 0,
        free_power_sources: Vec::new(),
        temporary_power_sources: Vec::new(),
        required_power_sources: Vec::new(),
        generated_power_sources: Vec::new(),
        modified_free_power: 0,
    };
    Ok(City {
        header,
        districts: city_wire::Districts {
            district_ids: Vec::new(),
            extra_districts: 0,
        },
        citizens,
        culture,
        gold,
        growth,
        religion,
        resources,
        build_queue,
        buildings,
        combat,
        trade,
        cultural_identity,
        power,
        properties: TypedVariantMap::default(),
        ai_controller: u32::MAX,
        turn_city_founded: 0,
        allowed_improvement_hashes: Vec::new(),
        population_change_on_creation: Vec::new(),
        yield_change_bonus_with_power_modifier: pairs(definitions.yields, 0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definitions() -> Definitions<'static> {
        Definitions {
            yields: &[9, 7],
            units: &[22, 21, 24],
            buildings: &[41, 49],
            districts: &[80],
            projects: &[105],
            terrains: &[90, 91, 92, 93],
            resources: &[300],
            features: &[12, 14, 11],
            production_yield: 1,
            loyalty_start: 100,
            building_support: &[],
        }
    }
    #[test]
    fn constructor_keeps_native_sentinels_and_definition_sized_arrays() {
        let city = construct(
            &definitions(),
            3,
            0x10000,
            Parameters {
                original_capital: true,
                ..Parameters::default()
            },
        )
        .unwrap();
        assert_eq!(
            (
                city.header.owner,
                city.header.original_owner,
                city.header.owner_before_occupation
            ),
            (3, 3, 3)
        );
        assert_eq!(
            (city.header.location_x, city.header.location_y),
            (-9999, -9999)
        );
        assert!(!city.header.capital);
        assert!(city.header.original_capital);
        assert_eq!(city.header.max_unit_level, 2);
        assert_eq!(city.header.attacks_per_turn, 1);
        assert!(city.header.settler_consume_pop);
        assert_eq!(city.header.city_ai_id, u32::MAX);
        assert_eq!(city.citizens.working_plot, vec![0; 37]);
        assert_eq!(city.header.yield_modifiers, pairs(&[9, 7], 0));
        assert_eq!(city.resources.extra_accumulation_terrain_modifier.len(), 4);
        assert_eq!(city.cultural_identity.loyalty_raw_q8, 25600);
        assert_eq!(city.cultural_identity.conversion_outcome, None);
        assert_eq!(city.cultural_identity.turns_to_conversion, None);
        assert_eq!(city.growth.turn_last_notified_need_housing, i32::MIN);
        assert_eq!(city.religion.previous_real_religion_hash, u32::MAX);
        assert_eq!(
            city.resources.city_id,
            city_services::CityId {
                owner: 3,
                component_type: 2,
                id: 0x10000
            }
        );
    }
    #[test]
    fn building_support_is_ordered_and_null_or_minus_one_rows_do_not_overwrite() {
        let mut definitions = definitions();
        definitions.building_support = &[
            BuildingSupport {
                building_index: Some(1),
                num_supported: 3,
            },
            BuildingSupport {
                building_index: None,
                num_supported: 8,
            },
            BuildingSupport {
                building_index: Some(1),
                num_supported: -1,
            },
            BuildingSupport {
                building_index: Some(1),
                num_supported: 5,
            },
        ];
        let city = construct(&definitions, 0, 0x10000, Parameters::default()).unwrap();
        assert_eq!(
            city.buildings.num_units_supported,
            pairs(&[41, 49], -1)
                .into_iter()
                .enumerate()
                .map(|(i, mut p)| {
                    if i == 1 {
                        p.value = 5
                    }
                    p
                })
                .collect::<Vec<_>>()
        );
        assert_eq!(city.buildings.building_locations, pairs(&[41, 49], -1));
        assert_eq!(city.buildings.era_created, pairs(&[41, 49], -1));
    }
    #[test]
    fn production_q8_defaults_use_the_actual_collection_for_each_array() {
        let city = construct(&definitions(), 0, 0x10000, Parameters::default()).unwrap();
        assert_eq!(
            city.build_queue.building_production_multiplier,
            pairs(&[41, 49], 256)
        );
        assert_eq!(
            city.build_queue.unit_production_multiplier,
            pairs(&[22, 21, 24], 0)
        );
        assert_eq!(
            city.build_queue.project_production_multiplier,
            pairs(&[105], 256)
        );
        assert_eq!(
            city.build_queue
                .district_adjacent_natural_wonder_production_modifier,
            pairs(&[12, 14, 11], 256)
        );
        assert_eq!(city.build_queue.construction_plot, pairs(&[41, 49], -1));
        assert_eq!(city.build_queue.zoning_plot, pairs(&[80], -1));
    }
    #[test]
    fn original_owner_override_and_fixed_point_wrap_are_not_normalized() {
        let mut definitions = definitions();
        definitions.loyalty_start = i32::MAX;
        let city = construct(
            &definitions,
            2,
            0x10003,
            Parameters {
                original_owner: 7,
                transfer_type: 4,
                founded_turn: 23,
                original_capital: false,
            },
        )
        .unwrap();
        assert_eq!(city.header.original_owner, 7);
        assert_eq!(city.header.owner_before_occupation, 2);
        assert_eq!(city.header.last_transfer_type, 4);
        assert_eq!(city.cultural_identity.loyalty_raw_q8, -256);
        assert_eq!(city.turn_city_founded, 0); // written only after CityBuild continuation
    }
}
