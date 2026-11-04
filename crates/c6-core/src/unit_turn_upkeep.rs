//! Source-derived Linux XP2 attack and movement replenishment on a live Unit.
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
/// their changes. Accessors return an error when the current retained record is
/// unavailable; implementations must not panic on invalidated handles. No default
/// reporting implementation or RNG assumption exists.
pub trait RestoreAttacksContext {
    type Error;

    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;

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
    UnitUnavailable(E),
    /// Assignment completed; subsequent access to the retained unit failed.
    UnitAccessFailed(E),
    RuleUnavailable(E),
    ReportUnavailable(E),
    /// The assignment and any completed subscriber effects have already happened.
    ReportFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for RestoreAttacksError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnitUnavailable(e) => write!(f, "current attack unit unavailable: {e}"),
            Self::UnitAccessFailed(e) => {
                write!(f, "current attack unit access failed after assignment: {e}")
            }
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
    let restored = base.wrapping_add(
        context
            .unit()
            .map_err(RestoreAttacksError::UnitUnavailable)?
            .k_combat_modifers
            .i_bonus_attacks,
    );
    if context
        .unit()
        .map_err(RestoreAttacksError::UnitUnavailable)?
        .i_attacks_remaining
        == restored
    {
        return Ok(RestoreAttacksOutcome::Unchanged);
    }
    context
        .preflight_attacks_report()
        .map_err(RestoreAttacksError::ReportUnavailable)?;
    context
        .unit_mut()
        .map_err(RestoreAttacksError::UnitUnavailable)?
        .i_attacks_remaining = restored;
    // GetComponentID reads the current owner/ID, then the setter rereads attacks.
    // UnitID construction uses ComponentID type 1 and the owner's low 16 bits.
    let identity = {
        let unit = context
            .unit()
            .map_err(RestoreAttacksError::UnitAccessFailed)?;
        ComponentId {
            owner: unit.e_owner as i16,
            component_type: 1,
            id: unit.id,
        }
    };
    let value = context
        .unit()
        .map_err(RestoreAttacksError::UnitAccessFailed)?
        .i_attacks_remaining;
    context
        .send_attacks_report(AttacksRemainingReport {
            event_hash: ATTACKS_REMAINING_CHANGED,
            identity,
            value,
        })
        .map_err(RestoreAttacksError::ReportFailed)?;
    Ok(RestoreAttacksOutcome::Reported)
}

/// Linux movement events carry raw signed Q8, never rounded whole moves.
pub const MOVES_REMAINING_CHANGED: u32 = 0xc50d66d6;
pub const MOVES_EXHAUSTED: u32 = 0xe992f3cd;
pub const MOVEMENT_RESTORED: u32 = 0x6e8a2f93;
pub const ACTIVITY_HOLD: i32 = 0x5ff1bbdf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovementReport {
    pub event_hash: u32,
    pub identity: ComponentId,
    /// None is the exhausted event's identity-only overload.
    pub raw_q8: Option<i32>,
}

/// Same authoritative actor/world contract as RestoreAttacksContext.
/// Native null input is represented by None at the function boundary. Accessors
/// return errors for unavailable current records, including after callbacks.
pub trait RestoreMovementContext {
    type Error;
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;

    /// Query actual current Linux MaxMoves in whole moves. The data-layer
    /// implementation is c6_data::unit_movement_state::unit_max_moves; bindings
    /// must delegate that exact rules/world/diplomacy query, not a base-move
    /// approximation or a caller-invented maximum. Missing inputs return Err.
    fn max_moves(&self) -> Result<i32, Self::Error>;

    /// Pure admission check, before any write/RNG. Check the owner query and,
    /// when movement_branch is true, all reachable synchronous subscribers and
    /// current-formation updates, including formations introduced by callbacks.
    /// Unknown required effects reject; this is not a native event.
    fn preflight_restore_movement(&self, movement_branch: bool) -> Result<(), Self::Error>;

    /// Complete synchronous delivery, including gameplay/RNG in native order.
    /// Subscribers may mutate the same Unit, owner, formation and world.
    fn send_movement_report(&mut self, report: MovementReport) -> Result<(), Self::Error>;

    /// At this exact point read current EditFormation (+0xbc0); if nonnull,
    /// reread EditFormation and run SyncMovesRemaining -> UpdateAttributes.
    /// That updates actual formation attributes/domain/movement and reports
    /// changed formation movement for each current member in original order.
    /// A validated absent formation needs no update; unknown state is Err.
    /// This is not equivalent to logging a sync or copying this Unit's moves.
    fn sync_current_formation(&mut self) -> Result<(), Self::Error>;

    /// Resolve the current owner's actual Player::IsBarbarian after reporting.
    fn owner_is_barbarian(&self) -> Result<bool, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementStage {
    UnitReadback,
    HealingEligibility,
    ChangedReport,
    ExhaustedReport,
    FormationSync,
    RestoredReport,
    OwnerQuery,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreMovementError<E> {
    UnitUnavailable(E),
    MaxMovesUnavailable(E),
    EffectsUnavailable(E),
    /// The actual field assignment and completed callbacks remain visible.
    /// No rollback, successful action, or complete turn is implied.
    ContinuationFailed {
        stage: MovementStage,
        error: E,
    },
}
impl<E: std::fmt::Display> std::fmt::Display for RestoreMovementError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnitUnavailable(e) => write!(f, "current movement unit unavailable: {e}"),
            Self::MaxMovesUnavailable(e) => write!(f, "maximum movement unavailable: {e}"),
            Self::EffectsUnavailable(e) => write!(f, "movement effects unavailable: {e}"),
            Self::ContinuationFailed { stage, error } => {
                write!(f, "movement continuation failed at {stage:?}: {error}")
            }
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for RestoreMovementError<E> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreMovementOutcome {
    NullUnit,
    /// The maximum matched; healing eligibility was still assigned.
    MovementUnchanged,
    /// The outer restoration event completed; callbacks own subsequent state.
    Restored,
}

fn movement_identity(unit: &Unit) -> ComponentId {
    ComponentId {
        owner: unit.e_owner as i16,
        component_type: 1,
        id: unit.id,
    }
}

/// Manager::RestoreMovement (0x26af6c0), including the local-authority writes of
/// ChangeMovesRemaining (0x2c277f0) and SetEligibleToHeal (0x26b0c90).
/// Native archive dirty-tree bookkeeping is excluded under the audited profile.
/// Formation/report continuations must execute fully; this leaf establishes
/// neither initialized-world/native parity nor complete activation/turn support.
pub fn restore_movement<C: RestoreMovementContext>(
    context: Option<&mut C>,
) -> Result<RestoreMovementOutcome, RestoreMovementError<C::Error>> {
    let Some(context) = context else {
        return Ok(RestoreMovementOutcome::NullUnit);
    };
    let maximum = context
        .max_moves()
        .map_err(RestoreMovementError::MaxMovesUnavailable)?
        .wrapping_shl(8);
    let branch = maximum
        != context
            .unit()
            .map_err(RestoreMovementError::UnitUnavailable)?
            .x_moves_remaining;
    context
        .preflight_restore_movement(branch)
        .map_err(RestoreMovementError::EffectsUnavailable)?;
    if branch {
        let delta = maximum.wrapping_sub(
            context
                .unit()
                .map_err(RestoreMovementError::UnitUnavailable)?
                .x_moves_remaining,
        );
        let changed = context
            .unit()
            .map_err(RestoreMovementError::UnitUnavailable)?
            .x_moves_remaining
            .wrapping_add(delta)
            .max(0);
        if changed
            != context
                .unit()
                .map_err(RestoreMovementError::UnitUnavailable)?
                .x_moves_remaining
        {
            context
                .unit_mut()
                .map_err(RestoreMovementError::UnitUnavailable)?
                .x_moves_remaining = changed;
            let report = MovementReport {
                event_hash: MOVES_REMAINING_CHANGED,
                identity: movement_identity(context.unit().map_err(|error| {
                    RestoreMovementError::ContinuationFailed {
                        stage: MovementStage::UnitReadback,
                        error,
                    }
                })?),
                raw_q8: Some(
                    context
                        .unit()
                        .map_err(|error| RestoreMovementError::ContinuationFailed {
                            stage: MovementStage::UnitReadback,
                            error,
                        })?
                        .x_moves_remaining,
                ),
            };
            context.send_movement_report(report).map_err(|error| {
                RestoreMovementError::ContinuationFailed {
                    stage: MovementStage::ChangedReport,
                    error,
                }
            })?;
            // Test current field after synchronous subscribers, not changed/max.
            if context
                .unit()
                .map_err(|error| RestoreMovementError::ContinuationFailed {
                    stage: MovementStage::UnitReadback,
                    error,
                })?
                .x_moves_remaining
                == 0
            {
                let report = MovementReport {
                    event_hash: MOVES_EXHAUSTED,
                    identity: movement_identity(context.unit().map_err(|error| {
                        RestoreMovementError::ContinuationFailed {
                            stage: MovementStage::UnitReadback,
                            error,
                        }
                    })?),
                    raw_q8: None,
                };
                context.send_movement_report(report).map_err(|error| {
                    RestoreMovementError::ContinuationFailed {
                        stage: MovementStage::ExhaustedReport,
                        error,
                    }
                })?;
            }
            context.sync_current_formation().map_err(|error| {
                RestoreMovementError::ContinuationFailed {
                    stage: MovementStage::FormationSync,
                    error,
                }
            })?;
        }
        // Outer report uses cached maximum, even after clamping or subscribers.
        let report = MovementReport {
            event_hash: MOVEMENT_RESTORED,
            identity: movement_identity(context.unit().map_err(|error| {
                RestoreMovementError::ContinuationFailed {
                    stage: MovementStage::UnitReadback,
                    error,
                }
            })?),
            raw_q8: Some(maximum),
        };
        context.send_movement_report(report).map_err(|error| {
            RestoreMovementError::ContinuationFailed {
                stage: MovementStage::RestoredReport,
                error,
            }
        })?;
    }
    let barbarian =
        context
            .owner_is_barbarian()
            .map_err(|error| RestoreMovementError::ContinuationFailed {
                stage: MovementStage::OwnerQuery,
                error,
            })?;
    let unit = context
        .unit()
        .map_err(|error| RestoreMovementError::ContinuationFailed {
            stage: MovementStage::HealingEligibility,
            error,
        })?;
    let eligible = !barbarian
        && (unit.e_activity != ACTIVITY_HOLD || unit.k_health_modifiers.b_can_heal_after_action);
    context
        .unit_mut()
        .map_err(|error| RestoreMovementError::ContinuationFailed {
            stage: MovementStage::HealingEligibility,
            error,
        })?
        .b_eligible_to_heal = eligible;
    Ok(if branch {
        RestoreMovementOutcome::Restored
    } else {
        RestoreMovementOutcome::MovementUnchanged
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // SourceOnly synthetic DTO carrier, adapted from loaded_actions' literal.
    // No initialized engine World, native capture, or native parity is implied.
    pub(crate) fn source_only_unit() -> Unit {
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
        MissingUnit,
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
        invalidate_after_write: bool,
        unit_available: bool,
        write_available: bool,
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
                invalidate_after_write: false,
                unit_available: true,
                write_available: true,
            }
        }
    }

    impl RestoreAttacksContext for SourceOnlyContext {
        type Error = Failure;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            self.unit_available
                .then_some(&self.actor)
                .ok_or(Failure::MissingUnit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            if !self.write_available {
                return Err(Failure::MissingUnit);
            }
            if self.invalidate_after_write {
                self.unit_available = false;
            }
            Ok(&mut self.actor)
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
    #[test]
    fn attacks_missing_read_or_write_rejects_without_assignment() {
        for write_failure in [false, true] {
            let mut c = SourceOnlyContext::new(2, 0, 19);
            c.unit_available = write_failure;
            c.write_available = !write_failure;
            let before = c.actor.clone();
            assert_eq!(
                restore_attacks_remaining(&mut c),
                Err(RestoreAttacksError::UnitUnavailable(Failure::MissingUnit))
            );
            assert_eq!(c.actor, before);
            assert!(c.reports.is_empty());
        }
    }
    #[test]
    fn attacks_post_assignment_access_failure_preserves_field_without_reporting() {
        let mut c = SourceOnlyContext::new(2, 0, 19);
        c.invalidate_after_write = true;
        assert_eq!(
            restore_attacks_remaining(&mut c),
            Err(RestoreAttacksError::UnitAccessFailed(Failure::MissingUnit))
        );
        assert_eq!(c.actor.i_attacks_remaining, 2);
        assert!(c.reports.is_empty());
    }

    struct MovementContext {
        actor: Unit,
        unit_available: bool,
        write_available: bool,
        invalidate_changed: bool,
        fail_healing_write: bool,
        maximum: Result<i32, &'static str>,
        admitted: bool,
        barbarian: bool,
        mutate_changed: bool,
        fail: Option<MovementStage>,
        reports: Vec<MovementReport>,
        order: Vec<&'static str>,
    }
    impl MovementContext {
        fn new(maximum: i32, current: i32) -> Self {
            let mut actor = source_only_unit();
            actor.x_moves_remaining = current;
            actor.i_damage = 43;
            actor.i_attacks_remaining = 7;
            Self {
                actor,
                unit_available: true,
                write_available: true,
                invalidate_changed: false,
                fail_healing_write: false,
                maximum: Ok(maximum),
                admitted: true,
                barbarian: false,
                mutate_changed: false,
                fail: None,
                reports: vec![],
                order: vec![],
            }
        }
    }
    impl RestoreMovementContext for MovementContext {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            self.unit_available
                .then_some(&self.actor)
                .ok_or("missing current unit")
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            if !self.write_available || (self.fail_healing_write && self.reports.len() == 2) {
                Err("current unit write unavailable")
            } else {
                Ok(&mut self.actor)
            }
        }
        fn max_moves(&self) -> Result<i32, Self::Error> {
            self.maximum
        }
        fn preflight_restore_movement(&self, _: bool) -> Result<(), Self::Error> {
            if self.admitted {
                Ok(())
            } else {
                Err("unknown effects")
            }
        }
        fn send_movement_report(&mut self, report: MovementReport) -> Result<(), Self::Error> {
            assert_eq!(report.identity, movement_identity(&self.actor));
            let stage = match report.event_hash {
                MOVES_REMAINING_CHANGED => {
                    assert_eq!(report.raw_q8, Some(self.actor.x_moves_remaining));
                    self.order.push("changed");
                    if self.invalidate_changed {
                        self.unit_available = false;
                    }
                    if self.mutate_changed {
                        self.actor.x_moves_remaining = 0;
                        self.actor.id = 77;
                        self.actor.e_owner = -7;
                        self.actor.e_activity = ACTIVITY_HOLD;
                    }
                    MovementStage::ChangedReport
                }
                MOVES_EXHAUSTED => {
                    self.order.push("zero");
                    MovementStage::ExhaustedReport
                }
                MOVEMENT_RESTORED => {
                    self.order.push("restored");
                    MovementStage::RestoredReport
                }
                _ => panic!("unexpected event"),
            };
            self.reports.push(report);
            if self.fail == Some(stage) {
                Err("subscriber failed")
            } else {
                Ok(())
            }
        }
        fn sync_current_formation(&mut self) -> Result<(), Self::Error> {
            self.order.push("formation");
            // Synthetic required continuation stands for fully checked absent
            // formation, or a failing admitted provider. No native parity claim.
            if self.fail == Some(MovementStage::FormationSync) {
                self.actor.i_damage = 88;
                Err("formation failed")
            } else {
                Ok(())
            }
        }
        fn owner_is_barbarian(&self) -> Result<bool, Self::Error> {
            if self.fail == Some(MovementStage::OwnerQuery) {
                Err("owner missing")
            } else {
                Ok(self.barbarian)
            }
        }
    }

    #[test]
    fn movement_actual_fields_order_and_reentrant_readback() {
        let mut c = MovementContext::new(3, 17);
        c.mutate_changed = true;
        assert_eq!(
            restore_movement(Some(&mut c)),
            Ok(RestoreMovementOutcome::Restored)
        );
        assert_eq!(c.order, ["changed", "zero", "formation", "restored"]);
        assert_eq!(c.reports[0].raw_q8, Some(768));
        assert_eq!(c.reports[1].raw_q8, None);
        assert_eq!(c.reports[1].identity.id, 77);
        assert_eq!(c.reports[2].raw_q8, Some(768));
        assert_eq!(c.actor.x_moves_remaining, 0);
        assert!(!c.actor.b_eligible_to_heal); // callback changed activity
        assert_eq!(c.actor.i_damage, 43);
        assert_eq!(c.actor.i_attacks_remaining, 7);
    }

    #[test]
    fn movement_equal_still_updates_healing_and_null_skips_everything() {
        assert_eq!(
            restore_movement::<MovementContext>(None),
            Ok(RestoreMovementOutcome::NullUnit)
        );
        for (barbarian, activity, after_action, expected) in [
            (false, 0, false, true),
            (true, 0, true, false),
            (false, ACTIVITY_HOLD, false, false),
            (false, ACTIVITY_HOLD, true, true),
            (true, ACTIVITY_HOLD, true, false),
        ] {
            let mut c = MovementContext::new(2, 512);
            c.barbarian = barbarian;
            c.actor.e_activity = activity;
            c.actor.k_health_modifiers.b_can_heal_after_action = after_action;
            c.actor
                .k_health_religion_modifiers
                .b_can_heal_after_action_religion = !after_action;
            assert_eq!(
                restore_movement(Some(&mut c)),
                Ok(RestoreMovementOutcome::MovementUnchanged)
            );
            assert_eq!(c.actor.b_eligible_to_heal, expected);
            assert!(c.order.is_empty());
        }
    }

    #[test]
    fn movement_signed_wrap_clamp_and_outer_report_on_inner_no_change() {
        for (maximum, current, expected, events) in [
            (-1, 0, 0, vec!["restored"]),
            (-1, 5, 0, vec!["changed", "zero", "formation", "restored"]),
            (0, -1, 0, vec!["changed", "zero", "formation", "restored"]),
            (
                i32::MAX,
                i32::MIN,
                0,
                vec!["changed", "zero", "formation", "restored"],
            ),
            (
                0x01000001,
                i32::MIN,
                256,
                vec!["changed", "formation", "restored"],
            ),
        ] {
            let mut c = MovementContext::new(maximum, current);
            restore_movement(Some(&mut c)).unwrap();
            assert_eq!(c.actor.x_moves_remaining, expected);
            assert_eq!(c.order, events);
            assert_eq!(
                c.reports.last().unwrap().raw_q8,
                Some(maximum.wrapping_shl(8))
            );
        }
    }

    #[test]
    fn movement_rejects_unavailable_inputs_before_writes() {
        let mut c = MovementContext::new(3, 1);
        let before = c.actor.clone();
        c.maximum = Err("unbound exact query");
        assert_eq!(
            restore_movement(Some(&mut c)),
            Err(RestoreMovementError::MaxMovesUnavailable(
                "unbound exact query"
            ))
        );
        assert_eq!(c.actor, before);
        c.maximum = Ok(3);
        c.admitted = false;
        assert_eq!(
            restore_movement(Some(&mut c)),
            Err(RestoreMovementError::EffectsUnavailable("unknown effects"))
        );
        assert_eq!(c.actor, before);
        assert!(c.order.is_empty());
    }

    #[test]
    fn movement_continuation_failure_keeps_exact_completed_prefix() {
        for (stage, order) in [
            (MovementStage::ChangedReport, vec!["changed"]),
            (MovementStage::ExhaustedReport, vec!["changed", "zero"]),
            (
                MovementStage::FormationSync,
                vec!["changed", "zero", "formation"],
            ),
            (
                MovementStage::RestoredReport,
                vec!["changed", "zero", "formation", "restored"],
            ),
            (
                MovementStage::OwnerQuery,
                vec!["changed", "zero", "formation", "restored"],
            ),
        ] {
            let mut c = MovementContext::new(3, 1);
            c.mutate_changed = true;
            c.actor.b_eligible_to_heal = true;
            c.fail = Some(stage);
            let error = restore_movement(Some(&mut c)).unwrap_err();
            assert!(
                matches!(error, RestoreMovementError::ContinuationFailed { stage: actual, .. } if actual == stage)
            );
            assert_eq!(c.order, order);
            assert_eq!(c.actor.x_moves_remaining, 0);
            assert_eq!(c.actor.id, 77);
            assert!(c.actor.b_eligible_to_heal); // final assignment not reached
            assert_eq!(
                c.actor.i_damage,
                if stage == MovementStage::FormationSync {
                    88
                } else {
                    43
                }
            );
        }
    }
    #[test]
    fn movement_missing_access_preserves_state_before_assignment_and_callback_prefix_after() {
        for write_failure in [false, true] {
            let mut c = MovementContext::new(3, 1);
            c.unit_available = write_failure;
            c.write_available = !write_failure;
            let before = c.actor.clone();
            assert!(matches!(
                restore_movement(Some(&mut c)),
                Err(RestoreMovementError::UnitUnavailable(_))
            ));
            assert_eq!(c.actor, before);
            assert!(c.reports.is_empty());
        }
        let mut c = MovementContext::new(3, 1);
        c.invalidate_changed = true;
        c.mutate_changed = true;
        assert_eq!(
            restore_movement(Some(&mut c)),
            Err(RestoreMovementError::ContinuationFailed {
                stage: MovementStage::UnitReadback,
                error: "missing current unit"
            })
        );
        assert_eq!(c.order, ["changed"]);
        assert_eq!(c.actor.x_moves_remaining, 0);
        assert_eq!(c.actor.id, 77);
        assert!(!c.actor.b_eligible_to_heal);
    }

    #[test]
    fn movement_final_write_failure_keeps_completed_reports_and_healing_value() {
        let mut c = MovementContext::new(3, 1);
        c.fail_healing_write = true;
        assert_eq!(
            restore_movement(Some(&mut c)),
            Err(RestoreMovementError::ContinuationFailed {
                stage: MovementStage::HealingEligibility,
                error: "current unit write unavailable"
            })
        );
        assert_eq!(c.order, ["changed", "formation", "restored"]);
        assert_eq!(c.actor.x_moves_remaining, 768);
        assert!(!c.actor.b_eligible_to_heal);
    }
}
