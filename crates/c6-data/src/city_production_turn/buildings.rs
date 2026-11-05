//! Cached Linux definitions and canonical non-wonder building creation.
//! Pre/PostCreate preserve reached rule hooks, grants, reports and signals;
//! mandatory effects cannot be replaced by silent success implementations.
use super::Error;
use crate::{
    registry::linux_type_hash, resource_binding::ResourceAuthority,
    unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    city_production::{GreatWorkBuilding, GreatWorkSlot},
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    movement_world::DefinitionLookup,
};

#[derive(Clone, Debug)]
pub struct Building {
    pub hash: u32,
    pub capital: bool,
    pub prerequisite_district: Option<u32>,
    pub requires_placement: bool,
    pub wonder: bool,
    pub enabled_by_religion: bool,
    pub outer_defense_strength: i32,
    pub base_cost: i32,
    pub housing: i32,
    pub(super) yields: super::building_yields::StaticYields,
    pub great_work_slots: Vec<(i32, i32)>,
    great_work_rows_present: bool,
    pub mutually_exclusive: Vec<u32>,
    dynamic_climate_cost: bool,
    xp_construction_effects: bool,
    coastal_construction_effects: bool,
    resource_cost_rows_present: bool,
    known_era_score_noop: bool,
}
pub struct Definitions {
    inputs: InputIdentities,
    pub(super) yield_hashes: Vec<u32>,
    pub buildings: Vec<Building>,
    speed_cost_multiplier: i32,
    production_rate: i32,
    government_bonuses: crate::registry::LinuxDefinitionRegistry,
}
fn integer(row: crate::Row<'_>, column: &'static str) -> Result<i32, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Integer(value) => Ok(*value as i32),
        SqlValue::Null => Ok(0),
        _ => Err(Error::Unsupported(column)),
    }
}
fn hash(row: crate::Row<'_>, column: &'static str) -> Result<Option<u32>, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Text(value) => Ok(Some(linux_type_hash(value))),
        SqlValue::Null => Ok(None),
        _ => Err(Error::Unsupported(column)),
    }
}
impl Definitions {
    /// Era/speed are actual current configuration hashes. Source2b2bed0 resolves
    /// StartEras.ProductionRate; source2be3300 resolves economic speed scaling.
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
        start_era: u32,
        game_speed: u32,
    ) -> Result<Self, Error> {
        if authority.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(Error::Definitions)?;
        Self::from_current_rules(state, rules, start_era, game_speed)
    }
    /// Cache static rows from an already profile-validated current Rules owner.
    /// Like growth parameters, this does not need Unit runtime reconstruction.
    /// The caller joins that Rules owner and actual era/speed configuration to
    /// the state; the cache remains bound to the state's complete input identity.
    pub fn from_current_rules(
        state: &LoadedState,
        rules: &Rules,
        start_era: u32,
        game_speed: u32,
    ) -> Result<Self, Error> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Unsupported("Linux building rules"));
        }
        let registry = |table, column| collection(rules, table, column).map_err(Error::Definitions);
        let building_registry = registry("Buildings", "BuildingType")?;
        // GetBuilderSql21fdd10 ->2e3c7b1 orders GreatWorkSlotTypes by rowid.
        // Culture.Initialize29778d0 stores Types.GetIndex into the named globals.
        let slots = registry("GreatWorkSlotTypes", "GreatWorkSlotType")?;
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let buildings_table = table("Buildings")?;
        let slots_table = table("Building_GreatWorks")?;
        let exclusive_table = table("MutuallyExclusiveBuildings")?;
        let resource_costs = table("Building_ResourceCosts")?;
        let resources = registry("Resources", "ResourceType")?;
        let xp = table("Buildings_XP2")?;
        let mut buildings = Vec::with_capacity(building_registry.entries().len());
        for (index, entry) in building_registry.entries().iter().enumerate() {
            let hash_value = building_registry.type_hash(index).ok_or(Error::Binding)?;
            let row = buildings_table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Binding)?;
            let mut slot_rows = slots_table
                .rows()
                .filter_map(|r| match hash(r, "BuildingType") {
                    Ok(Some(h)) if h == hash_value => Some(Ok(r)),
                    Err(e) => Some(Err(e)),
                    _ => None,
                })
                .collect::<Result<Vec<_>, Error>>()?;
            slot_rows.sort_by_key(|r| r.row_id());
            let great_work_rows_present = !slot_rows.is_empty();
            let mut great_work_slots = Vec::new();
            for slot in slot_rows {
                let slot_hash = hash(slot, "GreatWorkSlotType")?.ok_or(Error::Binding)?;
                // NameToGreatWorkSlotType21f13e0 rejects names outside these
                // named native globals, even if a mod defines an extra row.
                let known = [
                    0x699389cb, 0x6868a539, 0x6db6854c, 0x2f6c7bc9, 0xc8609506, 0x8abfc848,
                    0x00b22be8, 0xd8878949, 0x3e60d2de,
                ];
                if known.contains(&slot_hash) {
                    if let Some(index) = slots.find_index(slot_hash) {
                        great_work_slots.push((index as i32, integer(slot, "NumSlots")?));
                    }
                }
            }
            let mut mutually_exclusive = Vec::new();
            for row in exclusive_table.rows() {
                if hash(row, "Building")? == Some(hash_value) {
                    if let Some(value) = hash(row, "MutuallyExclusiveBuilding")? {
                        mutually_exclusive.push(value);
                    }
                }
            }
            let mut dynamic_climate_cost = false;
            let mut xp_construction_effects = false;
            let mut coastal_construction_effects = false;
            for row in xp.rows() {
                if hash(row, "BuildingType")? == Some(hash_value) {
                    coastal_construction_effects = integer(row, "BlocksCoastalFlooding")? != 0;
                    xp_construction_effects = coastal_construction_effects
                        || integer(row, "NuclearReactor")? != 0
                        || integer(row, "Bridge")? != 0
                        || integer(row, "CanalWonder")? != 0;
                    dynamic_climate_cost = integer(row, "CostMultiplierPerTile")? > 0
                        && integer(row, "CostMultiplierPerSeaLevel")? > 0;
                }
            }
            // UpdateBuildingProgress284ff30 and OnNewFrontBuilding285ada0
            // skip rows unless both definition references resolve.
            let mut resource_cost_rows_present = false;
            for row in resource_costs.rows() {
                if hash(row, "BuildingType")? == Some(hash_value)
                    && hash(row, "ResourceType")?.is_some_and(|h| resources.find(h).is_some())
                {
                    resource_cost_rows_present = true;
                }
            }
            let yield_data = super::building_yields::from_rules(
                rules,
                hash_value,
                integer(row, "RegionalRange")?,
            )?;
            let mut air_district = false;
            if let Some(prerequisite) = hash(row, "PrereqDistrict")? {
                for district in table("Districts")?.rows() {
                    if hash(district, "DistrictType")? == Some(prerequisite) {
                        air_district = hash(district, "MilitaryDomain")?
                            == Some(linux_type_hash(b"DOMAIN_AIR"));
                    }
                }
            }
            // Era subscriber299ca00 returns for wonders. Its remaining score
            // branches require science rows, GreatWork rows, an air district,
            // or a technology/civic prerequisite reaching chronology>=5.
            let known_era_score_noop = integer(row, "IsWonder")? != 0
                || (!yield_data.has_science_row()
                    && !great_work_rows_present
                    && !air_district
                    && hash(row, "PrereqTech")?.is_none()
                    && hash(row, "PrereqCivic")?.is_none());
            buildings.push(Building {
                hash: hash_value,
                capital: integer(row, "Capital")? != 0,
                prerequisite_district: hash(row, "PrereqDistrict")?,
                requires_placement: integer(row, "RequiresPlacement")? != 0,
                wonder: integer(row, "IsWonder")? != 0,
                enabled_by_religion: integer(row, "EnabledByReligion")? != 0,
                outer_defense_strength: integer(row, "OuterDefenseStrength")?,
                base_cost: integer(row, "Cost")?,
                housing: integer(row, "Housing")?,
                yields: yield_data,
                great_work_slots,
                great_work_rows_present,
                mutually_exclusive,
                dynamic_climate_cost,
                xp_construction_effects,
                coastal_construction_effects,
                resource_cost_rows_present,
                known_era_score_noop,
            });
        }
        let speed_registry = registry("GameSpeeds", "GameSpeedType")?;
        let speed_cost_multiplier = match speed_registry.find(game_speed) {
            Some(entry) => integer(
                table("GameSpeeds")?
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Binding)?,
                "CostMultiplier",
            )?,
            None => 100,
        };
        let eras = registry("StartEras", "EraType")?;
        // GetProductionRate first maps the configured EraTypes through Eras,
        // then passes that native index to StartEras.Find (direct-index priority).
        let era_index = registry("Eras", "EraType")?.find_index(start_era);
        let production_rate = match era_index.and_then(|index| eras.find(index as u32)) {
            Some(entry) => integer(
                table("StartEras")?
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Binding)?,
                "ProductionRate",
            )?,
            None => 1,
        };
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            yield_hashes: {
                let r = registry("Yields", "YieldType")?;
                (0..r.entries().len())
                    .map(|i| r.type_hash(i).ok_or(Error::Binding))
                    .collect::<Result<Vec<_>, _>>()?
            },
            buildings,
            speed_cost_multiplier,
            production_rate,
            government_bonuses: registry("GovernmentBonusNames", "GovernmentBonusType")?,
        })
    }
    /// Source299ca00 has no reachable ChangeEraScore branch for this definition.
    /// False requires the general current Era-rate/prerequisite query path;
    /// it does not imply that such a path necessarily awards a score.
    pub fn building_era_score_noop(
        &self,
        state: &LoadedState,
        building_hash: u32,
    ) -> Result<bool, Error> {
        self.validate(state)?;
        Ok(self.building(building_hash)?.known_era_score_noop)
    }
    /// Same subscriber with actual current PlayerScores rates. Static yield or
    /// prerequisite presence alone does not establish a positive era reward.
    /// All four zero rates prove every reached ChangeEraScore arm absent.
    pub fn building_era_score_noop_current(
        &self,
        state: &LoadedState,
        owner: i32,
        building_hash: u32,
    ) -> Result<bool, Error> {
        if self.building_era_score_noop(state, building_hash)? {
            return Ok(true);
        }
        let rates = state
            .game_era_construction_rates(owner)
            .map_err(|_| Error::Unsupported("current Era construction rates"))?;
        Ok(rates.is_none_or(|values| values.iter().all(|&value| value == 0)))
    }
    /// First-progress resource path284ff30 for a building with no resolved
    /// strategic-resource rows. Such a definition reaches no reservation,
    /// resource-stockpile or climate mutation. Resource-bearing definitions
    /// require their actual player-resource/climate services.
    pub fn start_building_progress(
        &self,
        state: &LoadedState,
        building_hash: u32,
    ) -> Result<(), Error> {
        self.validate(state)?;
        if self.building(building_hash)?.resource_cost_rows_present {
            return Err(Error::Unsupported(
                "building strategic-resource consumption",
            ));
        }
        Ok(())
    }
    /// Live non-wonder production factors; no captured yield/multiplier answer
    /// cache. Native GetFlatBonus returns zero in anarchy or for unknown types.
    pub fn building_parameters(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        building_hash: u32,
    ) -> Result<super::CurrentBuild, Error> {
        self.validate(state)?;
        let building = self.building(building_hash)?;
        if building.wonder {
            return Err(Error::Unsupported("wonder production factors"));
        }
        let value = state.city(owner, city)?;
        let queue = &value.build_queue;
        if queue.adjacent_city_river_building_production_multiplier > 256 {
            return Err(Error::Unsupported(
                "adjacent city river building multiplier",
            ));
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("production owner"))?;
        let bonus_hash = 0xc1a005d2;
        let bonus = if player.culture.anarchy_end_turn >= 0
            || self.government_bonuses.find_index(bonus_hash).is_none()
        {
            0
        } else {
            player.bonuses.flat_bonuses[index(&player.bonuses.flat_bonuses, bonus_hash)?].value
        };
        let definition_multiplier = queue.building_production_multiplier
            [index(&queue.building_production_multiplier, building_hash)?]
        .value;
        let mut factors = vec![
            queue.building_non_wonder_production_modifier,
            c6_core::city_production::execution::multiply(
                c6_core::city_production::execution::percent_multiplier(bonus),
                definition_multiplier,
            ),
        ];
        if let Some(district) = building.prerequisite_district {
            let mut multiplier = 256i32;
            for modifier in &queue.district_building_production_modifiers {
                if modifier.district_hash == district {
                    multiplier =
                        multiplier.wrapping_add(c6_core::city_production::execution::from_float(
                            modifier.modifier as f32 * 0.01,
                        ));
                }
            }
            factors.push(multiplier);
        }
        Ok(super::CurrentBuild {
            cost: self.production_cost(building)?,
            factors_q8: factors,
        })
    }
    pub fn validate(&self, state: &LoadedState) -> Result<(), Error> {
        if self.inputs != state.binding().inputs {
            Err(Error::Binding)
        } else {
            Ok(())
        }
    }
    pub fn building(&self, hash: u32) -> Result<&Building, Error> {
        self.buildings
            .iter()
            .find(|b| b.hash == hash)
            .ok_or(Error::Unsupported("Building definition"))
    }
    pub fn adjusted_base_cost(&self, building: &Building) -> i32 {
        self.adjust_cost(building.base_cost)
    }
    /// Economic.AdjustForGameSpeed(int), shared by gold purchases.
    pub fn adjust_cost(&self, cost: i32) -> i32 {
        let raw = cost.wrapping_shl(8) as i64;
        let speed = self.speed_cost_multiplier.wrapping_shl(8) as i64;
        ((raw * speed / 25_600).clamp(i32::MIN as i64, i32::MAX as i64) as i32) >> 8
    }
    /// GetBuildingProductionCost2854870 for no feature surcharge. Flood-barrier
    /// scaling remains a live climate query, never a guessed base-cost answer.
    pub fn production_cost(&self, building: &Building) -> Result<i32, Error> {
        if building.dynamic_climate_cost {
            return Err(Error::Unsupported("flood-barrier cost"));
        }
        let fraction = (i64::from(100i32.wrapping_sub(self.production_rate).wrapping_shl(8)) * 256
            / 25_600) as i32;
        Ok(c6_core::city_production::execution::multiply(
            self.adjusted_base_cost(building).wrapping_shl(8),
            fraction,
        ) >> 8)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Report {
    /// 4d2a6519/d8a30267; the report carries no location.
    QueueChanged,
    ConstructionStarted,
    /// OnNewFrontBuilding285b141: event2842ce14, Construct order=1.
    ProductionChanged,
    /// DoNewFrontCheck284dc11 / Add285a904: event92492f56,
    /// subtype d3a03c77 and current queue index.
    QueueEntryChanged { index: usize },
    BuildingProgress {
        percent: i32,
    },
    BuildingCompleted {
        construction_plot: i32,
    },
}
pub trait Effects {
    /// Recheck current Construct after CreateBuilding changes legality.
    fn check_active_production(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
    fn current_era(&mut self, state: &LoadedState) -> Result<i32, Error>;
    fn set_as_capital(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
    fn grant_yield(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        yield_hash: u32,
        amount_q8: i32,
    ) -> Result<(), Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        building: u32,
        location: [i32; 2],
        report: Report,
    ) -> Result<(), Error>;
    fn update_appeal(&mut self, state: &mut LoadedState, location: [i32; 2]) -> Result<(), Error>;
    fn building_signals(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        building: u32,
        plot: i32,
    ) -> Result<(), Error>;
}
pub(super) fn index<T>(values: &[c6_core::unit::HashValue<T>], hash: u32) -> Result<usize, Error> {
    let mut found = values
        .iter()
        .enumerate()
        .filter(|(_, v)| v.definition_hash == hash);
    let (index, _) = found
        .next()
        .ok_or(Error::Unsupported("missing building array entry"))?;
    if found.next().is_some() {
        return Err(Error::Unsupported("duplicate building array entry"));
    }
    Ok(index)
}
pub(super) fn plot_index(state: &LoadedState, location: [i32; 2]) -> Result<i32, Error> {
    let map = state.world().map();
    map.plot_at(location[0], location[1])
        .ok_or(Error::Unsupported("building plot"))?;
    Ok(location[1]
        .wrapping_mul(map.header.width)
        .wrapping_add(location[0]))
}
/// PreCreate's ordinary district/city plot branch. Replacements, religious
/// substitutions and wonder placement are explicit unsupported branches.
pub fn pre_create(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    building: &Building,
    requested_location: Option<[i32; 2]>,
    effects: &mut impl Effects,
) -> Result<Option<[i32; 2]>, Error> {
    if building.requires_placement
        || building.wonder
        || building.enabled_by_religion
        || !building.mutually_exclusive.is_empty()
        || building.coastal_construction_effects
    {
        return Err(Error::Unsupported("building placement/replacement branch"));
    }
    let value = state.city(owner, city)?;
    let mut location = [value.header.location_x, value.header.location_y];
    if let Some(prerequisite) = building.prerequisite_district {
        let mut found = None;
        for id in &value.districts.district_ids {
            let district = state.district(owner, *id).map_err(Error::District)?;
            if district.district_hash == prerequisite
                && requested_location
                    .is_none_or(|p| p == [district.location.0, district.location.1])
            {
                found = Some([district.location.0, district.location.1]);
                break;
            }
        }
        let Some(found) = found else {
            return Ok(None);
        };
        location = found;
    }
    let plot = plot_index(state, location)?;
    let queue = &value.build_queue;
    let construction = index(&queue.construction_plot, building.hash)?;
    let existing = index(&value.buildings.building_locations, building.hash)?;
    if queue.construction_plot[construction].value == -1 {
        let first = value.buildings.building_locations[existing].value == -1;
        state.city_mut(owner, city)?.build_queue.construction_plot[construction].value =
            plot as i16;
        // OnConstructionChanged2bd4380 only queries this plot unless the
        // actual XP2 definition blocks coastal flooding (rejected above).
        effects.report(
            state,
            owner,
            city,
            building.hash,
            location,
            if first {
                Report::ConstructionStarted
            } else {
                Report::BuildingProgress { percent: 0 }
            },
        )?;
    }
    Ok(Some(location))
}
/// UpdateBuildingProgress after the canonical progress addition: lazily begin
/// construction, compute percent while overflow remains live, publish the queue
/// change, then publish the sub-100 construction update. Synchronous consumers
/// can mutate the current construction plot, so it is re-read after the report.
pub fn progress_changed(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    building_hash: u32,
    definitions: &Definitions,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    definitions.validate(state)?;
    let building = definitions.building(building_hash)?;
    if building.coastal_construction_effects {
        return Err(Error::Unsupported("coastal construction progress effects"));
    }
    let queue = &state.city(owner, city)?.build_queue;
    let construction = index(&queue.construction_plot, building_hash)?;
    if queue.construction_plot[construction].value < 0 {
        pre_create(state, owner, city, building, None, effects)?;
    }
    let queue = &state.city(owner, city)?.build_queue;
    let progress = queue.building_progress[index(&queue.building_progress, building_hash)?].value;
    let percent = c6_core::city_production::execution::percent_complete(
        definitions.production_cost(building)?,
        progress,
        queue.production_overflow,
    )
    .ok_or(Error::Unsupported("wrapped building percentage divisor"))?;
    effects.report(
        state,
        owner,
        city,
        building_hash,
        [-9999, -9999],
        Report::QueueChanged,
    )?;
    let plot = state.city(owner, city)?.build_queue.construction_plot[construction].value;
    if percent < 100 && plot >= 0 {
        let width = state.world().map().header.width;
        if width <= 0 {
            return Err(Error::Unsupported("building map width"));
        }
        let location = [i32::from(plot) % width, i32::from(plot) / width];
        plot_index(state, location)?;
        // OnConstructionChanged2bd4380 has no gameplay write or RNG once
        // the actual BlocksCoastalFlooding definition branch is excluded.
        effects.report(
            state,
            owner,
            city,
            building_hash,
            location,
            Report::BuildingProgress { percent },
        )?;
    }
    Ok(())
}

/// PostCreate's non-wonder/non-defense branch, including actual slots, era,
/// completion grants and the mandatory synchronous subscribers.
pub fn post_create(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    building: &Building,
    location: [i32; 2],
    definitions: &Definitions,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    definitions.validate(state)?;
    validate_completion(state, owner, building)?;
    if building.wonder || building.requires_placement || building.outer_defense_strength != 0 {
        return Err(Error::Unsupported("wonder/defensive building completion"));
    }
    let plot = plot_index(state, location)?;
    let value = state.city(owner, city)?;
    let location_index = index(&value.buildings.building_locations, building.hash)?;
    let era_index = index(&value.buildings.era_created, building.hash)?;
    let construction_index = index(&value.build_queue.construction_plot, building.hash)?;
    let grants = value.header.yield_per_building_complete.clone();
    let adjusted = definitions.adjusted_base_cost(building).wrapping_shl(8);
    for grant in grants {
        if grant.value > 0 {
            let amount = (i64::from(adjusted) * i64::from(grant.value.wrapping_shl(8)) / 25_600)
                .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
            effects.grant_yield(state, owner, grant.definition_hash, amount)?;
        }
    }
    let old = state.city(owner, city)?.buildings.building_locations[location_index].value;
    if old != plot as i16 {
        state.city_mut(owner, city)?.buildings.building_locations[location_index].value =
            plot as i16;
        if building.capital {
            effects.set_as_capital(state, owner, city)?;
        }
        let extra = state.city(owner, city)?.buildings.extra_great_works.clone();
        let extra_count = extra
            .iter()
            .filter(|e| e.building_hash == building.hash)
            .fold(0i32, |n, e| n.wrapping_add(e.count));
        let mut slots = Vec::new();
        for (slot_type, count) in &building.great_work_slots {
            for _ in 0..*count {
                slots.push(GreatWorkSlot {
                    wire_version: 10,
                    slot_type: *slot_type,
                    great_work_index: -1,
                });
            }
        }
        for value in extra {
            if value.building_hash == building.hash {
                for _ in 0..value.count {
                    slots.push(GreatWorkSlot {
                        wire_version: 10,
                        slot_type: value.slot_type,
                        great_work_index: -1,
                    });
                }
            }
        }
        if building.great_work_rows_present || extra_count > 0 {
            state
                .city_mut(owner, city)?
                .buildings
                .great_work_buildings
                .push(GreatWorkBuilding {
                    building_hash: building.hash,
                    slots,
                });
        }
    }
    let era = effects.current_era(state)?;
    state.city_mut(owner, city)?.buildings.era_created[era_index].value = era as i16;
    state.city_mut(owner, city)?.build_queue.construction_plot[construction_index].value = -1;
    // OnConstructionComplete2bd4580 has no remaining writes/RNG for this
    // source-gated non-wonder/non-XP2/non-culture-bomb branch.
    effects.report(
        state,
        owner,
        city,
        building.hash,
        location,
        Report::BuildingProgress { percent: 100 },
    )?;
    effects.update_appeal(state, location)?;
    effects.report(
        state,
        owner,
        city,
        building.hash,
        location,
        Report::BuildingCompleted {
            construction_plot: plot,
        },
    )?;
    effects.building_signals(state, owner, city, building.hash, plot)?;
    // OnPostConstructionComplete2bd49e0 only dispatches for wonders.
    Ok(())
}

/// CreateBuilding285bea0's 100-percent, no-replacement, no-overflow-recovery
/// entry used by AddPalace. Production completion invokes PostCreate directly.
/// Queue removal/replacement and partial-percent construction stay explicit.
pub fn create_full(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    building_hash: u32,
    location: [i32; 2],
    definitions: &Definitions,
    effects: &mut impl Effects,
) -> Result<bool, Error> {
    definitions.validate(state)?;
    let building = definitions.building(building_hash)?;
    validate_completion(state, owner, building)?;
    if building.outer_defense_strength != 0 {
        return Err(Error::Unsupported("defensive building completion"));
    }
    if state
        .city(owner, city)?
        .build_queue
        .entries
        .iter()
        .any(|entry| entry.order == c6_core::city_production::Order::Construct { building_hash })
    {
        return Err(Error::Unsupported("CreateBuilding removes queued building"));
    }
    if pre_create(state, owner, city, building, Some(location), effects)?.is_none() {
        return Ok(false);
    }
    post_create(state, owner, city, building, location, definitions, effects)?;
    if matches!(
        state
            .city(owner, city)?
            .build_queue
            .entries
            .first()
            .map(|e| e.order),
        Some(c6_core::city_production::Order::Construct { .. })
    ) {
        effects.check_active_production(state, owner, city)?;
    }
    Ok(true)
}

fn validate_completion(state: &LoadedState, owner: i32, building: &Building) -> Result<(), Error> {
    if building.xp_construction_effects {
        return Err(Error::Unsupported("XP2 construction effects"));
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("building owner"))?;
    // IsCultureBombTrigger2a78a20 visits the actual ordered trigger records.
    if player
        .culture
        .culture_bomb_triggers
        .iter()
        .any(|t| t.building_hash == building.hash)
    {
        return Err(Error::Unsupported("construction culture bomb"));
    }
    Ok(())
}

/// CheckIfBuildingFinished2858e80: threshold equality pops exactly one entry,
/// updates previous-production identity, performs PostCreate, then resets that
/// building's progress. Repairing pillaged buildings needs SetPillaged effects.
pub fn finish_current(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    definitions: &Definitions,
    effects: &mut impl Effects,
) -> Result<bool, Error> {
    definitions.validate(state)?;
    let Some(c6_core::city_production::Order::Construct { building_hash }) = state
        .city(owner, city)?
        .build_queue
        .entries
        .first()
        .map(|e| e.order)
    else {
        return Ok(false);
    };
    let building = definitions.building(building_hash)?;
    let value = state.city(owner, city)?;
    let progress_index = index(&value.build_queue.building_progress, building_hash)?;
    let cost = definitions.production_cost(building)?.wrapping_shl(8);
    if value.build_queue.building_progress[progress_index].value < cost {
        return Ok(false);
    }
    if value.buildings.pillaged[index(&value.buildings.pillaged, building_hash)?].value {
        return Err(Error::Unsupported("building repair SetPillaged"));
    }
    validate_completion(state, owner, building)?;
    if building.wonder || building.requires_placement || building.outer_defense_strength != 0 {
        return Err(Error::Unsupported("wonder/defensive building completion"));
    }
    let plot = value.build_queue.construction_plot
        [index(&value.build_queue.construction_plot, building_hash)?]
    .value;
    let location = if plot == -1 {
        None
    } else {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot as i32 as u32)
            .ok_or(Error::Unsupported("construction plot coordinate"))?;
        Some([x, y])
    };
    let queue = &mut state.city_mut(owner, city)?.build_queue;
    queue.entries.remove(0);
    queue.previous_production_type_hash = building_hash;
    if let Some(location) = location {
        post_create(state, owner, city, building, location, definitions, effects)?;
    }
    let index = index(
        &state.city(owner, city)?.build_queue.building_progress,
        building_hash,
    )?;
    state.city_mut(owner, city)?.build_queue.building_progress[index].value = 0;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Definitions, Building) {
        use c6_core::core_player::initialization::{DatabaseIdentity, InputIdentities};
        let definitions = Definitions {
            inputs: InputIdentities {
                configuration_sha256: [0; 32],
                rules_inputs_sha256: [0; 32],
                database: DatabaseIdentity::LiveSnapshotUnavailable,
                dlc_inputs_sha256: [0; 32],
                mods_inputs_sha256: [0; 32],
                content_inputs_sha256: [0; 32],
            },
            buildings: vec![],
            yield_hashes: vec![],
            speed_cost_multiplier: 150,
            production_rate: -100,
            government_bonuses: crate::registry::LinuxDefinitionRegistry::from_native_order(vec![]),
        };
        let building = Building {
            hash: 7,
            capital: false,
            prerequisite_district: None,
            requires_placement: false,
            wonder: false,
            enabled_by_religion: false,
            outer_defense_strength: 0,
            base_cost: 1,
            housing: 0,
            yields: super::super::building_yields::StaticYields::default(),
            great_work_slots: vec![],
            great_work_rows_present: false,
            mutually_exclusive: vec![],
            dynamic_climate_cost: false,
            xp_construction_effects: false,
            coastal_construction_effects: false,
            resource_cost_rows_present: false,
            known_era_score_noop: true,
        };
        (definitions, building)
    }
    #[test]
    fn building_cost_truncates_speed_before_start_era_adjustment() {
        let (rules, building) = fixture();
        assert_eq!(rules.adjusted_base_cost(&building), 1);
        assert_eq!(rules.production_cost(&building).unwrap(), 2);
        let rules = Definitions {
            speed_cost_multiplier: 100,
            production_rate: 10,
            ..rules
        };
        let building = Building {
            base_cost: 67,
            ..building
        };
        assert_eq!(rules.production_cost(&building).unwrap(), 60);
    }
    #[test]
    fn climate_scaled_building_never_silently_uses_base_cost() {
        let (rules, mut building) = fixture();
        building.dynamic_climate_cost = true;
        assert!(matches!(
            rules.production_cost(&building),
            Err(Error::Unsupported("flood-barrier cost"))
        ));
    }
}
