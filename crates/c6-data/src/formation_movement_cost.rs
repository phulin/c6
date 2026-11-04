//! Linux XP2 Formation movement over current owned state and admitted definitions.
//! The cost is computed from the traversed plots; no path or expenditure answer
//! is imported. Complete MoveTo effects and native differential acceptance remain
//! separate from this read-only query boundary.
use crate::{
    city_movement::NativeTerrainDefinitions,
    formation_rules,
    game_parameters::{IntegerParameter, Parameters},
    loaded_actions::Error as CombatError,
    movement_placement_state::{Context as Placement, Definitions as PlacementDefinitions},
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    unit_sight_state::Runtime as VisibilityRuntime,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::{CityAttributes, Plot, PlotFlag},
    movement::{self, CostAccumulator, FormationFacts, MoveBudget, RuleParameters, ZocValue},
    movement_world::{
        self, DefinitionLookup, DefinitionTable, DistrictInstance, MovementDefinitions,
        RuntimePlotInputs,
    },
    unit::{Formation, PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    Binding,
    Rules(&'static str),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Movement(movement_world::MissingInput),
    Placement(crate::movement_placement_state::Error),
    Combat(CombatError),
    Relations(c6_core::game_diplomatic_relations::Error),
    Missing(&'static str),
}
impl From<CombatError> for Error {
    fn from(e: CombatError) -> Self {
        Self::Combat(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Formation movement: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy)]
enum PropertyValue {
    Absent,
    Integer(i32),
}

struct Properties<T> {
    registry: LinuxDefinitionRegistry,
    values: Vec<T>,
}
impl<T> Properties<T> {
    fn load(
        rules: &Rules,
        table: &'static str,
        key: &'static str,
        mut read: impl FnMut(&crate::Row<'_>) -> Result<T, Error>,
    ) -> Result<Self, Error> {
        let registry = collection(rules, table, key).map_err(|_| Error::Rules(table))?;
        let rows = rules.gameplay().table(table).ok_or(Error::Rules(table))?;
        let values = registry
            .entries()
            .iter()
            .map(|e| {
                read(
                    &rows
                        .row_by_id(e.database_rowid)
                        .ok_or(Error::Rules(table))?,
                )
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { registry, values })
    }
    fn table(&self) -> Result<DefinitionTable<'_, T>, Error> {
        DefinitionTable::new(&self.registry, &self.values).map_err(Error::Movement)
    }
}
fn integer(row: &crate::Row<'_>, key: &'static str) -> Result<i32, Error> {
    i32::try_from(row.integer(key).map_err(|_| Error::Rules(key))?).map_err(|_| Error::Rules(key))
}
fn boolean(row: &crate::Row<'_>, key: &'static str) -> Result<bool, Error> {
    match integer(row, key)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Rules(key)),
    }
}

/// Construct once for the admitted resources. Costs retain native collection
/// order, including binary32 route getter values. Mutable Unit/Plot state is
/// read again on every call; this cache contains no action result.
pub struct Definitions<'a> {
    binding: WireBinding,
    terrains: &'a NativeTerrainDefinitions,
    features: Properties<i32>,
    improvements: Properties<i32>,
    routes: Properties<u32>,
    districts: Properties<bool>,
    district_hit_points: Vec<Option<i32>>,
    parameters: RuleParameters,
    plot_unit_limit: u32,
    max_hit_points: i32,
    combat: formation_rules::Definitions,
    improvement_damage: Vec<[PropertyValue; 2]>,
}
impl<'a> Definitions<'a> {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        resources: &ResourceAuthority<'_>,
        terrains: &'a NativeTerrainDefinitions,
        placement: &PlacementDefinitions<'_, '_>,
    ) -> Result<Self, Error> {
        if state.binding() != resources.destination_binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, resources)
            .map_err(|_| Error::Rules("resources"))?;
        let terrain = terrains.as_core().map_err(|_| Error::Rules("Terrains"))?;
        let admitted = placement.terrain_definitions();
        if terrain.lookup.count() != admitted.lookup.count()
            || (0..terrain.lookup.count()).any(|i| {
                terrain.lookup.type_hash(i) != admitted.lookup.type_hash(i)
                    || terrain.at(i as i32) != admitted.at(i as i32)
            })
        {
            return Err(Error::Rules("Terrain binding"));
        }
        let parameters =
            Parameters::from_rules(rules).map_err(|_| Error::Rules("GlobalParameters"))?;
        let districts = Properties::load(rules, "Districts", "DistrictType", |row| {
            boolean(row, "FreeEmbark")
        })?;
        let district_rows = rules
            .gameplay()
            .table("Districts")
            .ok_or(Error::Rules("Districts"))?;
        let district_hit_points = districts
            .registry
            .entries()
            .iter()
            .map(|entry| {
                let row = district_rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Rules("Districts"))?;
                match row
                    .get("HitPoints")
                    .map_err(|_| Error::Rules("HitPoints"))?
                {
                    SqlValue::Null => Ok(None),
                    _ => integer(&row, "HitPoints").map(Some),
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            terrains,
            features: Properties::load(rules, "Features", "FeatureType", |row| {
                integer(row, "MovementChange")
            })?,
            improvements: Properties::load(rules, "Improvements", "ImprovementType", |row| {
                integer(row, "MovementChange")
            })?,
            routes: Properties::load(rules, "Routes", "RouteType", |row| {
                match row
                    .get("MovementCost")
                    .map_err(|_| Error::Rules("MovementCost"))?
                {
                    SqlValue::Integer(n) => Ok((*n as f32).to_bits()),
                    SqlValue::RealBits(n) => Ok((f64::from_bits(*n) as f32).to_bits()),
                    _ => Err(Error::Rules("Route MovementCost")),
                }
            })?,
            districts,
            district_hit_points,
            parameters: RuleParameters {
                city_movement_cost: parameters
                    .integer(IntegerParameter::CityMovementCost)
                    .map_err(|_| Error::Rules("CITY_MOVEMENT_COST"))?,
                movement_river_cost: parameters
                    .integer(IntegerParameter::MovementRiverCost)
                    .map_err(|_| Error::Rules("MOVEMENT_RIVER_COST"))?,
                movement_portal_cost: parameters
                    .integer(IntegerParameter::MovementPortalCost)
                    .map_err(|_| Error::Rules("MOVEMENT_PORTAL_COST"))?,
                movement_embark_cost: parameters
                    .integer(IntegerParameter::MovementEmbarkCost)
                    .map_err(|_| Error::Rules("MOVEMENT_EMBARK_COST"))?,
            },
            plot_unit_limit: parameters
                .plot_unit_limit()
                .map_err(|_| Error::Rules("PLOT_UNIT_LIMIT"))?,
            max_hit_points: parameters
                .integer(IntegerParameter::CombatMaxHitPoints)
                .map_err(|_| Error::Rules("COMBAT_MAX_HIT_POINTS"))?,
            combat: formation_rules::Definitions::from_rules(state, rules, resources)
                .map_err(Error::Combat)?,
            improvement_damage: improvement_properties(rules)?,
        })
    }
    pub fn plot_unit_limit(&self) -> u32 {
        self.plot_unit_limit
    }
    fn check(&self, state: &LoadedState) -> Result<(), Error> {
        if state.binding() != &self.binding {
            Err(Error::Binding)
        } else {
            Ok(())
        }
    }
    fn as_core(&self) -> Result<MovementDefinitions<'_>, Error> {
        Ok(MovementDefinitions {
            terrains: self
                .terrains
                .as_core()
                .map_err(|_| Error::Rules("Terrains"))?,
            features: self.features.table()?,
            improvements: self.improvements.table()?,
            routes: self.routes.table()?,
            districts: self.districts.table()?,
        })
    }
}

struct Runtime<'s, 'p, 'a, 'r> {
    state: &'s LoadedState,
    placement: &'p Placement<'s, 'a, 'r>,
    definitions: &'p Definitions<'a>,
    can_embark: bool,
}
impl Runtime<'_, '_, '_, '_> {
    fn district_record(
        &self,
        attrs: &CityAttributes,
    ) -> Result<Option<&c6_core::player_districts::District>, Error> {
        find_district(self.state, attrs)
    }
}
impl RuntimePlotInputs for Runtime<'_, '_, '_, '_> {
    fn district(
        &self,
        attrs: &CityAttributes,
    ) -> Result<Option<DistrictInstance>, movement_world::MissingInput> {
        self.district_record(attrs)
            .map_err(|_| movement_world::MissingInput::DistrictLookup {
                player: attrs.player,
                district: attrs.district,
            })
            .map(|record| {
                record.map(|d| DistrictInstance {
                    type_index: self
                        .definitions
                        .districts
                        .registry
                        .find_index(d.district_hash)
                        .map_or(-1, |i| i as i32),
                    complete: d.is_complete,
                })
            })
    }
    fn domain_valid(
        &self,
        index: usize,
        domain: i32,
    ) -> Result<bool, movement_world::MissingInput> {
        // Both shore phases return before querying a domain when CanEmbark is
        // false. This unused fact must not demand an unsaved city-domain cache.
        if !self.can_embark {
            return Ok(false);
        }
        self.placement
            .city_domain_valid(index, domain)
            .map_err(|_| movement_world::MissingInput::CityDomainValidity {
                plot: index,
                domain,
            })
    }
}

fn formation_facts(
    state: &LoadedState,
    formation: &Formation,
    terrain: i32,
    feature: i32,
) -> Result<FormationFacts, Error> {
    // Each native aggregate starts a fresh ordered member traversal and has its
    // own escort early return. No aggregate derives another aggregate's answer.
    let aggregate = |kind: u8, index: i32| -> Result<bool, Error> {
        let player = state
            .world()
            .player(formation.owner)
            .map_err(Error::World)?;
        let mut all = true;
        for &id in &formation.unit_ids {
            let Some(found) = player.unit(id).map_err(Error::World)? else {
                continue;
            };
            let unit = found.record;
            let ignored = match kind {
                0 => unit.k_movement_modifiers.b_ignore_rivers,
                1 => unit.k_movement_modifiers.b_ignore_shores,
                _ => {
                    let runtime = state
                        .unit_runtime(unit.e_owner, unit.id)
                        .map_err(Error::State)?;
                    let (bounds, values) = if kind == 2 {
                        (&runtime.invalid_terrain, &runtime.ignore_terrain)
                    } else {
                        (&runtime.ignore_feature, &runtime.ignore_feature)
                    };
                    if index == -1 || index >= bounds.len() as i32 {
                        false
                    } else {
                        *values
                            .get(index as usize)
                            .ok_or(Error::Missing("Unit movement array"))?
                    }
                }
            };
            if ignored && unit.k_movement_modifiers.b_escort_mobility {
                return Ok(true);
            }
            all &= ignored;
        }
        Ok(all)
    };
    Ok(FormationFacts {
        ignore_rivers: aggregate(0, 0)?,
        ignore_shores: aggregate(1, 0)?,
        ignore_terrain: aggregate(2, terrain)?,
        ignore_feature: aggregate(3, feature)?,
        can_embark: formation.can_embark,
    })
}

/// Native Values-aware MovementCost (0x2949170), including ZOC hard minimum,
/// all nine shared movement phases and separate destination exit addition.
#[allow(clippy::too_many_arguments)]
pub fn movement_cost(
    state: &LoadedState,
    definitions: &Definitions<'_>,
    placement: &Placement<'_, '_, '_>,
    formation: &Formation,
    from: PlotCoord,
    to: PlotCoord,
    exception: Option<PlotCoord>,
    budget: MoveBudget,
    zoc: ZocValue,
    flags: u32,
    visibility: Option<&VisibilityRuntime>,
) -> Result<CostAccumulator, Error> {
    definitions.check(state)?;
    let slowed = if flags & (1 << 11) == 0 && zoc == ZocValue::Query {
        slowed_by_zoc(
            state,
            definitions,
            placement,
            formation,
            to,
            exception,
            visibility,
        )?
    } else {
        false
    };
    let mut result = CostAccumulator::default();
    movement::apply_zoc_minimum(&mut result, budget, zoc, flags, slowed);
    let map = state.world().map();
    let (source_index, source) = movement_world::plot_at(map, from).map_err(Error::Movement)?;
    let (target_index, target) = movement_world::plot_at(map, to).map_err(Error::Movement)?;
    let defs = definitions.as_core()?;
    let runtime = Runtime {
        state,
        placement,
        definitions,
        can_embark: formation.can_embark,
    };
    let from_facts =
        movement_world::resolve_plot(source, source_index, formation.domain, &defs, &runtime)
            .map_err(Error::Movement)?;
    let to_facts =
        movement_world::resolve_plot(target, target_index, formation.domain, &defs, &runtime)
            .map_err(Error::Movement)?;
    let facts = formation_facts(
        state,
        formation,
        defs.terrains
            .plot_index(target.terrain_hash)
            .map_err(Error::Movement)?,
        defs.features
            .plot_index(target.feature_hash)
            .map_err(Error::Movement)?,
    )?;
    let edge = movement_world::resolve_edge(map, from, to, from_facts, to_facts)
        .map_err(Error::Movement)?;
    movement::apply_rules(
        &mut result,
        budget,
        facts,
        from_facts,
        to_facts,
        edge,
        definitions.parameters,
    );
    Ok(result)
}

impl Definitions<'_> {
    pub fn district_at<'s>(
        &self,
        state: &'s LoadedState,
        plot: &Plot,
    ) -> Result<Option<&'s c6_core::player_districts::District>, Error> {
        self.check(state)?;
        match &plot.city_attributes {
            Some(attrs) => find_district(state, attrs),
            None => Ok(None),
        }
    }
    pub fn district_is_city(
        &self,
        district: &c6_core::player_districts::District,
    ) -> Result<bool, Error> {
        Ok(self
            .districts
            .registry
            .find_index(district.district_hash)
            .and_then(|index| self.districts.registry.type_hash(index))
            == Some(c6_core::city_district::CITY_CENTER_HASH))
    }
    /// IsDefensible (0x291f660): city centers short-circuit damage/completion.
    pub fn district_defensible(
        &self,
        district: &c6_core::player_districts::District,
    ) -> Result<bool, Error> {
        let Some(index) = self.districts.registry.find_index(district.district_hash) else {
            return Ok(false);
        };
        if self.districts.registry.type_hash(index)
            == Some(c6_core::city_district::CITY_CENTER_HASH)
        {
            return Ok(true);
        }
        Ok(self
            .district_hit_points
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(0)
            > 0
            && district.is_complete
            && !district.pillaged)
    }
    pub fn unit_visible(
        &self,
        state: &LoadedState,
        unit: &Unit,
        viewer: i32,
        visibility: &VisibilityRuntime,
    ) -> Result<bool, Error> {
        self.combat.unit_visible_with_counts(
            state,
            unit,
            viewer,
            |hash, normal| {
                let layers = visibility.shared(viewer).map_err(Error::Missing)?;
                let Some(layer) = layers.0.iter().find(|l| l.type_hash == hash) else {
                    return if normal {
                        Err(Error::Missing("shared normal visibility layer"))
                    } else {
                        Ok(0)
                    };
                };
                let index = layer
                    .grid_width
                    .wrapping_mul(unit.k_location.y as u32)
                    .wrapping_add(unit.k_location.x as u32) as usize;
                Ok(layer.values.get(index).copied().unwrap_or(0))
            },
            || {
                visibility
                    .hidden_layer_hashes
                    .first()
                    .copied()
                    .ok_or(Error::Missing("hidden visibility layer0"))
            },
        )
    }
    fn maximum_damage(&self, unit: &Unit) -> Result<i32, Error> {
        let mut entries = unit
            .k_properties
            .entries
            .iter()
            .filter(|e| e.key == 0x7d3fc711);
        let Some(entry) = entries.next() else {
            return Ok(self.max_hit_points);
        };
        if entries.next().is_some() {
            return Err(Error::Missing("duplicate MaxDamage property"));
        }
        let v = &entry.value;
        if v.type_bits != 2
            || v.modifier_bits != 0
            || v.value.count_flags != 0
            || v.value.element_size != 0
        {
            return Err(Error::Missing("MaxDamage typed conversion"));
        }
        match v.value.payload {
            c6_core::unit::variants::VariantPayload::Inline32(n) => Ok(n as i32),
            _ => Err(Error::Missing("MaxDamage typed payload")),
        }
    }
}
fn find_district<'s>(
    state: &'s LoadedState,
    attrs: &CityAttributes,
) -> Result<Option<&'s c6_core::player_districts::District>, Error> {
    if attrs.player == -1 || attrs.district == u32::MAX {
        return Ok(None);
    }
    if state
        .world()
        .valid_player(i32::from(attrs.player))
        .map_err(Error::World)?
        .is_none()
    {
        return Ok(None);
    }
    let owner = state
        .retained()
        .players
        .iter()
        .find(|p| p.requested_owner == i32::from(attrs.player))
        .ok_or(Error::Missing("District owner"))?;
    Ok(owner
        .districts
        .records
        .entries
        .iter()
        .find(|s| s.index == (attrs.district & 0xffff) as i32)
        .map(|s| &s.value))
}
fn plot_owner(plot: &Plot) -> Result<i32, Error> {
    crate::movement_placement_state::plot_owner(plot).map_err(Error::Placement)
}

fn at_war(state: &LoadedState, owner: i32, other: i32) -> Result<bool, Error> {
    let Some(target) = state.world().valid_player(other).map_err(Error::World)? else {
        return Ok(false);
    };
    if !target.is_alive.ok_or(Error::Missing("Player alive"))? {
        return Ok(false);
    }
    state
        .game_relations()
        .map_err(Error::State)?
        .is_at_war(owner, other)
        .map_err(Error::Relations)
}

/// Cache fields are the actual PathInitialize projection, not a spatial ZOC
/// answer. The direct wrapper below obtains these values from current state.
#[derive(Clone, Copy, Debug)]
pub struct ZocIdentity {
    pub owner: i32,
    pub domain: i32,
    pub religion: u32,
    pub layer: u32,
    pub ignores_zoc: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn slowed_by_zoc(
    state: &LoadedState,
    definitions: &Definitions<'_>,
    placement: &Placement<'_, '_, '_>,
    formation: &Formation,
    to: PlotCoord,
    exception: Option<PlotCoord>,
    visibility: Option<&VisibilityRuntime>,
) -> Result<bool, Error> {
    definitions.check(state)?;
    // Formation::IgnoresZOC is its owned stored flag; no member aggregation here.
    if formation.ignores_zoc {
        return Ok(false);
    }
    let owner = state
        .world()
        .player(formation.owner)
        .map_err(Error::World)?;
    let mut religion = u32::MAX;
    for &id in &formation.unit_ids {
        if let Some(found) = owner.unit(id).map_err(Error::World)? {
            religion = found.record.k_religion.e_religion_type;
            break;
        }
    }
    let layer = c6_core::skip_turn::placement::formation_layer(state.world(), formation)
        .map_err(|_| Error::Missing("Formation layer"))?
        .unwrap_or(u32::MAX);
    slowed_by_zoc_cached(
        state,
        definitions,
        placement,
        formation,
        to,
        exception,
        ZocIdentity {
            owner: formation.owner,
            domain: formation.domain,
            religion,
            layer,
            ignores_zoc: false,
        },
        visibility,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn slowed_by_zoc_cached(
    state: &LoadedState,
    definitions: &Definitions<'_>,
    placement: &Placement<'_, '_, '_>,
    formation: &Formation,
    to: PlotCoord,
    exception: Option<PlotCoord>,
    identity: ZocIdentity,
    visibility: Option<&VisibilityRuntime>,
) -> Result<bool, Error> {
    definitions.check(state)?;
    if identity.ignores_zoc {
        return Ok(false);
    }
    let full = placement
        .formation_count(formation, to, identity.layer)
        .map_err(Error::Placement)?
        >= definitions.plot_unit_limit;
    is_in_enemy_zoc(
        state,
        definitions,
        to,
        exception,
        identity,
        full,
        visibility,
    )
}

/// Rules::Movement::IsInEnemyZOC (0x2becf40). The exception destination can
/// cancel slowing only when that tile is not full. River, district, religious
/// and visible Unit branches retain native order and early returns.
#[allow(clippy::too_many_arguments)]
pub fn is_in_enemy_zoc(
    state: &LoadedState,
    definitions: &Definitions<'_>,
    to: PlotCoord,
    exception: Option<PlotCoord>,
    identity: ZocIdentity,
    full: bool,
    visibility: Option<&VisibilityRuntime>,
) -> Result<bool, Error> {
    definitions.check(state)?;
    // Native obtains the viewer visibility and Player/Diplomacy before looping.
    state.world().player(identity.owner).map_err(Error::World)?;
    let map = state.world().map();
    let around = c6_core::skip_turn::placement::query_plots(map, to)
        .map_err(|_| Error::Missing("ZOC neighborhood"))?;
    // The radius-one improvement pass includes the center. Properties remain
    // resolved only after owner, pillage and war checks.
    for location in improvement_neighborhood(map, to)?.into_iter().flatten() {
        let (_, plot) = movement_world::plot_at(map, location).map_err(Error::Movement)?;
        let table = definitions.improvements.table()?;
        let index = table
            .plot_index(plot.improvement_hash)
            .map_err(Error::Movement)?;
        if index == -1 {
            continue;
        }
        let owner = i32::from(plot.improvement_owner);
        if owner == -1
            || plot.flags.get(PlotFlag::ImprovementPillaged)
            || !at_war(state, identity.owner, owner)?
        {
            continue;
        }
        if table.at(index).is_none() {
            continue;
        }
        let index = index as usize;
        let property = if location == to { 0 } else { 1 };
        match definitions.improvement_damage[index][property] {
            PropertyValue::Absent => {}
            PropertyValue::Integer(n) => {
                if n > 0 {
                    return Ok(true);
                }
            }
        }
    }
    let mut slowed = false;
    for (direction, location) in around[1..].iter().enumerate() {
        let Some(location) = *location else { continue };
        let (_, plot) = movement_world::plot_at(map, location).map_err(Error::Movement)?;
        if let Some(district) = definitions.district_at(state, plot)? {
            if definitions.district_defensible(district)?
                && at_war(state, identity.owner, plot_owner(plot)?)?
            {
                slowed = true;
                if Some(location) == exception && !full {
                    return Ok(false);
                }
                continue;
            }
        }
        let opposite = (direction + 3) % 6;
        if plot.river_crossing_mask & (1 << opposite) != 0 {
            let city = definitions.district_at(state, plot)?.is_some_and(|d| {
                definitions
                    .districts
                    .registry
                    .find_index(d.district_hash)
                    .and_then(|i| definitions.districts.registry.type_hash(i))
                    == Some(c6_core::city_district::CITY_CENTER_HASH)
            });
            if !city {
                continue;
            }
        }
        for &reference in
            c6_core::skip_turn::placement::units_at(state.world(), Some(identity.layer), location)
                .map_err(|_| Error::Missing("ZOC occupancy layer"))?
        {
            if reference.owner == identity.owner {
                continue;
            }
            if identity.layer != 3 && !at_war(state, identity.owner, reference.owner)? {
                continue;
            }
            let Some(found) = state.world().unit(reference).map_err(Error::World)? else {
                continue;
            };
            let unit = found.record;
            if unit.i_damage >= definitions.maximum_damage(unit)? || unit.b_delayed_death {
                continue;
            }
            if state
                .unit_runtime(unit.e_owner, unit.id)
                .map_err(Error::State)?
                .domain
                != identity.domain
            {
                continue;
            }
            if identity.layer == 3 {
                if unit.k_religion.e_religion_type == identity.religion {
                    continue;
                }
            } else if crate::unit_movement_state::unit_is_embarked(unit) || !unit.b_exert_zoc {
                continue;
            }
            let visible = definitions.unit_visible(
                state,
                unit,
                identity.owner,
                visibility.ok_or(Error::Missing("current shared visibility"))?,
            )?;
            if visible {
                slowed = true;
                if Some(location) == exception && !full {
                    return Ok(false);
                }
            }
        }
    }
    Ok(slowed)
}

fn improvement_properties(rules: &Rules) -> Result<Vec<[PropertyValue; 2]>, Error> {
    let registry = collection(rules, "Improvements", "ImprovementType")
        .map_err(|_| Error::Rules("Improvements"))?;
    let table = rules
        .gameplay()
        .table("TypeProperties")
        .ok_or(Error::Rules("TypeProperties"))?;
    Ok(registry
        .entries()
        .iter()
        .map(|entry| {
            let mut result = [PropertyValue::Absent; 2];
            for row in table.rows() {
                if row.text("Type").ok().map(str::as_bytes) != Some(entry.type_name.as_slice()) {
                    continue;
                }
                let property = match row.text("Name").ok() {
                    Some("PLOT_DAMAGE_TO_WALKING_INTO") => 0,
                    Some("PLOT_DAMAGE_TO_WALKING_ADJACENT") => 1,
                    _ => continue,
                };
                result[property] = match row.text("Value").ok() {
                    None => PropertyValue::Absent,
                    Some("true") => PropertyValue::Integer(1),
                    Some("false") => PropertyValue::Integer(0),
                    Some(text) => PropertyValue::Integer(native_property_integer(text)),
                };
            }
            result
        })
        .collect())
}

fn improvement_neighborhood(
    map: &c6_core::map::Map,
    to: PlotCoord,
) -> Result<[Option<PlotCoord>; 7], Error> {
    movement_world::plot_at(map, to).map_err(Error::Movement)?;
    let half = |y: i32| {
        if y >= 0 {
            y >> 1
        } else {
            let n = y.wrapping_sub(1);
            n.wrapping_add(((n as u32) >> 31) as i32) >> 1
        }
    };
    let hex_x = to.x.wrapping_sub(half(to.y));
    let mut result = [None; 7];
    result[0] = Some(to);
    for (i, (dx, dy)) in [(0i32, 1i32), (-1, 0), (1, -1), (0, -1), (1, 0), (-1, 1)]
        .into_iter()
        .enumerate()
    {
        let y = to.y.wrapping_add(dy);
        let x = hex_x.wrapping_add(dx).wrapping_add(half(y));
        let x = movement_world::coord_range(x, map.header.width, map.header.wrap_x);
        let y = movement_world::coord_range(y, map.header.height, map.header.wrap_y);
        if x >= 0 && y >= 0 && x < map.header.width && y < map.header.height {
            result[i + 1] = Some(PlotCoord { x, y })
        }
    }
    Ok(result)
}

// IProperties::_ResolveProperties numeric recognition and Linux strtol(base10),
// followed by TypedVariant::operator int. Float/string variants convert to zero.
fn native_property_integer(text: &str) -> i32 {
    let mut decimal = false;
    for (i, byte) in text.bytes().enumerate() {
        match byte {
            b'0'..=b'9' => {}
            b'+' | b'-' if i == 0 => {}
            b'.' if !decimal => decimal = true,
            _ => return 0,
        }
    }
    if decimal {
        return 0;
    }
    let negative = text.starts_with('-');
    let digits = if text.starts_with(['+', '-']) {
        &text[1..]
    } else {
        text
    };
    let limit = if negative {
        1u64 << 63
    } else {
        i64::MAX as u64
    };
    let mut value = 0u64;
    for byte in digits.bytes() {
        value = value
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'))
            .min(limit)
    }
    if negative {
        (value as i64).wrapping_neg() as i32
    } else {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn property_integer_follows_native_recognition_strtol_and_variant_conversion() {
        for (source, wanted) in [
            ("", 0),
            ("+", 0),
            ("-", 0),
            (".", 0),
            ("+.", 0),
            ("17", 17),
            ("-17", -17),
            ("4294967297", 1),
            ("-4294967297", -1),
            ("9223372036854775807", -1),
            ("9223372036854775808", -1),
            ("-9223372036854775808", 0),
            ("-9223372036854775809", 0),
            ("2147483648", i32::MIN),
            ("1.0", 0),
            ("1e2", 0),
            (" 7", 0),
            ("7 ", 0),
            ("--1", 0),
            ("a", 0),
        ] {
            assert_eq!(native_property_integer(source), wanted, "{source:?}");
        }
    }
    fn empty<T>() -> c6_core::map::Recycled<T> {
        c6_core::map::Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: 0,
            free_tail: 0,
            free_links: vec![],
        }
    }
    fn map() -> c6_core::map::Map {
        use c6_core::map::*;
        Map {
            header: Header {
                width: 5,
                height: 5,
                top_latitude: 90,
                bottom_latitude: -90,
                wrap_x: true,
                wrap_y: false,
                map_size_hash: 0,
                land_plots: 25,
                owned_plots: 0,
                natural_wonders: 0,
                resource_counts: vec![],
                land_resource_counts: vec![],
            },
            visibility: vec![],
            areas: empty(),
            landmasses: empty(),
            regions: empty(),
            chokepoints: empty(),
            continents: vec![],
            feature_layouts: empty(),
            plots: vec![Plot::decode(&[0; 55], 6, Limits::default()).unwrap(); 25],
            units: UnitContainer {
                width: 5,
                height: 5,
                layers: vec![],
            },
            components: vec![],
        }
    }
    #[test]
    fn hostile_improvement_pass_retains_native_order_distinct_from_unit_directions() {
        let map = map();
        let location = PlotCoord { x: 2, y: 2 };
        let actual = improvement_neighborhood(&map, location).unwrap();
        assert_eq!(
            actual.map(|p| p.map(|p| (p.x, p.y))),
            [
                Some((2, 2)),
                Some((2, 3)),
                Some((1, 2)),
                Some((2, 1)),
                Some((1, 1)),
                Some((3, 2)),
                Some((1, 3))
            ]
        );
        let directions = c6_core::skip_turn::placement::query_plots(&map, location).unwrap();
        assert_ne!(actual, directions);
        assert_eq!(
            improvement_neighborhood(&map, PlotCoord { x: 0, y: 0 }).unwrap()[2],
            Some(PlotCoord { x: 4, y: 0 })
        );
    }
}
