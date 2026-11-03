//! Lossless consuming split of bounded loaded wire components into World lookup
//! records and retained components. No native Read, runtime initialization or
//! gameplay acceptance is inferred from the saved data.
use crate::{core_player, core_stream, map, player_units, unit, world};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    PlayerOwner { requested: i32, record: i32 },
    UnitOwner { player: i32, record: i32, slot: i32 },
    FormationOwner { player: i32, record: i32, slot: i32 },
    Allocation { collection: &'static str },
    World(world::MissingInput),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loaded component split: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Every CorePrefix field except the separately moved Map and borrowed tail.
#[derive(Debug, Eq, PartialEq)]
pub struct OwnedCore {
    pub metadata: unit::variants::TypedVariantMap,
    pub game_processor: core_stream::GameProcessor,
    pub player_processor: core_stream::PlayerProcessor,
    pub phase_processor: core_stream::PhaseProcessor,
    pub current_turn: i32,
    pub max_turns: i32,
    pub local_player: i32,
    pub game_properties: unit::variants::VariantMap,
    pub map_start: usize,
    pub map_end: usize,
    pub consumed: usize,
    pub items_used: usize,
    pub logical_allocation_used: usize,
}

/// Player.Units manager fields after its complete records/allocator move into
/// World. These are saved values, not inferred initialized runtime defaults.
#[derive(Debug, Eq, PartialEq)]
pub struct RetainedUnits {
    pub current_unit_cost: Vec<unit::HashValue<i32>>,
    pub unit_production_multiplier_q8: Vec<unit::HashValue<i32>>,
    pub extra_unit_copy: Vec<unit::HashValue<i32>>,
    pub enable_wall_attack_whole_game_promotion_class: Vec<unit::HashValue<i32>>,
    pub unit_build_disabled: Vec<unit::HashValue<bool>>,
    pub no_settling_cities: bool,
    pub no_foreign_heal: bool,
    pub no_neutral_heal: bool,
    pub add_on_district_founding: Vec<player_units::DistrictFoundingUnits>,
    pub unit_free_pass_hashes: Vec<u32>,
    pub unit_embarkation_pass_hashes: Vec<u32>,
    pub extra_build_charges: i32,
    pub extra_build_charges_by_unit: Vec<unit::HashValue<i32>>,
    pub extra_spread_charges: i32,
    pub embarked_movement_modifier: i32,
    pub upgrade_military_formation_on_city_conquest_count: i32,
    pub promotion_selections: Vec<player_units::RandomPromotionSelection>,
    pub proposed_promotions: Vec<player_units::ProposedPromotion>,
    pub base_promotions: Vec<player_units::BasePromotion>,
    pub unlimited_promotion_choice_unit_hashes: Vec<u32>,
    pub land_corps_unlock_civic_hash: u32,
    pub land_army_unlock_civic_hash: u32,
    pub naval_corps_unlock_civic_hash: u32,
    pub naval_army_unlock_civic_hash: u32,
    pub land_corps_unlock_tech_hash: u32,
    pub land_army_unlock_tech_hash: u32,
    pub naval_corps_unlock_tech_hash: u32,
    pub naval_army_unlock_tech_hash: u32,
    pub peak_units: u32,
    pub land_corps_modified_strength: i32,
    pub land_army_modified_strength: i32,
    pub naval_corps_modified_strength: i32,
    pub naval_army_modified_strength: i32,
    pub enable_wall_attack_whole_game: i32,
    pub rock_band_album_sales_modifier: i32,
    pub banned_production_yield: i32,
    pub favored_production_yield: i32,
    pub favored_production_modifier_q8: i32,
    pub upgrade_cost_modifier_q8: i32,
    pub current_unit_resource_cost: Vec<unit::HashValue<i32>>,
    pub unit_fuel_demands: Vec<unit::HashValue<i32>>,
}

/// Every FirstPlayer field except its separately moved Formation roster and
/// Units.records. Saved owners/flags and immutable snapshot provenance survive.
#[derive(Debug, Eq, PartialEq)]
pub struct RetainedPlayer {
    pub requested_owner: i32,
    pub header: core_player::PlayerHeader,
    pub goody_hut: crate::player_components::GoodyHut,
    pub bonuses: crate::player_components::Bonuses,
    pub congress: crate::player_components::Congress,
    pub cities: core_player::Cities,
    pub diplomacy: crate::player_diplomacy::Diplomacy,
    pub districts: crate::player_districts::Districts,
    pub name_manager: crate::player_name_manager::NameManager,
    pub culture: crate::player_culture::Culture,
    pub espionage: crate::player_espionage::Espionage,
    pub religion: crate::player_religion::Religion,
    pub resources: crate::player_resources::Resources,
    pub stats: crate::player_stats::Stats,
    pub techs: crate::player_techs::Techs,
    pub treasury: crate::player_treasury::Treasury,
    pub units: RetainedUnits,
    pub eras: crate::player_eras::Eras,
    pub trade: crate::player_trade::Trade,
    pub great_people_points: crate::player_great_people_points::GreatPeoplePoints,
    pub influence: crate::player_influence::Influence,
    pub quests: crate::player_quests::Quests,
    pub wmds: crate::player_wmds::Wmds,
    pub improvements: crate::player_improvements::Improvements,
    pub governors: crate::player_governors::Governors,
    pub cultural_identity: crate::player_cultural_identity::CulturalIdentity,
    pub heroes: crate::player_heroes::Heroes,
    pub post_heroes_arrays: Option<core_player::PostHeroesArrays>,
    pub ai_control: Option<crate::player_ai_control::ControlManager>,
    pub ai_influence_map: Option<crate::player_ai_influence_map::InfluenceMap>,
    pub ai_planning: Option<crate::player_ai_identity::PlanningManagerIdentity>,
    pub ai_city_manager: Option<crate::player_ai_city_manager::CityManager>,
    pub ai_culture: Option<crate::player_ai_culture::CultureAi>,
    pub ai_diplomatic: Option<crate::player_ai_diplomatic::Diplomatic>,
    pub ai_economic: Option<crate::player_ai_economic::Economic>,
    pub ai_governors: Option<crate::player_ai_governors::Governors>,
    pub ai_grand_strategic: Option<crate::player_ai_grand_strategic::GrandStrategic>,
    pub ai_homeland: Option<crate::player_ai_homeland::Homeland>,
    pub initialization_snapshot:
        Option<std::sync::Arc<core_player::initialization::InitializationSnapshot>>,
    pub ai_military: Option<crate::player_ai_military::Military>,
    pub ai_research: Option<crate::player_ai_research::Research>,
    pub ai_tactical: Option<crate::player_ai_tactical::Tactical>,
    pub ai_targets: Option<crate::player_ai_targets::Targets>,
    pub ai_behavior_tree: Option<crate::player_ai_behavior_tree::BehaviorTreeManager>,
    pub tail: Option<crate::player_tail::Tail>,
}

pub fn split_core(core: core_stream::CorePrefix<'_>) -> (map::Map, OwnedCore) {
    let core_stream::CorePrefix {
        metadata,
        game_processor,
        player_processor,
        phase_processor,
        current_turn,
        max_turns,
        local_player,
        game_properties,
        map,
        map_start,
        map_end,
        consumed,
        items_used,
        logical_allocation_used,
        tail: _,
    } = core;
    (
        map,
        OwnedCore {
            metadata,
            game_processor,
            player_processor,
            phase_processor,
            current_turn,
            max_turns,
            local_player,
            game_properties,
            map_start,
            map_end,
            consumed,
            items_used,
            logical_allocation_used,
        },
    )
}

fn move_unit_records(records: player_units::UnitTable) -> Result<map::Recycled<unit::Unit>, Error> {
    let player_units::UnitTable {
        capacity,
        entries,
        current_id_bits,
        free_head,
        free_tail,
        free_links,
    } = records;
    entries
        .len()
        .checked_mul(std::mem::size_of::<map::Slot<unit::Unit>>())
        .filter(|bytes| *bytes <= isize::MAX as usize)
        .ok_or(Error::Allocation {
            collection: "Unit slots",
        })?;
    let mut moved = Vec::new();
    moved
        .try_reserve_exact(entries.len())
        .map_err(|_| Error::Allocation {
            collection: "Unit slots",
        })?;
    for entry in entries {
        let player_units::Slot { index, value } = entry;
        moved.push(map::Slot { index, value });
    }
    Ok(map::Recycled {
        capacity,
        entries: moved,
        current_id_bits,
        free_head,
        free_tail,
        free_links,
    })
}

/// Consumes all fields without cloning or manufacturing replacement records.
/// World supplies the established low16 Find/iteration behavior. Runtime
/// configuration/team/alive/technology/civic facts remain explicit None.
pub fn split_player(
    player: core_player::FirstPlayer,
) -> Result<(world::Player, RetainedPlayer), Error> {
    let owner = player.header.owner;
    if player.requested_owner != owner {
        return Err(Error::PlayerOwner {
            requested: player.requested_owner,
            record: owner,
        });
    }
    for slot in &player.units.records.entries {
        if slot.value.e_owner != owner {
            return Err(Error::UnitOwner {
                player: owner,
                record: slot.value.e_owner,
                slot: slot.index,
            });
        }
    }
    for slot in &player.formations.entries {
        if slot.value.owner != owner {
            return Err(Error::FormationOwner {
                player: owner,
                record: slot.value.owner,
                slot: slot.index,
            });
        }
    }
    let core_player::FirstPlayer {
        requested_owner,
        header,
        formations,
        goody_hut,
        bonuses,
        congress,
        cities,
        diplomacy,
        districts,
        name_manager,
        culture,
        espionage,
        religion,
        resources,
        stats,
        techs,
        treasury,
        units,
        eras,
        trade,
        great_people_points,
        influence,
        quests,
        wmds,
        improvements,
        governors,
        cultural_identity,
        heroes,
        post_heroes_arrays,
        ai_control,
        ai_influence_map,
        ai_planning,
        ai_city_manager,
        ai_culture,
        ai_diplomatic,
        ai_economic,
        ai_governors,
        ai_grand_strategic,
        ai_homeland,
        initialization_snapshot,
        ai_military,
        ai_research,
        ai_tactical,
        ai_targets,
        ai_behavior_tree,
        tail,
    } = player;
    let player_units::Units {
        records,
        current_unit_cost,
        unit_production_multiplier_q8,
        extra_unit_copy,
        enable_wall_attack_whole_game_promotion_class,
        unit_build_disabled,
        no_settling_cities,
        no_foreign_heal,
        no_neutral_heal,
        add_on_district_founding,
        unit_free_pass_hashes,
        unit_embarkation_pass_hashes,
        extra_build_charges,
        extra_build_charges_by_unit,
        extra_spread_charges,
        embarked_movement_modifier,
        upgrade_military_formation_on_city_conquest_count,
        promotion_selections,
        proposed_promotions,
        base_promotions,
        unlimited_promotion_choice_unit_hashes,
        land_corps_unlock_civic_hash,
        land_army_unlock_civic_hash,
        naval_corps_unlock_civic_hash,
        naval_army_unlock_civic_hash,
        land_corps_unlock_tech_hash,
        land_army_unlock_tech_hash,
        naval_corps_unlock_tech_hash,
        naval_army_unlock_tech_hash,
        peak_units,
        land_corps_modified_strength,
        land_army_modified_strength,
        naval_corps_modified_strength,
        naval_army_modified_strength,
        enable_wall_attack_whole_game,
        rock_band_album_sales_modifier,
        banned_production_yield,
        favored_production_yield,
        favored_production_modifier_q8,
        upgrade_cost_modifier_q8,
        current_unit_resource_cost,
        unit_fuel_demands,
    } = units;
    let units = move_unit_records(records)?;
    let lookup = world::Player::new(owner, Some(units), Some(formations)).map_err(Error::World)?;
    let units = RetainedUnits {
        current_unit_cost,
        unit_production_multiplier_q8,
        extra_unit_copy,
        enable_wall_attack_whole_game_promotion_class,
        unit_build_disabled,
        no_settling_cities,
        no_foreign_heal,
        no_neutral_heal,
        add_on_district_founding,
        unit_free_pass_hashes,
        unit_embarkation_pass_hashes,
        extra_build_charges,
        extra_build_charges_by_unit,
        extra_spread_charges,
        embarked_movement_modifier,
        upgrade_military_formation_on_city_conquest_count,
        promotion_selections,
        proposed_promotions,
        base_promotions,
        unlimited_promotion_choice_unit_hashes,
        land_corps_unlock_civic_hash,
        land_army_unlock_civic_hash,
        naval_corps_unlock_civic_hash,
        naval_army_unlock_civic_hash,
        land_corps_unlock_tech_hash,
        land_army_unlock_tech_hash,
        naval_corps_unlock_tech_hash,
        naval_army_unlock_tech_hash,
        peak_units,
        land_corps_modified_strength,
        land_army_modified_strength,
        naval_corps_modified_strength,
        naval_army_modified_strength,
        enable_wall_attack_whole_game,
        rock_band_album_sales_modifier,
        banned_production_yield,
        favored_production_yield,
        favored_production_modifier_q8,
        upgrade_cost_modifier_q8,
        current_unit_resource_cost,
        unit_fuel_demands,
    };
    let retained = RetainedPlayer {
        requested_owner,
        header,
        goody_hut,
        bonuses,
        congress,
        cities,
        diplomacy,
        districts,
        name_manager,
        culture,
        espionage,
        religion,
        resources,
        stats,
        techs,
        treasury,
        units,
        eras,
        trade,
        great_people_points,
        influence,
        quests,
        wmds,
        improvements,
        governors,
        cultural_identity,
        heroes,
        post_heroes_arrays,
        ai_control,
        ai_influence_map,
        ai_planning,
        ai_city_manager,
        ai_culture,
        ai_diplomatic,
        ai_economic,
        ai_governors,
        ai_grand_strategic,
        ai_homeland,
        initialization_snapshot,
        ai_military,
        ai_research,
        ai_tactical,
        ai_targets,
        ai_behavior_tree,
        tail,
    };
    Ok((lookup, retained))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn with_formation() -> core_player::FirstPlayer {
        let mut player = core_player::tests::source_first_player();
        player.header.ordered_flags[0] = true;
        player.formations = map::Recycled {
            capacity: 17,
            current_id_bits: 0xffff0000,
            free_head: 0x12345678,
            free_tail: u32::MAX,
            free_links: vec![99, 0xffff0001],
            entries: vec![map::Slot {
                index: 5,
                value: unit::Formation {
                    id: 0x12340005,
                    location: unit::PlotCoord { x: -1, y: i32::MIN },
                    owner: player.header.owner,
                    moves_remaining_raw: -7,
                    domain: -1,
                    is_embarked: false,
                    can_embark: true,
                    can_attack: false,
                    is_ranged_combat: true,
                    can_capture: false,
                    has_moved: true,
                    has_moved_into_zoc: false,
                    ignores_zoc: true,
                    last_path: vec![unit::PathNode {
                        data1_raw: i32::MIN,
                        data2: -1,
                        flags: i32::MAX,
                        x: i16::MIN,
                        y: i16::MAX,
                    }],
                    last_path_cache_location: unit::PlotCoord { x: i32::MAX, y: -1 },
                    unit_ids: vec![0xdead0007, 0xbeef0003],
                },
            }],
        };
        player
    }
    #[test]
    fn split_moves_rosters_allocator_tails_and_provenance_without_runtime_inference() {
        let player = with_formation();
        let formations_pointer = player.formations.entries.as_ptr();
        let path_pointer = player.formations.entries[0].value.last_path.as_ptr();
        let members_pointer = player.formations.entries[0].value.unit_ids.as_ptr();
        let formation_free_pointer = player.formations.free_links.as_ptr();
        let unit_free_pointer = player.units.records.free_links.as_ptr();
        let costs_pointer = player.units.current_unit_cost.as_ptr();
        let alive = player.header.saved_is_alive();
        let snapshot_pointer =
            std::sync::Arc::as_ptr(player.initialization_snapshot.as_ref().unwrap());
        let snapshot_count =
            std::sync::Arc::strong_count(player.initialization_snapshot.as_ref().unwrap());
        let (lookup, retained) = split_player(player).unwrap();
        assert!(alive && retained.header.saved_is_alive());
        assert_eq!(lookup.owner(), retained.header.owner);
        assert_eq!(
            (lookup.slot_status, lookup.team, lookup.is_alive),
            (None, None, None)
        );
        assert!(lookup.technologies.is_none() && lookup.civics.is_none());
        let formations = lookup.formations().unwrap();
        assert_eq!(formations.entries.as_ptr(), formations_pointer);
        assert_eq!(formations.entries[0].value.last_path.as_ptr(), path_pointer);
        assert_eq!(
            formations.entries[0].value.unit_ids.as_ptr(),
            members_pointer
        );
        assert_eq!(formations.free_links.as_ptr(), formation_free_pointer);
        assert_eq!(
            (
                formations.capacity,
                formations.current_id_bits,
                formations.free_head,
                formations.free_tail
            ),
            (17, 0xffff0000, 0x12345678, u32::MAX)
        );
        let units = lookup.units().unwrap();
        assert_eq!(units.free_links.as_ptr(), unit_free_pointer);
        assert_eq!(
            (
                units.capacity,
                units.current_id_bits,
                units.free_head,
                units.free_tail
            ),
            (257, 0xffff0000, 0x12345678, u32::MAX)
        );
        assert!(units.entries.is_empty());
        assert_eq!(retained.units.current_unit_cost.as_ptr(), costs_pointer);
        assert_eq!(retained.units.current_unit_cost[0].value, i32::MIN);
        assert_eq!(
            std::sync::Arc::as_ptr(retained.initialization_snapshot.as_ref().unwrap()),
            snapshot_pointer
        );
        assert_eq!(
            std::sync::Arc::strong_count(retained.initialization_snapshot.as_ref().unwrap()),
            snapshot_count
        );
        // Existing World Find semantics retain requested generation bits and
        // resolve by the exact saved low16 slot; the splitter does not remap.
        let found = lookup.formation(0xabcd0005).unwrap().unwrap();
        assert_eq!(found.requested_id, 0xabcd0005);
        assert_eq!(found.resolved_id, 0x12340005);
        assert!(retained.ai_behavior_tree.is_some() && retained.tail.is_some());
    }
    #[test]
    fn owner_and_slot_failures_are_named_and_never_normalize_records() {
        let mut player = with_formation();
        player.requested_owner = -1;
        assert!(matches!(
            split_player(player),
            Err(Error::PlayerOwner {
                requested: -1,
                record: 0
            })
        ));
        let mut player = with_formation();
        player.formations.entries[0].value.owner = 62;
        assert!(matches!(
            split_player(player),
            Err(Error::FormationOwner {
                player: 0,
                record: 62,
                slot: 5
            })
        ));
        let mut player = with_formation();
        player.formations.entries[0].index = -1;
        assert!(matches!(
            split_player(player),
            Err(Error::World(world::MissingInput::InvalidSlot))
        ));
    }
}
