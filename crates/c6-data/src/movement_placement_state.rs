//! Borrowed placement queries against actual LoadedState and bound definitions.
//! This does not admit actions/effects or replace unavailable runtime capabilities
//! with false. Plot-cell candidate order is preserved; World is never cloned.
use crate::{
    city_movement::NativeTerrainDefinitions,
    resource_binding::{ResourceAuthority, ResourceIdentity},
    unit_initialization_definitions::{self, collection},
    Rules,
};
use c6_core::{
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
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    ReservedAttributeOwner,
    ReservedAttributeDistrict,
    CityDomain,
    UnitEmbark,
    VisibilityContext,
    ExtendedUnitDefinition,
    CombatStrength,
    SparseCoordinate,
    BlockedEntry,
    ForeignContext(PlacementMissing),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "placement state: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Construct once from the same registered rules/resources and actual exported
/// terrain definitions. Fields are borrowed immutable definitions; each query
/// reads the current state rather than a per-action oracle result or clone.
pub struct Context<'s, 'a, 'r> {
    state: &'s LoadedState,
    units: DefinitionTable<'a, Option<placement::UnitProperties>>,
    terrains: DefinitionTable<'a, TerrainProperties>,
    districts: crate::registry::LinuxDefinitionRegistry,
    _resources: &'a ResourceAuthority<'r>,
}
impl<'s, 'a, 'r> Context<'s, 'a, 'r> {
    pub fn new(
        state: &'s LoadedState,
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
        Ok(Self {
            state,
            units: resources
                .placement_definitions()
                .map_err(|_| Error::Definitions)?,
            terrains: table,
            districts: collection(rules, "Districts", "DistrictType").map_err(Error::Rules)?,
            _resources: resources,
        })
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
    fn water(&self, plot: &Plot) -> Result<bool, Error> {
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
                    logic::MissingFact::CanEmbark => {
                        return Err(Error::Capability(Capability::UnitEmbark))
                    }
                    logic::MissingFact::CityDomainValid => {
                        return Err(Error::Capability(Capability::CityDomain))
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
        let map = self.state.world().map();
        let raw_index = destination
            .y
            .wrapping_mul(map.units.width as i32)
            .wrapping_add(destination.x) as u32 as usize;
        let layer = placement::unit_layer(self.state.world(), unit).map_err(|_| Error::Geometry)?;
        let references = match layer {
            None => &[][..], // Original UnitContainer rejects layer UINT_MAX.
            Some(layer) => {
                if raw_index >= map.plots.len() {
                    return Err(Error::Capability(Capability::SparseCoordinate));
                }
                self.state
                    .world()
                    .plot_units(layer, raw_index)
                    .map_err(Error::World)?
            }
        };
        let candidates = references.iter().map(|reference| {
            self.state
                .world()
                .unit(*reference)
                .map_err(Error::World)
                .map(|found| {
                    found.map(|found| {
                        let u = found.record;
                        ShoreDefender {
                            unit: u,
                            owner: u.e_owner,
                            delayed_death: u.b_delayed_death,
                            location: Some(u.k_location),
                            definition: DefenderDefinition::Unresolved,
                            religious_strength: Some(u.k_religion.i_religious_strength),
                        }
                    })
                })
        });
        logic::select_shore_defender(
            candidates,
            unit.e_owner,
            None,
            Some(unit.k_religion.i_religious_strength),
            |_| Err(Error::Capability(Capability::CombatStrength)),
        )
        .map_err(|error| match error {
            DefenderError::Candidate(e) | DefenderError::CombatStrength(e) => e,
            DefenderError::VisibilityContext => Error::Capability(Capability::VisibilityContext),
            _ => Error::Capability(Capability::ExtendedUnitDefinition),
        })
    }
    fn query(
        &self,
        unit: &Unit,
        destination: PlotCoord,
        request: PlacementRequest,
    ) -> Result<i32, Error> {
        let Some((_index, plot)) = self.plot(destination)? else {
            return logic::shore_unit_result(ShorePlacementPlot::KnownAbsent)
                .map_err(Error::Placement);
        };
        let mut f = ShoreUnitFacts {
            terrain: Some(self.terrain(unit, plot, request)?),
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
                        return Err(Error::Capability(Capability::BlockedEntry))
                    }
                    _ => return Err(Error::Capability(Capability::ForeignContext(missing))),
                },
            }
        }
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
