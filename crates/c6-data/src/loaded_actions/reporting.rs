//! Bounded synchronous Unit activity reporting over authoritative loaded state.
//! Reflected Unit snapshots are omitted under the evidenced no-UI boundary;
//! their conditional gameplay-query effects are checked at the original Send.
//! This does not drain host event playback or admit a complete action.
#[path = "yield_monitor.rs"]
pub mod yield_monitor;

use super::{Error, IdentityReport, Reports};
use crate::resource_binding::ResourceAuthority;
use crate::{
    registry::LinuxDefinitionRegistry,
    unit_initialization_definitions::{self, collection},
    unit_movement_state, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::{Plot, PlotFlag, UnitReference},
    skip_turn::{
        legality,
        placement::{self as unit_placement, UnitProperties},
    },
    unit::{ComponentId, PlotCoord, Unit},
};

const ACTIVITY_CHANGED: u32 = 0xe23b112d;
const OPERATION_DEACTIVATED: u32 = 0xa60328c5;
const OPERATIONS_CLEARED: u32 = 0x383669f1;
const OPERATION_ADDED: u32 = 0x15c35863;
const OPERATION_STARTED: u32 = 0x3d4006f2;
const AWAKE: i32 = 0x6b25012e;
const OPERATION: i32 = 0x490cc8e1;

/// Immutable, configuration-admitted definitions; never effective query answers.
pub struct Definitions {
    binding: WireBinding,
    terrains: LinuxDefinitionRegistry,
    terrain_water: Vec<bool>,
    unit_properties: Vec<Option<UnitProperties>>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if bound.destination_binding() != state.binding() {
            return Err(Error::ConfigurationIdentity);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(|_| Error::Provider("activity reporting rules resource identity"))?;
        let terrains = collection(rules, "Terrains", "TerrainType")
            .map_err(|_| Error::Provider("activity reporting terrain order"))?;
        let table = rules
            .gameplay()
            .table("Terrains")
            .ok_or(Error::Provider("activity reporting Terrains"))?;
        let terrain_water = terrains
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Provider("activity reporting terrain row"))?;
                match row.integer("Water") {
                    Ok(0) => Ok(false),
                    Ok(1) => Ok(true),
                    _ => Err(Error::Provider("activity reporting Water scalar")),
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            terrains,
            terrain_water,
            unit_properties: {
                let native = bound
                    .placement_definitions()
                    .map_err(|_| Error::Provider("activity reporting unit definitions"))?;
                (0..native.lookup.count())
                    .map(|i| native.at(i as i32).copied().flatten())
                    .collect()
            },
        })
    }
}

/// A host event35 output prepared at its original synchronous translation point.
/// Delivery remains pending until an actual playback boundary. No invocation of
/// unknown later subscribers, or successful queue drain, is implied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityPublication {
    pub identity: ComponentId,
    pub activity: i32,
}

/// Logical host publication fields, prepared in original Send order. EventData1
/// initializes missing scalar parameters to zero; activity operation defaults
/// are the distinct constructor values, retained below.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Publication {
    pub host_event: u32,
    pub source: IdentityReport,
    pub operation: i32,
    pub value: i32,
    pub activity_operation: Option<u32>,
    pub activity_parameters: Option<[i32; 4]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessPath {
    MemberNotReady,
    OtherFormationMemberNotReady,
    NoFormation,
    CyclePastGuard,
    AdjacentMove(AdjacentMoveWitness),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdjacentMoveWitness {
    pub direction: u8,
    pub location: PlotCoord,
    pub has_moved: bool,
    pub plot_cost: Option<i32>,
    pub remaining_integer: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotObservation {
    pub identity: ComponentId,
    pub current_activity: i32,
    pub queue_nodes: usize,
    pub remaining_q8: i32,
    pub path: ReadinessPath,
}

/// Scope must be admitted by the caller for its active configuration. Stock Lua
/// source hashes alone do not establish absence of an added gameplay consumer.
pub enum Scope {
    StockHeadlessNoAddedConsumers,
}

pub struct Provider<'a> {
    definitions: &'a Definitions,
    movement: &'a unit_movement_state::Inputs<'a>,
    publications: Vec<Publication>,
    skip_turn_handler: Option<i32>,
    snapshots: Vec<SnapshotObservation>,
}
impl<'a> Provider<'a> {
    pub fn new(
        definitions: &'a Definitions,
        movement: &'a unit_movement_state::Inputs<'a>,
        _: Scope,
    ) -> Self {
        Self {
            definitions,
            movement,
            publications: Vec::new(),
            skip_turn_handler: None,
            snapshots: Vec::new(),
        }
    }
    pub fn pending_publications(&self) -> &[Publication] {
        &self.publications
    }
    pub fn pending_activity_publications(&self) -> Vec<ActivityPublication> {
        self.publications
            .iter()
            .filter(|p| p.host_event == 35)
            .map(|p| ActivityPublication {
                identity: p.source.identity,
                activity: p.source.first.expect("validated activity"),
            })
            .collect()
    }
    /// Admit the current actual handler registry, not a fabricated hash lookup.
    pub fn bind_skip_turn_handler(
        &mut self,
        state: &LoadedState,
        authority: &super::handlers::BoundHandlers<'_>,
        operations: &crate::unit_operation_manager::OperationDefinitions,
    ) -> Result<(), Error> {
        let registry = authority.registry().map_err(Error::HandlerObservation)?;
        authority
            .validate_execution(state, &registry, operations)
            .map_err(Error::HandlerObservation)?;
        let id = authority
            .runtime_id(c6_core::unit::SKIP_TURN_HASH)
            .map_err(Error::HandlerObservation)?;
        let handler = registry
            .handler(id)
            .map_err(Error::Registry)?
            .ok_or(Error::Provider("actual SkipTurn facade handler absent"))?;
        if handler.identity().operation_hash.0 != c6_core::unit::SKIP_TURN_HASH {
            return Err(Error::Provider("actual SkipTurn facade handler identity"));
        }
        self.skip_turn_handler = Some(id.0);
        Ok(())
    }
    pub fn snapshot_observations(&self) -> &[SnapshotObservation] {
        &self.snapshots
    }
    /// This bounded provider has no admitted playback subscriber registry.
    pub fn drain_playback(&mut self) -> Result<(), Error> {
        Err(Error::Provider(
            "host event35 queued playback consumers unavailable",
        ))
    }
    pub fn send_loaded_report(
        &mut self,
        state: &LoadedState,
        reference: UnitReference,
        report: IdentityReport,
    ) -> Result<(), Error> {
        if state.binding() != &self.definitions.binding {
            return Err(Error::ConfigurationIdentity);
        }
        let (host_event, snapshot) = match report.event {
            ACTIVITY_CHANGED
                if report.subtype == 0 && report.first.is_some() && report.second.is_none() =>
            {
                (35, true)
            }
            OPERATION_DEACTIVATED
                if report.subtype == 0 && report.first.is_some() && report.second.is_some() =>
            {
                (70, false)
            }
            OPERATIONS_CLEARED
                if report.subtype == 0 && report.first.is_none() && report.second.is_none() =>
            {
                (71, true)
            }
            OPERATION_ADDED
                if report.subtype == 0 && report.first.is_some() && report.second.is_none() =>
            {
                (67, true)
            }
            OPERATION_STARTED
                if report.subtype == c6_core::unit::SKIP_TURN_HASH
                    && report.first.is_some()
                    && report.second.is_none() =>
            {
                (68, true)
            }
            ACTIVITY_CHANGED
            | OPERATION_DEACTIVATED
            | OPERATIONS_CLEARED
            | OPERATION_ADDED
            | OPERATION_STARTED => {
                return Err(Error::Provider("Unit reporting typed argument shape"))
            }
            _ => {
                return Err(Error::Provider(
                    "Unit reporting consumer unavailable for this event",
                ))
            }
        };
        let unit = state
            .world()
            .unit(reference)
            .map_err(Error::World)?
            .ok_or(Error::MissingUnit)?
            .record;
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        if runtime.archive.identity() != report.identity {
            return Err(Error::Provider("activity reporting actual Unit identity"));
        }
        if snapshot {
            if !unit.k_properties.entries.is_empty() {
                return Err(Error::Provider(
                    "Unit snapshot nonempty/custom property copy effects",
                ));
            }
            let path = self.check_snapshot_query_effects(state, unit)?;
            self.snapshots.push(SnapshotObservation {
                identity: report.identity,
                current_activity: unit.e_activity,
                queue_nodes: unit.k_operations.nodes.len(),
                remaining_q8: unit.x_moves_remaining,
                path,
            });
        }
        // UnitActivityChanged's constructor initializes activity=-1 and
        // operation=0 in one u64 store; its four parameter fields are -1.
        let mut activity_operation = (host_event == 35).then_some(0);
        if host_event == 35 && report.first == Some(OPERATION) {
            activity_operation = Some(match unit.k_operations.nodes.first() {
                None => 0,
                Some(node)
                    if node.operation_hash == c6_core::unit::SKIP_TURN_HASH
                        && matches!(node.parameters, c6_core::unit::OperationParameters::None) =>
                {
                    self.skip_turn_handler.ok_or(Error::Provider(
                        "actual SkipTurn facade handler binding unavailable",
                    ))?;
                    // Original handler +10 returns the SkipTurn hash; concrete
                    // parameter writer +28 is RET and leaves the fresh map empty.
                    c6_core::unit::SKIP_TURN_HASH
                }
                Some(_) => {
                    return Err(Error::Provider(
                        "event35 current front facade type/parameter writer unavailable",
                    ))
                }
            });
        }
        let (operation, value) = match host_event {
            67 | 68 => (report.subtype as i32, report.first.unwrap_or(0)),
            70 | 71 => (report.first.unwrap_or(0), report.second.unwrap_or(0)),
            _ => (activity_operation.unwrap_or(0) as i32, -1),
        };
        self.publications.push(Publication {
            host_event,
            source: report,
            operation,
            value,
            activity_operation,
            activity_parameters: (host_event == 35).then_some([-1; 4]),
        });
        Ok(())
    }

    fn check_snapshot_query_effects(
        &self,
        state: &LoadedState,
        unit: &Unit,
    ) -> Result<ReadinessPath, Error> {
        if let Some(path) = self.member_readiness_path(state, unit)? {
            return Ok(path);
        }
        if !unit_placement::coordinate_valid(unit.k_location)
            || unit.b_delayed_death
            || unit.b_needs_attention
        {
            return Ok(ReadinessPath::CyclePastGuard);
        }
        if let Some(witness) = self.has_adjacent_move(state, unit)? {
            return Ok(ReadinessPath::AdjacentMove(witness));
        }
        Err(Error::Provider("CanCyclePast command HoldCycling then conditional operation HoldCycling callbacks unavailable"))
    }

    /// Native IsReadyToMove: member guards precede Formation lookup, and other
    /// members are visited in stored order. CanCyclePast is a separate query.
    pub fn is_ready_to_move(&self, state: &LoadedState, unit: &Unit) -> Result<bool, Error> {
        self.validate_readiness_unit(state, unit)?;
        Ok(!matches!(
            self.member_readiness_path(state, unit)?,
            Some(ReadinessPath::MemberNotReady | ReadinessPath::OtherFormationMemberNotReady)
        ))
    }

    /// Independent native CanCyclePast. Pending operations do not short-circuit
    /// this getter. Reached HoldCycling callbacks remain genuine dependencies.
    pub fn can_cycle_past(&self, state: &LoadedState, unit: &Unit) -> Result<bool, Error> {
        self.validate_readiness_unit(state, unit)?;
        if !unit_placement::coordinate_valid(unit.k_location) || unit.b_delayed_death {
            return Ok(true);
        }
        if self.bound_formation(state, unit)?.is_none() {
            return Ok(true);
        }
        if unit.b_needs_attention || self.has_adjacent_move(state, unit)?.is_some() {
            return Ok(false);
        }
        Err(Error::Provider("CanCyclePast command HoldCycling then conditional operation HoldCycling callbacks unavailable"))
    }

    fn validate_readiness_unit(&self, state: &LoadedState, unit: &Unit) -> Result<(), Error> {
        if self.definitions.binding != *state.binding() {
            return Err(Error::ConfigurationIdentity);
        }
        let current = state
            .world()
            .player(unit.e_owner)
            .map_err(Error::World)?
            .unit(unit.id)
            .map_err(Error::World)?
            .ok_or(Error::MissingUnit)?
            .record;
        if !std::ptr::eq(current, unit) {
            return Err(Error::Provider(
                "readiness query requires authoritative Unit",
            ));
        }
        Ok(())
    }

    fn bound_formation<'s>(
        &self,
        state: &'s LoadedState,
        unit: &Unit,
    ) -> Result<Option<&'s c6_core::unit::Formation>, Error> {
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let Some(reference) = runtime.formation else {
            if state
                .world()
                .player(unit.e_owner)
                .map_err(Error::World)?
                .formation(unit.formation_id)
                .map_err(Error::World)?
                .is_some()
            {
                return Err(Error::Provider("snapshot formation binding changed"));
            }
            return Ok(None);
        };
        if reference.owner != unit.e_owner || reference.requested_id != unit.formation_id {
            return Err(Error::Provider("snapshot formation binding changed"));
        }
        let formation = state
            .world()
            .player(reference.owner)
            .map_err(Error::World)?
            .formation(reference.requested_id)
            .map_err(Error::World)?
            .ok_or(Error::Provider("snapshot formation binding absent"))?
            .record;
        if formation.id != reference.resolved_id {
            return Err(Error::Provider("snapshot formation binding changed"));
        }
        Ok(Some(formation))
    }

    fn member_readiness_path(
        &self,
        state: &LoadedState,
        unit: &Unit,
    ) -> Result<Option<ReadinessPath>, Error> {
        if !member_ready(unit) {
            return Ok(Some(ReadinessPath::MemberNotReady));
        }
        let Some(formation) = self.bound_formation(state, unit)? else {
            return Ok(Some(ReadinessPath::NoFormation));
        };
        if formation.unit_ids.len() >= 2 {
            let player = state
                .world()
                .player(formation.owner)
                .map_err(Error::World)?;
            for &id in &formation.unit_ids {
                if id != unit.id {
                    if let Some(member) = player.unit(id).map_err(Error::World)? {
                        if !member_ready(member.record) {
                            return Ok(Some(ReadinessPath::OtherFormationMemberNotReady));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    fn has_adjacent_move(
        &self,
        state: &LoadedState,
        unit: &Unit,
    ) -> Result<Option<AdjacentMoveWitness>, Error> {
        let map = state.world().map();
        if map.header.width <= 0 || map.header.height <= 0 {
            return Err(Error::Provider("HasAdjacentMove Map dimensions"));
        }
        // Same native axial direction order as movement_world::adjacent.
        const X: [i32; 6] = [0, 1, 1, 0, -1, -1];
        const Y: [i32; 6] = [1, 0, -1, -1, 0, 1];
        let half = |y: i32| {
            if y >= 0 {
                y >> 1
            } else {
                let m = y.wrapping_sub(1);
                m.wrapping_add(((m as u32) >> 31) as i32) >> 1
            }
        };
        let axial_x = unit.k_location.x.wrapping_sub(half(unit.k_location.y));
        for direction in 0..6 {
            let y = unit.k_location.y.wrapping_add(Y[direction]);
            let x = axial_x.wrapping_add(X[direction]).wrapping_add(half(y));
            let x = c6_core::movement_world::coord_range(x, map.header.width, map.header.wrap_x);
            let y = c6_core::movement_world::coord_range(y, map.header.height, map.header.wrap_y);
            let Some(plot) = map.plot_at(x, y) else {
                continue;
            };
            let coord = PlotCoord { x, y };
            if !unit_placement::coordinate_valid(coord) {
                continue;
            }
            if !self.placement_valid(state, unit, plot, coord)? {
                continue;
            }
            if !unit_movement_state::unit_has_moved(state, unit, self.movement)
                .map_err(Error::Movement)?
            {
                return Ok(Some(AdjacentMoveWitness {
                    direction: direction as u8,
                    location: coord,
                    has_moved: false,
                    plot_cost: None,
                    remaining_integer: unit.x_moves_remaining >> 8,
                }));
            }
            let cost =
                unit_movement_state::plot_movement_cost(state, plot, self.movement.definitions)
                    .map_err(Error::Movement)?;
            if cost <= unit.x_moves_remaining >> 8 {
                return Ok(Some(AdjacentMoveWitness {
                    direction: direction as u8,
                    location: coord,
                    has_moved: true,
                    plot_cost: Some(cost),
                    remaining_integer: unit.x_moves_remaining >> 8,
                }));
            }
        }
        Ok(None)
    }

    fn placement_valid(
        &self,
        state: &LoadedState,
        unit: &Unit,
        plot: &Plot,
        coord: PlotCoord,
    ) -> Result<bool, Error> {
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let terrain = self.definitions.terrains.find_index(plot.terrain_hash);
        let invalid = terrain
            .and_then(|i| runtime.invalid_terrain.get(i))
            .copied()
            .unwrap_or(false);
        if plot.flags.get(PlotFlag::Impassable) || invalid {
            return Ok(false);
        }
        let water = match terrain {
            Some(15 | 16) => true,
            Some(i) => self.definitions.terrain_water[i],
            None => false,
        };
        if (runtime.domain == 0 && !water) || (runtime.domain == 2 && water) {
            return Err(Error::Provider(
                "adjacent placement embark/city valid-domain branch unavailable",
            ));
        }
        let properties = usize::try_from(runtime.definition_index)
            .ok()
            .and_then(|i| self.definitions.unit_properties.get(i))
            .and_then(Option::as_ref)
            .ok_or(Error::Provider("adjacent placement Unit definition"))?;
        if properties.spy {
            return Err(Error::Provider(
                "adjacent placement spy district branch unavailable",
            ));
        }
        let owner = plot
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player));
        if owner != -1 && owner != unit.e_owner {
            return Err(Error::Provider(
                "adjacent placement foreign blocked-entry/border/district queries unavailable",
            ));
        }
        // For the bounded empty-cell branch the original best-defender scan has
        // no candidate to invoke. Nonempty cells require exact filtering/strength.
        for layer in &state.world().map().units.layers {
            if !unit_placement::units_at(state.world(), Some(layer.layer_id), coord)
                .map_err(Error::Placement)?
                .is_empty()
            {
                return Err(Error::Provider(
                    "adjacent placement nonempty defender candidate scan unavailable",
                ));
            }
        }
        if unit.i_barbarian_tribe_index >= 0 {
            return Err(Error::Provider(
                "adjacent placement barbarian clan/bribe branch unavailable",
            ));
        }
        Ok(true)
    }
}
fn member_ready(unit: &Unit) -> bool {
    !unit.b_delayed_death
        && unit.x_moves_remaining > 0
        && unit.k_operations.nodes.is_empty()
        && unit.e_activity == AWAKE
        && unit.e_automation == -1
        && !legality::is_busy(unit)
}

impl Reports for Provider<'_> {
    fn send(&mut self, _: &mut Unit, _: IdentityReport) -> Result<i32, Error> {
        Err(Error::Provider(
            "activity provider requires authoritative LoadedState reporting boundary",
        ))
    }
    fn send_loaded(
        &mut self,
        context: &mut super::ReportingContext<'_>,
        report: IdentityReport,
    ) -> Result<i32, Error> {
        self.send_loaded_report(context.state(), context.reference(), report)?;
        // Existing loaded continuation ignores this diagnostic hook result.
        Ok(0)
    }
}
