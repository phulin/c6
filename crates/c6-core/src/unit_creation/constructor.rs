//! Source Unit constructor/reset state immediately before MaxMoves and initial
//! abilities/promotions. No placed/initial-unit snapshot supplies these values.
use super::{Definitions, Error, Properties};
use crate::unit::*;
const NULL_CITY: ComponentId = ComponentId {
    owner: -1,
    component_type: 2,
    id: u32::MAX,
};
pub const INVALID_LOCATION: PlotCoord = PlotCoord { x: -9999, y: -9999 };
fn dense<T: Copy>(hashes: &[u32], value: T) -> Result<Vec<HashValue<T>>, Error> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(hashes.len())
        .map_err(|_| Error::ResourceLimit)?;
    values.extend(hashes.iter().map(|&definition_hash| HashValue {
        definition_hash,
        value,
    }));
    Ok(values)
}
/// Reset helpers: Health2c2bfe0, Combat2c2c000, Movement2c2c050,
/// Experience2c18060, Religion2c43ec0, RockBand2c450d0, Trade2c485f0,
/// Archaeology2c120d0, Espionage2c16040, Hero2c1bc30. Source Initialize
/// sets sight/range before MaxMoves. Charges and Religion Reset follow it.
pub fn before_max_moves(
    owner: i32,
    id: u32,
    properties: &Properties,
    constructor_definition: &Properties,
    definitions: &Definitions<'_>,
    default_ai: i32,
    configured_attacks: i32,
) -> Result<Unit, Error> {
    definitions.validate()?;
    Ok(Unit {
        wire_version: 28,
        id,
        e_type_index: properties.type_hash,
        e_ai_type: default_ai,
        ai_controller: u32::MAX,
        ai_goal: INVALID_LOCATION,
        k_location: INVALID_LOCATION,
        e_facing_direction: 0,
        e_owner: owner,
        e_original_owner: owner,
        b_is_levied: false,
        formation_id: 0,
        e_military_formation: if properties.combat > 0 { 0 } else { -1 },
        e_automation: 0,
        e_activity: 0,
        i_damage: 0,
        i_fortify_turns: 0,
        i_upgrade_discount: 0,
        x_moves_remaining: 0,
        i_attacks_remaining: if configured_attacks == 0 {
            1
        } else {
            configured_attacks
        },
        i_operation_timer: 0,
        i_build_charges: 0,
        i_park_charges: 0,
        i_action_charges: 0,
        i_air_slot_modifier: 0,
        b_dominant_conquer: false,
        i_barbarian_tribe_index: -1,
        e_barbarian_player_target: -1,
        i_disaster_charges: Some(0),
        i_era_strength_modifier: 0,
        b_embarked: false,
        b_delayed_death: false,
        b_immobile: false,
        b_processed_turn: false,
        b_needs_movement_completion: false,
        b_eligible_to_heal: false,
        b_has_hidden_visibility: false,
        b_hidden_visibility_forfeited: false,
        b_ignore_terrain_cost: false,
        b_ignore_zoc: false,
        b_exert_zoc: false,
        b_can_see_through_features: false,
        b_can_see_through_terrain: false,
        b_attack_after_moving: true,
        i_move_after_attacking: 0,
        b_needs_attention: false,
        b_ignore_resource_maintenance: false,
        k_health_modifiers: HealthModifiers {
            i_extra_enemy_heal: 0,
            i_extra_neutral_heal: 0,
            i_extra_friendly_heal: 0,
            b_can_heal_after_action: false,
        },
        k_health_religion_modifiers: HealthReligionModifiers {
            i_extra_enemy_heal_religion: 0,
            i_extra_neutral_heal_religion: 0,
            i_extra_friendly_heal_religion: 0,
            b_can_heal_after_action_religion: false,
        },
        k_combat_modifers: CombatModifiers {
            wire_version: 28,
            i_versus_barbarians: 0,
            i_friendly_territory_combat_mod: 0,
            b_bypass_walls: false,
            b_enable_wall_attack: false,
            b_cannot_attack: false,
            i_flanking_bonus_modifier: 0,
            i_support_bonus_modifier: 0,
            i_per_luxury_attack_modifier: 0,
            b_advanced_pillaging: false,
            i_plunder_yield_modifier: 0,
            b_bypass_combat_units: false,
            b_no_reduction_for_damage: false,
            b_capture_combat_units: false,
            e_captured_combat_unit_type: u32::MAX,
            i_bonus_attacks: 0,
            b_conduct_raid: false,
            i_raid_bonus: 0,
            i_strength_reduction_for_damage_modifier: 0,
            i_post_combat_heal: 0,
            b_can_force_retreat: false,
            i_higher_diplo_viz_combat_mod: 0,
            i_each_diplo_viz_combat_mod: 0,
            i_adjacent_levy_combat_modifier: 0,
            i_number_allies_combat_modifier: 0,
            i_strength_reduction_insufficient_fuel: 0,
            i_per_unused_movement_combat_modifier: 0,
            i_advanced_coastal_raid: 0,
            i_combat_bonus_against_district: 0,
            b_ignore_ranged_vs_district_penalty: false,
            i_wmd_blast_resistance: 0,
            i_wmd_fallout_resistance: 0,
            i_water_damage_resistance: 0,
            i_number_holy_cities_combat_modifier: 0,
            i_number_military_policies_combat_modifier: 0,
            i_number_neighbor_combat_bonus: 0,
            b_fight_while_embarked: false,
            i_anti_air_strength_modifier: 0,
            i_combat_strength_modifier: 0,
            i_ranged_strength_modifier: 0,
            i_bombard_strength_modifier: 0,
            i_direct_loyalty_damage: 0,
        },
        k_movement_modifiers: MovementModifiers {
            i_extra_moves: 0,
            i_clear_terrain_start_movement: 0,
            b_enter_foreign_lands: false,
            b_ignore_rivers: false,
            b_ignore_cliffs: false,
            b_ignore_shores: false,
            b_escort_mobility: false,
            i_friendly_territory_start_movement: 0,
            i_enemy_territory_start_movement: 0,
            b_can_paradrop: false,
            i_jump_range: 0,
        },
        k_sight_modifiers: SightModifiers {
            i_sight_range: properties.sight,
            e_can_see_hidden_type: -1,
        },
        k_attack_range_modifiers: AttackRangeModifiers {
            i_attack_range: properties.range,
        },
        k_sight_plots: vec![],
        ai_abilities_possessed: dense(definitions.abilities, 0)?,
        ab_invalid_terrain_types: dense(definitions.terrains, false)?,
        ab_ignore_terrain_types: dense(definitions.terrains, false)?,
        ab_ignore_feature_types: dense(definitions.features, false)?,
        ai_bonus_points_per_kill: dense(definitions.great_person_classes, 0)?,
        ai_bonus_points_per_kill_strength: dense(definitions.great_person_classes, 0)?,
        ai_bypass_walls_promotion_class: vec![],
        ai_enable_wall_attack_promotion_class: vec![],
        k_experience: Experience {
            wire_version: 7,
            b_can_promote: false,
            b_experience_locked: false,
            i_experience: 0,
            i_level: 1,
            i_stored_promotions: 0,
            i_promote_no_finish_moves: 0,
            x_general_experience_modifier: 256,
            ab_promotions_earned: dense(definitions.promotions, false)?,
            s_veteran_name: vec![],
            b_can_promote_notify_needed: false,
            x_attacking_experience_modifier_percent: 0,
        },
        k_great_person: GreatPerson {
            e_individual: u32::MAX,
            e_class: u32::MAX,
            i_action_charges: 0,
        },
        k_religion: Religion {
            wire_version: 8,
            i_religious_strength: constructor_definition.religious_strength,
            e_religion_type: u32::MAX,
            i_spread_charges: constructor_definition.spread_charges,
            i_religious_heal_charges: constructor_definition.religious_heal_charges,
            i_nw_deferred_charges: 0,
            e_initiation_yield: -1,
            i_initiation_amount: 0,
            i_evict_percent: constructor_definition.religion_evict_percent,
            i_foreign_spread_modifier: 0,
            b_converts_barbarians: false,
            b_no_foreign_spread: false,
            b_relic_upon_death: false,
            b_land_victory_spread: false,
            i_land_victory_spread: 0,
            i_plunder_improvement_faith_yield: 0,
            i_plunder_district_faith_yield: 0,
            a_initiation_yield_population: vec![],
            e_relic_source: -1,
        },
        k_rock_band: RockBand {
            wire_version: 10,
            b_max_level: false,
            b_max_promotions: false,
            i_level: 1,
            i_album_sales: 0,
            i_post_tourism_bomb_foreign_loyalty: 0,
            i_modified_rockband_level_national_park: 0,
            i_modified_tourism_bomb_value_national_park: 0,
            i_modified_rockband_level_natural_wonder: 0,
            i_modified_tourism_bomb_value_natural_wonder: 0,
            i_modified_tourism_bomb_player_range: 0,
            i_modified_tourism_bomb_value_at_range: 0,
            i_modified_tourism_bomb_convert_city: 0,
            i_modified_tourism_bomb_value_target_peace: 0,
            a_modified_yield_per_tourism_bomb: dense(definitions.yields, 0)?,
            ai_concert_plots: vec![],
            ai_modified_rockband_level_district: dense(definitions.districts, 0)?,
            ai_modified_rockband_level_improvement: dense(definitions.improvements, 0)?,
            ai_modified_tourism_bomb_value_district: dense(definitions.districts, 0)?,
            ai_modified_tourism_bomb_value_improvement: dense(definitions.improvements, 0)?,
        },
        k_trade: Trade {
            k_origin_trade_city: NULL_CITY,
            k_destination_trade_city: NULL_CITY,
            v_domain_plunder_immunities: [0; 5],
        },
        k_archaeology: Archaeology {
            k_home_city_id: u32::MAX,
            e_home_building: u32::MAX,
            i_artifact_index: -1,
            i_great_work_index: -1,
            b_can_extract_sea_artifacts: false,
            b_choosing_artifact: false,
        },
        k_espionage: Espionage {
            wire_version: 7,
            a_espionage_bonuses: vec![],
            a_espionage_time_reductions: vec![],
            e_operation: u32::MAX,
            i_operation_end_turn: 0,
            k_operation_end_destination: INVALID_LOCATION,
            i_establish_in_city_time_reduction: 0,
            i_escape_boost: 0,
            i_boost_all_spies: 0,
            i_nerf_all_enemy_spies: 0,
            i_visibility_reduction_all_cities: 0,
            b_counter_spy_entire_city: false,
            i_counter_spy_adjacent_level_boost: 0,
            e_escape_route: u32::MAX,
            b_escaping: false,
            e_stolen_boost: u32::MAX,
            i_siphoned_gold: 0,
            i_stolen_great_work_index: -1,
        },
        k_attributes: variants::VariantMap::default(),
        k_operations: OperationQueue {
            nodes: OperationNodes::default(),
        },
        k_operations_disabled: definitions
            .operations
            .iter()
            .map(|&(definition_hash, value)| HashValue {
                definition_hash,
                value,
            })
            .collect(),
        k_hero: Hero {
            e_class: u32::MAX,
            k_origin_city_id: NULL_CITY,
        },
        k_properties: variants::TypedVariantMap::default(),
    })
}

/// Unit.Initialize2c26ea6 assigns GetMaxMoves before charge setters and
/// Religion.Reset. Keep a failed MaxMoves prefix distinct from later fields.
pub fn after_max_moves(unit: &mut Unit, properties: &Properties, max_moves: i32) {
    unit.x_moves_remaining = max_moves.wrapping_shl(8);
    unit.i_build_charges = properties.build_charges;
    unit.i_park_charges = properties.park_charges;
    unit.i_disaster_charges = Some(properties.disaster_charges);
    unit.i_action_charges = properties.action_charges;
    unit.b_exert_zoc = properties.zone_of_control;
    unit.k_religion.i_religious_strength = properties.religious_strength;
    unit.k_religion.i_spread_charges = properties.spread_charges;
    unit.k_religion.i_religious_heal_charges = properties.religious_heal_charges;
    unit.k_religion.i_evict_percent = properties.religion_evict_percent;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_prefix_has_fresh_nested_state_and_definition_sized_arrays() {
        let definitions = Definitions {
            abilities: &[11, 12],
            terrains: &[21],
            features: &[31],
            great_person_classes: &[41],
            promotions: &[51, 52],
            districts: &[61],
            improvements: &[71, 72],
            yields: &[81],
            operations: &[(91, false), (92, true)],
            max_array_items: 64,
        };
        let properties = Properties {
            type_hash: 99,
            combat: 0,
            sight: 2,
            range: 0,
            build_charges: 3,
            park_charges: 0,
            disaster_charges: 0,
            action_charges: 0,
            religious_strength: 0,
            spread_charges: 0,
            religious_heal_charges: 0,
            religion_evict_percent: 0,
            zone_of_control: false,
        };
        let mut unit =
            before_max_moves(4, 0x30002, &properties, &properties, &definitions, 3, 0).unwrap();
        assert_eq!(unit.i_build_charges, 0);
        after_max_moves(&mut unit, &properties, 2);
        assert_eq!(
            (unit.id, unit.e_owner, unit.e_original_owner, unit.e_ai_type),
            (0x30002, 4, 4, 3)
        );
        assert_eq!(unit.formation_id, 0);
        assert_eq!(unit.k_location, INVALID_LOCATION);
        assert_eq!(unit.ai_controller, u32::MAX);
        assert_eq!(
            (
                unit.i_build_charges,
                unit.i_attacks_remaining,
                unit.x_moves_remaining
            ),
            (3, 1, 512)
        );
        assert_eq!(unit.k_experience.i_level, 1);
        assert_eq!(unit.k_experience.x_general_experience_modifier, 256);
        assert_eq!(unit.k_experience.ab_promotions_earned.len(), 2);
        assert_eq!(unit.k_combat_modifers.e_captured_combat_unit_type, u32::MAX);
        assert_eq!(unit.k_hero.k_origin_city_id, NULL_CITY);
        assert_eq!(unit.k_trade.k_origin_trade_city, NULL_CITY);
        assert_eq!(unit.k_espionage.i_operation_end_turn, 0);
        assert_eq!(
            unit.ai_abilities_possessed
                .iter()
                .map(|r| (r.definition_hash, r.value))
                .collect::<Vec<_>>(),
            [(11, 0), (12, 0)]
        );
        assert!(unit.k_operations_disabled[1].value);
        assert_eq!(unit.k_operations.nodes.native_capacity(), Some(0));
        let bytes = unit.encode(crate::unit::Limits::default()).unwrap();
        assert_eq!(
            Unit::decode(&bytes, crate::unit::Limits::default()).unwrap(),
            unit
        );
    }
}
