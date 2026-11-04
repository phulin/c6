//! Linux CityRangeAttack auto-notification subtype and retained city lookup.
//!
//! This source projection does not certify final native runtime or full Update.
//! Stock command initialization is source-derived. Missing actual configuration
//! levels or unclosed sight/target traversal return errors; no saved notification
//! or precomputed readiness flag substitutes for those dependencies.
use super::{notifications, reporting, Error};
use crate::resource_binding::ResourceAuthority;
use crate::{
    registry::LinuxDefinitionRegistry, unit_initialization_definitions,
    unit_operation_manager::PhaseDefinitions, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState,
    notifications::Notification,
};

struct UnitCombatDefinition {
    melee: i32,
    ranged: i32,
    bombard: i32,
    use_max_melee: bool,
    team_visibility: bool,
    formation_class: i32,
}
/// Immutable rules projected only after the same configuration/resource gate
/// used by other loaded-state providers. Collection indexes remain distinct
/// from SQL row IDs and serialized definition hashes.
pub struct Definitions {
    binding: WireBinding,
    districts: LinuxDefinitionRegistry,
    district_hit_points: Vec<Option<i32>>,
    buildings: LinuxDefinitionRegistry,
    building_outer_hit_points: Vec<Option<i32>>,
    district_xp2: LinuxDefinitionRegistry,
    district_range: Vec<i32>,
    minimum_strike: i32,
    terrains: LinuxDefinitionRegistry,
    terrain_sight: Vec<i32>,
    terrain_through: Vec<i32>,
    features: LinuxDefinitionRegistry,
    feature_through: Vec<i32>,
    units: LinuxDefinitionRegistry,
    unit_combat: Vec<UnitCombatDefinition>,
    eras: LinuxDefinitionRegistry,
    hero_classes: LinuxDefinitionRegistry,
    era_chronology: Vec<i32>,
    era_embarked_strength: Vec<i32>,
    corps_strength: i32,
    army_strength: i32,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if state.binding() != bound.destination_binding() {
            return Err(Error::ConfigurationIdentity);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(|_| Error::Unsupported("CityRangeAttack admitted rules/resources"))?;
        fn values(
            rules: &Rules,
            table: &'static str,
            field: &'static str,
        ) -> Result<Vec<Option<i32>>, Error> {
            rules
                .gameplay()
                .table(table)
                .ok_or(Error::Unsupported(table))?
                .rows()
                .map(
                    |row| match row.get(field).map_err(|_| Error::Unsupported(field))? {
                        SqlValue::Null => Ok(None),
                        SqlValue::Integer(n) => i32::try_from(*n)
                            .map(Some)
                            .map_err(|_| Error::Unsupported(field)),
                        _ => Err(Error::Unsupported(field)),
                    },
                )
                .collect()
        }
        let collection = |table, field| {
            unit_initialization_definitions::collection(rules, table, field)
                .map_err(|_| Error::Unsupported(table))
        };
        let minimum_strike = parameter(rules, "COMBAT_MINIMUM_CITY_STRIKE_STRENGTH")?;
        let integer = |table, field| {
            values(rules, table, field)?
                .into_iter()
                .map(|v| v.ok_or(Error::Unsupported(field)))
                .collect::<Result<Vec<i32>, Error>>()
        };
        let mut unit_combat = Vec::new();
        for row in rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Unsupported("Units"))?
            .rows()
        {
            let int = |field| match row.get(field).map_err(|_| Error::Unsupported(field))? {
                SqlValue::Integer(n) => i32::try_from(*n).map_err(|_| Error::Unsupported(field)),
                _ => Err(Error::Unsupported(field)),
            };
            let flag = |field| match int(field)? {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(Error::Unsupported(field)),
            };
            let hash = |field| match row.get(field).map_err(|_| Error::Unsupported(field))? {
                SqlValue::Text(name) => Ok(crate::registry::linux_type_hash(name)),
                SqlValue::Null => Ok(0),
                _ => Err(Error::Unsupported(field)),
            };
            unit_combat.push(UnitCombatDefinition {
                melee: int("Combat")?,
                ranged: int("RangedCombat")?,
                bombard: int("Bombard")?,
                use_max_melee: flag("UseMaxMeleeTrainedStrength")?,
                team_visibility: flag("TeamVisibility")?,
                formation_class: formation_class(hash("FormationClass")?),
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            districts: collection("Districts", "DistrictType")?,
            district_hit_points: values(rules, "Districts", "HitPoints")?,
            buildings: collection("Buildings", "BuildingType")?,
            building_outer_hit_points: values(rules, "Buildings", "OuterDefenseHitPoints")?,
            district_xp2: collection("Districts_XP2", "DistrictType")?,
            district_range: values(rules, "Districts_XP2", "AttackRange")?
                .into_iter()
                .map(|n| n.ok_or(Error::Unsupported("Districts_XP2.AttackRange")))
                .collect::<Result<_, _>>()?,
            minimum_strike,
            units: collection("Units", "UnitType")?,
            unit_combat,
            eras: collection("Eras", "EraType")?,
            hero_classes: collection("HeroClasses", "HeroClassType")?,
            era_chronology: integer("Eras", "ChronologyIndex")?,
            era_embarked_strength: integer("Eras", "EmbarkedUnitStrength")?,
            corps_strength: parameter(rules, "COMBAT_CORPS_STRENGTH_MODIFIER")?,
            army_strength: parameter(rules, "COMBAT_ARMY_STRENGTH_MODIFIER")?,
            terrains: collection("Terrains", "TerrainType")?,
            terrain_sight: values(rules, "Terrains", "SightModifier")?
                .into_iter()
                .map(|n| n.ok_or(Error::Unsupported("Terrains.SightModifier")))
                .collect::<Result<_, _>>()?,
            terrain_through: values(rules, "Terrains", "SightThroughModifier")?
                .into_iter()
                .map(|n| n.ok_or(Error::Unsupported("Terrains.SightThroughModifier")))
                .collect::<Result<_, _>>()?,
            features: collection("Features", "FeatureType")?,
            feature_through: values(rules, "Features", "SightThroughModifier")?
                .into_iter()
                .map(|n| n.ok_or(Error::Unsupported("Features.SightThroughModifier")))
                .collect::<Result<_, _>>()?,
        })
    }
}

/// Stock XP2 InitializeComponents registers RangeAttack; its empty-map request
/// has an invalid location and succeeds. This explicit source initialization is
/// not an observation of the final native registry or an added-mod handler.
pub enum CommandScope {
    SourceInitializedStockXp2,
}
pub struct Queries<'a> {
    pub definitions: &'a Definitions,
    pub phases: &'a PhaseDefinitions,
    pub scope: CommandScope,
    /// Actual Configuration.Player civilization-level type hashes, attributed
    /// to this bound session; absence is unknown, never an ordinary-player default.
    pub civilization_level_types: &'a [(i32, u32)],
    pub combined_visibility: Option<&'a CombinedVisibility>,
}
/// Raw authoritative group1 visibility fields. These are mutable gameplay
/// counts, not saved group0 counts or precomputed Unit-visible answers.
pub struct CombinedVisibilityOwner {
    pub owner: i32,
    /// Actual Visibility.Instance grid width used by location indexing.
    pub grid_width: u32,
    pub layers: Vec<c6_core::map::VisibilityLayer>,
}
/// Explicit raw-state authority tied to a precise LoadedState revision. The
/// caller must supply actual initialized/replayed group1 state; construction
/// checks identity/shape, not capture authenticity. No constructor fabricates
/// counts from source saved group0 or assumes later outgoing effects occurred.
pub struct CombinedVisibility {
    binding: WireBinding,
    revision: u64,
    owners: Vec<CombinedVisibilityOwner>,
    hidden_layer0: u32,
}
impl CombinedVisibility {
    pub fn from_raw(
        state: &LoadedState,
        owners: Vec<CombinedVisibilityOwner>,
        hidden_layer0: u32,
    ) -> Result<Self, Error> {
        for (at, owner) in owners.iter().enumerate() {
            if owners[..at]
                .iter()
                .any(|earlier| earlier.owner == owner.owner)
            {
                return Err(Error::Unsupported(
                    "CityRangeAttack duplicate combined visibility owner",
                ));
            }
            if owner
                .layers
                .iter()
                .any(|layer| layer.values.len() != layer.count as usize)
            {
                return Err(Error::Unsupported(
                    "CityRangeAttack raw combined visibility layer extent",
                ));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            revision: state.revision(),
            owners,
            hidden_layer0,
        })
    }
    fn owner(&self, state: &LoadedState, viewer: i32) -> Result<&CombinedVisibilityOwner, Error> {
        if self.binding != *state.binding() || self.revision != state.revision() {
            return Err(Error::Unsupported(
                "CityRangeAttack combined visibility binding/revision",
            ));
        }
        self.owners
            .iter()
            .find(|owner| owner.owner == viewer)
            .ok_or(Error::Unsupported(
                "CityRangeAttack actual combined visibility owner",
            ))
    }
}

fn parameter(rules: &Rules, name: &'static str) -> Result<i32, Error> {
    let parameters = rules
        .gameplay()
        .table("GlobalParameters")
        .ok_or(Error::Unsupported("GlobalParameters"))?;
    let mut rows = parameters
        .rows()
        .filter(|r| r.text("Name").ok() == Some(name));
    let row = rows.next().ok_or(Error::Unsupported(name))?;
    if rows.next().is_some() {
        return Err(Error::Unsupported("duplicate city combat parameter"));
    }
    match row.get("Value").map_err(|_| Error::Unsupported(name))? {
        SqlValue::Integer(n) => i32::try_from(*n).ok(),
        SqlValue::Text(s) => std::str::from_utf8(s).ok().and_then(|s| s.parse().ok()),
        _ => None,
    }
    .ok_or(Error::Unsupported(name))
}
fn formation_class(hash: u32) -> i32 {
    match hash {
        0x9ad485f8 => 0,
        0xf1c32241 => 1,
        0xc2510aa7 => 2,
        0xfeeb5b2f => 3,
        0x31b700cb => 4,
        _ => -1,
    }
}

pub const TYPE_HASH: u32 = 0xb998_5c59;
pub const VTABLE_ADDRESSPOINT: u64 = 0x3af_5130;
pub const DEFAULT_BLOCKING: i32 = 0;

/// Subtype stores after Instance::Initialize, in native order. These stores
/// contain no city query. The caller retains the logical allocated candidate.
pub fn initialize_auto(
    _state: &LoadedState,
    _queries: &reporting::Provider<'_>,
    created: &mut notifications::Created,
) -> Result<(), Error> {
    if created.record.type_hash != TYPE_HASH
        || created.record.factory_type != TYPE_HASH
        || created.vtable_addresspoint != VTABLE_ADDRESSPOINT
        || !created.record.parameters.entries.is_empty()
        || !created.record.native_19b
    {
        return Err(Error::Unsupported(
            "CityRangeAttack auto Initialize receiver binding",
        ));
    }
    created.record.native_199 = true;
    created.record.native_19a = true;
    created.record.native_18c = 0;
    created.record.native_190 = 0;
    created.default_blocking = DEFAULT_BLOCKING;
    Ok(())
}

/// Native City::Districts::GetDistrictAt resolves each saved ID through the
/// owner's Districts container, compares location, and skips missing slots.
/// Generation bits are not used by RecycledIDContainer::Find.
fn center_district<'a>(
    city: &c6_core::city::City,
    districts: &'a c6_core::player_districts::Districts,
) -> Option<&'a c6_core::player_districts::District> {
    city.districts.district_ids.iter().find_map(|id| {
        if *id == u32::MAX {
            return None;
        }
        let district = &districts
            .records
            .entries
            .iter()
            .find(|slot| slot.index as u32 == (*id & 0xffff))?
            .value;
        (district.location == (city.header.location_x, city.header.location_y)).then_some(district)
    })
}

/// GetFirstRangedAttackCityID's closed typed prefix, in native city order.
/// A city-center district reaches the source-initialized stock RangeAttack
/// empty-map request, then the actual Manager control gates and handler prefix.
pub fn first_ranged_attack_city_id(
    state: &LoadedState,
    owner: i32,
    queries: &Queries<'_>,
) -> Result<u32, Error> {
    if state.binding() != &queries.definitions.binding {
        return Err(Error::ConfigurationIdentity);
    }
    state.world().player(owner).map_err(Error::World)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|player| player.requested_owner == owner)
        .ok_or(Error::Unsupported(
            "CityRangeAttack retained Player.Cities owner",
        ))?;
    for slot in &player.cities.records.entries {
        let city = &slot.value;
        if city.header.owner != owner {
            return Err(Error::Unsupported("CityRangeAttack City owner consistency"));
        }
        let Some(district) = center_district(city, &player.districts) else {
            continue;
        };
        // Manager::CanStart(..., testVisible=false, results=null), in order.
        if !state.turn_active(owner).map_err(Error::State)? {
            continue;
        }
        if !queries
            .phases
            .from_processor(&state.retained().core.phase_processor, owner)
            .map_err(Error::Manager)?
        {
            continue;
        }
        let human = state.player_slot_status(owner).map_err(Error::State)? == 3;
        if human && state.pause_storage().map_err(Error::State)? != 0 {
            continue;
        }
        if can_start_range_attack(state, city, district, queries)? {
            return Ok(city.header.id);
        }
    }
    Ok(u32::MAX)
}

pub fn is_expired(
    state: &LoadedState,
    queries: &Queries<'_>,
    notification: &Notification,
) -> Result<bool, Error> {
    let owner = i32::from(notification.id.owner);
    if owner == -1 {
        return Ok(true);
    }
    Ok(first_ranged_attack_city_id(state, owner, queries)? == u32::MAX)
}

/// Inherited Instance::IsNeeded; +0x19a gates the virtual IsExpired call.
/// IsValidForPhase is a separate slot and is not evaluated by this function.
pub fn is_needed(
    state: &LoadedState,
    queries: &Queries<'_>,
    notification: &Notification,
) -> Result<bool, Error> {
    if !notification.native_19a {
        return Ok(true);
    }
    Ok(!is_expired(state, queries, notification)?)
}

pub fn is_duplicate(candidate: &Notification, existing: &Notification) -> Result<bool, Error> {
    let duplicate = notifications::base_auto_duplicate(candidate, existing)
        .map_err(|_| Error::Unsupported("CityRangeAttack base auto duplicate input"))?;
    Ok(duplicate && candidate.native_190 == existing.native_190)
}

/// Finite CanStart prefix. GetAttackStrength executes even for no remaining
/// attacks; preserving this ordering keeps reached missing inputs visible.
fn can_start_range_attack(
    state: &LoadedState,
    city: &c6_core::city::City,
    district: &c6_core::player_districts::District,
    queries: &Queries<'_>,
) -> Result<bool, Error> {
    let remaining = district.remaining_attacks > 0;
    let strength = attack_strength_q8(state, city, district, queries)?;
    if strength <= 0 || !remaining || district.pillaged || !district.is_complete {
        return Ok(false);
    }
    let _owner = district_owner(state, district)?;
    let owning_city = district_city(state, district)?;
    let range = attack_range(owning_city, district, strength, queries.definitions);
    let plots = sight_plots(state, district.location, range, queries.definitions)?;
    for candidate in plots {
        let location =
            state
                .world()
                .map()
                .coordinates(candidate.index)
                .ok_or(Error::Unsupported(
                    "CityRangeAttack sight target plot index",
                ))?;
        if plot_distance(state, district.location, location) <= range
            && test_ranged_attack_into(state, district, location, queries)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// HasWalls/GetMaxDamage source prefix. No district definition returns zero;
/// nonpositive/absent District.HitPoints skips City wall HP entirely.
pub fn attack_strength_q8(
    state: &LoadedState,
    _city: &c6_core::city::City,
    district: &c6_core::player_districts::District,
    queries: &Queries<'_>,
) -> Result<i32, Error> {
    if state.binding() != &queries.definitions.binding {
        return Err(Error::ConfigurationIdentity);
    }
    let hp = queries
        .definitions
        .districts
        .find_index(district.district_hash)
        .and_then(|i| queries.definitions.district_hit_points.get(i))
        .copied()
        .flatten()
        .unwrap_or(0);
    let maximum = if hp > 0 {
        let owning_city = district_city(state, district)?;
        outer_defense_hit_points(state, owning_city, queries)?
    } else {
        0
    };
    if district.outer_defenses_damage >= maximum {
        return Ok(0);
    }
    let owner = district_owner(state, district)?;
    state.world().player(owner).map_err(Error::World)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.requested_owner == owner)
        .ok_or(Error::Unsupported(
            "CityRangeAttack actual plot owner's Player.Stats",
        ))?;
    // District.GetCity resolves this owner's City ID, which may differ from
    // the outer query's City; do not use the enclosing roster as ownership.
    let owning_city = district_city(state, district)?;
    Ok(trained_attack_strength_q8(
        owning_city,
        &player.stats,
        queries.definitions,
    ))
}

fn outer_defense_hit_points(
    state: &LoadedState,
    city: &c6_core::city::City,
    queries: &Queries<'_>,
) -> Result<i32, Error> {
    let mut levels = queries
        .civilization_level_types
        .iter()
        .filter(|(owner, _)| *owner == city.header.owner);
    let level = levels
        .next()
        .ok_or(Error::Unsupported(
            "CityRangeAttack Player.GetLevel actual Configuration.Player.CivilizationLevelTypeID",
        ))?
        .1;
    if levels.next().is_some() {
        return Err(Error::Unsupported(
            "duplicate Configuration.Player civilization-level owner",
        ));
    }
    let urban = if level == 0x04e6_b0a0 {
        state
            .retained()
            .free_cities
            .instances
            .iter()
            .find(|free| {
                free.living_raw != 0
                    && city.header.location_x != -9999
                    && city.header.location_y != -9999
                    && free.city_location.x == city.header.location_x
                    && free.city_location.y == city.header.location_y
            })
            .map_or(0, |free| free.urban_defenses)
    } else {
        state
            .retained()
            .players
            .iter()
            .find(|p| p.requested_owner == city.header.owner)
            .ok_or(Error::Unsupported("CityRangeAttack wall HP Player.Cities"))?
            .cities
            .urban_defenses
    };
    ordinary_outer_defense_hit_points(city, urban, queries.definitions)
}

fn district_city_optional<'a>(
    state: &'a LoadedState,
    district: &c6_core::player_districts::District,
) -> Result<Option<&'a c6_core::city::City>, Error> {
    let owner = district_owner(state, district)?;
    if owner == -1 {
        return Ok(None);
    }
    state.world().player(owner).map_err(Error::World)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.requested_owner == owner)
        .ok_or(Error::Unsupported(
            "CityRangeAttack actual plot owner's Player.Cities",
        ))?;
    Ok(player
        .cities
        .records
        .entries
        .iter()
        .find(|slot| {
            district.city_id != u32::MAX && slot.index as u32 == (district.city_id & 0xffff)
        })
        .map(|slot| &slot.value))
}
fn district_city<'a>(
    state: &'a LoadedState,
    district: &c6_core::player_districts::District,
) -> Result<&'a c6_core::city::City, Error> {
    district_city_optional(state, district)?.ok_or(Error::Unsupported(
        "CityRangeAttack reached null District.GetCity dereference",
    ))
}

fn district_owner(
    state: &LoadedState,
    district: &c6_core::player_districts::District,
) -> Result<i32, Error> {
    let map = state.world().map();
    if map.header.width <= 0 || map.header.height <= 0 {
        return Err(Error::Unsupported("CityRangeAttack map dimensions"));
    }
    let (x, y) = district.location;
    if x == -9999 || y == -9999 {
        return Err(Error::Unsupported(
            "CityRangeAttack District.GetPlot invalid location",
        ));
    }
    let x = c6_core::movement_world::coord_range(x, map.header.width, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(y, map.header.height, map.header.wrap_y);
    let plot = map.plot_at(x, y).ok_or(Error::Unsupported(
        "CityRangeAttack District.GetPlot null result",
    ))?;
    if plot.flags.get(c6_core::map::PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
        return Err(Error::Unsupported(
            "CityRangeAttack Plot city-attribute pointer/flag",
        ));
    }
    Ok(plot
        .city_attributes
        .as_ref()
        .map_or(-1, |attributes| i32::from(attributes.player)))
}

/// Closed remainder of City.Combat wall HP once Configuration.GetLevel has
/// established the ordinary-player branch. This helper does not assume that
/// branch for any LoadedState owner.
pub fn ordinary_outer_defense_hit_points(
    city: &c6_core::city::City,
    urban_defenses: i32,
    definitions: &Definitions,
) -> Result<i32, Error> {
    let base = if urban_defenses > 0 {
        urban_defenses
    } else {
        let mut total = 0i32;
        for (index, entry) in definitions.buildings.entries().iter().enumerate() {
            let hash = crate::registry::linux_type_hash(&entry.type_name);
            let mut locations = city
                .buildings
                .building_locations
                .iter()
                .filter(|v| v.definition_hash == hash);
            // Until constructor/Read normalization is source-closed, do not
            // silently interpret a missing/duplicate saved index as -1.
            let location = locations.next().ok_or(Error::Unsupported(
                "CityRangeAttack Buildings location constructor/Read missing definition projection",
            ))?;
            if locations.next().is_some() {
                return Err(Error::Unsupported(
                    "CityRangeAttack Buildings duplicate saved definition projection",
                ));
            }
            if location.value >= 0 {
                total =
                    total.wrapping_add(definitions.building_outer_hit_points[index].unwrap_or(0));
            }
        }
        total
    };
    Ok(base.wrapping_add(city.combat.hit_points_modifier).max(0))
}

/// Closed remainder after actual plot owner selects the Player.Stats record.
/// Preserve native q8 wrapping conversion and addition rather than comparing
/// the unscaled integers before conversion.
pub fn trained_attack_strength_q8(
    city: &c6_core::city::City,
    stats: &c6_core::player_stats::Stats,
    definitions: &Definitions,
) -> i32 {
    stats
        .max_ranged_strength_trained
        .max(definitions.minimum_strike)
        .wrapping_shl(8)
        .wrapping_add(city.combat.ranged_strike_strength_modifier.wrapping_shl(8))
}

pub fn attack_range(
    city: &c6_core::city::City,
    district: &c6_core::player_districts::District,
    strength_q8: i32,
    definitions: &Definitions,
) -> i32 {
    if strength_q8 <= 0 {
        return 0;
    }
    let type_hash = definitions
        .districts
        .find_index(district.district_hash)
        .map(|i| crate::registry::linux_type_hash(&definitions.districts.entries()[i].type_name))
        .unwrap_or(0);
    definitions
        .district_xp2
        .find_index(type_hash)
        .map(|i| definitions.district_range[i].wrapping_add(city.combat.attack_range_modifier))
        .unwrap_or(0)
}

/// Native IsDefensible first checks geometric CityCenter, then ordinary
/// garrison hit points, completion and pillage; it does not call HasWalls.
fn district_defensible(
    state: &LoadedState,
    district: &c6_core::player_districts::District,
    definitions: &Definitions,
) -> Result<bool, Error> {
    if let Some(city) = district_city_optional(state, district)? {
        if district.location == (city.header.location_x, city.header.location_y) {
            return Ok(true);
        }
    }
    let hp = definitions
        .districts
        .find_index(district.district_hash)
        .and_then(|i| definitions.district_hit_points[i])
        .unwrap_or(0);
    Ok(hp > 0 && district.is_complete && !district.pillaged)
}

fn plot_at(
    state: &LoadedState,
    location: (i32, i32),
) -> Result<Option<&c6_core::map::Plot>, Error> {
    let map = state.world().map();
    if map.header.width <= 0 || map.header.height <= 0 {
        return Err(Error::Unsupported("CityRangeAttack map dimensions"));
    }
    if location.0 == -9999 || location.1 == -9999 {
        return Ok(None);
    }
    let x = c6_core::movement_world::coord_range(location.0, map.header.width, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(location.1, map.header.height, map.header.wrap_y);
    Ok(map.plot_at(x, y))
}

fn plot_district<'a>(
    state: &'a LoadedState,
    plot: &c6_core::map::Plot,
) -> Result<Option<&'a c6_core::player_districts::District>, Error> {
    if plot.flags.get(c6_core::map::PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
        return Err(Error::Unsupported(
            "CityRangeAttack Plot city-attribute pointer/flag",
        ));
    }
    let Some(attributes) = &plot.city_attributes else {
        return Ok(None);
    };
    if attributes.player == -1 || attributes.district == u32::MAX {
        return Ok(None);
    }
    state
        .partial_player_controls(i32::from(attributes.player))
        .map_err(Error::State)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.requested_owner == i32::from(attributes.player))
        .ok_or(Error::Unsupported(
            "CityRangeAttack Plot district actual owner",
        ))?;
    Ok(player
        .districts
        .records
        .entries
        .iter()
        .find(|slot| slot.index as u32 == (attributes.district & 0xffff))
        .map(|slot| &slot.value))
}

fn sight_distance_modifier(
    state: &LoadedState,
    location: (i32, i32),
    definitions: &Definitions,
) -> Result<i32, Error> {
    let Some(plot) = plot_at(state, location)? else {
        return Ok(0);
    };
    let mut value = if plot.terrain_hash == u32::MAX {
        0
    } else {
        let index =
            definitions
                .terrains
                .find_index(plot.terrain_hash)
                .ok_or(Error::Unsupported(
                    "CityRangeAttack sight terrain type Read/definition",
                ))?;
        definitions.terrain_sight[index]
    };
    if let Some(district) = plot_district(state, plot)? {
        if district_defensible(state, district, definitions)? {
            value = 1;
        }
    }
    Ok(value)
}

fn sight_through_modifier(
    state: &LoadedState,
    location: (i32, i32),
    features: bool,
    terrain: bool,
    definitions: &Definitions,
) -> Result<i32, Error> {
    let Some(plot) = plot_at(state, location)? else {
        return Ok(0);
    };
    let mut value = 0i32;
    if terrain && plot.terrain_hash != u32::MAX {
        let index =
            definitions
                .terrains
                .find_index(plot.terrain_hash)
                .ok_or(Error::Unsupported(
                    "CityRangeAttack sight-through terrain definition",
                ))?;
        value = definitions.terrain_through[index];
    }
    if features && plot.feature_hash != u32::MAX {
        let index =
            definitions
                .features
                .find_index(plot.feature_hash)
                .ok_or(Error::Unsupported(
                    "CityRangeAttack sight-through feature definition",
                ))?;
        value = value.wrapping_add(definitions.feature_through[index]);
    }
    Ok(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SightPlot {
    pub index: u32,
    pub distance: i32,
}
const HEX_X: [i32; 6] = [0, 1, 1, 0, -1, -1];
const HEX_Y: [i32; 6] = [1, 0, -1, -1, 0, 1];
fn row_half(y: i32) -> i32 {
    if y >= 0 {
        y >> 1
    } else {
        let a = y.wrapping_sub(1);
        a.wrapping_add(((a as u32) >> 31) as i32) >> 1
    }
}
fn hex_distance(x: i32, y: i32) -> i32 {
    let a = x.wrapping_abs();
    let b = y.wrapping_abs();
    if x ^ y < 0 {
        a.max(b)
    } else {
        a.wrapping_add(b)
    }
}
pub(crate) fn plot_distance(state: &LoadedState, a: (i32, i32), b: (i32, i32)) -> i32 {
    let map = state.world().map();
    let dx =
        b.0.wrapping_sub(row_half(b.1))
            .wrapping_sub(a.0.wrapping_sub(row_half(a.1)));
    let dy = b.1.wrapping_sub(a.1);
    let distance = hex_distance(dx, dy);
    if map.header.wrap_x {
        distance
            .min(hex_distance(dx.wrapping_sub(map.header.width), dy))
            .min(hex_distance(dx.wrapping_add(map.header.width), dy))
    } else {
        distance
    }
}
pub(crate) fn adjacent_location(
    state: &LoadedState,
    location: (i32, i32),
    direction: usize,
) -> Option<(i32, i32)> {
    let map = state.world().map();
    let y = location.1.wrapping_add(HEX_Y[direction]);
    let x = location
        .0
        .wrapping_sub(row_half(location.1))
        .wrapping_add(HEX_X[direction])
        .wrapping_add(row_half(y));
    let x = c6_core::movement_world::coord_range(x, map.header.width, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(y, map.header.height, map.header.wrap_y);
    (x >= 0 && y >= 0 && x < map.header.width && y < map.header.height).then_some((x, y))
}
fn sight_index(state: &LoadedState, p: (i32, i32)) -> Result<usize, Error> {
    let map = state.world().map();
    if p.0 < 0 || p.1 < 0 || p.0 >= map.header.width || p.1 >= map.header.height {
        return Err(Error::Unsupported(
            "CityRangeAttack sight scratch index outside map",
        ));
    }
    let index = p.1.wrapping_mul(map.header.width).wrapping_add(p.0) as u32 as usize;
    if index >= map.plots.len() {
        return Err(Error::Unsupported("CityRangeAttack sight map plot extent"));
    }
    Ok(index)
}
fn qmul(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}
fn line_offset(origin: (i32, i32), target: (i32, i32), neighbor: (i32, i32)) -> i32 {
    let pixel = |p: (i32, i32)| {
        (
            p.0.wrapping_shl(8)
                .wrapping_add(if p.1 & 1 != 0 { 128 } else { 0 }),
            p.1.wrapping_shl(8),
        )
    };
    let (sx, sy) = pixel(origin);
    let (tx, ty) = pixel(target);
    let (nx, ny) = pixel(neighbor);
    let a = ty.wrapping_sub(sy);
    let b = sx.wrapping_sub(tx);
    let c = qmul(sy, tx).wrapping_sub(qmul(ty, sx));
    qmul(a, nx)
        .wrapping_add(qmul(b, ny))
        .wrapping_add(c)
        .wrapping_abs()
}

/// Exact source spiral/height propagation for GetSightPlots. Both ignore flags
/// are false in the City RangeAttack empty-target path. Scratch height and its
/// 0x1000000 split-ray marker are ephemeral owned query storage, not a gameplay
/// cache or a supplied visibility/readiness answer. Native append order remains
/// observable to the caller, including wrapped duplicate coordinates.
pub fn sight_plots(
    state: &LoadedState,
    origin: (i32, i32),
    radius: i32,
    definitions: &Definitions,
) -> Result<Vec<SightPlot>, Error> {
    sight_plots_with_flags(state, origin, radius, false, false, definitions)
}

/// Shared native GetSightPlots for Unit sight. The through flags suppress the
/// corresponding obstruction modifier; they do not change the origin height.
/// The final radius+1 lookahead ring suppresses both modifiers independently.
/// GetSightDistanceModifier ignores its native PlayerTypes argument, so no
/// invented owner decision or player visibility answer is needed here.
pub fn sight_plots_with_flags(
    state: &LoadedState,
    origin: (i32, i32),
    radius: i32,
    through_features: bool,
    through_terrain: bool,
    definitions: &Definitions,
) -> Result<Vec<SightPlot>, Error> {
    if state.binding() != &definitions.binding {
        return Err(Error::ConfigurationIdentity);
    }
    let origin_height = sight_distance_modifier(state, origin, definitions)?;
    let map = state.world().map();
    let cells = usize::try_from(map.header.width)
        .ok()
        .and_then(|w| {
            usize::try_from(map.header.height)
                .ok()
                .and_then(|h| w.checked_mul(h))
        })
        .filter(|n| *n == map.plots.len())
        .ok_or(Error::Unsupported(
            "CityRangeAttack sight scratch dimensions",
        ))?;
    let mut scratch = Vec::new();
    scratch
        .try_reserve_exact(cells)
        .map_err(|_| Error::Unsupported("CityRangeAttack sight scratch allocation"))?;
    scratch.resize(cells, 0i32);
    let mut out = Vec::new();
    if radius < -1 {
        return Ok(out);
    }
    let last = radius
        .checked_add(1)
        .ok_or(Error::Unsupported("CityRangeAttack sight radius overflow"))?;
    // Native iteration is finite only while this signed cumulative ring count
    // does not overflow. Refuse oversized external contexts without truncating.
    if i64::from(last) * (i64::from(last) + 1) * 3 > i64::from(i32::MAX) - 1 {
        return Err(Error::Unsupported(
            "CityRangeAttack sight ring-count overflow",
        ));
    }
    let mut ring = 0i32;
    let mut end = 0i32;
    let mut steps = 0i32;
    let mut direction = 5usize;
    let (mut hx, mut hy) = (0i32, 0i32);
    let mut current = Some(origin);
    while ring <= last {
        if let Some(location) = current.filter(|p| p.0 > -9999 && p.1 > -9999) {
            let index = sight_index(state, location)?;
            if ring == 0 {
                out.push(SightPlot {
                    index: index as u32,
                    distance: 0,
                });
                scratch[index] = 0;
            } else {
                const PAIRS: [(usize, usize); 6] = [(1, 2), (3, 2), (3, 4), (4, 5), (5, 0), (1, 0)];
                let (d1, d2) = PAIRS[direction];
                let p1 = adjacent_location(state, location, d1);
                let p2 = adjacent_location(state, location, d2);
                let predecessor = |p: Option<(i32, i32)>| -> Result<(i32, bool), Error> {
                    let Some(p) = p else {
                        return Ok((-1, false));
                    };
                    if plot_distance(state, origin, p) != ring - 1 {
                        return Ok((-1, false));
                    }
                    let raw = scratch[sight_index(state, p)?];
                    Ok((
                        if raw < 0x1000000 {
                            raw
                        } else {
                            raw.wrapping_sub(0x1000000)
                        },
                        raw > 0xffffff,
                    ))
                };
                let (a, flag_a) = predecessor(p1)?;
                let (b, flag_b) = predecessor(p2)?;
                let modifier = sight_through_modifier(
                    state,
                    location,
                    ring != last && !through_features,
                    ring != last && !through_terrain,
                    definitions,
                )?;
                let visible;
                if flag_b || a == b || a == -1 || b == -1 || flag_a {
                    let chosen;
                    if !flag_a && a != -1 {
                        scratch[index] = modifier.max(a);
                        chosen = Some(a);
                    } else if flag_b || b == -1 {
                        if a == -1 {
                            if b == -1 {
                                scratch[index] = modifier;
                                chosen = None;
                            } else {
                                scratch[index] = modifier.max(b);
                                chosen = Some(b);
                            }
                        } else {
                            scratch[index] = modifier.max(a);
                            chosen = Some(a);
                        }
                    } else {
                        scratch[index] = modifier.max(b);
                        chosen = Some(b);
                    }
                    visible = chosen.is_some_and(|height| {
                        height < modifier || (ring <= radius && height <= origin_height)
                    });
                } else {
                    let o1 = line_offset(
                        origin,
                        location,
                        p1.ok_or(Error::Unsupported("CityRangeAttack sight predecessor1"))?,
                    );
                    let o2 = line_offset(
                        origin,
                        location,
                        p2.ok_or(Error::Unsupported("CityRangeAttack sight predecessor2"))?,
                    );
                    // Target float0.05 converts to q8 raw12; 0.5 converts128.
                    let selected = if o1.wrapping_sub(o2) > 12 {
                        Some(b)
                    } else if o2.wrapping_sub(o1) > 12 {
                        Some(a)
                    } else {
                        None
                    };
                    if let Some(height) = selected {
                        scratch[index] = modifier.max(height);
                        visible = modifier > height || (ring <= radius && height <= origin_height);
                    } else {
                        let low = a.min(b);
                        let high = a.max(b);
                        scratch[index] = if modifier < high {
                            modifier.max(low).wrapping_add(0x1000000)
                        } else {
                            modifier
                        };
                        visible = modifier > low || (ring <= radius && low <= origin_height);
                    }
                }
                if visible {
                    out.push(SightPlot {
                        index: index as u32,
                        distance: ring,
                    });
                }
            }
        }
        let (mut nx, mut ny);
        if steps < end {
            nx = hx.wrapping_add(HEX_X[direction]);
            ny = hy.wrapping_add(HEX_Y[direction]);
            if hex_distance(nx, ny) > ring {
                direction = (direction + 1) % 6;
                nx = hx.wrapping_add(HEX_X[direction]);
                ny = hy.wrapping_add(HEX_Y[direction]);
            }
        } else {
            nx = hx.wrapping_add(HEX_X[0]);
            ny = hy.wrapping_add(HEX_Y[0]);
            ring = ring.wrapping_add(1);
            end = end.wrapping_add(ring.wrapping_mul(6));
            direction = 1;
        }
        hx = nx;
        hy = ny;
        if hex_distance(nx, ny) > last {
            current = None;
        } else {
            let y = origin.1.wrapping_add(ny);
            let x = origin
                .0
                .wrapping_sub(row_half(origin.1))
                .wrapping_add(nx)
                .wrapping_add(row_half(y));
            let x = c6_core::movement_world::coord_range(x, map.header.width, map.header.wrap_x);
            let y = c6_core::movement_world::coord_range(y, map.header.height, map.header.wrap_y);
            current = (x >= 0 && y >= 0 && x < map.header.width && y < map.header.height)
                .then_some((x, y));
        }
        steps = steps.wrapping_add(1);
    }
    Ok(out)
}

fn is_enemy(state: &LoadedState, owner: i32, other: i32) -> Result<bool, Error> {
    state.world().player(owner).map_err(Error::World)?;
    if state
        .world()
        .valid_player(other)
        .map_err(Error::World)?
        .is_none()
    {
        return Ok(false);
    }
    if !state.player_alive(other).map_err(Error::State)? {
        return Ok(false);
    }
    state
        .game_relations()
        .map_err(Error::State)?
        .is_at_war(owner, other)
        .map_err(|_| Error::Unsupported("CityRangeAttack actual Game.DiplomaticRelations.IsAtWar"))
}

/// District ranged attacks reject an enemy defensible District, then select a
/// visible enemy combat Unit from the actual ground-layer ordered references.
/// Missing Unit combat/visibility closure is reached only for a real candidate.
pub fn test_ranged_attack_into(
    state: &LoadedState,
    district: &c6_core::player_districts::District,
    location: (i32, i32),
    queries: &Queries<'_>,
) -> Result<bool, Error> {
    let definitions = queries.definitions;
    let attack_owner = district_owner(state, district)?;
    let plot = if location.0 > -9999 && location.1 > -9999 {
        plot_at(state, location)?
    } else {
        None
    };
    if let Some(plot) = plot {
        if let Some(attributes) = &plot.city_attributes {
            if attributes.player != -1
                && attributes.district != u32::MAX
                && is_enemy(state, i32::from(attributes.player), attack_owner)?
            {
                if let Some(enemy) = plot_district(state, plot)? {
                    if district_defensible(state, enemy, definitions)? {
                        return Ok(false);
                    }
                }
            }
        }
    }
    // Native UnitContainer.GetUnits(...,layer=0) normalizes coordinates; absent
    // cells are the actual empty list rather than invented candidate objects.
    let Some(_plot) = plot else {
        return Ok(false);
    };
    let map = state.world().map();
    let x = c6_core::movement_world::coord_range(location.0, map.header.width, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(location.1, map.header.height, map.header.wrap_y);
    let index = sight_index(state, (x, y))?;
    let mut selected: Option<&c6_core::unit::Unit> = None;
    for reference in state.world().plot_units(0, index).map_err(Error::World)? {
        let Some(found) = state.world().unit(*reference).map_err(Error::World)? else {
            continue;
        };
        let unit = found.record;
        if unit.b_delayed_death || unit.e_owner == attack_owner {
            continue;
        }
        if attack_owner != -1 && !is_enemy(state, unit.e_owner, attack_owner)? {
            continue;
        }
        if !unit_is_combat(state, unit, definitions)? {
            continue;
        }
        if attack_owner != -1 && !unit_visible(state, unit, attack_owner, queries)? {
            continue;
        }
        if selected.is_none()
            || unit_combat(state, unit, definitions)?
                > unit_combat(
                    state,
                    selected.ok_or(Error::Unsupported(
                        "CityRangeAttack selected defender identity",
                    ))?,
                    definitions,
                )?
        {
            selected = Some(unit);
        }
    }
    Ok(selected.is_some())
}

fn unit_visible(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    viewer: i32,
    queries: &Queries<'_>,
) -> Result<bool, Error> {
    unit_visible_with_counts(
        state,
        unit,
        viewer,
        queries.definitions,
        |layer, normal| {
            combined_visibility_count(
                state,
                viewer,
                (unit.k_location.x, unit.k_location.y),
                layer,
                normal,
                queries,
            )
        },
        || {
            queries
                .combined_visibility
                .map(|v| v.hidden_layer0)
                .ok_or(Error::Unsupported(
                    "actual hidden visibility layer0 identity",
                ))
        },
    )
}

/// Shared source IsVisible(Unit*) control flow. Callers bind these lazy reads
/// to actual current shared-layer storage, never per-Unit visibility answers.
pub(crate) fn unit_visible_with_counts<E: From<Error>>(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    viewer: i32,
    definitions: &Definitions,
    mut count: impl FnMut(u32, bool) -> Result<i16, E>,
    hidden_layer0: impl FnOnce() -> Result<u32, E>,
) -> Result<bool, E> {
    let location = (unit.k_location.x, unit.k_location.y);
    if location.0 <= -9999 || location.1 <= -9999 {
        return Ok(false);
    }
    if count(0x11351026, true)? <= 0 {
        return Ok(false);
    }
    if unit.e_owner != viewer
        && unit.b_has_hidden_visibility
        && !unit.b_hidden_visibility_forfeited
        && count(hidden_layer0()?, false)? <= 0
    {
        return Ok(false);
    }
    Ok(unit_definition(state, unit, definitions)
        .map_err(E::from)?
        .is_some_and(|data| !data.team_visibility || unit.e_owner == viewer))
}
fn combined_visibility_count(
    state: &LoadedState,
    viewer: i32,
    location: (i32, i32),
    layer: u32,
    normal: bool,
    queries: &Queries<'_>,
) -> Result<i16, Error> {
    let authority=queries.combined_visibility.ok_or(Error::Unsupported("CityRangeAttack Visibility group1 normal layer11351026 signed i16 counts after Read/recalculate/outgoing visibility effects"))?;
    let owner = authority.owner(state, viewer)?;
    let index = owner
        .grid_width
        .wrapping_mul(location.1 as u32)
        .wrapping_add(location.0 as u32) as usize;
    // FindLayer traverses by type hash in vector order; small IDs are not
    // collection ordinals here. A missing hidden layer is native false. The
    // normal cached pointer is mandatory after InitializeVisibilityLayer.
    let Some(layer) = owner.layers.iter().find(|entry| entry.type_hash == layer) else {
        if normal {
            return Err(Error::Unsupported(
                "CityRangeAttack initialized normal combined visibility layer",
            ));
        }
        return Ok(0);
    };
    Ok(layer.values.get(index).copied().unwrap_or(0))
}

fn retained_player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state.world().player(owner).map_err(Error::World)?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.requested_owner == owner)
        .ok_or(Error::Unsupported("CityRangeAttack retained Unit owner"))
}
fn era_index(
    player: &c6_core::loaded_components::RetainedPlayer,
    definitions: &Definitions,
) -> Option<usize> {
    definitions.eras.find_index(player.eras.era_hash)
}
pub(crate) fn era_strength(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    definitions: &Definitions,
) -> Result<i32, Error> {
    let player = retained_player(state, unit.e_owner)?;
    Ok(era_index(player, definitions).map_or(0, |era| {
        unit.i_era_strength_modifier
            .wrapping_mul(definitions.era_chronology[era])
    }))
}
pub(crate) fn unit_formation_class(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    definitions: &Definitions,
) -> Result<i32, Error> {
    Ok(unit_definition(state, unit, definitions)?.map_or(-1, |d| d.formation_class))
}
fn unit_definition<'a>(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    definitions: &'a Definitions,
) -> Result<Option<&'a UnitCombatDefinition>, Error> {
    if state.binding() != &definitions.binding {
        return Err(Error::ConfigurationIdentity);
    }
    let runtime = state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(Error::State)?;
    let expected = definitions
        .units
        .find_index(unit.e_type_index)
        .and_then(|i| i32::try_from(i).ok())
        .unwrap_or(-1);
    if runtime.definition_index != expected {
        return Err(Error::Unsupported(
            "CityRangeAttack initialized Unit definition collection identity",
        ));
    }
    if runtime.definition_index == -1 {
        return Ok(None);
    }
    definitions
        .unit_combat
        .get(runtime.definition_index as usize)
        .map(Some)
        .ok_or(Error::Unsupported("CityRangeAttack Unit definition index"))
}
// Native bool order is domain!=0, isCorps. Wire order is land corps/army,
// then naval corps/army; domain0 is naval despite the historical reversed use.
fn modified_formation_strength(domain: i32, corps: bool, values: [i32; 4]) -> i32 {
    values[if domain != 0 {
        if corps {
            0
        } else {
            1
        }
    } else if corps {
        2
    } else {
        3
    }]
}
fn formation_combat_modifier(
    unit: &c6_core::unit::Unit,
    player: &c6_core::loaded_components::RetainedPlayer,
    definitions: &Definitions,
    domain: i32,
) -> i32 {
    let values = [
        player.units.land_corps_modified_strength,
        player.units.land_army_modified_strength,
        player.units.naval_corps_modified_strength,
        player.units.naval_army_modified_strength,
    ];
    match unit.e_military_formation {
        1 => definitions
            .corps_strength
            .wrapping_add(modified_formation_strength(domain, true, values)),
        2 => definitions
            .army_strength
            .wrapping_add(modified_formation_strength(domain, false, values)),
        _ => 0,
    }
}
/// Source GetCombat(false), including embarked era replacement and ordered
/// positive-strength guards. Typed combat modifiers and source era remapping
/// remain gameplay inputs; no native AI strength policy is executed.
pub fn unit_combat(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    definitions: &Definitions,
) -> Result<i32, Error> {
    let player = retained_player(state, unit.e_owner)?;
    let data = unit_definition(state, unit, definitions)?;
    let mut base = data.map_or(0, |d| d.melee);
    if data.is_some_and(|d| d.use_max_melee) && player.stats.max_melee_strength_trained > base {
        base = player.stats.max_melee_strength_trained;
    }
    if crate::unit_movement_state::unit_is_embarked(unit)
        && !unit.k_combat_modifers.b_fight_while_embarked
        && !data.is_some_and(|d| d.formation_class == 1 || d.formation_class == 2)
        && definitions
            .hero_classes
            .find_index(unit.k_hero.e_class)
            .is_none()
    {
        if let Some(era) = era_index(player, definitions) {
            base = definitions.era_embarked_strength[era];
        }
    }
    let modifier = unit.k_combat_modifers.i_combat_strength_modifier.max(0);
    let mut strength = base.wrapping_add(modifier);
    if i64::from(base) + i64::from(modifier) > 0 {
        if matches!(unit.e_military_formation, 1 | 2) {
            let addition = formation_combat_modifier(
                unit,
                player,
                definitions,
                state
                    .unit_runtime(unit.e_owner, unit.id)
                    .map_err(Error::State)?
                    .domain,
            );
            let positive = i64::from(strength) + i64::from(addition) > 0;
            strength = strength.wrapping_add(addition);
            if !positive {
                return Ok(strength);
            }
        }
        strength = strength.wrapping_add(era_strength(state, unit, definitions)?);
    }
    Ok(strength)
}
pub(crate) fn unit_ranged_combat(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    definitions: &Definitions,
) -> Result<i32, Error> {
    let player = retained_player(state, unit.e_owner)?;
    let data = unit_definition(state, unit, definitions)?;
    let base = data.map_or(0, |d| d.ranged);
    let modifier = unit.k_combat_modifers.i_ranged_strength_modifier.max(0);
    let mut strength = base.wrapping_add(modifier);
    if i64::from(base) + i64::from(modifier) > 0 {
        if matches!(unit.e_military_formation, 1 | 2) {
            let addition = formation_combat_modifier(
                unit,
                player,
                definitions,
                state
                    .unit_runtime(unit.e_owner, unit.id)
                    .map_err(Error::State)?
                    .domain,
            );
            let positive = i64::from(strength) + i64::from(addition) > 0;
            strength = strength.wrapping_add(addition);
            if !positive {
                return Ok(strength);
            }
        }
        strength = strength.wrapping_add(era_strength(state, unit, definitions)?);
    }
    Ok(strength)
}
pub(crate) fn unit_bombard_combat(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    definitions: &Definitions,
) -> Result<i32, Error> {
    let base = unit_definition(state, unit, definitions)?.map_or(0, |d| d.bombard);
    let modifier = unit.k_combat_modifers.i_bombard_strength_modifier.max(0);
    let mut strength = base.wrapping_add(modifier);
    if i64::from(base) + i64::from(modifier) > 0 {
        if matches!(unit.e_military_formation, 1 | 2) {
            let addition = if unit.e_military_formation == 1 {
                definitions.corps_strength
            } else {
                definitions.army_strength
            };
            let positive = i64::from(strength) + i64::from(addition) > 0;
            strength = strength.wrapping_add(addition);
            if !positive {
                return Ok(strength);
            }
        }
        strength = strength.wrapping_add(era_strength(state, unit, definitions)?);
    }
    Ok(strength)
}
fn unit_is_combat(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
    definitions: &Definitions,
) -> Result<bool, Error> {
    Ok(unit_combat(state, unit, definitions)? > 0
        || unit_ranged_combat(state, unit, definitions)? > 0
        || unit_bombard_combat(state, unit, definitions)? > 0)
}

#[cfg(test)]
mod formation_modifier_tests {
    #[test]
    fn native_bool_selector_preserves_distinct_land_naval_corps_army_fields() {
        use c6_core::unit_initialization::domain_from_hash;
        let sea = domain_from_hash(crate::registry::linux_type_hash(b"DOMAIN_SEA"));
        let land = domain_from_hash(crate::registry::linux_type_hash(b"DOMAIN_LAND"));
        assert_eq!(sea, 0);
        assert_eq!(land, 2);
        let values = [11, -23, 47, -89];
        assert_eq!(super::modified_formation_strength(sea, true, values), 47);
        assert_eq!(super::modified_formation_strength(sea, false, values), -89);
        assert_eq!(super::modified_formation_strength(land, true, values), 11);
        assert_eq!(super::modified_formation_strength(land, false, values), -23);
        for domain in [1, 2, -1, i32::MIN] {
            assert_eq!(super::modified_formation_strength(domain, true, values), 11);
            assert_eq!(
                super::modified_formation_strength(domain, false, values),
                -23
            );
        }
        assert_eq!(super::modified_formation_strength(0, true, values), 47);
        assert_eq!(super::modified_formation_strength(0, false, values), -89);
    }
}
