//! Bounded synchronous Unit activity reporting over authoritative loaded state.
//! Reflected Unit snapshots are omitted under the evidenced no-UI boundary;
//! their conditional gameplay-query effects are checked at the original Send.
//! This does not drain host event playback or admit a complete action.
use super::{resources::Bound, Error, IdentityReport, Reports};
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
        bound: &Bound<'_>,
        rules_runtime: &[u8],
    ) -> Result<Self, Error> {
        if bound.destination_binding() != state.binding() {
            return Err(Error::ConfigurationIdentity);
        }
        unit_initialization_definitions::Definitions::validate_resources(
            rules,
            bound,
            rules_runtime,
        )
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessPath {
    MemberNotReady,
    OtherFormationMemberNotReady,
    NoFormation,
    CyclePastGuard,
    AdjacentMove,
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
    publications: Vec<ActivityPublication>,
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
            snapshots: Vec::new(),
        }
    }
    pub fn pending_activity_publications(&self) -> &[ActivityPublication] {
        &self.publications
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
    pub fn send_loaded_activity(
        &mut self,
        state: &LoadedState,
        reference: UnitReference,
        report: IdentityReport,
    ) -> Result<(), Error> {
        if state.binding() != &self.definitions.binding {
            return Err(Error::ConfigurationIdentity);
        }
        if report.event != ACTIVITY_CHANGED {
            return Err(Error::Provider(
                "activity provider supports only UnitActivityChanged",
            ));
        }
        // Type.subtype is zero. EventData's separate identity type 6447c33d
        // comes from decoding the Unit ComponentID, not Type.subtype.
        if report.subtype != 0 || report.first.is_none() || report.second.is_some() {
            return Err(Error::Provider(
                "UnitActivityChanged typed identity/int shape",
            ));
        }
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
        if !unit.k_properties.entries.is_empty() {
            return Err(Error::Provider(
                "Unit snapshot nonempty/custom property copy effects",
            ));
        }
        // The concrete output queue/consumer proof permits dropping its values,
        // but the producer's effectful readiness branch must still be resolved.
        let path = self.check_snapshot_query_effects(state, unit)?;
        self.snapshots.push(SnapshotObservation {
            identity: report.identity,
            current_activity: unit.e_activity,
            queue_nodes: unit.k_operations.nodes.len(),
            remaining_q8: unit.x_moves_remaining,
            path,
        });
        let activity = report.first.expect("shape checked");
        if activity == OPERATION {
            return Err(Error::Provider(
                "event35 operation facade projection unavailable",
            ));
        }
        // CacheUpdated275 and event35 prepublish callbacks were actually NULL
        // on this closed stock reporting path. Their later queued consumers are
        // not called here. The no-UI reflected Unit stream itself is omitted.
        self.publications.push(ActivityPublication {
            identity: report.identity,
            activity,
        });
        Ok(())
    }

    fn check_snapshot_query_effects(
        &self,
        state: &LoadedState,
        unit: &Unit,
    ) -> Result<ReadinessPath, Error> {
        if !member_ready(unit) {
            return Ok(ReadinessPath::MemberNotReady);
        }
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let Some(reference) = runtime.formation else {
            return Ok(ReadinessPath::NoFormation);
        };
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
        if formation.unit_ids.len() >= 2 {
            let player = state
                .world()
                .player(formation.owner)
                .map_err(Error::World)?;
            for &id in &formation.unit_ids {
                if id != unit.id {
                    if let Some(member) = player.unit(id).map_err(Error::World)? {
                        if !member_ready(member.record) {
                            return Ok(ReadinessPath::OtherFormationMemberNotReady);
                        }
                    }
                }
            }
        }
        if !unit_placement::coordinate_valid(unit.k_location)
            || unit.b_delayed_death
            || unit.b_needs_attention
        {
            return Ok(ReadinessPath::CyclePastGuard);
        }
        if self.has_adjacent_move(state, unit)? {
            return Ok(ReadinessPath::AdjacentMove);
        }
        Err(Error::Provider("CanCyclePast command HoldCycling then conditional operation HoldCycling callbacks unavailable"))
    }

    fn has_adjacent_move(&self, state: &LoadedState, unit: &Unit) -> Result<bool, Error> {
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
                return Ok(true);
            }
            let cost =
                unit_movement_state::plot_movement_cost(state, plot, self.movement.definitions)
                    .map_err(Error::Movement)?;
            if cost <= unit.x_moves_remaining >> 8 {
                return Ok(true);
            }
        }
        Ok(false)
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
        state: &mut LoadedState,
        reference: UnitReference,
        report: IdentityReport,
    ) -> Result<i32, Error> {
        self.send_loaded_activity(state, reference, report)?;
        // Existing loaded continuation ignores this diagnostic hook result.
        Ok(0)
    }
}
