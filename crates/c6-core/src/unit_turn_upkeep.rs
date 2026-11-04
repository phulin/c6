//! Source-derived Linux XP2 attack replenishment on a retained live Unit.
//!
//! Implements Manager::RestoreAttacksRemaining (0x26af860), GetMaxNumAttacks
//! (0x2c27140), and the local-authority SetAttacksRemaining field/report order
//! (0x2c27eb0). Native archive bookkeeping is excluded under the audited local
//! authority profile. Native differential acceptance, active subscriber census,
//! complete action execution and turn progression remain unimplemented.

use crate::unit::{ComponentId, Unit};

pub const ATTACKS_REMAINING_CHANGED: u32 = 0x35597c21;

/// The original Event::Send(Type, ComponentID const&, int) inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttacksRemainingReport {
    pub event_hash: u32,
    pub identity: ComponentId,
    pub value: i32,
}

/// A context bound to the same retained live actor throughout this call.
///
/// Accessors must borrow the authoritative Unit rather than a scratch copy.
/// Reporting receives the entire context so synchronous subscribers can read or
/// mutate the current actor and its world without later copyback overwriting
/// their changes. No default reporting implementation or RNG assumption exists.
pub trait RestoreAttacksContext {
    type Error;

    fn unit(&self) -> &Unit;
    fn unit_mut(&mut self) -> &mut Unit;

    /// Read the actual configured COMBAT_MAX_NUM_ATTACKS. Missing/unavailable
    /// rules must return Err; actual zero has distinct native semantics.
    fn combat_max_num_attacks(&self) -> Result<i32, Self::Error>;

    /// Pure capability check for complete synchronous delivery of this event.
    /// This is an admission check, not a native effect. Reject unknown required
    /// subscribers/effects here. Must not mutate state or consume RNG.
    fn preflight_attacks_report(&self) -> Result<(), Self::Error>;

    /// Execute all required subscribers in original order, including gameplay
    /// and RNG effects, before returning. A transcript alone is diagnostic only.
    /// Native Send's integer return is ignored by this leaf; Err represents an
    /// execution failure and retains the field write and callback effects.
    fn send_attacks_report(&mut self, report: AttacksRemainingReport) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreAttacksOutcome {
    Unchanged,
    Reported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreAttacksError<E> {
    RuleUnavailable(E),
    ReportUnavailable(E),
    /// The assignment and any completed subscriber effects have already happened.
    ReportFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for RestoreAttacksError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RuleUnavailable(e) => write!(f, "attack rule unavailable: {e}"),
            Self::ReportUnavailable(e) => write!(f, "attack reporting unavailable: {e}"),
            Self::ReportFailed(e) => write!(f, "attack reporting failed after assignment: {e}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for RestoreAttacksError<E> {}

/// Replenish attacks, preserving signed wrapping arithmetic and write-before-send.
///
/// Equal values skip both assignment and reporting capability checks. Missing
/// rules or a failed pure capability check reject before mutation. Once reporting
/// starts this is a non-atomic leaf: report failure preserves the native prefix,
/// and subscriber changes are never overwritten. This does not admit an arena
/// action or establish complete subscriber/native parity.
pub fn restore_attacks_remaining<C: RestoreAttacksContext>(
    context: &mut C,
) -> Result<RestoreAttacksOutcome, RestoreAttacksError<C::Error>> {
    let base = context
        .combat_max_num_attacks()
        .map_err(RestoreAttacksError::RuleUnavailable)?;
    let base = if base == 0 { 1 } else { base };
    let restored = base.wrapping_add(context.unit().k_combat_modifers.i_bonus_attacks);
    if context.unit().i_attacks_remaining == restored {
        return Ok(RestoreAttacksOutcome::Unchanged);
    }
    context
        .preflight_attacks_report()
        .map_err(RestoreAttacksError::ReportUnavailable)?;
    context.unit_mut().i_attacks_remaining = restored;
    // GetComponentID reads the current owner/ID, then the setter rereads attacks.
    // UnitID construction uses ComponentID type 1 and the owner's low 16 bits.
    let identity = {
        let unit = context.unit();
        ComponentId {
            owner: unit.e_owner as i16,
            component_type: 1,
            id: unit.id,
        }
    };
    let value = context.unit().i_attacks_remaining;
    context
        .send_attacks_report(AttacksRemainingReport {
            event_hash: ATTACKS_REMAINING_CHANGED,
            identity,
            value,
        })
        .map_err(RestoreAttacksError::ReportFailed)?;
    Ok(RestoreAttacksOutcome::Reported)
}

#[cfg(test)]
mod tests {
    use super::*;

    // SourceOnly synthetic DTO carrier, adapted from loaded_actions' literal.
    // No initialized engine World, native capture, or native parity is implied.
    fn source_only_unit() -> Unit {
        crate::unit::Unit {
            id: 0,
            e_type_index: 0,
            e_ai_type: 0,
            ai_controller: 0,
            ai_goal: crate::unit::PlotCoord { x: 0, y: 0 },
            k_location: crate::unit::PlotCoord { x: 0, y: 0 },
            e_facing_direction: 0,
            e_owner: 0,
            e_original_owner: 0,
            b_is_levied: false,
            formation_id: 0,
            e_military_formation: 0,
            e_automation: 0,
            e_activity: 0,
            i_damage: 0,
            i_fortify_turns: 0,
            i_upgrade_discount: 0,
            x_moves_remaining: 0,
            i_attacks_remaining: 0,
            i_operation_timer: 0,
            i_build_charges: 0,
            i_park_charges: 0,
            i_action_charges: 0,
            i_air_slot_modifier: 0,
            b_dominant_conquer: false,
            i_barbarian_tribe_index: 0,
            e_barbarian_player_target: 0,
            i_disaster_charges: 0,
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
            b_attack_after_moving: false,
            i_move_after_attacking: 0,
            b_needs_attention: false,
            b_ignore_resource_maintenance: false,
            k_health_modifiers: crate::unit::HealthModifiers {
                i_extra_enemy_heal: 0,
                i_extra_neutral_heal: 0,
                i_extra_friendly_heal: 0,
                b_can_heal_after_action: false,
            },
            k_health_religion_modifiers: crate::unit::HealthReligionModifiers {
                i_extra_enemy_heal_religion: 0,
                i_extra_neutral_heal_religion: 0,
                i_extra_friendly_heal_religion: 0,
                b_can_heal_after_action_religion: false,
            },
            k_combat_modifers: crate::unit::CombatModifiers {
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
                e_captured_combat_unit_type: 0,
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
            k_movement_modifiers: crate::unit::MovementModifiers {
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
            k_sight_modifiers: crate::unit::SightModifiers {
                i_sight_range: 0,
                e_can_see_hidden_type: 0,
            },
            k_attack_range_modifiers: crate::unit::AttackRangeModifiers { i_attack_range: 0 },
            k_sight_plots: Vec::new(),
            ai_abilities_possessed: Vec::new(),
            ab_invalid_terrain_types: Vec::new(),
            ab_ignore_terrain_types: Vec::new(),
            ab_ignore_feature_types: Vec::new(),
            ai_bonus_points_per_kill: Vec::new(),
            ai_bonus_points_per_kill_strength: Vec::new(),
            ai_bypass_walls_promotion_class: Vec::new(),
            ai_enable_wall_attack_promotion_class: Vec::new(),
            k_experience: crate::unit::Experience {
                b_can_promote: false,
                b_experience_locked: false,
                i_experience: 0,
                i_level: 0,
                i_stored_promotions: 0,
                i_promote_no_finish_moves: 0,
                x_general_experience_modifier: 0,
                ab_promotions_earned: Vec::new(),
                s_veteran_name: Vec::new(),
                b_can_promote_notify_needed: false,
                x_attacking_experience_modifier_percent: 0,
            },
            k_great_person: crate::unit::GreatPerson {
                e_individual: 0,
                e_class: 0,
                i_action_charges: 0,
            },
            k_religion: crate::unit::Religion {
                i_religious_strength: 0,
                e_religion_type: 0,
                i_spread_charges: 0,
                i_religious_heal_charges: 0,
                i_nw_deferred_charges: 0,
                e_initiation_yield: 0,
                i_initiation_amount: 0,
                i_evict_percent: 0,
                i_foreign_spread_modifier: 0,
                b_converts_barbarians: false,
                b_no_foreign_spread: false,
                b_relic_upon_death: false,
                b_land_victory_spread: false,
                i_land_victory_spread: 0,
                i_plunder_improvement_faith_yield: 0,
                i_plunder_district_faith_yield: 0,
                a_initiation_yield_population: Vec::new(),
                e_relic_source: 0,
            },
            k_rock_band: crate::unit::RockBand {
                b_max_level: false,
                b_max_promotions: false,
                i_level: 0,
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
                a_modified_yield_per_tourism_bomb: Vec::new(),
                ai_concert_plots: Vec::new(),
                ai_modified_rockband_level_district: Vec::new(),
                ai_modified_rockband_level_improvement: Vec::new(),
                ai_modified_tourism_bomb_value_district: Vec::new(),
                ai_modified_tourism_bomb_value_improvement: Vec::new(),
            },
            k_trade: crate::unit::Trade {
                k_origin_trade_city: crate::unit::ComponentId {
                    owner: 0,
                    component_type: 0,
                    id: 0,
                },
                k_destination_trade_city: crate::unit::ComponentId {
                    owner: 0,
                    component_type: 0,
                    id: 0,
                },
                v_domain_plunder_immunities: [0; 5],
            },
            k_archaeology: crate::unit::Archaeology {
                k_home_city_id: 0,
                e_home_building: 0,
                i_artifact_index: 0,
                i_great_work_index: 0,
                b_can_extract_sea_artifacts: false,
                b_choosing_artifact: false,
            },
            k_espionage: crate::unit::Espionage {
                a_espionage_bonuses: Vec::new(),
                a_espionage_time_reductions: Vec::new(),
                e_operation: 0,
                i_operation_end_turn: 0,
                k_operation_end_destination: crate::unit::PlotCoord { x: 0, y: 0 },
                i_establish_in_city_time_reduction: 0,
                i_escape_boost: 0,
                i_boost_all_spies: 0,
                i_nerf_all_enemy_spies: 0,
                i_visibility_reduction_all_cities: 0,
                b_counter_spy_entire_city: false,
                i_counter_spy_adjacent_level_boost: 0,
                e_escape_route: 0,
                b_escaping: false,
                e_stolen_boost: 0,
                i_siphoned_gold: 0,
                i_stolen_great_work_index: 0,
            },
            k_attributes: crate::unit::variants::VariantMap {
                scalars: Vec::new(),
                arrays: Vec::new(),
            },
            k_operations: crate::unit::OperationQueue { nodes: Vec::new() },
            k_operations_disabled: Vec::new(),
            k_hero: crate::unit::Hero {
                e_class: 0,
                k_origin_city_id: crate::unit::ComponentId {
                    owner: 0,
                    component_type: 0,
                    id: 0,
                },
            },
            k_properties: crate::unit::variants::TypedVariantMap {
                header: Default::default(),
                entries: Vec::new(),
            },
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Failure {
        MissingRule,
        UnknownSubscribers,
        Subscriber,
    }

    struct SourceOnlyContext {
        actor: Unit,
        rule: Result<i32, Failure>,
        report_available: bool,
        fail_report: bool,
        mutate_report: bool,
        reports: Vec<AttacksRemainingReport>,
        world_counter: u32,
    }

    impl SourceOnlyContext {
        fn new(base: i32, bonus: i32, current: i32) -> Self {
            let mut actor = source_only_unit();
            actor.id = 0xfedcba98;
            actor.e_owner = -32769;
            actor.i_damage = 43;
            actor.i_attacks_remaining = current;
            actor.k_combat_modifers.i_bonus_attacks = bonus;
            Self {
                actor,
                rule: Ok(base),
                report_available: true,
                fail_report: false,
                mutate_report: false,
                reports: Vec::new(),
                world_counter: 0,
            }
        }
    }

    impl RestoreAttacksContext for SourceOnlyContext {
        type Error = Failure;
        fn unit(&self) -> &Unit {
            &self.actor
        }
        fn unit_mut(&mut self) -> &mut Unit {
            &mut self.actor
        }
        fn combat_max_num_attacks(&self) -> Result<i32, Failure> {
            self.rule
        }
        fn preflight_attacks_report(&self) -> Result<(), Failure> {
            if self.report_available {
                Ok(())
            } else {
                Err(Failure::UnknownSubscribers)
            }
        }
        fn send_attacks_report(&mut self, report: AttacksRemainingReport) -> Result<(), Failure> {
            // Synchronous subscriber sees the authoritative field already written
            // and the current actor identity, before making world/actor changes.
            assert_eq!(self.actor.i_attacks_remaining, report.value);
            assert_eq!(report.identity.id, self.actor.id);
            assert_eq!(report.identity.owner, self.actor.e_owner as i16);
            assert_eq!(report.identity.component_type, 1);
            assert_eq!(report.event_hash, 0x35597c21);
            self.reports.push(report);
            self.world_counter += 1;
            if self.mutate_report {
                self.actor.i_attacks_remaining = -901;
                self.actor.k_combat_modifers.i_bonus_attacks = 77;
                self.actor.id = 99;
                self.actor.e_owner = -7;
                self.actor.i_damage = 64;
            }
            if self.fail_report {
                Err(Failure::Subscriber)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn source_only_leaf_signed_rules_wrap_and_complete_effect_order() {
        for (base, bonus, expected) in [
            (0, 2, 3),
            (4, 2, 6),
            (-4, 2, -2),
            (0, -5, -4),
            (i32::MAX, 1, i32::MIN),
            (i32::MIN, -1, i32::MAX),
        ] {
            let mut context = SourceOnlyContext::new(base, bonus, 19);
            let mut expected_unit = context.actor.clone();
            expected_unit.i_attacks_remaining = expected;
            assert_eq!(
                restore_attacks_remaining(&mut context),
                Ok(RestoreAttacksOutcome::Reported)
            );
            assert_eq!(context.actor, expected_unit);
            assert_eq!(context.world_counter, 1);
            assert_eq!(
                context.reports,
                [AttacksRemainingReport {
                    event_hash: 0x35597c21,
                    identity: ComponentId {
                        owner: 32767,
                        component_type: 1,
                        id: 0xfedcba98
                    },
                    value: expected,
                }]
            );
        }
    }

    #[test]
    fn source_only_unchanged_skips_assignment_and_unavailable_reporting() {
        let mut context = SourceOnlyContext::new(0, -3, -2);
        context.report_available = false;
        let before = context.actor.clone();
        assert_eq!(
            restore_attacks_remaining(&mut context),
            Ok(RestoreAttacksOutcome::Unchanged)
        );
        assert_eq!(context.actor, before);
        assert!(context.reports.is_empty());
        assert_eq!(context.world_counter, 0);
    }

    #[test]
    fn source_only_missing_rule_and_pure_preflight_refuse_before_mutation() {
        let mut context = SourceOnlyContext::new(0, 2, 19);
        context.rule = Err(Failure::MissingRule);
        let before = context.actor.clone();
        assert_eq!(
            restore_attacks_remaining(&mut context),
            Err(RestoreAttacksError::RuleUnavailable(Failure::MissingRule))
        );
        assert_eq!(context.actor, before);
        context.rule = Ok(0);
        context.report_available = false;
        assert_eq!(
            restore_attacks_remaining(&mut context),
            Err(RestoreAttacksError::ReportUnavailable(
                Failure::UnknownSubscribers
            ))
        );
        assert_eq!(context.actor, before);
        assert!(context.reports.is_empty());
        assert_eq!(context.world_counter, 0);
    }

    #[test]
    fn source_only_callback_mutations_survive_success_and_failure() {
        for fail_report in [false, true] {
            let mut context = SourceOnlyContext::new(2, 3, 19);
            context.mutate_report = true;
            context.fail_report = fail_report;
            let mut expected_unit = context.actor.clone();
            expected_unit.i_attacks_remaining = -901;
            expected_unit.k_combat_modifers.i_bonus_attacks = 77;
            expected_unit.id = 99;
            expected_unit.e_owner = -7;
            expected_unit.i_damage = 64;
            let result = restore_attacks_remaining(&mut context);
            assert_eq!(
                result,
                if fail_report {
                    Err(RestoreAttacksError::ReportFailed(Failure::Subscriber))
                } else {
                    Ok(RestoreAttacksOutcome::Reported)
                }
            );
            assert_eq!(context.actor, expected_unit);
            assert_eq!(context.world_counter, 1);
            assert_eq!(context.reports[0].value, 5);
            assert_eq!(context.reports[0].identity.id, 0xfedcba98);
        }
    }

    #[test]
    fn source_only_failed_report_retains_assignment_and_prior_world_effects() {
        let mut context = SourceOnlyContext::new(3, 4, 19);
        context.fail_report = true;
        let mut expected_unit = context.actor.clone();
        expected_unit.i_attacks_remaining = 7;
        assert_eq!(
            restore_attacks_remaining(&mut context),
            Err(RestoreAttacksError::ReportFailed(Failure::Subscriber))
        );
        assert_eq!(context.actor, expected_unit);
        assert_eq!(context.world_counter, 1);
        assert_eq!(context.reports.len(), 1);
    }
}
