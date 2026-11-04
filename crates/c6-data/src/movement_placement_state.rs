//! Borrowed placement queries against actual LoadedState and bound definitions.
//! This does not admit actions/effects or replace unavailable runtime capabilities
//! with false. Plot-cell candidate order is preserved; World is never cloned.
use crate::{
    city_movement::NativeTerrainDefinitions,
    resource_binding::{ResourceAuthority, ResourceIdentity},
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::{Plot, PlotFlag},
    movement_placement::{
        self as logic, DefenderDefinition, DefenderError, PlacementDefinition, PlacementMissing,
        ShoreDefender, ShorePlacementPlot, ShoreUnitFacts, TerrainFacts,
    },
    movement_shore::PlacementRequest,
    movement_world::{self, DefinitionLookup, DefinitionTable, TerrainProperties},
    skip_turn::placement,
    unit::{PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    World(c6_core::world::MissingInput),
    Rules(unit_initialization_definitions::Error),
    Binding,
    Definitions,
    Geometry,
    NotAdjacent,
    MissingUnit,
    MissingFormation,
    Terrain(logic::MissingFact),
    Placement(PlacementMissing),
    Capability(Capability),
    BlockedEntryState,
    CityDomains(crate::city_movement::Error),
    CityDomainRebuild(c6_core::city_district::Error),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    ReservedAttributeOwner,
    ReservedAttributeDistrict,
    CityDomain,
    PlayerCivilizationLevel(i32),
    GlobalTechnologyAvailability(i32),
    GlobalCivicAvailability(i32),
    VisibilityContext,
    ExtendedUnitDefinition,
    CombatStrength,
    SparseCoordinate,
    ForeignContext(PlacementMissing),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "placement state: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Reconstruct only the requested scalar from an actual bound collection.
/// The admitted owner Reset initializes false before ordered hash overlay;
/// omitted-wire support is specific to the diplomacy owner. This does
/// not interpret an empty wire list as an empty runtime definition collection.
fn reset_bool_value(
    lookup: &dyn DefinitionLookup,
    unit_index: i32,
    wire_present: bool,
    pairs: &[c6_core::unit::HashValue<bool>],
) -> Result<bool, Error> {
    let index = usize::try_from(unit_index).map_err(|_| Error::BlockedEntryState)?;
    if index >= lookup.count() || lookup.count() > i32::MAX as usize {
        return Err(Error::BlockedEntryState);
    }
    if !wire_present && !pairs.is_empty() {
        return Err(Error::BlockedEntryState);
    }
    let mut value = false;
    for pair in pairs {
        if let Some(found) = lookup.find_index(pair.definition_hash) {
            if found >= lookup.count() {
                return Err(Error::BlockedEntryState);
            }
            if found == index {
                value = pair.value;
            }
        }
    }
    Ok(value)
}

/// Borrowed current configuration state, maintained by its owning simulator.
/// Values are type IDs, not CanEmbark answers. A source-only Configuration
/// decoder is not implicitly promoted to this current runtime projection.
/// Sparse entries remain unknown; binding checks consistency, not authenticity.
pub struct CurrentCivilizationLevels<'a> {
    pub binding: &'a WireBinding,
    pub values: &'a [(i32, u32)],
}

fn validate_current_levels(
    expected: &WireBinding,
    levels: &CurrentCivilizationLevels<'_>,
) -> Result<(), Error> {
    if levels.binding != expected {
        return Err(Error::Binding);
    }
    for (index, &(owner, _)) in levels.values.iter().enumerate() {
        if !(0..65).contains(&owner) || levels.values[..index].iter().any(|p| p.0 == owner) {
            return Err(Error::Definitions);
        }
    }
    Ok(())
}
fn open_borders_from(
    count: i32,
    allied: impl FnOnce() -> Result<bool, Error>,
    permanently_allied: impl FnOnce() -> Result<bool, Error>,
) -> Result<bool, Error> {
    if count != 0 || allied()? {
        return Ok(true);
    }
    permanently_allied()
}

pub(crate) fn unit_embark_decision(
    domain: i32,
    owner: i32,
    original_owner: i32,
    mut ability: impl FnMut(i32) -> Result<bool, Error>,
    mut valid: impl FnMut(i32) -> Result<bool, Error>,
) -> Result<bool, Error> {
    if domain == 0 {
        return Ok(false);
    }
    if ability(owner)? {
        return Ok(true);
    }
    if original_owner == owner || !valid(original_owner)? {
        return Ok(false);
    }
    ability(original_owner)
}
fn player_embark_decision<'a>(
    level: u32,
    unlock: bool,
    pass: impl FnOnce() -> Result<bool, Error>,
    definition: impl FnOnce() -> Option<&'a [u8]>,
    unlocked: impl FnOnce(&[u8]) -> Result<bool, Error>,
) -> Result<bool, Error> {
    if level == 0xd959ec24 {
        return Ok(false);
    }
    if unlock || pass()? {
        return Ok(true);
    }
    match definition() {
        Some(name) => unlocked(name),
        None => Ok(false),
    }
}
fn acquired_with_global(
    len: usize,
    index: usize,
    global: impl FnOnce() -> Result<bool, Error>,
    local: impl FnOnce() -> Result<bool, Error>,
) -> Result<bool, Error> {
    if index >= len || !global()? {
        return Ok(false);
    }
    local()
}
fn unlocks_embark(
    unit: &[u8],
    technologies: &[EmbarkUnlock],
    civics: &[EmbarkUnlock],
    mut acquired: impl FnMut(bool, usize) -> Result<bool, Error>,
) -> Result<bool, Error> {
    for (civic, definitions) in [(false, technologies), (true, civics)] {
        for (index, unlock) in definitions.iter().enumerate() {
            if acquired(civic, index)? && (unlock.all || unlock.unit_type.as_deref() == Some(unit))
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

struct EmbarkUnlock {
    all: bool,
    unit_type: Option<Vec<u8>>,
}
struct EmbarkDefinitions {
    registry: crate::registry::LinuxDefinitionRegistry,
    properties: Vec<EmbarkUnlock>,
}
impl EmbarkDefinitions {
    fn from_rules(rules: &Rules, table: &'static str, key: &'static str) -> Result<Self, Error> {
        let registry = collection(rules, table, key).map_err(Error::Rules)?;
        let rows = rules.gameplay().table(table).ok_or(Error::Definitions)?;
        let properties = registry
            .entries()
            .iter()
            .map(|entry| {
                let row = rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Definitions)?;
                let all = match row.get("EmbarkAll").map_err(|_| Error::Definitions)? {
                    SqlValue::Integer(0) => false,
                    SqlValue::Integer(1) => true,
                    _ => return Err(Error::Definitions),
                };
                let unit_type = match row.get("EmbarkUnitType").map_err(|_| Error::Definitions)? {
                    SqlValue::Null => None,
                    SqlValue::Text(value) if !value.contains(&0) => Some(value.clone()),
                    _ => return Err(Error::Definitions),
                };
                Ok(EmbarkUnlock { all, unit_type })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            registry,
            properties,
        })
    }
}
fn unlock_embarkation(rules: &Rules) -> Result<bool, Error> {
    let table = rules
        .gameplay()
        .table("GlobalParameters")
        .ok_or(Error::Definitions)?;
    let mut rows = table
        .rows()
        .filter(|row| row.text("Name").ok() == Some("UNLOCK_EMBARKATION"));
    let Some(row) = rows.next() else {
        return Ok(false);
    }; // native Get default
    if rows.next().is_some() {
        return Err(Error::Definitions);
    }
    let value = match row.get("Value").map_err(|_| Error::Definitions)? {
        SqlValue::Integer(value) => i32::try_from(*value).map_err(|_| Error::Definitions)?,
        SqlValue::Text(value) => std::str::from_utf8(value)
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .ok_or(Error::Definitions)?,
        _ => return Err(Error::Definitions),
    };
    Ok(value != 0)
}

/// Source GetBestDefender prefix over the subject's actual map layer. This
/// reads current owned state and keeps stored candidate order. Friendly,
/// delayed-death and missing candidates need no visibility/strength query;
/// reached foreign candidates retain explicit unsupported capabilities.
/// No gameplay state or script/RNG callback is changed by this bounded profile.
pub fn best_defender<'s>(
    state: &'s LoadedState,
    unit: &Unit,
    destination: PlotCoord,
) -> Result<Option<&'s Unit>, Error> {
    best_defender_with_visibility(state, unit, destination, None, |_| {
        Ok(DefenderDefinition::Unresolved)
    })
}

// The selected flags0x108 call uses only the presence of this context, not
// normal/hidden visibility arrays. A missing/contradictory row is unknown.
pub(crate) fn visibility_context_present(
    rows: &[c6_core::map::PlayerVisibility],
    mapping: Option<&[i32; 65]>,
    owner: i32,
) -> Result<bool, Error> {
    if owner == -1 {
        return Ok(false); // GetBestDefender explicitly skips Globals lookup.
    }
    if !(0..64).contains(&owner) {
        return Err(Error::Capability(Capability::VisibilityContext));
    }
    let index = mapping
        .and_then(|m| usize::try_from(m[owner as usize]).ok())
        .ok_or(Error::Capability(Capability::VisibilityContext))?;
    let row = rows
        .get(index)
        .filter(|row| row.player == owner && row.visibility.player_id == owner)
        .ok_or(Error::Capability(Capability::VisibilityContext))?;
    let _ = row; // Source Write dereferences this actual vector occurrence.
    Ok(true)
}

fn best_defender_with_visibility<'s>(
    state: &'s LoadedState,
    unit: &Unit,
    destination: PlotCoord,
    visibility_present: Option<bool>,
    definition: impl Fn(&Unit) -> Result<DefenderDefinition, Error>,
) -> Result<Option<&'s Unit>, Error> {
    let map = state.world().map();
    let raw_index = destination
        .y
        .wrapping_mul(map.units.width as i32)
        .wrapping_add(destination.x) as u32 as usize;
    let layer = placement::unit_layer(state.world(), unit).map_err(|_| Error::Geometry)?;
    let references = match layer {
        None => &[][..], // Original UnitContainer rejects layer UINT_MAX.
        Some(layer) => {
            if raw_index >= map.plots.len() {
                return Err(Error::Capability(Capability::SparseCoordinate));
            }
            state
                .world()
                .plot_units(layer, raw_index)
                .map_err(Error::World)?
        }
    };
    let candidates = references.iter().map(|reference| {
        let Some(found) = state.world().unit(*reference).map_err(Error::World)? else {
            return Ok(None);
        };
        let candidate = found.record;
        // Resolve the pure definition getter only on the branch on which the
        // core selector will consume it. Own/delayed/invalid-coordinate guards
        // retain their source short circuits and never demand a definition.
        let definition = if !candidate.b_delayed_death
            && candidate.e_owner != unit.e_owner
            && visibility_present == Some(true)
            && placement::coordinate_valid(candidate.k_location)
        {
            definition(candidate)?
        } else {
            DefenderDefinition::Unresolved
        };
        Ok(Some(ShoreDefender {
            unit: candidate,
            owner: candidate.e_owner,
            delayed_death: candidate.b_delayed_death,
            location: Some(candidate.k_location),
            definition,
            religious_strength: Some(candidate.k_religion.i_religious_strength),
        }))
    });
    logic::select_shore_defender(
        candidates,
        unit.e_owner,
        visibility_present,
        Some(unit.k_religion.i_religious_strength),
        |_| Err(Error::Capability(Capability::CombatStrength)),
    )
    .map_err(|error| match error {
        DefenderError::Candidate(e) | DefenderError::CombatStrength(e) => e,
        DefenderError::VisibilityContext => Error::Capability(Capability::VisibilityContext),
        _ => Error::Capability(Capability::ExtendedUnitDefinition),
    })
}

/// Construct once from the same registered rules/resources and actual exported
/// terrain definitions. Fields are borrowed immutable definitions; each query
/// reads the current state rather than a per-action oracle result or clone.
pub struct Definitions<'a, 'r> {
    units: DefinitionTable<'a, Option<placement::UnitProperties>>,
    terrains: DefinitionTable<'a, TerrainProperties>,
    districts: crate::registry::LinuxDefinitionRegistry,
    unit_names: crate::registry::LinuxDefinitionRegistry,
    unit_team_visibility: Vec<bool>,
    technologies: EmbarkDefinitions,
    civics: EmbarkDefinitions,
    unlock_embarkation: bool,
    _resources: &'a ResourceAuthority<'r>,
}
impl<'a, 'r> Definitions<'a, 'r> {
    pub fn new(
        state: &LoadedState,
        rules: &Rules,
        resources: &'a ResourceAuthority<'r>,
        terrains: &'a NativeTerrainDefinitions,
        terrain_resources: &ResourceIdentity,
    ) -> Result<Self, Error> {
        if terrain_resources != resources.resource_identity() {
            return Err(Error::Binding);
        }
        if state.binding() != resources.destination_binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, resources)
            .map_err(Error::Rules)?;
        let table = terrains.as_core().map_err(|_| Error::Definitions)?;
        let native = collection(rules, "Terrains", "TerrainType").map_err(Error::Rules)?;
        if table.lookup.count() != native.count()
            || (0..native.count()).any(|i| table.lookup.type_hash(i) != native.type_hash(i))
        {
            return Err(Error::Definitions);
        }
        let unit_names = collection(rules, "Units", "UnitType").map_err(Error::Rules)?;
        let units = resources
            .placement_definitions()
            .map_err(|_| Error::Definitions)?;
        if unit_names.count() != units.lookup.count()
            || (0..unit_names.count()).any(|i| unit_names.type_hash(i) != units.lookup.type_hash(i))
        {
            return Err(Error::Definitions);
        }
        let unit_rows = rules.gameplay().table("Units").ok_or(Error::Definitions)?;
        let unit_team_visibility = unit_names
            .entries()
            .iter()
            .map(|entry| {
                match unit_rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Definitions)?
                    .integer("TeamVisibility")
                {
                    Ok(0) => Ok(false),
                    Ok(1) => Ok(true),
                    _ => Err(Error::Definitions),
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            unit_names,
            unit_team_visibility,
            technologies: EmbarkDefinitions::from_rules(rules, "Technologies", "TechnologyType")?,
            civics: EmbarkDefinitions::from_rules(rules, "Civics", "CivicType")?,
            unlock_embarkation: unlock_embarkation(rules)?,
            units,
            terrains: table,
            districts: collection(rules, "Districts", "DistrictType").map_err(Error::Rules)?,
            _resources: resources,
        })
    }
}

impl<'a, 'r> Definitions<'a, 'r> {
    pub fn unit_definitions(&self) -> &DefinitionTable<'a, Option<placement::UnitProperties>> {
        &self.units
    }
    pub fn terrain_definitions(&self) -> &DefinitionTable<'a, TerrainProperties> {
        &self.terrains
    }
}

enum BoundDefinitions<'a, 'r> {
    Owned(Box<Definitions<'a, 'r>>),
    Borrowed(&'a Definitions<'a, 'r>),
}

/// A cheap current-state view over definitions admitted once for their binding.
pub struct Context<'s, 'a, 'r> {
    state: &'s LoadedState,
    definitions: BoundDefinitions<'a, 'r>,
    civilization_levels: Option<&'a [(i32, u32)]>,
    city_domains: Option<std::borrow::Cow<'a, [Option<c6_core::city_district::DomainMask>]>>,
    civic_activity: Option<&'a crate::loaded_actions::culture_notifications::GameCulture>,
}
impl<'a, 'r> std::ops::Deref for Context<'_, 'a, 'r> {
    type Target = Definitions<'a, 'r>;
    fn deref(&self) -> &Self::Target {
        match &self.definitions {
            BoundDefinitions::Owned(value) => value,
            BoundDefinitions::Borrowed(value) => value,
        }
    }
}
impl<'s, 'a, 'r> Context<'s, 'a, 'r> {
    pub fn new(
        state: &'s LoadedState,
        rules: &Rules,
        resources: &'a ResourceAuthority<'r>,
        terrains: &'a NativeTerrainDefinitions,
        terrain_resources: &ResourceIdentity,
    ) -> Result<Self, Error> {
        Ok(Self {
            state,
            definitions: BoundDefinitions::Owned(Box::new(Definitions::new(
                state,
                rules,
                resources,
                terrains,
                terrain_resources,
            )?)),
            civilization_levels: None,
            city_domains: None,
            civic_activity: None,
        })
    }
    /// Reborrow the live state without rebuilding native registries or rules.
    pub fn from_definitions(
        state: &'s LoadedState,
        definitions: &'a Definitions<'a, 'r>,
    ) -> Result<Self, Error> {
        if state.binding() != definitions._resources.destination_binding() {
            return Err(Error::Binding);
        }
        Ok(Self {
            state,
            definitions: BoundDefinitions::Borrowed(definitions),
            civilization_levels: None,
            city_domains: None,
            civic_activity: None,
        })
    }
    /// Borrow masks rebuilt once from this exact current state. Identity and
    /// action stamp must match the rebuild's dependency epoch.
    pub fn with_city_domain_masks(
        mut self,
        binding: &WireBinding,
        stamp: c6_core::loaded_state::ActionStamp,
        masks: &'a [Option<c6_core::city_district::DomainMask>],
    ) -> Result<Self, Error> {
        if binding != self.state.binding()
            || stamp != self.state.action_stamp()
            || masks.len() != self.state.world().map().plots.len()
        {
            return Err(Error::Binding);
        }
        self.city_domains = Some(std::borrow::Cow::Borrowed(masks));
        Ok(self)
    }
    pub fn city_domain_masks(&self) -> Option<&[Option<c6_core::city_district::DomainMask>]> {
        self.city_domains.as_deref()
    }
    /// Bind a borrowed current configuration projection once. Its owner must
    /// maintain it when configuration changes; this accepts no per-query oracle.
    pub fn with_civilization_levels(
        mut self,
        levels: CurrentCivilizationLevels<'a>,
    ) -> Result<Self, Error> {
        validate_current_levels(self.state.binding(), &levels)?;
        self.civilization_levels = Some(levels.values);
        Ok(self)
    }
    /// Rebuild the native CityAttributes domain masks from current owned
    /// City/District/Building state and admitted ordered static definitions.
    /// This uses both original passes, including coastal centers, canals,
    /// adjacent-city propagation and Canal/Bridge wonders. No mask answers or
    /// native pointers are inputs. The borrowed World cannot change underneath
    /// this context; constructing another current context recomputes its masks.
    pub fn with_rebuilt_city_domains(
        mut self,
        domains: &crate::city_movement::NativeDomainDefinitions,
        domain_resources: &ResourceIdentity,
        max_records: usize,
        max_plots: usize,
    ) -> Result<Self, Error> {
        use crate::city_movement::{
            CityBuildings, MapDomainInputs, TypedDomainInputs, WorldInstances,
        };
        if domain_resources != self._resources.resource_identity() {
            return Err(Error::Binding);
        }
        if domains.district_lookup().count() != self.districts.count()
            || (0..self.districts.count()).any(|index| {
                domains.district_lookup().type_hash(index) != self.districts.type_hash(index)
            })
        {
            return Err(Error::Definitions);
        }
        let instances = WorldInstances::from_current(self.state, domains, max_records)
            .map_err(Error::CityDomains)?;
        let registry = instances.registry().map_err(Error::CityDomains)?;
        let cities = CityBuildings::from_references(
            self.state
                .retained()
                .players
                .iter()
                .flat_map(|p| p.cities.records.entries.iter().map(|s| &s.value)),
            max_records,
        )
        .map_err(Error::CityDomains)?;
        let definitions = domains.as_core().map_err(Error::CityDomains)?;
        let inputs = TypedDomainInputs {
            map: MapDomainInputs::new(self.state.world().map(), &self.terrains)
                .map_err(Error::CityDomains)?,
            registry: &registry,
            cities: &cities,
            building_definitions: domains.building_lookup(),
        };
        self.city_domains = Some(std::borrow::Cow::Owned(
            c6_core::city_district::rebuild_valid_domains(
                self.state.world().map(),
                &registry,
                &definitions,
                &inputs,
                max_plots,
            )
            .map_err(Error::CityDomainRebuild)?,
        ));
        Ok(self)
    }

    /// Borrow current global Civic node activity from an existing typed owner.
    /// The caller maintains current values and their source provenance. This
    /// uses only HasCivic, not unrelated graph/cost/prerequisite lifecycle.
    /// Absence leaves the existing owned-world input/missing-input behavior.
    pub fn with_civic_activity(
        mut self,
        culture: &'a crate::loaded_actions::culture_notifications::GameCulture,
    ) -> Result<Self, Error> {
        if culture.binding() != self.state.binding() {
            return Err(Error::Binding);
        }
        self.civic_activity = Some(culture);
        Ok(self)
    }
    fn has_embark_prerequisite(
        &self,
        player: &c6_core::loaded_components::RetainedPlayer,
        index: usize,
        civic: bool,
    ) -> Result<bool, Error> {
        let current = self
            .state
            .world()
            .player(player.header.owner)
            .map_err(Error::World)?;
        let acquired = if civic {
            &current.civics
        } else {
            &current.technologies
        };
        // Native bounds precede global-node activity, which precedes local bit.
        let lookup = if civic {
            &self.civics.registry
        } else {
            &self.technologies.registry
        };
        let len = acquired.as_ref().map_or(lookup.count(), Vec::len);
        acquired_with_global(
            len,
            index,
            || {
                if civic {
                    if let Some(culture) = self.civic_activity {
                        return Ok(culture.has_civic(index as i32));
                    }
                    self.state
                        .world()
                        .civic_availability
                        .as_ref()
                        .and_then(|v| v.get(index))
                        .copied()
                        .ok_or(Error::Capability(Capability::GlobalCivicAvailability(
                            index as i32,
                        )))
                } else if let Ok(graph) = self.state.game_techs() {
                    Ok(graph.has_tech(index as i32))
                } else {
                    self.state
                        .world()
                        .technology_availability
                        .as_ref()
                        .and_then(|v| v.get(index))
                        .copied()
                        .ok_or(Error::Capability(Capability::GlobalTechnologyAvailability(
                            index as i32,
                        )))
                }
            },
            || {
                if let Some(values) = acquired {
                    return Ok(values[index]);
                }
                let pairs = if civic {
                    &player.culture.has_civic.pairs
                } else {
                    &player.techs.has_tech.pairs
                };
                reset_bool_value(lookup, index as i32, true, pairs).map_err(|_| Error::Definitions)
            },
        )
    }
    fn civilization_level(&self, owner: i32) -> Result<u32, Error> {
        self.state.world().player(owner).map_err(Error::World)?;
        self.civilization_levels
            .and_then(|v| v.iter().find(|p| p.0 == owner))
            .map(|p| p.1)
            .ok_or(Error::Capability(Capability::PlayerCivilizationLevel(
                owner,
            )))
    }
    pub fn open_borders(&self, owner: i32, from: i32) -> Result<bool, Error> {
        self.state.world().player(owner).map_err(Error::World)?;
        let diplomacy = &self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Definitions)?
            .diplomacy;
        let count = *diplomacy
            .open_borders_from_count
            .get(usize::try_from(from).map_err(|_| Error::Definitions)?)
            .ok_or(Error::Definitions)?;
        open_borders_from(
            count,
            || {
                self.state
                    .game_relations()
                    .map_err(Error::State)?
                    .is_allied(owner, from)
                    .map_err(Error::Relations)
            },
            || {
                self.state
                    .game_relations()
                    .map_err(Error::State)?
                    .is_permanently_allied(owner, from)
                    .map_err(Error::Relations)
            },
        )
    }
    fn player_can_embark(&self, owner: i32, unit_type: i32) -> Result<bool, Error> {
        let level = self.civilization_level(owner)?;
        player_embark_decision(
            level,
            self.unlock_embarkation,
            || {
                let player = self
                    .state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Definitions)?;
                // Enum Read maps unknown hashes to -1 and retains every element.
                Ok(player
                    .units
                    .unit_embarkation_pass_hashes
                    .iter()
                    .any(|hash| {
                        self.units
                            .lookup
                            .find_index(*hash)
                            .map_or(-1, |index| index as i32)
                            == unit_type
                    }))
            },
            || {
                self.unit_names
                    .at(unit_type as u32)
                    .map(|u| u.type_name.as_slice())
            },
            |name| {
                let player = self
                    .state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Definitions)?;
                unlocks_embark(
                    name,
                    &self.technologies.properties,
                    &self.civics.properties,
                    |civic, index| self.has_embark_prerequisite(player, index, civic),
                )
            },
        )
    }
    pub fn city_domain_valid(&self, plot_index: usize, domain: i32) -> Result<bool, Error> {
        self.city_domains
            .as_ref()
            .and_then(|m| m.get(plot_index))
            .copied()
            .flatten()
            .map(|m| m.is_valid(domain))
            .ok_or(Error::Capability(Capability::CityDomain))
    }
    /// Actual layered candidate order and native per-member maximum. War state
    /// is demanded by the core count only if a resolved candidate reaches it.
    pub fn formation_count(
        &self,
        formation: &c6_core::unit::Formation,
        location: PlotCoord,
        layer: u32,
    ) -> Result<u32, Error> {
        let candidates = placement::units_at(self.state.world(), Some(layer), location)
            .map_err(|_| Error::Definitions)?;
        let owner = self.state.game_relations().ok();
        let relations = owner.and_then(|r| {
            r.war().map(|w| placement::WarRelations {
                player_count: r.player_count(),
                war_types: w.values(),
            })
        });
        placement::Inputs {
            world: self.state.world(),
            definitions: Some(&self.units),
            relations: relations.as_ref(),
            plot_unit_limit: None,
        }
        .formation_count(formation, candidates)
        .map_err(|_| Error::Definitions)
    }
    pub fn can_embark(&self, unit: &Unit) -> Result<bool, Error> {
        let runtime = self
            .state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        unit_embark_decision(
            runtime.domain,
            unit.e_owner,
            unit.e_original_owner,
            |owner| self.player_can_embark(owner, runtime.definition_index),
            |owner| {
                self.state
                    .world()
                    .player_is_valid(owner)
                    .map_err(Error::World)
            },
        )
    }
    fn subject(&self, owner: i32, id: u32) -> Result<&Unit, Error> {
        self.state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .unit(id)
            .map_err(Error::World)?
            .map(|u| u.record)
            .ok_or(Error::MissingUnit)
    }
    fn plot(&self, coord: PlotCoord) -> Result<Option<(usize, &Plot)>, Error> {
        let map = self.state.world().map();
        if map.header.width <= 0 || map.header.height <= 0 {
            return Err(Error::Geometry);
        }
        if coord.x == -9999 || coord.y == -9999 {
            return Ok(None);
        }
        let x = movement_world::coord_range(coord.x, map.header.width, map.header.wrap_x);
        let y = movement_world::coord_range(coord.y, map.header.height, map.header.wrap_y);
        Ok(map
            .plot_at(x, y)
            .map(|p| ((y as usize) * (map.header.width as usize) + x as usize, p)))
    }
    fn definition(&self, unit: &Unit) -> Result<Option<placement::UnitProperties>, Error> {
        let runtime = self
            .state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        Ok(self.units.at(runtime.definition_index).copied().flatten())
    }
    pub fn water(&self, plot: &Plot) -> Result<bool, Error> {
        let index = self
            .terrains
            .plot_index(plot.terrain_hash)
            .map_err(|_| Error::Definitions)?;
        Ok(index == 15 || index == 16 || self.terrains.at(index).is_some_and(|p| p.is_water))
    }
    fn terrain(
        &self,
        unit: &Unit,
        plot: &Plot,
        plot_index: usize,
        request: PlacementRequest,
    ) -> Result<logic::TerrainAssessment, Error> {
        if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
            return Err(Error::Geometry);
        }
        let runtime = self
            .state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let mut f = TerrainFacts::default();
        loop {
            match logic::terrain_prefix(&f, request) {
                Ok(value) => return Ok(value),
                Err(missing) => match missing {
                    logic::MissingFact::Impassable => {
                        f.impassable = Some(plot.flags.get(PlotFlag::Impassable))
                    }
                    logic::MissingFact::InvalidTerrain => {
                        let index = self
                            .terrains
                            .plot_index(plot.terrain_hash)
                            .map_err(|_| Error::Definitions)?;
                        if runtime.invalid_terrain.len() > i32::MAX as usize {
                            return Err(Error::Definitions);
                        }
                        f.invalid_terrain = Some(
                            if index == -1 || index >= runtime.invalid_terrain.len() as i32 {
                                false
                            } else {
                                *runtime
                                    .invalid_terrain
                                    .get(index as usize)
                                    .ok_or(Error::Definitions)?
                            },
                        );
                    }
                    logic::MissingFact::Domain => f.domain = Some(runtime.domain),
                    logic::MissingFact::Water => f.water = Some(self.water(plot)?),
                    logic::MissingFact::HasCityAttributes => {
                        f.has_city_attributes = Some(plot.city_attributes.is_some())
                    }
                    logic::MissingFact::CanEmbark => f.can_embark = Some(self.can_embark(unit)?),
                    logic::MissingFact::CityDomainValid => {
                        f.city_domain_valid = Some(
                            self.city_domains
                                .as_ref()
                                .and_then(|masks| masks.get(plot_index))
                                .copied()
                                .flatten()
                                .ok_or(Error::Capability(Capability::CityDomain))?
                                .is_valid(runtime.domain),
                        );
                    }
                },
            }
        }
    }
    fn district_type(&self, plot: &Plot) -> Result<i32, Error> {
        let attrs = plot
            .city_attributes
            .as_ref()
            .ok_or(Error::Capability(Capability::ReservedAttributeDistrict))?;
        if attrs.player == -1 || attrs.district == u32::MAX {
            return Ok(-1);
        }
        if self
            .state
            .world()
            .valid_player(i32::from(attrs.player))
            .map_err(Error::World)?
            .is_none()
        {
            return Ok(-1);
        }
        let owner = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == i32::from(attrs.player))
            .ok_or(Error::Definitions)?;
        let found = owner
            .districts
            .records
            .entries
            .iter()
            .find(|s| s.index == (attrs.district & 0xffff) as i32);
        Ok(found
            .and_then(|d| self.districts.find_index(d.value.district_hash))
            .map_or(-1, |i| i as i32))
    }
    fn best_defender(&self, unit: &Unit, destination: PlotCoord) -> Result<Option<&Unit>, Error> {
        let present = visibility_context_present(
            &self.state.world().map().visibility,
            self.state.world().player_mapping.as_ref(),
            unit.e_owner,
        )
        .ok();
        best_defender_with_visibility(self.state, unit, destination, present, |candidate| {
            let runtime = self
                .state
                .unit_runtime(candidate.e_owner, candidate.id)
                .map_err(Error::State)?;
            if self
                .units
                .at(runtime.definition_index)
                .copied()
                .flatten()
                .is_none()
            {
                return Ok(DefenderDefinition::Absent);
            }
            let index =
                usize::try_from(runtime.definition_index).map_err(|_| Error::Definitions)?;
            Ok(DefenderDefinition::Present {
                team_visibility: *self
                    .unit_team_visibility
                    .get(index)
                    .ok_or(Error::Definitions)?,
            })
        })
    }
    fn query(
        &self,
        unit: &Unit,
        destination: PlotCoord,
        request: PlacementRequest,
    ) -> Result<i32, Error> {
        let Some((plot_index, plot)) = self.plot(destination)? else {
            return logic::shore_unit_result(ShorePlacementPlot::KnownAbsent)
                .map_err(Error::Placement);
        };
        let mut f = ShoreUnitFacts {
            terrain: Some(self.terrain(unit, plot, plot_index, request)?),
            ..ShoreUnitFacts::default()
        };
        let mut selected: Option<Option<&Unit>> = None;
        loop {
            match logic::shore_unit_result(ShorePlacementPlot::Present(&f)) {
                Ok(value) => return Ok(value),
                Err(missing) => match missing {
                    PlacementMissing::Definition => {
                        f.definition = Some(self.definition(unit)?.map_or(
                            PlacementDefinition::Absent,
                            |d| PlacementDefinition::Present {
                                spy: d.spy,
                                trade_route: d.make_trade_route,
                            },
                        ))
                    }
                    PlacementMissing::DistrictType => {
                        f.district_type = Some(self.district_type(plot)?)
                    }
                    PlacementMissing::UnitOwner => f.unit_owner = Some(unit.e_owner),
                    PlacementMissing::PlotOwner => {
                        f.plot_owner = Some(i32::from(
                            plot.city_attributes
                                .as_ref()
                                .ok_or(Error::Capability(Capability::ReservedAttributeOwner))?
                                .player,
                        ))
                    }
                    PlacementMissing::BestDefender => {
                        let found = self.best_defender(unit, destination)?;
                        f.best_defender_present = Some(found.is_some());
                        selected = Some(found);
                    }
                    PlacementMissing::SubjectEmbarked => {
                        f.subject_embarked =
                            Some(crate::unit_movement_state::unit_is_embarked(unit))
                    }
                    PlacementMissing::DefenderEmbarked => {
                        f.defender_embarked = Some(crate::unit_movement_state::unit_is_embarked(
                            selected.flatten().ok_or(Error::Definitions)?,
                        ))
                    }
                    PlacementMissing::War => {
                        f.at_war = Some(
                            self.state
                                .game_relations()
                                .map_err(Error::State)?
                                .is_at_war(unit.e_owner, f.plot_owner.ok_or(Error::Definitions)?)
                                .map_err(|_| Error::Definitions)?,
                        )
                    }
                    PlacementMissing::BlockedEntry => {
                        let owner = f.plot_owner.ok_or(Error::Definitions)?;
                        // GetPlayer, not GetValidPlayer: no extra validity filter.
                        self.state.world().player(owner).map_err(Error::World)?;
                        let diplomacy = &self
                            .state
                            .retained()
                            .players
                            .iter()
                            .find(|p| p.header.owner == owner)
                            .ok_or(Error::BlockedEntryState)?
                            .diplomacy;
                        let runtime = self
                            .state
                            .unit_runtime(unit.e_owner, unit.id)
                            .map_err(Error::State)?;
                        f.blocked_entry = Some(reset_bool_value(
                            self.units.lookup,
                            runtime.definition_index,
                            diplomacy.blocked_entry_wire_present(),
                            &diplomacy.units_blocked_entry,
                        )?);
                    }
                    PlacementMissing::EnterForeign => {
                        f.enter_foreign = Some(unit.k_movement_modifiers.b_enter_foreign_lands)
                    }
                    PlacementMissing::OpenBorders => {
                        f.open_borders =
                            Some(self.open_borders(
                                unit.e_owner,
                                f.plot_owner.ok_or(Error::Definitions)?,
                            )?)
                    }
                    PlacementMissing::EnforceBorders => {
                        let owner = f.plot_owner.ok_or(Error::Definitions)?;
                        self.state.world().player(owner).map_err(Error::World)?;
                        f.enforce_borders = Some(
                            self.state
                                .retained()
                                .players
                                .iter()
                                .find(|p| p.header.owner == owner)
                                .ok_or(Error::Definitions)?
                                .diplomacy
                                .can_enforce_borders,
                        )
                    }
                    PlacementMissing::Barbarian => {
                        f.barbarian = Some(self.civilization_level(unit.e_owner)? == 0xd959ec24)
                    }
                    PlacementMissing::TribeIndex => {
                        f.tribe_index = Some(unit.i_barbarian_tribe_index)
                    }
                    _ => return Err(Error::Capability(Capability::ForeignContext(missing))),
                },
            }
        }
    }
    /// Query the actual supplied current/staged Unit record against this
    /// borrowed World. Identity is resolved, but its fields are not replaced by
    /// the older World actor during readonly action preparation.
    pub fn unit_record_placement(
        &self,
        current: &LoadedState,
        unit: &Unit,
        destination: PlotCoord,
        request: PlacementRequest,
    ) -> Result<i32, Error> {
        if current.binding() != self.state.binding()
            || current.action_stamp() != self.state.action_stamp()
        {
            return Err(Error::Binding);
        }
        self.subject(unit.e_owner, unit.id)?;
        self.query(unit, destination, request)
    }

    /// Actual Unit placement query; no adjacency requirement is invented here.
    pub fn unit_placement(
        &self,
        owner: i32,
        id: u32,
        destination: PlotCoord,
        request: PlacementRequest,
    ) -> Result<i32, Error> {
        self.query(self.subject(owner, id)?, destination, request)
    }
    /// Convenience entry for a genuine adjacent step. Other destination classes
    /// are outside this API, not silently native-denied by the placement query.
    pub fn adjacent_placement(
        &self,
        owner: i32,
        id: u32,
        destination: PlotCoord,
        request: PlacementRequest,
    ) -> Result<i32, Error> {
        let unit = self.subject(owner, id)?;
        if !movement_world::adjacent(self.state.world().map(), unit.k_location, destination)
            .map_err(|_| Error::Geometry)?
        {
            return Err(Error::NotAdjacent);
        }
        self.query(unit, destination, request)
    }
    pub fn formation_placement(
        &self,
        owner: i32,
        id: u32,
        destination: PlotCoord,
        request: PlacementRequest,
    ) -> Result<i32, Error> {
        let player = self.state.world().player(owner).map_err(Error::World)?;
        let formation = player
            .formation(id)
            .map_err(Error::World)?
            .ok_or(Error::MissingFormation)?;
        let members = player.members(formation.record).map(|entry| {
            entry
                .map(|found| found.map(|found| found.record))
                .map_err(Error::World)
        });
        logic::formation_result(members, |unit| self.query(unit, destination, request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::LinuxDefinitionHashRegistry;
    use c6_core::unit::HashValue;

    #[test]
    fn defender_flag100_context_needs_no_fog_arrays_but_keeps_native_mapping() {
        let rows = [c6_core::map::PlayerVisibility {
            player: 3,
            visibility: c6_core::map::Visibility {
                player_id: 3,
                cached_num_revealed_hexes: 0,
                cached_old: false,
                revealed: vec![],
                layers: vec![],
                outgoing_visibility: vec![],
                outgoing_visibility_blockers: vec![],
            },
        }];
        let mut mapping = [-1; 65];
        mapping[3] = 0;
        assert!(visibility_context_present(&rows, Some(&mapping), 3).unwrap());
        assert!(visibility_context_present(&rows, None, 3).is_err());
        assert!(visibility_context_present(&rows, Some(&mapping), 2).is_err());
        mapping[3] = 1;
        assert!(visibility_context_present(&rows, Some(&mapping), 3).is_err());
        mapping[2] = 0;
        assert!(visibility_context_present(&rows, Some(&mapping), 2).is_err());
        assert!(!visibility_context_present(&[], None, -1).unwrap());
    }

    #[test]
    fn omitted_wire_resets_actual_definition_indexes() {
        let definitions = LinuxDefinitionHashRegistry::from_native_order(vec![100, 200]);
        assert!(!reset_bool_value(&definitions, 1, false, &[]).unwrap());
        assert!(reset_bool_value(&definitions, -1, false, &[]).is_err());
        assert!(reset_bool_value(&definitions, 2, false, &[]).is_err());
        let empty = LinuxDefinitionHashRegistry::from_native_order(vec![]);
        assert!(reset_bool_value(&empty, 0, false, &[]).is_err());
    }

    #[test]
    fn current_overlay_keeps_native_lookup_and_last_duplicate_order() {
        let definitions = LinuxDefinitionHashRegistry::from_native_order(vec![1, 200]);
        let pairs = [
            HashValue {
                definition_hash: 200,
                value: true,
            },
            HashValue {
                definition_hash: 999,
                value: true,
            },
            HashValue {
                definition_hash: 200,
                value: false,
            },
            // Small keys are indexes before hash lookup, even hash 1 at index0.
            HashValue {
                definition_hash: 1,
                value: true,
            },
        ];
        assert!(!reset_bool_value(&definitions, 0, true, &pairs).unwrap());
        assert!(reset_bool_value(&definitions, 1, true, &pairs).unwrap());
        assert!(!reset_bool_value(&definitions, 1, true, &pairs[..3]).unwrap());
        assert!(reset_bool_value(&definitions, 1, true, &pairs[..1]).unwrap());
        assert!(reset_bool_value(&definitions, 1, false, &pairs).is_err());
    }

    #[test]
    fn duplicate_definition_hash_uses_actual_last_inserted_index() {
        let definitions = LinuxDefinitionHashRegistry::from_native_order(vec![200, 200]);
        let pairs = [HashValue {
            definition_hash: 200,
            value: true,
        }];
        assert!(!reset_bool_value(&definitions, 0, true, &pairs).unwrap());
        assert!(reset_bool_value(&definitions, 1, true, &pairs).unwrap());
    }
    #[test]
    fn embark_domain_and_owner_fallback_are_lazy_and_ordered() {
        assert!(
            !unit_embark_decision(0, 2, 3, |_| panic!("ability"), |_| panic!("validity")).unwrap()
        );
        assert!(unit_embark_decision(
            1,
            2,
            3,
            |p| {
                assert_eq!(p, 2);
                Ok(true)
            },
            |_| panic!("validity")
        )
        .unwrap());
        assert!(
            !unit_embark_decision(1, 2, 2, |_| Ok(false), |_| panic!("same owner validity"))
                .unwrap()
        );
        let mut calls = Vec::new();
        assert!(unit_embark_decision(
            1,
            2,
            3,
            |p| {
                calls.push(p);
                Ok(p == 3)
            },
            |p| {
                assert_eq!(p, 3);
                Ok(true)
            }
        )
        .unwrap());
        assert_eq!(calls, [2, 3]);
        assert!(!unit_embark_decision(
            1,
            2,
            3,
            |p| {
                assert_eq!(p, 2);
                Ok(false)
            },
            |_| Ok(false)
        )
        .unwrap());
        assert!(unit_embark_decision(
            1,
            2,
            3,
            |_| Err(Error::Definitions),
            |_| panic!("error fallback")
        )
        .is_err());
    }
    #[test]
    fn embark_barbarian_unlock_pass_and_definition_gate_order() {
        assert!(!player_embark_decision(
            0xd959ec24,
            true,
            || panic!("pass"),
            || panic!("definition"),
            |_| panic!("unlock")
        )
        .unwrap());
        assert!(player_embark_decision(
            1,
            true,
            || panic!("pass"),
            || panic!("definition"),
            |_| panic!("unlock")
        )
        .unwrap());
        assert!(player_embark_decision(
            1,
            false,
            || Ok(true),
            || panic!("definition"),
            |_| panic!("unlock")
        )
        .unwrap());
        assert!(!player_embark_decision(
            1,
            false,
            || Ok(false),
            || None,
            |_| panic!("absent definition")
        )
        .unwrap());
        assert!(player_embark_decision(
            1,
            false,
            || Err(Error::Definitions),
            || panic!("definition"),
            |_| panic!("unlock")
        )
        .is_err());
    }
    #[test]
    fn embark_global_check_precedes_local_even_when_local_false() {
        assert!(!acquired_with_global(1, 1, || panic!("global"), || panic!("local")).unwrap());
        assert!(!acquired_with_global(1, 0, || Ok(false), || panic!("local")).unwrap());
        assert!(
            acquired_with_global(1, 0, || Err(Error::Definitions), || panic!("local")).is_err()
        );
        let reads = std::cell::RefCell::new(Vec::new());
        assert!(!acquired_with_global(
            1,
            0,
            || {
                reads.borrow_mut().push("global");
                Ok(true)
            },
            || {
                reads.borrow_mut().push("local");
                Ok(false)
            }
        )
        .unwrap());
        assert_eq!(*reads.borrow(), ["global", "local"]);
    }
    #[test]
    fn embark_technology_then_civic_order_and_first_unlock() {
        let technologies = [
            EmbarkUnlock {
                all: true,
                unit_type: None,
            },
            EmbarkUnlock {
                all: false,
                unit_type: Some(b"UNIT_A".to_vec()),
            },
        ];
        let civics = [EmbarkUnlock {
            all: true,
            unit_type: None,
        }];
        let mut reads = Vec::new();
        assert!(unlocks_embark(b"UNIT_A", &technologies, &civics, |c, i| {
            reads.push((c, i));
            Ok(c)
        })
        .unwrap());
        assert_eq!(reads, [(false, 0), (false, 1), (true, 0)]);
        reads.clear();
        assert!(unlocks_embark(b"UNIT_A", &technologies, &civics, |c, i| {
            reads.push((c, i));
            Ok(i == 1)
        })
        .unwrap());
        assert_eq!(reads, [(false, 0), (false, 1)]);
        assert!(!unlocks_embark(b"unit_a", &technologies[1..], &[], |_, _| Ok(true)).unwrap());
        assert!(
            unlocks_embark(b"UNIT_A", &technologies, &civics, |_, _| Err(
                Error::Definitions
            ))
            .is_err()
        );
        assert!(!unlocks_embark(b"UNIT_A", &[], &[], |_, _| panic!("empty definitions")).unwrap());
    }
    #[test]
    fn current_levels_are_state_bound_sparse_and_not_defaulted() {
        use c6_core::core_player::initialization::{DatabaseIdentity, InputIdentities, WireRole};
        let binding = WireBinding {
            role: WireRole::ReadInputCore,
            core_sha256: [1; 32],
            occurrence_id: b"1".to_vec(),
            output_id: None,
            inputs: InputIdentities {
                configuration_sha256: [2; 32],
                rules_inputs_sha256: [3; 32],
                database: DatabaseIdentity::LiveSnapshotUnavailable,
                dlc_inputs_sha256: [4; 32],
                mods_inputs_sha256: [5; 32],
                content_inputs_sha256: [6; 32],
            },
        };
        let levels = CurrentCivilizationLevels {
            binding: &binding,
            values: &[(2, 0xd959ec24)],
        };
        validate_current_levels(&binding, &levels).unwrap();
        assert!(levels.values.iter().all(|p| p.0 != 3));
        let mut changed = binding.clone();
        changed.core_sha256[0] ^= 1;
        assert!(matches!(
            validate_current_levels(&changed, &levels),
            Err(Error::Binding)
        ));
        let duplicates = CurrentCivilizationLevels {
            binding: &binding,
            values: &[(2, 0), (2, 1)],
        };
        assert!(validate_current_levels(&binding, &duplicates).is_err());
        let invalid = CurrentCivilizationLevels {
            binding: &binding,
            values: &[(65, 0)],
        };
        assert!(validate_current_levels(&binding, &invalid).is_err());
        validate_current_levels(
            &binding,
            &CurrentCivilizationLevels {
                binding: &binding,
                values: &[],
            },
        )
        .unwrap();
    }
    #[test]
    fn open_border_count_alliance_permanent_order_is_lazy() {
        for count in [-9, 1, 7] {
            assert!(
                open_borders_from(count, || panic!("alliance"), || panic!("permanent")).unwrap()
            );
        }
        assert!(open_borders_from(0, || Ok(true), || panic!("permanent")).unwrap());
        let calls = std::cell::RefCell::new(Vec::new());
        assert!(open_borders_from(
            0,
            || {
                calls.borrow_mut().push("alliance");
                Ok(false)
            },
            || {
                calls.borrow_mut().push("permanent");
                Ok(true)
            }
        )
        .unwrap());
        assert_eq!(*calls.borrow(), ["alliance", "permanent"]);
        assert!(!open_borders_from(0, || Ok(false), || Ok(false)).unwrap());
        assert!(open_borders_from(0, || Err(Error::Definitions), || panic!("permanent")).is_err());
        assert!(open_borders_from(0, || Ok(false), || Err(Error::Definitions)).is_err());
    }
}
