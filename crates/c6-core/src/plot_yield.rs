//! Linux XP2 Rules.Yields.GetYield arithmetic over typed rules and World inputs.
//!
//! Source implementation, not initialized native differential acceptance. Live
//! providers and ordered rules must be imported under the original module/rules
//! configuration gate. This does not admit gameplay actions or complete turns.

use crate::city_district::{ComponentId, Registry, CITY_CENTER_HASH};
use crate::improvement_yield::{
    improvement_yield, ImprovementAdjacency, ImprovementYieldError, ImprovementYieldPlot,
    OrderedImprovementYieldDefinitions,
};
use crate::map::{Component, DefinitionValue, Plot, PlotFlag, SparsePlotData};
use crate::movement_world::DefinitionLookup;
use crate::unit::variants::VariantPayload;
use crate::world::World;
use crate::yields::{NativeDefinitionIndex, NativeFixedPoint8, PrerequisiteState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    DefinitionShape,
    DefinitionIndex,
    Plot(u32),
    Adjacency { plot: u32, direction: u8 },
    References(&'static str),
    Provider(&'static str),
    City(crate::city_district::Error),
    Prerequisite(crate::yields::MissingInput),
    Improvement(ImprovementYieldError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "missing or inconsistent Linux plot-yield input: {self:?}"
        )
    }
}
impl std::error::Error for Error {}
impl From<crate::city_district::Error> for Error {
    fn from(value: crate::city_district::Error) -> Self {
        Self::City(value)
    }
}
impl From<crate::yields::MissingInput> for Error {
    fn from(value: crate::yields::MissingInput) -> Self {
        Self::Prerequisite(value)
    }
}
impl From<ImprovementYieldError> for Error {
    fn from(value: ImprovementYieldError) -> Self {
        Self::Improvement(value)
    }
}

/// Properties aligned with the actual native definition collection. Find uses
/// native small-index/hash/last-duplicate semantics supplied by DefinitionLookup.
pub struct Definitions<'a, T> {
    lookup: &'a dyn DefinitionLookup,
    values: &'a [T],
}
impl<T> Copy for Definitions<'_, T> {}
impl<T> Clone for Definitions<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, T> Definitions<'a, T> {
    pub fn new(lookup: &'a dyn DefinitionLookup, values: &'a [T]) -> Result<Self, Error> {
        if lookup.count() != values.len() || values.len() > i32::MAX as usize {
            return Err(Error::DefinitionShape);
        }
        Ok(Self { lookup, values })
    }
    fn at(&self, index: i32) -> Option<&T> {
        self.values.get(index as u32 as usize)
    }
    fn find_index(&self, key: u32) -> Result<i32, Error> {
        match self.lookup.find_index(key) {
            Some(index) if index < self.values.len() => Ok(index as i32),
            Some(_) => Err(Error::DefinitionIndex),
            None => Ok(-1),
        }
    }
    fn plot_index(&self, key: u32) -> Result<i32, Error> {
        Ok(i32::from(self.find_index(key)? as i16))
    }
    fn hash(&self, index: i32) -> Result<u32, Error> {
        self.lookup
            .type_hash(index as u32 as usize)
            .ok_or(Error::DefinitionIndex)
    }
}

/// Six original mutable YIELD_* global values. NameToYieldType recognizes only
/// these six hashes, not arbitrary registered yield definitions or small indices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YieldTypes {
    pub food: i32,
    pub production: i32,
    pub gold: i32,
    pub science: i32,
    pub culture: i32,
    pub faith: i32,
}
impl YieldTypes {
    pub fn from_name(self, hash: u32) -> i32 {
        match hash {
            0x24f0_57ec => self.food,
            0xbb19_ce0a => self.production,
            0xb761_634a => self.gold,
            0x3a81_f8ab => self.science,
            0xeb3f_55e8 => self.culture,
            0x7f0f_8e76 => self.faith,
            _ => -1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HashYieldChange {
    pub source_hash: u32,
    pub yield_hash: u32,
    pub change: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedYieldChange {
    pub yield_hash: u32,
    pub change: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YieldChange {
    pub yield_index: Option<NativeDefinitionIndex>,
    pub change: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppealYieldChange {
    pub yield_index: Option<NativeDefinitionIndex>,
    pub unimproved: bool,
    pub building_hash: u32,
    pub minimum: i32,
    pub maximum: i32,
    pub change: i32,
}
pub struct Feature<'a> {
    pub natural_wonder: bool,
    pub double_adjacent_terrain: bool,
    /// Original ordered references. Null references are not null yields.
    pub adjacent_yields: Option<&'a [YieldChange]>,
}
pub struct Improvement {
    pub appeal_yield_hash: u32,
    pub appeal_percent: i32,
}
pub struct District<'a> {
    pub citizen_yields: Option<&'a [Option<NamedYieldChange>]>,
    pub adjacent_appeal_yields: Option<&'a [AppealYieldChange]>,
}
pub struct Building<'a> {
    pub citizen_yields: Option<&'a [Option<NamedYieldChange>]>,
}
pub struct Resource<'a> {
    pub yields: Option<&'a [Option<YieldChange>]>,
}

#[derive(Clone, Copy)]
pub struct Rules<'a> {
    pub yield_types: YieldTypes,
    pub yields: Definitions<'a, ()>,
    pub terrains: Definitions<'a, ()>,
    pub features: Definitions<'a, Feature<'a>>,
    pub improvements: Definitions<'a, Improvement>,
    pub resources: Definitions<'a, Resource<'a>>,
    pub districts: Definitions<'a, District<'a>>,
    pub buildings: Definitions<'a, Building<'a>>,
    pub terrain_changes: Option<&'a [HashYieldChange]>,
    pub feature_changes: Option<&'a [HashYieldChange]>,
    pub improvement_yields: Option<OrderedImprovementYieldDefinitions<'a>>,
    pub city_food_minimum: Option<i32>,
    pub city_production_minimum: Option<i32>,
    pub drought_food_loss: Option<i32>,
}

/// Reconstructed native Feature-manager sparse IndexLists, never final plot
/// yields. Original reader Reset zeroes registry-sized vectors, then overlays
/// hash/value pairs sequentially. Unknown hashes are consumed, duplicate known
/// hashes overwrite, and the sparse default is imported just like other rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureYieldStorage {
    values: SparsePlotData<Vec<i32>>,
}
impl FeatureYieldStorage {
    pub fn from_world(world: &World, yields: &dyn DefinitionLookup) -> Result<Self, Error> {
        let mut features = world.map().components.iter().filter_map(|c| match c {
            Component::Feature(value) => Some(value),
            _ => None,
        });
        let feature = features
            .next()
            .ok_or(Error::Provider("Feature component"))?;
        if features.next().is_some() || yields.count() > i32::MAX as usize {
            return Err(Error::DefinitionShape);
        }
        fn overlay(
            rows: &[DefinitionValue],
            yields: &dyn DefinitionLookup,
        ) -> Result<Vec<i32>, Error> {
            let mut result = vec![0; yields.count()];
            for row in rows {
                if let Some(index) = yields.find_index(row.hash) {
                    *result.get_mut(index).ok_or(Error::DefinitionIndex)? = row.value;
                }
            }
            Ok(result)
        }
        let source = &feature.extra_yields;
        if source
            .indices
            .iter()
            .any(|&i| i < -1 || (i >= 0 && i as usize >= source.values.len()))
        {
            return Err(Error::Provider("Feature sparse index"));
        }
        Ok(Self {
            values: SparsePlotData {
                default: overlay(&source.default, yields)?,
                indices: source.indices.clone(),
                values: source
                    .values
                    .iter()
                    .map(|v| overlay(v, yields))
                    .collect::<Result<_, _>>()?,
            },
        })
    }
    pub fn get(&self, plot: i32, requested: i32) -> Result<i32, Error> {
        let values = self
            .values
            .get(plot)
            .map_err(|_| Error::Provider("Feature sparse index"))?;
        // Native const IndexList returns its reset default for an invalid index.
        Ok(values.get(requested as u32 as usize).copied().unwrap_or(0))
    }
}

/// Remaining live subsystems. Each default is unavailable, never a zero yield.
/// Implementations must reproduce source queries, not cache final plot results.
/// In particular city_effect_yield is actual PlotYieldData, and modifier methods are
/// separate City/Player rule queries. All methods borrow immutable state.
pub trait Runtime {
    fn terrain_work_impassable(&self, _player: i32, _terrain: i32) -> Result<bool, Error> {
        Err(Error::Provider("TerrainWorkImpassable"))
    }
    fn visible_resource(&self, _player: i32, _resource: i32) -> Result<i32, Error> {
        Err(Error::Provider("VisibleResource"))
    }
    fn improvement_property_yield(&self, _plot: u32, _yield_hash: u32) -> Result<i32, Error> {
        Err(Error::Provider("Improvement.Properties"))
    }
    fn unowned_appeal(&self, _plot: u32) -> Result<i32, Error> {
        Err(Error::Provider("Rules.Appeal"))
    }
    fn city_buildings_at_location(&self, _city: ComponentId, _plot: u32) -> Result<&[u32], Error> {
        Err(Error::Provider("BuildingsAtLocation"))
    }
    fn city_effect_yield(&self, _plot: u32, _yield: i32) -> Result<Option<i32>, Error> {
        Err(Error::Provider("CityPlots.PlotYieldData"))
    }
    fn building_location(&self, _city: ComponentId, _building: i32) -> Result<i32, Error> {
        Err(Error::Provider("BuildingLocation"))
    }
    fn building_pillaged(&self, _city: ComponentId, _building: i32) -> Result<bool, Error> {
        Err(Error::Provider("BuildingPillaged"))
    }
    /// None is the original GetValidPlayer null result, distinct from uncaptured.
    fn great_person_resource_yield(
        &self,
        _player: i32,
        _yield: i32,
        _resource: i32,
    ) -> Result<Option<i32>, Error> {
        Err(Error::Provider("GreatPeopleResourceYield"))
    }
    fn city_yield_per_flood(&self, _city: ComponentId, _yield: i32) -> Result<i32, Error> {
        Err(Error::Provider("YieldPerFlood"))
    }
    /// None is no existing named river; Some is GetFloods(-1), not rainfall.
    fn river_floods(&self, _plot: u32) -> Result<Option<i32>, Error> {
        Err(Error::Provider("RiverFloods"))
    }
    fn has_city_terrain_adjacent_improvement(
        &self,
        _city: ComponentId,
        _terrain: i32,
        _yield: i32,
    ) -> Result<bool, Error> {
        Err(Error::Provider("HasCityTerrainAdjacentImprovement"))
    }
    fn city_terrain_adjacent_improvement(
        &self,
        _city: ComponentId,
        _terrain: i32,
        _improvement: i32,
        _yield: i32,
    ) -> Result<NativeFixedPoint8, Error> {
        Err(Error::Provider("CityTerrainAdjacentImprovement"))
    }
    fn city_yield_per_terrain_class_improvement(
        &self,
        _city: ComponentId,
        _yield: i32,
        _improvement: i32,
    ) -> Result<i32, Error> {
        Err(Error::Provider("YieldPerTerrainClassCitiesImprovement"))
    }
    fn different_city_terrain_types(&self, _player: i32) -> Result<i32, Error> {
        Err(Error::Provider("DifferentCityTerrainTypes"))
    }
    fn active_drought_type(&self, _plot: u32) -> Result<i32, Error> {
        Err(Error::Provider("ActiveDroughtType"))
    }
    fn city_drought_prevention(&self, _city: ComponentId) -> Result<bool, Error> {
        Err(Error::Provider("DroughtPrevention"))
    }
}

/// Composition uses the actual owned Map/player state, existing concrete
/// City/District lookup, and original direction-ordered adjacency indices.
/// A missing adjacency table differs from a known boundary with no neighbor.
pub struct Context<'a, R, A> {
    pub world: &'a World,
    pub cities: &'a Registry<'a>,
    pub rules: &'a Rules<'a>,
    pub runtime: &'a R,
    pub improvement_adjacency: &'a A,
    pub feature_yields: Option<&'a FeatureYieldStorage>,
    pub adjacency: Option<&'a [[Option<u32>; 6]]>,
}
impl<R: Runtime, A: ImprovementAdjacency> Context<'_, R, A> {
    fn plot(&self, index: u32) -> Result<&Plot, Error> {
        self.world
            .map()
            .plot_by_index(index)
            .ok_or(Error::Plot(index))
    }
    fn adjacent(&self, index: u32, direction: u8) -> Result<Option<u32>, Error> {
        let neighbor = *self
            .adjacency
            .and_then(|rows| rows.get(index as usize))
            .and_then(|row| row.get(direction as usize))
            .ok_or(Error::Adjacency {
                plot: index,
                direction,
            })?;
        if let Some(id) = neighbor {
            self.plot(id)?;
        }
        Ok(neighbor)
    }
    fn owner(&self, index: u32) -> Result<i32, Error> {
        Ok(self
            .plot(index)?
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player)))
    }
    fn district_type(&self, index: u32) -> Result<i32, Error> {
        let Some(attributes) = &self.plot(index)?.city_attributes else {
            return Ok(-1);
        };
        Ok(self
            .cities
            .district(attributes)?
            .map_or(-1, |d| d.record.type_index))
    }
    fn is_city(&self, index: u32) -> Result<bool, Error> {
        let district = self.district_type(index)?;
        Ok(district != -1 && self.rules.districts.hash(district)? == CITY_CENTER_HASH)
    }
    fn owning_city(&self, index: u32) -> Result<Option<(ComponentId, i32)>, Error> {
        let Some(attributes) = &self.plot(index)?.city_attributes else {
            return Ok(None);
        };
        Ok(self
            .cities
            .owning_city(attributes)?
            .map(|c| (c.resolved, c.record.owner)))
    }
    fn appeal(&self, index: u32) -> Result<i32, Error> {
        if self.owner(index)? != -1 {
            Ok(i32::from(self.plot(index)?.appeal))
        } else {
            self.runtime.unowned_appeal(index)
        }
    }
    fn improvement_property_yield(&self, index: u32, yield_hash: u32) -> Result<i32, Error> {
        let Some(properties) = &self.plot(index)?.properties else {
            return Ok(0);
        };
        let mut entries = properties
            .entries
            .iter()
            .filter(|entry| entry.key == 0xd9a3_4c75);
        let Some(entry) = entries.next() else {
            return Ok(0);
        };
        // Only the native typed-map provenance domain is handled here. Other
        // conversion kinds and duplicate-key reconstruction need their provider.
        if entries.next().is_none()
            && entry.value.type_bits == 10
            && entry.value.modifier_bits == 0
            && entry.value.value.count_flags & 0x5e00_0000 == 0x0400_0000
            && entry.value.value.count_flags & 0x00ff_ffff == 0
            && entry.value.value.element_size == 0
        {
            if let VariantPayload::TypedVariantMap(map) = &entry.value.value.payload {
                if !map.entries.iter().any(|entry| entry.key == 0x877b_95f0) {
                    return Ok(0);
                }
            }
        }
        self.runtime.improvement_property_yield(index, yield_hash)
    }
}

fn named_sum(rows: &[Option<NamedYieldChange>], types: YieldTypes, requested: i32) -> i32 {
    rows.iter()
        .flatten()
        .filter(|r| types.from_name(r.yield_hash) == requested)
        .fold(0i32, |sum, r| sum.wrapping_add(r.change))
}
fn matches(row: &YieldChange, requested: i32) -> bool {
    row.yield_index.map(NativeDefinitionIndex::get) == Some(requested)
}

/// Original 0x2bf5ad0 for a concrete native Plot (not a hypothetical subclass).
/// `natural_only` is the native bool; monitor callers pass false. No output
/// contribution list or hypothetical map changes are accepted by this API.
pub fn get_yield<R: Runtime, A: ImprovementAdjacency>(
    ctx: &Context<'_, R, A>,
    plot: u32,
    requested: i32,
    fallback_player: i32,
    natural_only: bool,
) -> Result<i32, Error> {
    let rules = ctx.rules;
    let record = ctx.plot(plot)?;
    let owner = ctx.owner(plot)?;
    let player = if owner != -1 { owner } else { fallback_player };
    let wonder = match &record.city_attributes {
        Some(a) => rules.buildings.plot_index(a.wonder_building_hash)?,
        None => -1,
    };
    if wonder != -1 {
        return Ok(0);
    }
    if record.flags.get(PlotFlag::Impassable) {
        if owner == -1 {
            return Ok(0);
        }
        let terrain = rules.terrains.plot_index(record.terrain_hash)?;
        if !ctx.runtime.terrain_work_impassable(owner, terrain)? {
            return Ok(0);
        }
    }
    if !ctx.is_city(plot)? {
        let district = ctx.district_type(plot)?;
        if district != -1 {
            if natural_only {
                return Ok(0);
            }
            let attributes = record
                .city_attributes
                .as_ref()
                .ok_or(Error::Provider("CityAttributes"))?;
            if !ctx
                .cities
                .district(attributes)?
                .is_some_and(|d| d.record.complete)
            {
                return Ok(0);
            }
            let Some(definition) = rules.districts.at(district) else {
                return Ok(0);
            };
            let mut result = named_sum(
                definition
                    .citizen_yields
                    .ok_or(Error::References("District.CitizenYields"))?,
                rules.yield_types,
                requested,
            );
            if let Some((city, _)) = ctx.owning_city(plot)? {
                for &building in ctx.runtime.city_buildings_at_location(city, plot)? {
                    let index = rules.buildings.find_index(building)?;
                    let definition = rules.buildings.at(index).ok_or(Error::DefinitionIndex)?;
                    result = result.wrapping_add(named_sum(
                        definition
                            .citizen_yields
                            .ok_or(Error::References("Building.CitizenYields"))?,
                        rules.yield_types,
                        requested,
                    ));
                }
            }
            return Ok(result);
        }
    }
    let mut result = 0i32;
    let terrain = rules.terrains.plot_index(record.terrain_hash)?;
    if terrain != -1 {
        let hash = rules.terrains.hash(terrain)?;
        for row in rules
            .terrain_changes
            .ok_or(Error::References("Terrain.YieldChanges"))?
        {
            if row.source_hash == hash && rules.yield_types.from_name(row.yield_hash) == requested {
                let mut multiplier = 1;
                for direction in 0..6 {
                    if let Some(adjacent) = ctx.adjacent(plot, direction)? {
                        let index = rules
                            .features
                            .plot_index(ctx.plot(adjacent)?.feature_hash)?;
                        if index != -1
                            && rules
                                .features
                                .at(index)
                                .ok_or(Error::DefinitionIndex)?
                                .double_adjacent_terrain
                        {
                            multiplier = 2;
                        }
                    }
                }
                result = row.change.wrapping_mul(multiplier);
                break;
            }
        }
    }
    let feature = rules.features.plot_index(record.feature_hash)?;
    if feature != -1 {
        let definition = rules.features.at(feature).ok_or(Error::DefinitionIndex)?;
        if definition.natural_wonder {
            result = 0;
        }
        let hash = rules.features.hash(feature)?;
        for row in rules
            .feature_changes
            .ok_or(Error::References("Feature.YieldChanges"))?
        {
            if row.source_hash == hash && rules.yield_types.from_name(row.yield_hash) == requested {
                result = result.wrapping_add(row.change);
                break;
            }
        }
    }
    result = result.wrapping_add(
        ctx.feature_yields
            .ok_or(Error::Provider("Feature.PlotYield"))?
            .get(plot as i32, requested)?,
    );
    let resource = rules.resources.plot_index(record.resource_hash)?;
    let visible_resource = if player == -1 || player == 1000 {
        resource
    } else {
        ctx.runtime.visible_resource(player, resource)?
    };
    if visible_resource != -1 {
        let definition = rules
            .resources
            .at(visible_resource)
            .ok_or(Error::DefinitionIndex)?;
        for row in definition
            .yields
            .ok_or(Error::References("Resource.YieldChanges"))?
            .iter()
            .flatten()
        {
            if matches(row, requested) {
                result = result.wrapping_add(row.change);
                break;
            }
        }
    }
    let improvement = rules.improvements.plot_index(record.improvement_hash)?;
    if !natural_only && improvement != -1 && !record.flags.get(PlotFlag::ImprovementPillaged) {
        let definition = rules
            .improvements
            .at(improvement)
            .ok_or(Error::DefinitionIndex)?;
        let improvement_rules = rules
            .improvement_yields
            .ok_or(Error::Provider("Improvement.Yields"))?;
        if !improvement_rules.matches_lookup(rules.improvements.lookup) {
            return Err(Error::DefinitionShape);
        }
        if improvement_rules
            .at(improvement)
            .ok_or(Error::DefinitionIndex)?
            .base
            .ok_or(Error::References("Improvement.YieldChanges"))?
            .iter()
            .any(|r| r.yield_index.get() == requested)
        {
            result = result.wrapping_add(improvement_yield(
                improvement_rules,
                ctx.world,
                ctx.improvement_adjacency,
                improvement as u32,
                requested,
                Some(ImprovementYieldPlot {
                    id: plot,
                    political_owner: owner,
                }),
            )?);
        }
        if rules.yield_types.from_name(definition.appeal_yield_hash) == requested {
            let appeal_yield = ctx.appeal(plot)?.wrapping_mul(definition.appeal_percent) / 100;
            // Native constructs FixedPoint8 from this quotient then calls AsInt.
            result = result.wrapping_add(NativeFixedPoint8::integer(appeal_yield).0 >> 8);
        }
        if rules.yields.at(requested).is_some() {
            result = result
                .wrapping_add(ctx.improvement_property_yield(plot, rules.yields.hash(requested)?)?);
        }
    }
    if !natural_only {
        if ctx.is_city(plot)? {
            if requested == rules.yield_types.food {
                result = result.max(
                    rules
                        .city_food_minimum
                        .ok_or(Error::Provider("CityFoodMinimum"))?,
                );
            } else if requested == rules.yield_types.production {
                result = result.max(
                    rules
                        .city_production_minimum
                        .ok_or(Error::Provider("CityProductionMinimum"))?,
                );
            }
        }
        if let Some(value) = ctx.runtime.city_effect_yield(plot, requested)? {
            result = result.wrapping_add(value);
        }
    }
    for direction in 0..6 {
        let Some(adjacent) = ctx.adjacent(plot, direction)? else {
            continue;
        };
        let neighbor = ctx.plot(adjacent)?;
        let feature = rules.features.plot_index(neighbor.feature_hash)?;
        if feature != -1 {
            for row in rules
                .features
                .at(feature)
                .ok_or(Error::DefinitionIndex)?
                .adjacent_yields
                .ok_or(Error::References("Feature.AdjacentYields"))?
            {
                if matches(row, requested) {
                    result = result.wrapping_add(row.change);
                }
            }
        }
        let district = rules
            .districts
            .find_index(ctx.district_type(adjacent)? as u32)?;
        if district == -1 || natural_only {
            continue;
        }
        let rows = rules
            .districts
            .at(district)
            .ok_or(Error::DefinitionIndex)?
            .adjacent_appeal_yields;
        let Some((city, city_owner)) = ctx.owning_city(adjacent)? else {
            continue;
        };
        if city_owner != owner {
            continue;
        }
        for row in rows.ok_or(Error::References("District.AdjacentAppealYields"))? {
            if row.yield_index.map(NativeDefinitionIndex::get) != Some(requested) {
                continue;
            }
            if improvement != -1 && row.unimproved {
                continue;
            }
            let building = rules.buildings.find_index(row.building_hash)?;
            if building != -1 {
                if ctx.runtime.building_location(city, building)? != adjacent as i32 {
                    continue;
                }
                if ctx.runtime.building_pillaged(city, building)? {
                    continue;
                }
            }
            let appeal = ctx.appeal(plot)?;
            if appeal >= row.minimum && appeal <= row.maximum {
                result = result.wrapping_add(row.change);
            }
        }
    }
    if !natural_only {
        if ctx.world.is_valid_player(player)? && resource != -1 {
            if let Some(value) = ctx
                .runtime
                .great_person_resource_yield(player, requested, resource)?
            {
                result = result.wrapping_add(value);
            }
        }
        if let Some((city, _)) = ctx.owning_city(plot)? {
            if ctx.runtime.city_yield_per_flood(city, requested)? > 0 {
                if let Some(floods) = ctx.runtime.river_floods(plot)? {
                    result = result.wrapping_add(
                        ctx.runtime
                            .city_yield_per_flood(city, requested)?
                            .wrapping_mul(floods),
                    );
                }
            }
            if ctx
                .runtime
                .has_city_terrain_adjacent_improvement(city, terrain, requested)?
            {
                for direction in 0..6 {
                    if let Some(adjacent) = ctx.adjacent(plot, direction)? {
                        if ctx.owner(adjacent)? != owner {
                            continue;
                        }
                        let improvement = rules
                            .improvements
                            .plot_index(ctx.plot(adjacent)?.improvement_hash)?;
                        if improvement != -1 {
                            result = result.wrapping_add(
                                ctx.runtime
                                    .city_terrain_adjacent_improvement(
                                        city,
                                        terrain,
                                        improvement,
                                        requested,
                                    )?
                                    .0
                                    >> 8,
                            );
                        }
                    }
                }
            }
            if owner != -1
                && ctx.runtime.city_yield_per_terrain_class_improvement(
                    city,
                    requested,
                    improvement,
                )? > 0
            {
                let change = ctx.runtime.city_yield_per_terrain_class_improvement(
                    city,
                    requested,
                    improvement,
                )?;
                result = result.wrapping_add(
                    change.wrapping_mul(ctx.runtime.different_city_terrain_types(owner)?),
                );
            }
        }
    }
    if requested == rules.yield_types.food && ctx.runtime.active_drought_type(plot)? != -1 {
        let protected = if owner == -1 {
            false
        } else if let Some((city, _)) = ctx.owning_city(plot)? {
            ctx.runtime.city_drought_prevention(city)?
        } else {
            false
        };
        if !protected {
            result = result
                .wrapping_sub(
                    rules
                        .drought_food_loss
                        .ok_or(Error::Provider("DroughtFoodLoss"))?,
                )
                .max(0);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city_district::{CityRecord, DistrictRecord, PlayerRegistry, SlotTable};
    use crate::improvement_yield::{ImprovementBaseYield, ImprovementYieldDefinition};
    use crate::map::{
        CityAttributes, FeatureComponent, Header, Limits, Map, Recycled, UnitContainer,
    };
    use crate::unit::PlotCoord;
    use crate::yields::ImprovementDefinition;
    use std::cell::RefCell;

    struct Lookup(&'static [u32]);
    impl DefinitionLookup for Lookup {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            self.0.get(index).copied()
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            if (key as usize) < self.0.len() {
                Some(key as usize)
            } else {
                self.0.iter().rposition(|&hash| hash == key)
            }
        }
    }
    static YIELDS: Lookup = Lookup(&[
        0x24f057ec, 0xbb19ce0a, 0xb761634a, 0x3a81f8ab, 0xeb3f55e8, 0x7f0f8e76,
    ]);
    static TERRAINS: Lookup = Lookup(&[100]);
    static FEATURES: Lookup = Lookup(&[200, 201]);
    static RESOURCES: Lookup = Lookup(&[300]);
    static IMPROVEMENTS: Lookup = Lookup(&[400]);
    static DISTRICTS: Lookup = Lookup(&[500, CITY_CENTER_HASH]);
    static BUILDINGS: Lookup = Lookup(&[600]);
    fn index(value: u32) -> NativeDefinitionIndex {
        NativeDefinitionIndex::new(value, 6).unwrap()
    }
    fn recycled<T>() -> Recycled<T> {
        Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: 0,
            free_tail: 0,
            free_links: vec![],
        }
    }
    fn world() -> World {
        let mut plot = Plot::decode(&[0; 55], 6, Limits::default()).unwrap();
        plot.terrain_hash = 100;
        plot.feature_hash = u32::MAX;
        plot.resource_hash = u32::MAX;
        plot.improvement_hash = u32::MAX;
        let map = Map {
            header: Header {
                width: 3,
                height: 1,
                top_latitude: 90,
                bottom_latitude: -90,
                wrap_x: false,
                wrap_y: false,
                map_size_hash: 0,
                land_plots: 3,
                owned_plots: 0,
                natural_wonders: 0,
                resource_counts: vec![],
                land_resource_counts: vec![],
            },
            visibility: vec![],
            areas: recycled(),
            landmasses: recycled(),
            regions: recycled(),
            chokepoints: recycled(),
            continents: vec![],
            feature_layouts: recycled(),
            plots: vec![plot; 3],
            units: UnitContainer {
                width: 3,
                height: 1,
                layers: vec![],
            },
            components: vec![Component::Feature(Box::new(FeatureComponent {
                extra_yields: SparsePlotData {
                    default: vec![DefinitionValue {
                        hash: YIELDS.0[0],
                        value: 3,
                    }],
                    indices: vec![-1; 3],
                    values: vec![],
                },
                volcanoes: vec![],
                natural_wonder_volcanoes: vec![],
                plot_volcanoes: SparsePlotData {
                    default: vec![],
                    indices: vec![],
                    values: vec![],
                },
                ice: vec![],
            }))],
        };
        let mut world =
            World::new(map, vec![crate::world::Player::new(0, None, None).unwrap()]).unwrap();
        let mut mapping = [-1; 65];
        mapping[0] = 0;
        world.player_mapping = Some(mapping);
        world
    }
    fn attributes(district: u32, city: u32) -> CityAttributes {
        CityAttributes {
            player: 0,
            district,
            owning_city: city,
            working_city: u32::MAX,
            wonder_building_hash: u32::MAX,
        }
    }
    #[derive(Default)]
    struct Live {
        calls: RefCell<Vec<&'static str>>,
        drought: i32,
        effects: Option<i32>,
    }
    impl Runtime for Live {
        fn visible_resource(&self, _: i32, resource: i32) -> Result<i32, Error> {
            self.calls.borrow_mut().push("resource");
            Ok(resource)
        }
        fn city_effect_yield(&self, _: u32, _: i32) -> Result<Option<i32>, Error> {
            self.calls.borrow_mut().push("effects");
            Ok(self.effects)
        }
        fn city_buildings_at_location(&self, _: ComponentId, _: u32) -> Result<&[u32], Error> {
            self.calls.borrow_mut().push("buildings");
            Ok(&[600])
        }
        fn active_drought_type(&self, _: u32) -> Result<i32, Error> {
            self.calls.borrow_mut().push("drought");
            Ok(self.drought)
        }
        fn city_drought_prevention(&self, _: ComponentId) -> Result<bool, Error> {
            self.calls.borrow_mut().push("prevention");
            Ok(false)
        }
    }
    struct NoAdjacency;
    impl ImprovementAdjacency for NoAdjacency {
        fn player_adjacency_yield(
            &self,
            _: i32,
            _: NativeDefinitionIndex,
            _: i32,
            _: Option<u32>,
        ) -> Result<i32, ImprovementYieldError> {
            panic!("empty improvement adjacency references must skip provider")
        }
    }

    // Source fixtures contain duplicate terrain/feature/resource matches, null
    // citizen refs, all-row neighboring bonuses, and uncaptured unrelated data.
    fn with_context<R: Runtime, T>(
        world: &World,
        runtime: &R,
        natural_wonder: bool,
        body: impl FnOnce(&mut Context<'_, R, NoAdjacency>) -> T,
    ) -> T {
        let resource_rows = [
            None,
            Some(YieldChange {
                yield_index: Some(index(0)),
                change: 7,
            }),
            Some(YieldChange {
                yield_index: Some(index(0)),
                change: 999,
            }),
        ];
        let adjacent_rows = [
            YieldChange {
                yield_index: Some(index(0)),
                change: 2,
            },
            YieldChange {
                yield_index: None,
                change: 999,
            },
            YieldChange {
                yield_index: Some(index(0)),
                change: 4,
            },
        ];
        let district_rows = [
            None,
            Some(NamedYieldChange {
                yield_hash: YIELDS.0[0],
                change: 8,
            }),
            Some(NamedYieldChange {
                yield_hash: YIELDS.0[0],
                change: -2,
            }),
        ];
        let building_rows = [
            Some(NamedYieldChange {
                yield_hash: YIELDS.0[0],
                change: 3,
            }),
            None,
            Some(NamedYieldChange {
                yield_hash: YIELDS.0[0],
                change: 4,
            }),
        ];
        let feature_values = [
            Feature {
                natural_wonder,
                double_adjacent_terrain: true,
                adjacent_yields: Some(&adjacent_rows),
            },
            Feature {
                natural_wonder: false,
                double_adjacent_terrain: false,
                adjacent_yields: Some(&[]),
            },
        ];
        let resource_values = [Resource {
            yields: Some(&resource_rows),
        }];
        let improvement_values = [Improvement {
            appeal_yield_hash: YIELDS.0[0],
            appeal_percent: 250,
        }];
        let district_values = [
            District {
                citizen_yields: Some(&district_rows),
                adjacent_appeal_yields: Some(&[]),
            },
            District {
                citizen_yields: Some(&[]),
                adjacent_appeal_yields: Some(&[]),
            },
        ];
        let building_values = [Building {
            citizen_yields: Some(&building_rows),
        }];
        let terrain_changes = [
            HashYieldChange {
                source_hash: 100,
                yield_hash: YIELDS.0[0],
                change: 3,
            },
            HashYieldChange {
                source_hash: 100,
                yield_hash: YIELDS.0[0],
                change: 999,
            },
        ];
        let feature_changes = [
            HashYieldChange {
                source_hash: 200,
                yield_hash: YIELDS.0[0],
                change: 5,
            },
            HashYieldChange {
                source_hash: 200,
                yield_hash: YIELDS.0[0],
                change: 999,
            },
        ];
        let improvement_base = [ImprovementBaseYield {
            yield_index: index(0),
            change: 9,
        }];
        let improvement_defs = [ImprovementYieldDefinition {
            type_hash: 400,
            base: Some(&improvement_base),
            adjacency: Some(&[]),
        }];
        let improvement_bonuses = [ImprovementDefinition {
            type_hash: 400,
            bonus_yield_changes: Some(&[]),
        }];
        let rules = Rules {
            yield_types: YieldTypes {
                food: 0,
                production: 1,
                gold: 2,
                science: 3,
                culture: 4,
                faith: 5,
            },
            yields: Definitions::new(&YIELDS, &[(); 6]).unwrap(),
            terrains: Definitions::new(&TERRAINS, &[()]).unwrap(),
            features: Definitions::new(&FEATURES, &feature_values).unwrap(),
            resources: Definitions::new(&RESOURCES, &resource_values).unwrap(),
            improvements: Definitions::new(&IMPROVEMENTS, &improvement_values).unwrap(),
            districts: Definitions::new(&DISTRICTS, &district_values).unwrap(),
            buildings: Definitions::new(&BUILDINGS, &building_values).unwrap(),
            terrain_changes: Some(&terrain_changes),
            feature_changes: Some(&feature_changes),
            improvement_yields: Some(
                OrderedImprovementYieldDefinitions::new(
                    &IMPROVEMENTS,
                    &improvement_defs,
                    &improvement_bonuses,
                )
                .unwrap(),
            ),
            city_food_minimum: Some(2),
            city_production_minimum: Some(1),
            drought_food_loss: Some(i32::MIN),
        };
        // A complete concrete registry: district id 1 is non-city, id 2 is city.
        let districts = [
            DistrictRecord {
                id: 1,
                owner: 0,
                location: PlotCoord { x: 0, y: 0 },
                parent_city: 1,
                type_index: 0,
                complete: true,
            },
            DistrictRecord {
                id: 2,
                owner: 0,
                location: PlotCoord { x: 0, y: 0 },
                parent_city: 1,
                type_index: 1,
                complete: true,
            },
        ];
        let cities = [CityRecord {
            id: 1,
            owner: 0,
            location: PlotCoord { x: 0, y: 0 },
        }];
        let mut players = [PlayerRegistry::Invalid; 65];
        players[0] = PlayerRegistry::Valid {
            cities: Some(SlotTable::new(&cities).unwrap()),
            districts: Some(SlotTable::new(&districts).unwrap()),
        };
        let registry = Registry::new(players);
        let storage = FeatureYieldStorage::from_world(world, &YIELDS).unwrap();
        let adjacency = [
            [Some(1), Some(2), None, None, None, None],
            [None; 6],
            [None; 6],
        ];
        // Prevent accidental test accommodation by giving no general appeal-row,
        // City modifier, property conversion, or unowned appeal provider.
        let mut context = Context {
            world,
            cities: &registry,
            rules: &rules,
            runtime,
            improvement_adjacency: &NoAdjacency,
            feature_yields: Some(&storage),
            adjacency: Some(&adjacency),
        };
        body(&mut context)
    }

    #[test]
    fn first_base_matches_and_all_neighbor_rows_preserve_doubling_cap() {
        let mut world = world();
        for i in 1..3 {
            world.plot_mut(i).unwrap().feature_hash = 200;
        }
        let live = Live {
            drought: -1,
            ..Default::default()
        };
        with_context(&world, &live, false, |ctx| {
            // First terrain 3 doubled ONCE, stored Feature 3, two neighbors 6 each.
            assert_eq!(get_yield(ctx, 0, 0, -1, true), Ok(21));
            assert_eq!(*live.calls.borrow(), ["drought"]);
        });
    }

    #[test]
    fn natural_wonder_replaces_terrain_and_resource_uses_first_nonnull_match() {
        let mut world = world();
        world.plot_mut(0).unwrap().feature_hash = 200;
        world.plot_mut(0).unwrap().resource_hash = 300;
        let live = Live {
            drought: -1,
            ..Default::default()
        };
        with_context(&world, &live, true, |ctx| {
            assert_eq!(get_yield(ctx, 0, 0, -1, true), Ok(15))
        });
    }

    #[test]
    fn actual_world_improvement_owner_fixedpoint_and_drought_wrap_compose() {
        let mut world = world();
        let plot = world.plot_mut(0).unwrap();
        plot.city_attributes = Some(attributes(u32::MAX, u32::MAX));
        plot.improvement_hash = 400;
        plot.improvement_owner = 27; // Original GetYieldFrom uses political owner0.
        plot.appeal = -3;
        let live = Live {
            drought: -1,
            effects: Some(11),
            ..Default::default()
        };
        with_context(&world, &live, false, |ctx| {
            assert_eq!(get_yield(ctx, 0, 0, -1, false), Ok(19))
        });
        assert_eq!(*live.calls.borrow(), ["resource", "effects", "drought"]);
        let drought = Live {
            drought: 4,
            ..Default::default()
        };
        with_context(&world, &drought, false, |ctx| {
            // Natural-only still applies drought; signed wrapping subtraction
            // of i32::MIN produces a negative result then native cmovs zeros it.
            assert_eq!(get_yield(ctx, 0, 0, -1, true), Ok(0));
        });
    }

    #[test]
    fn completed_district_sums_citizen_refs_and_bypasses_terrain_and_climate() {
        let mut world = world();
        world.plot_mut(0).unwrap().city_attributes = Some(attributes(1, 1));
        let live = Live::default();
        with_context(&world, &live, false, |ctx| {
            ctx.feature_yields = None;
            ctx.adjacency = None;
            assert_eq!(get_yield(ctx, 0, 0, -1, false), Ok(13));
            assert_eq!(get_yield(ctx, 0, 0, -1, true), Ok(0));
        });
        assert_eq!(*live.calls.borrow(), ["buildings"]);
    }

    #[test]
    fn early_guards_and_missing_inputs_preserve_query_demand_and_world() {
        let mut world = world();
        world.plot_mut(0).unwrap().city_attributes = Some(CityAttributes {
            wonder_building_hash: 600,
            ..attributes(1, 1)
        });
        let live = Live::default();
        with_context(&world, &live, false, |ctx| {
            ctx.feature_yields = None;
            ctx.adjacency = None;
            assert_eq!(get_yield(ctx, 0, 0, -1, false), Ok(0));
        });
        assert!(live.calls.borrow().is_empty());
        world.plot_mut(0).unwrap().city_attributes = None;
        let before = world.clone();
        with_context(&world, &live, false, |ctx| {
            ctx.feature_yields = None;
            ctx.adjacency = None;
            // No matching terrain row for science: native skips its neighbor
            // scan, then demands Feature storage before the later neighbor scan.
            assert_eq!(
                get_yield(ctx, 0, 3, -1, true),
                Err(Error::Provider("Feature.PlotYield"))
            );
            // A matching food row demands terrain neighbors first.
            assert_eq!(
                get_yield(ctx, 0, 0, -1, true),
                Err(Error::Adjacency {
                    plot: 0,
                    direction: 0
                })
            );
        });
        assert_eq!(world, before);
    }

    #[test]
    fn feature_wire_overlay_retains_sparse_default_and_last_duplicate() {
        let mut world = world();
        // Owned World intentionally exposes only Plot mutation. Rebuild this
        // source fixture through the actual public map ownership constructor.
        let mut map = world.map().clone();
        let Component::Feature(component) = &mut map.components[0] else {
            unreachable!()
        };
        component.extra_yields.indices = vec![-1, 0];
        component.extra_yields.values = vec![vec![
            DefinitionValue {
                hash: YIELDS.0[0],
                value: 8,
            },
            DefinitionValue {
                hash: 123456,
                value: 999,
            },
            DefinitionValue {
                hash: YIELDS.0[0],
                value: -7,
            },
        ]];
        world = World::new(map, vec![]).unwrap();
        let storage = FeatureYieldStorage::from_world(&world, &YIELDS).unwrap();
        assert_eq!(storage.get(0, 0), Ok(3));
        assert_eq!(storage.get(1, 0), Ok(-7));
        assert_eq!(storage.get(2, 0), Ok(3));
        assert_eq!(storage.get(-1, 0), Ok(3));
        assert_eq!(storage.get(1, -1), Ok(0));
        assert_eq!(storage.get(1, 6), Ok(0));
    }

    #[test]
    fn name_to_yield_only_recognizes_six_hashes_and_reads_supplied_globals() {
        let types = YieldTypes {
            food: 17,
            production: 23,
            gold: 4,
            science: 8,
            culture: 9,
            faith: 12,
        };
        assert_eq!(types.from_name(0x24f057ec), 17);
        assert_eq!(types.from_name(0), -1);
        assert_eq!(types.from_name(17), -1);
        assert_eq!(types.from_name(0xb761634a), 4);
    }

    #[test]
    fn city_minimum_precedes_effects_and_terrain_multiply_wraps_before_additions() {
        let mut world = world();
        world.plot_mut(0).unwrap().city_attributes = Some(attributes(2, u32::MAX));
        let live = Live {
            drought: -1,
            effects: Some(11),
            ..Default::default()
        };
        with_context(&world, &live, false, |ctx| {
            let rules = Rules {
                city_food_minimum: Some(20),
                ..*ctx.rules
            };
            let local = Context {
                world: ctx.world,
                cities: ctx.cities,
                rules: &rules,
                runtime: ctx.runtime,
                improvement_adjacency: ctx.improvement_adjacency,
                feature_yields: ctx.feature_yields,
                adjacency: ctx.adjacency,
            };
            assert_eq!(get_yield(&local, 0, 0, -1, false), Ok(31));
        });
        world.plot_mut(0).unwrap().city_attributes = None;
        for index in 1..3 {
            world.plot_mut(index).unwrap().feature_hash = 200;
        }
        with_context(&world, &live, false, |ctx| {
            let rows = [HashYieldChange {
                source_hash: 100,
                yield_hash: YIELDS.0[0],
                change: i32::MAX,
            }];
            let rules = Rules {
                terrain_changes: Some(&rows),
                ..*ctx.rules
            };
            let local = Context {
                world: ctx.world,
                cities: ctx.cities,
                rules: &rules,
                runtime: ctx.runtime,
                improvement_adjacency: ctx.improvement_adjacency,
                feature_yields: ctx.feature_yields,
                adjacency: ctx.adjacency,
            };
            assert_eq!(get_yield(&local, 0, 0, -1, true), Ok(13));
        });
    }

    #[test]
    fn known_empty_improvement_map_is_zero_and_present_yield_property_requires_provider() {
        use crate::unit::variants::{ArrayVariant, TypedEntry, TypedVariant, TypedVariantMap};
        let mut world = world();
        let plot = world.plot_mut(0).unwrap();
        plot.improvement_hash = 400;
        plot.city_attributes = Some(attributes(u32::MAX, u32::MAX));
        plot.properties = Some(TypedVariantMap {
            entries: vec![TypedEntry {
                key: 0xd9a34c75,
                name: None,
                value: TypedVariant {
                    type_bits: 10,
                    modifier_bits: 0,
                    value: ArrayVariant {
                        count_flags: 0x04000000,
                        element_size: 0,
                        payload: VariantPayload::TypedVariantMap(Box::default()),
                    },
                },
            }],
            ..Default::default()
        });
        // Production has neither a matching declared base nor appeal-yield name.
        let live = Live {
            drought: -1,
            ..Default::default()
        };
        with_context(&world, &live, false, |ctx| {
            assert_eq!(get_yield(ctx, 0, 1, -1, false), Ok(0))
        });
        let properties = world.plot_mut(0).unwrap().properties.as_mut().unwrap();
        let VariantPayload::TypedVariantMap(map) = &mut properties.entries[0].value.value.payload
        else {
            unreachable!()
        };
        map.entries.push(TypedEntry {
            key: 0x877b95f0,
            name: None,
            value: TypedVariant {
                type_bits: 2,
                modifier_bits: 0,
                value: ArrayVariant {
                    count_flags: 1,
                    element_size: 4,
                    payload: VariantPayload::Inline32(17),
                },
            },
        });
        let before = world.clone();
        with_context(&world, &live, false, |ctx| {
            assert_eq!(
                get_yield(ctx, 0, 1, -1, false),
                Err(Error::Provider("Improvement.Properties"))
            )
        });
        assert_eq!(world, before);
    }

    struct Late {
        calls: RefCell<Vec<&'static str>>,
        building_location: i32,
        building_pillaged: bool,
    }
    impl Late {
        fn new() -> Self {
            Self {
                calls: RefCell::default(),
                building_location: 1,
                building_pillaged: false,
            }
        }
        fn called(&self, name: &'static str) {
            self.calls.borrow_mut().push(name);
        }
    }
    impl Runtime for Late {
        fn visible_resource(&self, _: i32, resource: i32) -> Result<i32, Error> {
            self.called("visible");
            Ok(resource)
        }
        fn city_effect_yield(&self, _: u32, _: i32) -> Result<Option<i32>, Error> {
            self.called("effects");
            Ok(None)
        }
        fn great_person_resource_yield(
            &self,
            _: i32,
            _: i32,
            _: i32,
        ) -> Result<Option<i32>, Error> {
            self.called("great-person");
            Ok(Some(i32::MAX))
        }
        fn city_yield_per_flood(&self, _: ComponentId, _: i32) -> Result<i32, Error> {
            self.called("flood-yield");
            Ok(3)
        }
        fn river_floods(&self, _: u32) -> Result<Option<i32>, Error> {
            self.called("river");
            Ok(Some(i32::MAX))
        }
        fn has_city_terrain_adjacent_improvement(
            &self,
            _: ComponentId,
            _: i32,
            _: i32,
        ) -> Result<bool, Error> {
            self.called("has-adjacent");
            Ok(true)
        }
        fn city_terrain_adjacent_improvement(
            &self,
            _: ComponentId,
            _: i32,
            _: i32,
            _: i32,
        ) -> Result<NativeFixedPoint8, Error> {
            self.called("adjacent");
            Ok(NativeFixedPoint8(-1))
        }
        fn city_yield_per_terrain_class_improvement(
            &self,
            _: ComponentId,
            _: i32,
            _: i32,
        ) -> Result<i32, Error> {
            self.called("terrain-class");
            Ok(3)
        }
        fn different_city_terrain_types(&self, _: i32) -> Result<i32, Error> {
            self.called("different-terrain");
            Ok(i32::MAX)
        }
        fn active_drought_type(&self, _: u32) -> Result<i32, Error> {
            self.called("drought");
            Ok(-1)
        }
        fn building_location(&self, _: ComponentId, _: i32) -> Result<i32, Error> {
            self.called("location");
            Ok(self.building_location)
        }
        fn building_pillaged(&self, _: ComponentId, _: i32) -> Result<bool, Error> {
            self.called("pillage");
            Ok(self.building_pillaged)
        }
    }

    #[test]
    fn late_queries_preserve_native_repetition_order_wrap_and_negative_fixedpoint_floor() {
        let mut world = world();
        let plot = world.plot_mut(0).unwrap();
        plot.city_attributes = Some(attributes(u32::MAX, 1));
        plot.resource_hash = 300;
        plot.improvement_hash = 400;
        plot.appeal = -3;
        for index in 1..3 {
            let plot = world.plot_mut(index).unwrap();
            plot.city_attributes = Some(attributes(u32::MAX, u32::MAX));
            plot.improvement_hash = 400;
        }
        let live = Late::new();
        let before = world.clone();
        with_context(&world, &live, false, |ctx| {
            // 15 + MAX wraps; 3*MAX wraps to2147483645; each raw -1
            // FixedPoint8 adjacent bonus floors to -1; final 3*MAX wraps again.
            assert_eq!(get_yield(ctx, 0, 0, -1, false), Ok(-2147483642));
        });
        assert_eq!(
            *live.calls.borrow(),
            [
                "visible",
                "effects",
                "great-person",
                "flood-yield",
                "river",
                "flood-yield",
                "has-adjacent",
                "adjacent",
                "adjacent",
                "terrain-class",
                "terrain-class",
                "different-terrain",
                "drought"
            ]
        );
        assert_eq!(world, before);
        let missing = Live {
            drought: -1,
            ..Default::default()
        };
        with_context(&world, &missing, false, |ctx| {
            assert_eq!(
                get_yield(ctx, 0, 0, -1, false),
                Err(Error::Provider("GreatPeopleResourceYield"))
            );
        });
        assert_eq!(*missing.calls.borrow(), ["resource", "effects"]);
        world.plot_mut(0).unwrap().resource_hash = u32::MAX;
        missing.calls.borrow_mut().clear();
        with_context(&world, &missing, false, |ctx| {
            assert_eq!(
                get_yield(ctx, 0, 0, -1, false),
                Err(Error::Provider("YieldPerFlood"))
            );
        });
        assert_eq!(*missing.calls.borrow(), ["resource", "effects"]);
    }

    fn appeal_query(
        world: &World,
        live: &Late,
        rows: Option<&[AppealYieldChange]>,
    ) -> Result<i32, Error> {
        with_context(world, live, false, |ctx| {
            let districts = [
                District {
                    citizen_yields: None,
                    adjacent_appeal_yields: rows,
                },
                District {
                    citizen_yields: None,
                    adjacent_appeal_yields: None,
                },
            ];
            let rules = Rules {
                districts: Definitions::new(&DISTRICTS, &districts).unwrap(),
                ..*ctx.rules
            };
            let local = Context {
                world: ctx.world,
                cities: ctx.cities,
                rules: &rules,
                runtime: ctx.runtime,
                improvement_adjacency: ctx.improvement_adjacency,
                feature_yields: ctx.feature_yields,
                adjacency: ctx.adjacency,
            };
            get_yield(&local, 0, 0, -1, false)
        })
    }

    #[test]
    fn adjacent_appeal_guards_precede_providers_and_range_bounds_are_inclusive() {
        let mut world = world();
        let plot = world.plot_mut(0).unwrap();
        plot.city_attributes = Some(attributes(u32::MAX, u32::MAX));
        plot.appeal = -3;
        world.plot_mut(1).unwrap().city_attributes = Some(attributes(1, 1));
        let mut live = Late::new();
        let rows = [
            AppealYieldChange {
                yield_index: Some(index(0)),
                unimproved: true,
                building_hash: 600,
                minimum: -3,
                maximum: -3,
                change: i32::MAX,
            },
            AppealYieldChange {
                yield_index: Some(index(0)),
                unimproved: true,
                building_hash: u32::MAX,
                minimum: -3,
                maximum: -3,
                change: 7,
            },
            AppealYieldChange {
                yield_index: Some(index(1)),
                unimproved: false,
                building_hash: 600,
                minimum: i32::MIN,
                maximum: i32::MAX,
                change: 999,
            },
        ];
        assert_eq!(appeal_query(&world, &live, Some(&rows)), Ok(-2147483636));
        assert_eq!(
            *live.calls.borrow(),
            ["visible", "effects", "location", "pillage", "drought"]
        );
        // A location mismatch skips the pillage getter; absent building hashes
        // do not invent a missing-building veto for the second matching row.
        live.calls.borrow_mut().clear();
        live.building_location = 2;
        assert_eq!(appeal_query(&world, &live, Some(&rows)), Ok(13));
        assert_eq!(
            *live.calls.borrow(),
            ["visible", "effects", "location", "drought"]
        );
        live.calls.borrow_mut().clear();
        live.building_location = 1;
        live.building_pillaged = true;
        assert_eq!(appeal_query(&world, &live, Some(&rows)), Ok(13));
        assert_eq!(
            *live.calls.borrow(),
            ["visible", "effects", "location", "pillage", "drought"]
        );
        live.calls.borrow_mut().clear();
        live.building_pillaged = false;
        world.plot_mut(0).unwrap().appeal = -4;
        assert_eq!(appeal_query(&world, &live, Some(&rows)), Ok(6));
        // An existing improvement suppresses unimproved rows before any building
        // provider. Its own base9 and appeal(-3*250/100)=-7 still contribute.
        world.plot_mut(0).unwrap().appeal = -3;
        world.plot_mut(0).unwrap().improvement_hash = 400;
        live.calls.borrow_mut().clear();
        assert_eq!(appeal_query(&world, &live, Some(&rows)), Ok(8));
        assert_eq!(*live.calls.borrow(), ["visible", "effects", "drought"]);
        world.plot_mut(0).unwrap().improvement_hash = u32::MAX;
        assert_eq!(
            appeal_query(&world, &live, None),
            Err(Error::References("District.AdjacentAppealYields"))
        );
        // Different source/neighbor-city owners skip even uncaptured appeal rows.
        world
            .plot_mut(0)
            .unwrap()
            .city_attributes
            .as_mut()
            .unwrap()
            .player = 1;
        live.calls.borrow_mut().clear();
        assert_eq!(appeal_query(&world, &live, None), Ok(6));
        assert_eq!(*live.calls.borrow(), ["visible", "effects", "drought"]);
        // Missing neighbor owning city skips those rows before owner comparison.
        world
            .plot_mut(0)
            .unwrap()
            .city_attributes
            .as_mut()
            .unwrap()
            .player = 0;
        world
            .plot_mut(1)
            .unwrap()
            .city_attributes
            .as_mut()
            .unwrap()
            .owning_city = u32::MAX;
        assert_eq!(appeal_query(&world, &live, None), Ok(6));
    }

    #[test]
    fn unowned_appeal_is_an_explicit_unavailable_query_not_a_cached_short_default() {
        let mut world = world();
        world.plot_mut(0).unwrap().improvement_hash = 400;
        world.plot_mut(0).unwrap().appeal = 32000;
        let before = world.clone();
        let live = Live {
            drought: -1,
            ..Default::default()
        };
        with_context(&world, &live, false, |ctx| {
            assert_eq!(
                get_yield(ctx, 0, 0, -1, false),
                Err(Error::Provider("Rules.Appeal"))
            );
        });
        assert_eq!(world, before);
        assert!(live.calls.borrow().is_empty());
    }
}
