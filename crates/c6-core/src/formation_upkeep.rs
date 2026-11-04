//! Linux XP2 Formation::UpdateAttributes (0x293c830) and SyncMovesRemaining.
//!
//! Computes the real Formation fields and ordered member reports. World/rules
//! getters and synchronous subscribers remain explicit dependencies. This is a
//! source-derived continuation, not native differential acceptance or a turn.
use crate::unit::{ComponentId, Formation, Unit};
use crate::unit_turn_upkeep::{MovementReport, ACTIVITY_HOLD, MOVES_REMAINING_CHANGED};

/// Remaining unresolved native queries. Combat means GetCombat(false).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerQuery {
    RangedCombat,
    Combat,
    BombardCombat,
    Domain,
    /// Actual Linux MaxMoves; bind c6-data's unit_movement_state::unit_max_moves.
    MaxMoves,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanQuery {
    CanEmbark,
    CanAttack,
    MovementDisabledThisTurn,
}

/// Accessors borrow the authoritative formation/world, never a scratch copy.
/// Handles retain the actual found Unit throughout its member visit even when
/// callbacks change owner tables. Queries execute lazily, in native order.
///
/// Preflight must admit all reachable getter/subscriber gameplay and RNG effects
/// and valid native member iterators (no invalidating reallocation/erasure).
/// Unknown effects reject before resets. Reentrant callbacks may mutate current
/// members and fields; completed effects survive subsequent failure. Accessors
/// return errors for unavailable retained records, including after callbacks;
/// invalidated handles must not panic.
pub trait FormationUpkeepContext {
    type Error;
    type UnitHandle: Copy;
    type Definitions: Copy;
    type Units: Copy;
    fn formation(&self) -> Result<&Formation, Self::Error>;
    fn formation_mut(&mut self) -> Result<&mut Formation, Self::Error>;
    fn preflight_update_attributes(&self) -> Result<(), Self::Error>;
    /// The actual definitions collection, obtained after initial field resets.
    fn definitions(&mut self) -> Result<Self::Definitions, Self::Error>;
    /// Current bound formation owner's actual Units collection.
    fn units(&mut self) -> Result<Self::Units, Self::Error>;
    /// Find on the retained collection. Unknown is Err; proven null is Ok(None).
    fn find_unit(
        &mut self,
        units: Self::Units,
        id: u32,
    ) -> Result<Option<Self::UnitHandle>, Self::Error>;
    fn unit(&self, handle: Self::UnitHandle) -> Result<&Unit, Self::Error>;
    fn integer(
        &mut self,
        handle: Self::UnitHandle,
        query: IntegerQuery,
    ) -> Result<i32, Self::Error>;
    fn boolean(
        &mut self,
        handle: Self::UnitHandle,
        query: BooleanQuery,
    ) -> Result<bool, Self::Error>;
    /// Current native runtime GetType (+0xd0), distinct from the saved type hash.
    fn native_type_index(&mut self, handle: Self::UnitHandle) -> Result<u32, Self::Error>;
    /// At(type_index), then IsCanCapture when nonnull. None means actual missing
    /// definition, which *clears* the accumulated capture flag.
    fn definition_can_capture(
        &mut self,
        definitions: Self::Definitions,
        type_index: u32,
    ) -> Result<Option<bool>, Self::Error>;
    /// Full synchronous delivery, including all reached gameplay/RNG effects.
    fn send_member_movement(&mut self, report: MovementReport) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormationUpkeepError<E> {
    EffectsUnavailable(E),
    /// Initial resets and all completed writes/callbacks remain visible.
    ContinuationFailed(E),
    /// UpdateAttributes/reporting dereferences native Find without a null check.
    /// Reject explicitly rather than inventing a skipped member.
    NullMember {
        index: usize,
        id: u32,
    },
    InvalidatedMemberCursor {
        index: usize,
    },
}

fn is_embarked(unit: &Unit) -> bool {
    unit.e_activity != ACTIVITY_HOLD && unit.b_embarked
}

fn member<C: FormationUpkeepContext>(
    c: &C,
    index: usize,
) -> Result<Option<u32>, FormationUpkeepError<C::Error>> {
    let ids = &c
        .formation()
        .map_err(FormationUpkeepError::ContinuationFailed)?
        .unit_ids;
    if index == ids.len() {
        Ok(None)
    } else {
        ids.get(index)
            .copied()
            .map(Some)
            .ok_or(FormationUpkeepError::InvalidatedMemberCursor { index })
    }
}
fn required<C: FormationUpkeepContext>(
    c: &mut C,
    index: usize,
    id: u32,
) -> Result<C::UnitHandle, FormationUpkeepError<C::Error>> {
    let units = c
        .units()
        .map_err(FormationUpkeepError::ContinuationFailed)?;
    c.find_unit(units, id)
        .map_err(FormationUpkeepError::ContinuationFailed)?
        .ok_or(FormationUpkeepError::NullMember { index, id })
}

/// Source-closed GetFormationsMoves<FixedPointT<8>> (0x293eb30), instantiated
/// with GetMovementMovesRemaining (0x2c27740). The movement fold caches End,
/// unlike attribute/report loops. Null movement members alone are skipped.
fn formation_moves<C: FormationUpkeepContext>(
    c: &mut C,
) -> Result<i32, FormationUpkeepError<C::Error>> {
    let units = c
        .units()
        .map_err(FormationUpkeepError::ContinuationFailed)?;
    let end = c
        .formation()
        .map_err(FormationUpkeepError::ContinuationFailed)?
        .unit_ids
        .len();
    let mut minimum = 9001_i32.wrapping_shl(8);
    let mut escort_maximum = 0;
    for index in 0..end {
        let id = c
            .formation()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .unit_ids
            .get(index)
            .copied()
            .ok_or(FormationUpkeepError::InvalidatedMemberCursor { index })?;
        if let Some(unit) = c
            .find_unit(units, id)
            .map_err(FormationUpkeepError::ContinuationFailed)?
        {
            let disabled = c
                .boolean(unit, BooleanQuery::MovementDisabledThisTurn)
                .map_err(FormationUpkeepError::ContinuationFailed)?;
            let moves = if disabled {
                0
            } else {
                c.unit(unit)
                    .map_err(FormationUpkeepError::ContinuationFailed)?
                    .x_moves_remaining
            };
            minimum = minimum.min(moves);
            if c.unit(unit)
                .map_err(FormationUpkeepError::ContinuationFailed)?
                .k_movement_modifiers
                .b_escort_mobility
            {
                escort_maximum = escort_maximum.max(moves);
            }
        }
    }
    Ok(minimum.max(escort_maximum))
}

/// SyncMovesRemaining (0x293d400) tail-jumps to this exact continuation.
/// No rollback or complete action/turn/subscriber parity is implied.
pub fn update_attributes<C: FormationUpkeepContext>(
    c: &mut C,
) -> Result<(), FormationUpkeepError<C::Error>> {
    c.preflight_update_attributes()
        .map_err(FormationUpkeepError::EffectsUnavailable)?;
    let f = c
        .formation_mut()
        .map_err(FormationUpkeepError::EffectsUnavailable)?;
    f.is_embarked = false;
    f.can_embark = false;
    f.can_attack = false;
    f.is_ranged_combat = false;
    f.can_capture = false;
    f.ignores_zoc = false;
    f.domain = -1;
    // has_moved and has_moved_into_zoc are deliberately not reset here.
    let definitions = c
        .definitions()
        .map_err(FormationUpkeepError::ContinuationFailed)?;
    let mut index = 0;
    while let Some(id) = member(c, index)? {
        let unit = required(c, index, id)?;
        let embarked = is_embarked(
            c.unit(unit)
                .map_err(FormationUpkeepError::ContinuationFailed)?,
        );
        c.formation_mut()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .is_embarked |= embarked;
        let can_embark = c
            .boolean(unit, BooleanQuery::CanEmbark)
            .map_err(FormationUpkeepError::ContinuationFailed)?;
        c.formation_mut()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .can_embark |= can_embark;
        let can_attack = c
            .boolean(unit, BooleanQuery::CanAttack)
            .map_err(FormationUpkeepError::ContinuationFailed)?;
        c.formation_mut()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .can_attack |= can_attack;
        let ignores_zoc = c
            .unit(unit)
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .b_ignore_zoc;
        c.formation_mut()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .ignores_zoc |= ignores_zoc;
        let ranged = c
            .integer(unit, IntegerQuery::RangedCombat)
            .map_err(FormationUpkeepError::ContinuationFailed)?;
        let combat = c
            .integer(unit, IntegerQuery::Combat)
            .map_err(FormationUpkeepError::ContinuationFailed)?;
        let preferred = if ranged > combat {
            true
        } else {
            let bombard = c
                .integer(unit, IntegerQuery::BombardCombat)
                .map_err(FormationUpkeepError::ContinuationFailed)?;
            let combat = c
                .integer(unit, IntegerQuery::Combat)
                .map_err(FormationUpkeepError::ContinuationFailed)?;
            bombard > combat
        };
        c.formation_mut()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .is_ranged_combat |= preferred;
        // HasMoved captures raw moves *before* querying actual MaxMoves.
        let raw = c
            .unit(unit)
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .x_moves_remaining;
        let maximum = c
            .integer(unit, IntegerQuery::MaxMoves)
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .wrapping_shl(8);
        c.formation_mut()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .has_moved |= raw < maximum;
        let type_index = c
            .native_type_index(unit)
            .map_err(FormationUpkeepError::ContinuationFailed)?;
        let capture = c
            .definition_can_capture(definitions, type_index)
            .map_err(FormationUpkeepError::ContinuationFailed)?;
        match capture {
            Some(value) => {
                c.formation_mut()
                    .map_err(FormationUpkeepError::ContinuationFailed)?
                    .can_capture |= value
            }
            None => {
                c.formation_mut()
                    .map_err(FormationUpkeepError::ContinuationFailed)?
                    .can_capture = false
            }
        }
        // Native re-queries embarked, and may query domain twice.
        let domain = if is_embarked(
            c.unit(unit)
                .map_err(FormationUpkeepError::ContinuationFailed)?,
        ) {
            0
        } else if c
            .formation()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .domain
            == -1
        {
            c.integer(unit, IntegerQuery::Domain)
                .map_err(FormationUpkeepError::ContinuationFailed)?
        } else {
            let previous = c
                .formation()
                .map_err(FormationUpkeepError::ContinuationFailed)?
                .domain;
            let current = c
                .integer(unit, IntegerQuery::Domain)
                .map_err(FormationUpkeepError::ContinuationFailed)?;
            if previous < current {
                c.formation()
                    .map_err(FormationUpkeepError::ContinuationFailed)?
                    .domain
            } else {
                c.integer(unit, IntegerQuery::Domain)
                    .map_err(FormationUpkeepError::ContinuationFailed)?
            }
        };
        c.formation_mut()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .domain = domain;
        index += 1;
    }
    let moves = formation_moves(c)?;
    if moves
        != c.formation()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .moves_remaining_raw
    {
        c.formation_mut()
            .map_err(FormationUpkeepError::ContinuationFailed)?
            .moves_remaining_raw = moves;
        let mut index = 0;
        while let Some(id) = member(c, index)? {
            let unit = required(c, index, id)?;
            let actor = c
                .unit(unit)
                .map_err(FormationUpkeepError::ContinuationFailed)?;
            let identity = ComponentId {
                owner: actor.e_owner as i16,
                component_type: 1,
                id: actor.id,
            };
            let report = MovementReport {
                event_hash: MOVES_REMAINING_CHANGED,
                identity,
                raw_q8: Some(
                    c.formation()
                        .map_err(FormationUpkeepError::ContinuationFailed)?
                        .moves_remaining_raw,
                ),
            };
            c.send_member_movement(report)
                .map_err(FormationUpkeepError::ContinuationFailed)?;
            index += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source_only_unit() -> Unit {
        crate::unit::Unit {
            wire_version: 28,
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
                wire_version: 7,
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
                wire_version: 8,
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
                wire_version: 10,
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
                wire_version: 7,
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
            k_operations: crate::unit::OperationQueue {
                nodes: Vec::new().into(),
            },
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
    struct Context {
        formation: Formation,
        formation_available: bool,
        invalidate_member: bool,
        member_available: bool,
        invalidate_report: bool,
        units: Vec<Unit>,
        calls: Vec<(usize, IntegerQuery)>,
        domains: Vec<i32>,
        capture: Vec<Option<bool>>,
        disabled: Vec<bool>,
        reports: Vec<MovementReport>,
        admitted: bool,
        fail_query: Option<IntegerQuery>,
        callback: bool,
        fail_report: bool,
        finds: usize,
        null_find: Option<usize>,
        max_query_mutation: bool,
        maximum: i32,
    }
    impl Context {
        fn new(moves: &[i32]) -> Self {
            let units = moves
                .iter()
                .enumerate()
                .map(|(i, &moves)| {
                    let mut u = source_only_unit();
                    u.id = i as u32;
                    u.e_type_index = i as u32;
                    u.e_owner = -32769;
                    u.x_moves_remaining = moves;
                    u
                })
                .collect();
            Self {
                formation_available: true,
                invalidate_member: false,
                member_available: true,
                invalidate_report: false,
                formation: Formation {
                    id: 7,
                    owner: 0,
                    location: crate::unit::PlotCoord { x: 1, y: 2 },
                    moves_remaining_raw: 42,
                    domain: 99,
                    is_embarked: true,
                    can_embark: true,
                    can_attack: true,
                    is_ranged_combat: true,
                    can_capture: true,
                    has_moved: true,
                    has_moved_into_zoc: true,
                    ignores_zoc: true,
                    last_path: vec![],
                    last_path_cache_location: crate::unit::PlotCoord { x: 3, y: 4 },
                    unit_ids: (0..moves.len() as u32).collect(),
                },
                units,
                calls: vec![],
                domains: vec![2; moves.len()],
                capture: vec![Some(true); moves.len()],
                disabled: vec![false; moves.len()],
                reports: vec![],
                admitted: true,
                fail_query: None,
                callback: false,
                fail_report: false,
                finds: 0,
                null_find: None,
                max_query_mutation: false,
                maximum: 3,
            }
        }
    }
    impl FormationUpkeepContext for Context {
        type Error = &'static str;
        type UnitHandle = usize;
        type Definitions = ();
        type Units = ();
        fn formation(&self) -> Result<&Formation, Self::Error> {
            self.formation_available
                .then_some(&self.formation)
                .ok_or("missing current formation")
        }
        fn formation_mut(&mut self) -> Result<&mut Formation, Self::Error> {
            self.formation_available
                .then_some(&mut self.formation)
                .ok_or("missing current formation")
        }
        fn preflight_update_attributes(&self) -> Result<(), Self::Error> {
            if self.admitted {
                Ok(())
            } else {
                Err("unknown effects")
            }
        }
        fn definitions(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn units(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn find_unit(&mut self, _: (), id: u32) -> Result<Option<usize>, Self::Error> {
            self.finds += 1;
            Ok(if self.null_find == Some(self.finds) {
                None
            } else {
                self.units.iter().position(|u| u.id as u16 == id as u16)
            })
        }
        fn unit(&self, unit: usize) -> Result<&Unit, Self::Error> {
            if self.member_available {
                self.units.get(unit).ok_or("missing current unit")
            } else {
                Err("invalidated current unit")
            }
        }
        fn integer(&mut self, unit: usize, query: IntegerQuery) -> Result<i32, Self::Error> {
            self.calls.push((unit, query));
            if self.invalidate_member {
                self.member_available = false;
            }
            if self.fail_query == Some(query) {
                return Err("getter failure");
            }
            Ok(match query {
                IntegerQuery::RangedCombat => 10,
                IntegerQuery::Combat => {
                    if unit == 0 {
                        9
                    } else {
                        10
                    }
                }
                IntegerQuery::BombardCombat => 11,
                IntegerQuery::MaxMoves => {
                    if self.max_query_mutation {
                        self.units[unit].x_moves_remaining = -100;
                    }
                    self.maximum
                }
                IntegerQuery::Domain => self.domains[unit],
            })
        }
        fn boolean(&mut self, unit: usize, query: BooleanQuery) -> Result<bool, Self::Error> {
            Ok(match query {
                BooleanQuery::CanEmbark | BooleanQuery::CanAttack => false,
                BooleanQuery::MovementDisabledThisTurn => self.disabled[unit],
            })
        }
        fn native_type_index(&mut self, unit: usize) -> Result<u32, Self::Error> {
            Ok(unit as u32)
        }
        fn definition_can_capture(
            &mut self,
            _: (),
            index: u32,
        ) -> Result<Option<bool>, Self::Error> {
            Ok(self.capture[index as usize])
        }
        fn send_member_movement(&mut self, report: MovementReport) -> Result<(), Self::Error> {
            if self.invalidate_report {
                self.formation_available = false;
            }
            assert_eq!(report.raw_q8, Some(self.formation.moves_remaining_raw));
            self.reports.push(report);
            if self.callback && self.reports.len() == 1 {
                self.formation.moves_remaining_raw = -19;
                self.formation.has_moved = false;
                self.formation.unit_ids.push(0); // refreshed End includes appended duplicate
                self.units[1].e_owner = 7;
            }
            if self.fail_report {
                Err("subscriber failure")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn empty_resets_only_actual_fields_and_uses_native_sentinel() {
        let mut c = Context::new(&[]);
        let mut expected = c.formation.clone();
        expected.is_embarked = false;
        expected.can_embark = false;
        expected.can_attack = false;
        expected.is_ranged_combat = false;
        expected.can_capture = false;
        expected.ignores_zoc = false;
        expected.domain = -1;
        expected.moves_remaining_raw = 9001 << 8;
        update_attributes(&mut c).unwrap();
        assert_eq!(c.formation, expected);
        assert!(c.calls.is_empty());
        assert!(c.reports.is_empty());
    }
    #[test]
    fn source_closed_escort_floor_disabled_and_signed_values() {
        for (moves, escort, disabled, expected) in [
            (vec![200, 500], false, false, 200),
            (vec![200, 500], true, false, 500),
            (vec![-20, -10], true, false, 0),
            (vec![200, 500], true, true, 0),
            (vec![i32::MAX, i32::MAX], false, false, 9001 << 8),
            (vec![i32::MAX, i32::MAX], true, false, i32::MAX),
        ] {
            let mut c = Context::new(&moves);
            c.units[1].k_movement_modifiers.b_escort_mobility = escort;
            c.disabled[1] = disabled;
            update_attributes(&mut c).unwrap();
            assert_eq!(c.formation.moves_remaining_raw, expected);
            assert!(c.reports.iter().all(|r| r.raw_q8 == Some(expected)));
            assert!(c.reports.iter().all(|r| r.identity.owner == 32767));
        }
    }
    #[test]
    fn lazy_combat_duplicate_domain_and_missing_definition_clear() {
        let mut c = Context::new(&[100, 200]);
        c.capture[1] = None;
        c.domains = vec![2, 1];
        update_attributes(&mut c).unwrap();
        use IntegerQuery::*;
        assert_eq!(
            c.calls,
            [
                (0, RangedCombat),
                (0, Combat),
                (0, MaxMoves),
                (0, Domain),
                (1, RangedCombat),
                (1, Combat),
                (1, BombardCombat),
                (1, Combat),
                (1, MaxMoves),
                (1, Domain),
                (1, Domain)
            ]
        );
        assert!(!c.formation.can_capture);
        assert!(c.formation.is_ranged_combat);
        assert_eq!(c.formation.domain, 1);
        // Later nonnull capture restores the flag; same ID duplicates are visited.
        c.formation.unit_ids.push(0);
        update_attributes(&mut c).unwrap();
        assert!(c.formation.can_capture);
        assert_eq!(c.formation.domain, 1);
        c.units[1].b_embarked = true;
        update_attributes(&mut c).unwrap();
        assert!(c.formation.is_embarked);
        assert_eq!(c.formation.domain, 0);
    }
    #[test]
    fn callbacks_change_live_fields_members_identity_and_failure_prefix() {
        let mut c = Context::new(&[100, 200]);
        c.callback = true;
        update_attributes(&mut c).unwrap();
        assert_eq!(c.reports.len(), 3);
        assert_eq!(c.reports[0].raw_q8, Some(100));
        assert_eq!(c.reports[1].raw_q8, Some(-19));
        assert_eq!(c.reports[1].identity.owner, 7);
        assert_eq!(c.reports[2].identity.id, 0);
        assert_eq!(c.formation.moves_remaining_raw, -19);
        assert!(!c.formation.has_moved);
        let mut c = Context::new(&[100, 200]);
        c.callback = true;
        c.fail_report = true;
        assert_eq!(
            update_attributes(&mut c),
            Err(FormationUpkeepError::ContinuationFailed(
                "subscriber failure"
            ))
        );
        assert_eq!(c.reports.len(), 1);
        assert_eq!(c.formation.moves_remaining_raw, -19);
        assert_eq!(c.formation.unit_ids, [0, 1, 0]);
    }
    #[test]
    fn admission_getter_and_null_failures_preserve_actual_prefix() {
        let mut c = Context::new(&[100]);
        c.admitted = false;
        let before = c.formation.clone();
        assert_eq!(
            update_attributes(&mut c),
            Err(FormationUpkeepError::EffectsUnavailable("unknown effects"))
        );
        assert_eq!(c.formation, before);
        c.admitted = true;
        c.fail_query = Some(IntegerQuery::MaxMoves);
        assert_eq!(
            update_attributes(&mut c),
            Err(FormationUpkeepError::ContinuationFailed("getter failure"))
        );
        assert!(c.formation.is_ranged_combat);
        assert_eq!(c.formation.moves_remaining_raw, 42);
        assert_eq!(c.formation.domain, -1);
        let mut c = Context::new(&[100]);
        c.null_find = Some(1);
        assert_eq!(
            update_attributes(&mut c),
            Err(FormationUpkeepError::NullMember { index: 0, id: 0 })
        );
        assert_eq!(c.formation.domain, -1);
        // The fold alone skips null; its remaining minimum is the sentinel.
        let mut c = Context::new(&[100]);
        c.null_find = Some(2);
        update_attributes(&mut c).unwrap();
        assert_eq!(c.formation.moves_remaining_raw, 9001 << 8);
        assert_eq!(c.reports.len(), 1);
    }
    #[test]
    fn equal_movement_skips_reports_but_refreshes_attributes() {
        let mut c = Context::new(&[42]);
        c.formation.has_moved = false;
        update_attributes(&mut c).unwrap();
        assert!(c.reports.is_empty());
        assert!(c.formation.has_moved);
        assert_eq!(c.formation.domain, 2);
    }
    #[test]
    fn effective_embark_and_has_moved_cached_raw_wrapping_maximum() {
        let mut c = Context::new(&[768]);
        c.formation.has_moved = false;
        c.units[0].b_embarked = true;
        c.units[0].e_activity = ACTIVITY_HOLD;
        c.max_query_mutation = true;
        update_attributes(&mut c).unwrap();
        assert!(!c.formation.is_embarked);
        assert_eq!(c.formation.domain, 2);
        assert!(!c.formation.has_moved); // cached 768, not callback's -100
        assert_eq!(c.units[0].x_moves_remaining, -100);
        let mut c = Context::new(&[0]);
        c.formation.has_moved = false;
        c.maximum = i32::MAX; // raw maximum wraps to -256
        update_attributes(&mut c).unwrap();
        assert!(!c.formation.has_moved);
    }
    #[test]
    fn missing_current_formation_rejects_before_resets() {
        let mut c = Context::new(&[7]);
        c.formation_available = false;
        let before = c.formation.clone();
        assert_eq!(
            update_attributes(&mut c),
            Err(FormationUpkeepError::EffectsUnavailable(
                "missing current formation"
            ))
        );
        assert_eq!(c.formation, before);
        assert!(c.calls.is_empty());
    }

    #[test]
    fn invalidated_member_and_formation_keep_completed_getter_and_report_prefixes() {
        let mut c = Context::new(&[7]);
        c.invalidate_member = true;
        assert_eq!(
            update_attributes(&mut c),
            Err(FormationUpkeepError::ContinuationFailed(
                "invalidated current unit"
            ))
        );
        assert!(!c.formation.is_embarked);
        assert!(c.formation.is_ranged_combat);
        assert_eq!(c.formation.domain, -1);
        assert!(c.reports.is_empty());

        let mut c = Context::new(&[7, 9]);
        c.invalidate_report = true;
        assert_eq!(
            update_attributes(&mut c),
            Err(FormationUpkeepError::ContinuationFailed(
                "missing current formation"
            ))
        );
        assert_eq!(c.reports.len(), 1);
        assert_eq!(c.formation.moves_remaining_raw, 7);
    }
}
