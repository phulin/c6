//! Bounded synchronous Unit activity reporting over authoritative loaded state.
//! Reflected Unit snapshots are omitted under the evidenced no-UI boundary;
//! their conditional gameplay-query effects are checked at the original Send.
//! This does not drain host event playback or admit a complete action.
#[path = "hold_cycling.rs"]
pub mod hold_cycling;
#[path = "yield_monitor.rs"]
pub mod yield_monitor;

use super::{arena::ActorRecords, Error, IdentityReport, Reports};
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
    unit_placement_transition::{PlacementPayload, PlacementReport},
    unit_turn_upkeep::{
        MovementReport, MOVEMENT_RESTORED, MOVES_EXHAUSTED, MOVES_REMAINING_CHANGED,
    },
};

const ACTIVITY_CHANGED: u32 = 0xe23b112d;
const OPERATION_DEACTIVATED: u32 = 0xa60328c5;
const OPERATIONS_CLEARED: u32 = 0x383669f1;
const OPERATION_ADDED: u32 = 0x15c35863;
const OPERATION_STARTED: u32 = 0x3d4006f2;
const OPERATION_SEGMENT_COMPLETE: u32 = 0xcb0345f1;
const AWAKE: i32 = 0x6b25012e;
const OPERATION: i32 = 0x490cc8e1;

/// Host69 translator copies the two current EventData words at+0x50/+0x54
/// without invoking an operation facade or remapping signed hash bits.
fn segment_complete_payload(report: IdentityReport) -> Result<(i32, i32), Error> {
    match report {
        IdentityReport {
            event: OPERATION_SEGMENT_COMPLETE,
            subtype: 0,
            first: Some(operation),
            second: Some(segment),
            ..
        } => Ok((operation, segment)),
        _ => Err(Error::Provider(
            "Unit segment-complete typed argument shape",
        )),
    }
}

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

/// Host257 NotificationActivated output pending later playback. Preparing this
/// payload does not execute or classify unknown later UI/Lua subscribers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotificationActivatedPublication {
    pub host_event: u32,
    pub event_data_type: u32,
    pub identity: c6_core::notifications::ComponentId,
    pub argument: bool,
}

/// Typed host movement output, pending later playback. Q8 is transmitted as
/// native f32 first, then reconstructed by the host FixedPointT<8> constructor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MovementPublication {
    pub host_event: u32,
    pub identity: ComponentId,
    pub source_raw_q8: Option<i32>,
    pub wire_float_bits: Option<u32>,
    pub host_raw_q8: Option<i32>,
}
fn movement_publication(report: MovementReport) -> Result<(MovementPublication, bool), Error> {
    let (host_event, snapshot, scalar) = match report.event_hash {
        MOVES_REMAINING_CHANGED => (25, true, true),
        MOVES_EXHAUSTED => (26, false, false),
        MOVEMENT_RESTORED => (27, false, true),
        _ => return Err(Error::Provider("unsupported movement report event")),
    };
    if report.raw_q8.is_some() != scalar
        || report.identity.component_type != c6_core::formation_initialization::UNIT_COMPONENT_TYPE
    {
        return Err(Error::Provider("movement report typed argument shape"));
    }
    let projection = report.raw_q8.map(|raw| {
        // XP2 AsFloat1fe2170: integer and low-byte fractional terms are
        // converted independently, then added with f32 precision.
        let value = ((raw >> 8) as f32) + (f32::from(raw as u8) * (1.0f32 / 256.0));
        // Host AssignFloat32272a0: floor, shift integer bits, retain the
        // fractional byte. Wrapping is observable at the positive endpoint.
        let floor = value.floor();
        let integer = (floor as i32).wrapping_shl(8);
        let fraction = (((value - floor) * 256.0) as i32) & 255;
        (value.to_bits(), integer | fraction)
    });
    Ok((
        MovementPublication {
            host_event,
            identity: report.identity,
            source_raw_q8: report.raw_q8,
            wire_float_bits: projection.map(|p| p.0),
            host_raw_q8: projection.map(|p| p.1),
        },
        snapshot,
    ))
}

/// Host24 completion output. Pure presentation-only reflection fields are not
/// represented; both reached FillVariants query surfaces are executed below.
/// Actual observer plus current native linked unit order. Unknown inputs stay
/// unavailable; an empty slice means a proven empty current roster.
#[derive(Clone, Copy)]
pub struct MoveCompleteInputs<'a> {
    pub local_observer: Option<i32>,
    pub native_unit_order: Option<&'a [UnitReference]>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveCompletePublication {
    pub identity: ComponentId,
    pub location: PlotCoord,
    pub blocking: bool,
}

/// Host UnitMoved event 23, prepared at the synchronous BE121408 translator.
/// These are its actual EventData1 identity/coordinate and four keyed values;
/// the record does not imply that prepublish or queued consumers were delivered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MovedPublication {
    pub host_event: u32,
    pub event_data_type: u32,
    pub identity: ComponentId,
    pub location: PlotCoord,
    /// Native TypedVariant bool at key 476bda03.
    pub locally_visible: bool,
    /// Native TypedVariant bool at key c6a27b74.
    pub newly_locally_visible: bool,
    /// Native TypedVariant unsigned integer at key 15808e44.
    pub embark_event: u32,
    /// Native GetComponentID at key c02eac8a.
    pub parent: ComponentId,
}

/// Source UnitAbilityGained65 publication, after current ability/cache queries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbilityPublication {
    pub host_event:u32,
    pub identity:ComponentId,
    pub ability_index:i32,
    /// Native GetAbilities snapshot after the Unit cache refresh.
    pub active_abilities:Vec<i32>,
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

/// Cross-kind source order of retained typed host outputs. Reflected DTO output
/// excluded by the admitted presentation scope is not a queued gameplay effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationIndex {
    Identity(usize),
    Movement(usize),
    Moved(usize),
    MoveComplete(usize),
    NotificationActivated(usize),
    Ability(usize),
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
#[derive(Clone, Copy)]
pub enum Scope {
    StockHeadlessNoAddedConsumers,
}

/// Borrowed replacement admitted against the exact current actor records.
/// Its private fields prevent bypassing the authoritative query checks.
pub(crate) struct ActorOverlay<'s> {
    state: &'s LoadedState,
    after: &'s ActorRecords,
}
impl<'s> ActorOverlay<'s> {
    pub(crate) fn checked(
        state: &'s LoadedState,
        before: &ActorRecords,
        after: &'s ActorRecords,
    ) -> Result<Self, Error> {
        use c6_core::formation_initialization::{FORMATION_COMPONENT_TYPE, UNIT_COMPONENT_TYPE};
        let reference = before.reference;
        let owner = reference.player();
        let actual = state
            .world()
            .unit(reference)
            .map_err(Error::World)?
            .ok_or(Error::MissingUnit)?;
        let formation = state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .formation(before.unit.formation_id)
            .map_err(Error::World)?
            .ok_or(Error::Provider("count overlay Formation absent"))?;
        if actual.record != &before.unit || formation.record != &before.formation {
            return Err(Error::Provider("count overlay before records changed"));
        }
        let runtime = state
            .unit_runtime(owner, reference.id)
            .map_err(Error::State)?;
        let formation_runtime = state
            .formation_runtime(owner, before.formation.id)
            .map_err(Error::State)?;
        let unit_identity = runtime.archive.identity();
        let formation_identity = formation_runtime.archive.identity();
        let link = runtime
            .formation
            .ok_or(Error::Provider("count overlay Formation binding absent"))?;
        if reference.component_id().component_type != UNIT_COMPONENT_TYPE
            || unit_identity != reference.component_id()
            || actual.resolved_id != reference.id
            || before.unit.id != reference.id
            || before.unit.e_owner != owner
            || formation_identity.component_type != FORMATION_COMPONENT_TYPE
            || i32::from(formation_identity.owner) != owner
            || formation_identity.id != before.formation.id
            || before.formation.owner != owner
            || formation.resolved_id != before.formation.id
            || before.unit.formation_id != before.formation.id
            || link.owner != owner
            || link.requested_id != before.unit.formation_id
            || link.resolved_id != before.formation.id
            || before.formation.unit_ids != [before.unit.id]
            || after.reference != reference
            || after.unit.id != before.unit.id
            || after.unit.e_owner != owner
            || after.unit.formation_id != before.unit.formation_id
            || after.formation.id != before.formation.id
            || after.formation.owner != owner
            || after.formation.unit_ids != before.formation.unit_ids
        {
            return Err(Error::Provider("count overlay actor/Formation identity"));
        }
        // LoadedState's runtime admission is derived from these wire fields.
        // The actual records above have already passed its current-basis guard.
        let a = &before.unit;
        let b = &after.unit;
        if a.e_type_index != b.e_type_index
            || a.ai_abilities_possessed != b.ai_abilities_possessed
            || a.ab_invalid_terrain_types != b.ab_invalid_terrain_types
            || a.ab_ignore_terrain_types != b.ab_ignore_terrain_types
            || a.ab_ignore_feature_types != b.ab_ignore_feature_types
            || a.ai_bonus_points_per_kill != b.ai_bonus_points_per_kill
            || a.ai_bonus_points_per_kill_strength != b.ai_bonus_points_per_kill_strength
            || a.k_operations_disabled != b.k_operations_disabled
        {
            return Err(Error::Provider("count overlay runtime basis changed"));
        }
        Ok(Self { state, after })
    }

    pub(crate) fn unit<'u>(&'u self, unit: &'u Unit) -> &'u Unit {
        if unit.e_owner == self.after.unit.e_owner && unit.id == self.after.unit.id {
            &self.after.unit
        } else {
            unit
        }
    }

    pub(crate) fn formation<'u>(
        &'u self,
        formation: &'u c6_core::unit::Formation,
    ) -> &'u c6_core::unit::Formation {
        if formation.owner == self.after.formation.owner && formation.id == self.after.formation.id
        {
            &self.after.formation
        } else {
            formation
        }
    }

    fn actor_formation(&self, unit: &Unit) -> Option<&c6_core::unit::Formation> {
        (unit.e_owner == self.after.unit.e_owner && unit.id == self.after.unit.id)
            .then_some(&self.after.formation)
    }
}

/// Immutable placement definitions and current configuration/domain services.
/// The state itself is reborrowed inside each reached query, after preceding
/// notification/script effects, instead of retaining an old World view.
#[derive(Clone, Copy)]
pub struct CurrentPlacementQueries<'a> {
    pub definitions: &'a crate::movement_placement_state::Definitions<'a, 'a>,
    pub queries: &'a crate::unit_placement_state::Queries<'a>,
}

pub struct Provider<'a> {
    definitions: &'a Definitions,
    movement: &'a unit_movement_state::Inputs<'a>,
    publications: Vec<Publication>,
    publication_order: Vec<PublicationIndex>,
    skip_turn_handler: Option<i32>,
    move_to_handler: Option<i32>,
    snapshots: Vec<SnapshotObservation>,
    moved_publications: Vec<MovedPublication>,
    ability_publications:Vec<AbilityPublication>,
    movement_publications: Vec<MovementPublication>,
    notification_activated_publications: Vec<NotificationActivatedPublication>,
    move_complete_publications: Vec<MoveCompletePublication>,
    command_queries: Option<hold_cycling::commands::Inputs<'a>>,
    placement_queries: Option<&'a crate::movement_placement_state::Context<'a, 'a, 'a>>,
    current_placement_queries: Option<CurrentPlacementQueries<'a>>,
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
            publication_order: Vec::new(),
            skip_turn_handler: None,
            move_to_handler: None,
            snapshots: Vec::new(),
            moved_publications: Vec::new(),
            ability_publications:Vec::new(),
            movement_publications: Vec::new(),
            notification_activated_publications: Vec::new(),
            move_complete_publications: Vec::new(),
            command_queries: None,
            placement_queries: None,
            current_placement_queries: None,
        }
    }
    /// Reuse the computed shore-placement provider for the exact flags1,
    /// null-output/empty-filter HasAdjacentMove call. The context borrows the
    /// same current World and admitted definitions/configuration; no predicate
    /// answers are supplied here. Actor overlay fields remain the query input.
    pub fn bind_placement_queries(
        &mut self,
        context: &'a crate::movement_placement_state::Context<'a, 'a, 'a>,
    ) -> Result<(), Error> {
        if self.placement_queries.is_some() || self.current_placement_queries.is_some() {
            return Err(Error::Provider("placement queries already bound"));
        }
        self.placement_queries = Some(context);
        Ok(())
    }

    /// Bind a cheap current-state placement factory. Definitions stay cached;
    /// a reached query reads the same LoadedState supplied to readiness now.
    pub fn bind_current_placement_queries(
        &mut self,
        queries: CurrentPlacementQueries<'a>,
    ) -> Result<(), Error> {
        if self.placement_queries.is_some() || self.current_placement_queries.is_some() {
            return Err(Error::Provider("placement queries already bound"));
        }
        self.current_placement_queries = Some(queries);
        Ok(())
    }

    /// Bind reconstructed/observed current stock command state once. This
    /// establishes consistency only; actual startup/code authority is external.
    pub fn bind_command_queries(
        &mut self,
        inputs: hold_cycling::commands::Inputs<'a>,
    ) -> Result<(), Error> {
        if self.command_queries.is_some() {
            return Err(Error::Provider("command queries already bound"));
        }
        if inputs.definitions.binding() != &self.definitions.binding
            || inputs.hold_definitions.binding() != &self.definitions.binding
        {
            return Err(Error::ConfigurationIdentity);
        }
        self.command_queries = Some(inputs);
        Ok(())
    }
    pub fn pending_publication_order(&self) -> &[PublicationIndex] {
        &self.publication_order
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
        self.skip_turn_handler = Some(Self::checked_handler(
            state,
            authority,
            operations,
            c6_core::unit::SKIP_TURN_HASH,
        )?);
        Ok(())
    }
    /// Admit the stock MoveTo facade used by CURRENT-front activity reporting.
    /// Its SingleLocation parameter object's map writer is inherited and empty;
    /// this is distinct from its coordinate wire writer and map reader.
    pub fn bind_move_to_handler(
        &mut self,
        state: &LoadedState,
        authority: &super::handlers::BoundHandlers<'_>,
        operations: &crate::unit_operation_manager::OperationDefinitions,
    ) -> Result<(), Error> {
        self.move_to_handler = Some(Self::checked_handler(
            state,
            authority,
            operations,
            c6_core::unit::MOVE_TO_HASH,
        )?);
        Ok(())
    }
    fn checked_handler(
        state: &LoadedState,
        authority: &super::handlers::BoundHandlers<'_>,
        operations: &crate::unit_operation_manager::OperationDefinitions,
        hash: u32,
    ) -> Result<i32, Error> {
        let registry = authority.registry().map_err(Error::HandlerObservation)?;
        authority
            .validate_execution(state, &registry, operations)
            .map_err(Error::HandlerObservation)?;
        let id = authority
            .runtime_id(hash)
            .map_err(Error::HandlerObservation)?;
        let handler = registry
            .handler(id)
            .map_err(Error::Registry)?
            .ok_or(Error::Provider("actual facade handler absent"))?;
        if handler.identity().operation_hash.0 != hash {
            return Err(Error::Provider("actual facade handler identity"));
        }
        Ok(id.0)
    }
    pub fn snapshot_observations(&self) -> &[SnapshotObservation] {
        &self.snapshots
    }
    pub fn pending_move_complete_publications(&self) -> &[MoveCompletePublication] {
        &self.move_complete_publications
    }
    /// Guarded native all-player-unit updates, then host GetUnit/Exports.FillCache
    /// (which calls the same FillVariants with false), then blocking host24.
    /// Caller qualifies actual6e7/24 and nested275 consumers before delivery.
    pub fn send_move_complete_report(
        &mut self,
        state: &LoadedState,
        report: PlacementReport,
        inputs: MoveCompleteInputs<'_>,
    ) -> Result<(), Error> {
        if state.binding() != &self.definitions.binding {
            return Err(Error::ConfigurationIdentity);
        }
        if report.event_hash != c6_core::unit_placement_transition::MOVEMENT_COMPLETED
            || report.identity.component_type
                != c6_core::formation_initialization::UNIT_COMPONENT_TYPE
        {
            return Err(Error::Provider("complete movement report identity/event"));
        }
        let PlacementPayload::Coordinate(location) = report.payload else {
            return Err(Error::Provider("complete movement coordinate shape"));
        };
        let owner = i32::from(report.identity.owner);
        // Actual27ff650 guards: validity, actual observer, then !IsAI. Native
        // IsAI2b31550 is !Configuration.Player.IsHuman (slot status3).
        if state.world().player_is_valid(owner).map_err(Error::World)? {
            let observer = inputs
                .local_observer
                .ok_or(Error::Provider("current completion local observer"))?;
            if owner == observer
                && state
                    .partial_player_controls(owner)
                    .map_err(Error::State)?
                    .slot_status
                    == 3
            {
                let order = inputs
                    .native_unit_order
                    .ok_or(Error::Provider("current completion native unit order"))?;
                let player = state.world().player(owner).map_err(Error::World)?;
                let records = player.unit_records().map_err(Error::World)?;
                // Validate membership only. Iterate the supplied native live
                // chain below, never an ID or slot sort.
                let mut current = std::collections::BTreeSet::new();
                for row in records {
                    current.insert((row.value.e_owner as i16, row.value.id));
                }
                let supplied: std::collections::BTreeSet<_> = order
                    .iter()
                    .map(|r| (r.component_id().owner, r.id))
                    .collect();
                if current != supplied
                    || order.len() != current.len()
                    || order.iter().any(|r| r.component_id().component_type != 1)
                {
                    return Err(Error::Provider(
                        "completion current unit roster differs from native order",
                    ));
                }
                for &unit in order {
                    self.observe_unit_snapshot(state, unit)?;
                }
            }
        }
        let actor = UnitReference::from_component_id(report.identity);
        // Host GetUnit/Exports.FillCache is separate from the guarded all-unit
        // cache loop and invokes the same false-mode FillVariants again.
        self.observe_unit_snapshot(state, actor)?;
        self.publication_order.push(PublicationIndex::MoveComplete(
            self.move_complete_publications.len(),
        ));
        self.move_complete_publications
            .push(MoveCompletePublication {
                identity: report.identity,
                location,
                blocking: true,
            });
        Ok(())
    }
    pub fn pending_notification_activated_publications(
        &self,
    ) -> &[NotificationActivatedPublication] {
        &self.notification_activated_publications
    }
    /// Prepare the original Activated identity/bool payload after the lifecycle
    /// write. Caller must qualify this event's CURRENT cache receiver list,
    /// translator and synchronous host257 prepublish delegate first. Unknown
    /// later playback is retained as pending output, not treated as inert.
    pub fn send_notification_activated_report(
        &mut self,
        state: &LoadedState,
        object: c6_core::player_notifications_owner::ObjectId,
        identity: c6_core::notifications::ComponentId,
        argument: bool,
    ) -> Result<(), Error> {
        if state.binding() != &self.definitions.binding {
            return Err(Error::ConfigurationIdentity);
        }
        let record = state
            .player_notifications()
            .map_err(Error::State)?
            .record_by_object(object)
            .map_err(|_| Error::Provider("current activated notification occurrence"))?;
        if record.id != identity || !record.has_been_activated {
            return Err(Error::Provider(
                "current activated notification identity/write",
            ));
        }
        self.publication_order
            .push(PublicationIndex::NotificationActivated(
                self.notification_activated_publications.len(),
            ));
        self.notification_activated_publications
            .push(NotificationActivatedPublication {
                host_event: 257,
                event_data_type: 0x6447c33d,
                identity,
                argument,
            });
        Ok(())
    }
    pub fn pending_movement_publications(&self) -> &[MovementPublication] {
        &self.movement_publications
    }
    /// Caller qualifies this event's CURRENT native receiver, translator and
    /// host prepublish/queue route. This executes the actual current snapshot
    /// for c50d66d6 only; exhausted/restored have no native cache receiver.
    pub fn send_movement_report(
        &mut self,
        state: &LoadedState,
        report: MovementReport,
    ) -> Result<(), Error> {
        if state.binding() != &self.definitions.binding {
            return Err(Error::ConfigurationIdentity);
        }
        let (publication, snapshot) = movement_publication(report)?;
        if snapshot {
            self.observe_unit_snapshot(state, UnitReference::from_component_id(report.identity))?;
        }
        self.publication_order
            .push(PublicationIndex::Movement(self.movement_publications.len()));
        self.movement_publications.push(publication);
        Ok(())
    }
    pub fn pending_moved_publications(&self) -> &[MovedPublication] {
        &self.moved_publications
    }

    /// Source-bounded UnitMoved native cache receiver followed by host event23
    /// translation. The caller must first qualify the CURRENT BE121408 receiver
    /// list, translator, nested CacheUpdated275 and host23 prepublish/queue
    /// bindings. A Scope value or an earlier event's bindings cannot do so.
    ///
    /// OnUnitUpdate27ff430 -> PublishUpdate27ff860 -> Add27ff2d0 invokes the
    /// existing CURRENT actor snapshot/query path, including readiness and the
    /// property-copy guard. The distinct host translator320ad70 follows those
    /// effects. A failed snapshot does not prepare the later host publication.
    /// Output-only reflected cache storage remains excluded by the established
    /// Unit stream source contract; reached gameplay queries are not omitted.
    pub fn send_moved_report(
        &mut self,
        state: &LoadedState,
        reference: UnitReference,
        report: PlacementReport,
    ) -> Result<(), Error> {
        if report.event_hash != 0xbe121408
            || report.identity != reference.component_id()
            || report.identity.component_type
                != c6_core::formation_initialization::UNIT_COMPONENT_TYPE
        {
            return Err(Error::Provider("UnitMoved current report identity/event"));
        }
        let PlacementPayload::Moved {
            location,
            locally_visible,
            newly_locally_visible,
            embark_event,
            parent,
        } = report.payload
        else {
            return Err(Error::Provider("UnitMoved typed argument shape"));
        };
        self.observe_unit_snapshot(state, reference)?;
        self.publication_order
            .push(PublicationIndex::Moved(self.moved_publications.len()));
        self.moved_publications.push(MovedPublication {
            host_event: 23,
            event_data_type: 0x6447c33d,
            identity: report.identity,
            location,
            locally_visible,
            newly_locally_visible,
            embark_event,
            parent,
        });
        Ok(())
    }
    /// Stock b8515a41 native consumers refresh Treasury then create the Unit
    /// cache. The ordinary no-deal, warm-route domain excludes the two dynamic
    /// deal-item/cold-route effects. Remaining Treasury/secondary Unit stream
    /// fills copy current read-only gameplay fields into presentation storage;
    /// the Unit snapshot still executes readiness/property-copy queries here.
    /// Caller must qualify both native consumers, all nested275 and host28
    /// synchronous callbacks before omitting their pure DTO publication.
    pub fn observe_initial_placement_report(&mut self,state:&LoadedState,reference:UnitReference,report:PlacementReport)->Result<(),Error>{
        if report.event_hash!=0xb8515a41 || report.identity!=reference.component_id() || !matches!(report.payload,PlacementPayload::Initial{..}){return Err(Error::Provider("initial placement report shape"));}
        let player=state.retained().players.iter().find(|p|p.header.owner==reference.player()).ok_or(Error::Provider("initial cache Treasury Player"))?;
        if !player.treasury.gold_diplomatic_deals.is_empty(){return Err(Error::Provider("initial cache Treasury deal-item queries"));}
        yield_monitor::warm_route_domain(state).map_err(|_|Error::Provider("initial cache Treasury cold route effects"))?;
        self.observe_unit_snapshot(state,reference).map(|_|())
    }
    ///9deb82e7 routes through the ordinary Unit update queries; the host22
    /// translator copies identity/coordinate. Current callback qualification is
    /// the caller's responsibility, as for the other report entry points.
    pub fn observe_teleported_report(&mut self,state:&LoadedState,reference:UnitReference,report:PlacementReport)->Result<(),Error>{
        if report.event_hash!=0x9deb82e7 || report.identity!=reference.component_id() || !matches!(report.payload,PlacementPayload::Coordinate(_)){return Err(Error::Provider("teleported report shape"));}
        self.observe_unit_snapshot(state,reference).map(|_|())
    }
    /// Stock host36 copies the visible state, initial-placement flag and Unit
    /// identity. In the caller-qualified absent-native-consumer domain it has
    /// no gameplay query or RNG effect; queued presentation DTOs are omitted.
    pub fn observe_visibility_report(&mut self,state:&LoadedState,report:PlacementReport)->Result<(),Error>{
        if state.binding()!=&self.definitions.binding{return Err(Error::ConfigurationIdentity);}
        if report.event_hash!=0xc1300ee7 || report.identity.component_type!=1 || !matches!(report.payload,PlacementPayload::Visibility{..}){return Err(Error::Provider("visibility report shape"));}
        Ok(())
    }
    pub fn pending_ability_publications(&self)->&[AbilityPublication]{&self.ability_publications}
    /// Native a96df790 consumers: Unit.OnUnitAbilityChanged27ff830 first
    /// executes current readiness/property-copy queries; Ability.PublishUpdate
    /// then reads positive native counts. Host320ca30 copies identity/index into event65.
    /// Caller qualifies the actual ordered consumers, nested275 and host65
    /// synchronous/prepublish bindings before invoking this method.
    pub fn send_ability_gained_report(&mut self,state:&LoadedState,reference:UnitReference,index:i32)->Result<(),Error>{
        self.observe_unit_snapshot(state,reference)?;
        let runtime=state.unit_runtime(reference.player(),reference.id).map_err(Error::State)?;
        let index_usize=usize::try_from(index).map_err(|_|Error::Provider("ability report native index"))?;
        if index_usize>=runtime.abilities.len(){return Err(Error::Provider("ability report definition extent"));}
        let active_abilities=runtime.abilities.iter().enumerate().filter(|(_,count)|**count>0).map(|(i,_)|i as i32).collect();
        self.publication_order.push(PublicationIndex::Ability(self.ability_publications.len()));
        self.ability_publications.push(AbilityPublication{host_event:65,identity:reference.component_id(),ability_index:index,active_abilities});
        Ok(())
    }
    /// Stock OnFormationUpdateEvent27ff480: validate the current Formation CID,
    /// capture its ordered member span and PublishUpdate for each member. This
    /// uses the same current query path as Unit updates. The caller qualifies
    /// the event's receivers and CacheUpdated consumers; no host event is added.
    pub fn observe_formation_snapshot(
        &mut self,
        state: &LoadedState,
        identity: c6_core::unit::ComponentId,
    ) -> Result<(), Error> {
        if identity.owner == -1 || identity.id == u32::MAX || identity.component_type != 0x2c {
            return Ok(());
        }
        let owner = i32::from(identity.owner);
        if !state.world().player_is_valid(owner).map_err(Error::World)? {
            return Ok(());
        }
        let Some(formation) = state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .formation(identity.id)
            .map_err(Error::World)?
        else {
            return Ok(());
        };
        for &id in &formation.record.unit_ids {
            let reference = UnitReference::from_component_id(c6_core::unit::ComponentId {
                owner: identity.owner,
                component_type: 1,
                id,
            });
            if id == u32::MAX
                || state
                    .world()
                    .unit(reference)
                    .map_err(Error::World)?
                    .is_none()
            {
                continue;
            }
            self.observe_unit_snapshot(
                state,
                UnitReference::from_component_id(c6_core::unit::ComponentId {
                    owner: identity.owner,
                    component_type: 1,
                    id,
                }),
            )?;
        }
        Ok(())
    }
    /// Execute the existing source-bounded Unit snapshot queries on the actual
    /// retained actor. Event bindings and omitted DTO/cache/output stores must
    /// be qualified by the caller; this does not invent a new event mapping.
    pub fn observe_unit_snapshot(
        &mut self,
        state: &LoadedState,
        reference: UnitReference,
    ) -> Result<SnapshotObservation, Error> {
        let unit = state
            .world()
            .unit(reference)
            .map_err(Error::World)?
            .ok_or(Error::MissingUnit)?
            .record;
        self.validate_readiness_unit(state, unit)?;
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        if runtime.archive.identity() != reference.component_id() {
            return Err(Error::Provider("snapshot actual Unit identity"));
        }
        if !unit.k_properties.entries.is_empty() {
            return Err(Error::Provider(
                "Unit snapshot nonempty/custom property copy effects",
            ));
        }
        let observation = SnapshotObservation {
            identity: reference.component_id(),
            current_activity: unit.e_activity,
            queue_nodes: unit.k_operations.nodes.len(),
            remaining_q8: unit.x_moves_remaining,
            path: self.check_snapshot_query_effects(state, unit, None)?,
        };
        self.snapshots.push(observation);
        Ok(observation)
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
        self.send_actor_report(state, reference, None, report)
    }
    /// Read the staged actor/Formation at the same synchronous query boundary.
    /// Only the source state-phase record fields may differ from World. This
    /// provider changes its local publications/observations, never live World.
    pub fn send_overlay_report(
        &mut self,
        state: &LoadedState,
        reference: UnitReference,
        unit: &Unit,
        formation: &c6_core::unit::Formation,
        report: IdentityReport,
    ) -> Result<(), Error> {
        self.send_actor_report(state, reference, Some((unit, formation)), report)
    }
    fn send_actor_report(
        &mut self,
        state: &LoadedState,
        reference: UnitReference,
        overlay: Option<(&Unit, &c6_core::unit::Formation)>,
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
            OPERATION_SEGMENT_COMPLETE
                if report.subtype == 0 && report.first.is_some() && report.second.is_some() =>
            {
                (69, true)
            }
            OPERATION_STARTED
                if (report.subtype == c6_core::unit::SKIP_TURN_HASH
                    || (report.subtype == c6_core::unit::MOVE_TO_HASH
                        && self.move_to_handler.is_some()))
                    && report.first.is_some()
                    && report.second.is_none() =>
            {
                (68, true)
            }
            ACTIVITY_CHANGED
            | OPERATION_DEACTIVATED
            | OPERATIONS_CLEARED
            | OPERATION_ADDED
            | OPERATION_STARTED
            | OPERATION_SEGMENT_COMPLETE => {
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
        let unit = match overlay {
            None => unit,
            Some((staged, formation)) => {
                if reference.component_id().component_type
                    != c6_core::formation_initialization::UNIT_COMPONENT_TYPE
                {
                    return Err(Error::Provider("report overlay Unit identity"));
                }
                let mut expected = unit.clone();
                expected.e_activity = staged.e_activity;
                expected.i_operation_timer = staged.i_operation_timer;
                expected.k_operations = staged.k_operations.clone();
                if expected != *staged {
                    return Err(Error::Provider(
                        "report overlay changed non-phase Unit fields",
                    ));
                }
                let actual = state
                    .world()
                    .player(staged.e_owner)
                    .map_err(Error::World)?
                    .formation(staged.formation_id)
                    .map_err(Error::World)?
                    .ok_or(Error::Provider("report overlay Formation absent"))?
                    .record;
                let mut expected = actual.clone();
                expected.last_path = formation.last_path.clone();
                expected.last_path_cache_location = formation.last_path_cache_location;
                if expected != *formation || formation.unit_ids != [staged.id] {
                    return Err(Error::Provider(
                        "report overlay changed non-phase Formation fields",
                    ));
                }
                staged
            }
        };
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
            let path = self.check_snapshot_query_effects(state, unit, overlay.map(|(_, f)| f))?;
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
                Some(node)
                    if node.operation_hash == c6_core::unit::MOVE_TO_HASH
                        && matches!(
                            node.parameters,
                            c6_core::unit::OperationParameters::MoveTo { .. }
                                | c6_core::unit::OperationParameters::None
                        ) =>
                {
                    self.move_to_handler.ok_or(Error::Provider(
                        "actual MoveTo facade handler binding unavailable",
                    ))?;
                    // Exports::Unit.GetOperationType calls BaseHandler +10.
                    // GetOperationParams calls SingleLocation +28, inherited
                    // Instance::Write(map) RET; constructor parameters stay -1.
                    c6_core::unit::MOVE_TO_HASH
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
            69 => segment_complete_payload(report)?,
            70 | 71 => (report.first.unwrap_or(0), report.second.unwrap_or(0)),
            _ => (activity_operation.unwrap_or(0) as i32, -1),
        };
        self.publication_order
            .push(PublicationIndex::Identity(self.publications.len()));
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
        formation: Option<&c6_core::unit::Formation>,
    ) -> Result<ReadinessPath, Error> {
        if let Some(path) = self.member_readiness_path(state, unit, formation)? {
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
        self.readiness_result(state, unit, None)
    }

    fn readiness_result(
        &self,
        state: &LoadedState,
        unit: &Unit,
        formation: Option<&c6_core::unit::Formation>,
    ) -> Result<bool, Error> {
        Ok(!matches!(
            self.member_readiness_path(state, unit, formation)?,
            Some(ReadinessPath::MemberNotReady | ReadinessPath::OtherFormationMemberNotReady)
        ))
    }

    /// Independent native CanCyclePast. Pending operations do not short-circuit
    /// this getter. Reached HoldCycling callbacks remain genuine dependencies.
    pub fn can_cycle_past(&self, state: &LoadedState, unit: &Unit) -> Result<bool, Error> {
        self.validate_readiness_unit(state, unit)?;
        self.cycle_past_result(state, unit, None)
    }

    pub(crate) fn overlay_ready_to_move(
        &self,
        overlay: &ActorOverlay<'_>,
        unit: &Unit,
    ) -> Result<bool, Error> {
        if self.definitions.binding != *overlay.state.binding() {
            return Err(Error::ConfigurationIdentity);
        }
        let unit = overlay.unit(unit);
        if !std::ptr::eq(unit, &overlay.after.unit) {
            self.validate_readiness_unit(overlay.state, unit)?;
        }
        self.readiness_result(overlay.state, unit, overlay.actor_formation(unit))
    }

    pub(crate) fn overlay_can_cycle_past(
        &self,
        overlay: &ActorOverlay<'_>,
        unit: &Unit,
    ) -> Result<bool, Error> {
        if self.definitions.binding != *overlay.state.binding() {
            return Err(Error::ConfigurationIdentity);
        }
        let unit = overlay.unit(unit);
        if !std::ptr::eq(unit, &overlay.after.unit) {
            self.validate_readiness_unit(overlay.state, unit)?;
        }
        self.cycle_past_result(overlay.state, unit, overlay.actor_formation(unit))
    }

    fn cycle_past_result(
        &self,
        state: &LoadedState,
        unit: &Unit,
        formation: Option<&c6_core::unit::Formation>,
    ) -> Result<bool, Error> {
        if let Some(result) = self.cycle_past_record_early_result(state, unit, formation)? {
            return Ok(result);
        }
        if let Some(inputs) = self.command_queries {
            let mut query = hold_cycling::commands::Computed {
                definitions: inputs.definitions,
                state,
                unit,
                phases: inputs.phases,
                operations: inputs.operations,
                registrations: inputs.registrations,
                tags: inputs.tags,
                temporary_scope: inputs.temporary_scope,
            };
            let mut budget = inputs.registrations.scan_budget();
            if inputs
                .hold_definitions
                .command_hold_cycling(&mut query, &mut budget)
                .map_err(|e| Error::CommandHoldCycling(Box::new(e)))?
            {
                return Ok(false);
            }
            return Err(Error::Provider(
                "operation HoldCycling Manager/Script effects after computed command scan",
            ));
        }
        Err(Error::Provider("CanCyclePast command HoldCycling then conditional operation HoldCycling callbacks unavailable"))
    }

    /// Compute command HoldCycling, then operation HoldCycling only if needed.
    /// The continuation runs after the original early guards and can retain
    /// synchronous gameplay/script/RNG effects in current state. The current
    /// Unit borrow ends before invoking it; no stale Unit snapshot is substituted.
    pub fn can_cycle_past_with_hold(
        &self,
        state: &mut LoadedState,
        reference: UnitReference,
        hold: impl FnOnce(&mut LoadedState, UnitReference) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        let early = {
            let unit = state
                .world()
                .unit(reference)
                .map_err(Error::World)?
                .ok_or(Error::MissingUnit)?
                .record;
            self.cycle_past_early_result(state, unit)?
        };
        if let Some(result) = early {
            return Ok(result);
        }
        Ok(!hold(state, reference)?)
    }

    /// Compute the command fallback with the stock command provider. Only a
    /// false command result acquires the separate operation provider; it may
    /// execute the existing operation Manager/scripts against mutable state.
    pub fn can_cycle_past_with_commands(
        &self,
        state: &mut LoadedState,
        reference: UnitReference,
        inputs: hold_cycling::commands::Inputs<'_>,
        budget: &mut usize,
        operation_hold: impl FnOnce(&mut LoadedState, UnitReference, &mut usize) -> Result<bool, Error>,
    ) -> Result<bool, hold_cycling::commands::CycleError> {
        use hold_cycling::commands::{Computed, CycleError};
        let commands = {
            let unit = state
                .world()
                .unit(reference)
                .map_err(|e| CycleError::Reporting(Error::World(e)))?
                .ok_or(CycleError::Reporting(Error::MissingUnit))?
                .record;
            if let Some(result) = self
                .cycle_past_early_result(state, unit)
                .map_err(CycleError::Reporting)?
            {
                return Ok(result);
            }
            if inputs.hold_definitions.binding() != state.binding() {
                return Err(CycleError::Binding);
            }
            let mut query = Computed {
                definitions: inputs.definitions,
                state,
                unit,
                phases: inputs.phases,
                operations: inputs.operations,
                registrations: inputs.registrations,
                tags: inputs.tags,
                temporary_scope: inputs.temporary_scope,
            };
            inputs
                .hold_definitions
                .command_hold_cycling(&mut query, budget)
                .map_err(CycleError::Command)?
        };
        if commands {
            return Ok(false);
        }
        Ok(!operation_hold(state, reference, budget).map_err(CycleError::Reporting)?)
    }

    fn cycle_past_early_result(
        &self,
        state: &LoadedState,
        unit: &Unit,
    ) -> Result<Option<bool>, Error> {
        self.validate_readiness_unit(state, unit)?;
        self.cycle_past_record_early_result(state, unit, None)
    }

    fn cycle_past_record_early_result(
        &self,
        state: &LoadedState,
        unit: &Unit,
        formation: Option<&c6_core::unit::Formation>,
    ) -> Result<Option<bool>, Error> {
        if !unit_placement::coordinate_valid(unit.k_location) || unit.b_delayed_death {
            return Ok(Some(true));
        }
        if self.bound_formation(state, unit, formation)?.is_none() {
            return Ok(Some(true));
        }
        if unit.b_needs_attention || self.has_adjacent_move(state, unit)?.is_some() {
            return Ok(Some(false));
        }
        Ok(None)
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
        overlay: Option<&'s c6_core::unit::Formation>,
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
        Ok(Some(overlay.unwrap_or(formation)))
    }

    fn member_readiness_path(
        &self,
        state: &LoadedState,
        unit: &Unit,
        formation: Option<&c6_core::unit::Formation>,
    ) -> Result<Option<ReadinessPath>, Error> {
        if !member_ready(unit) {
            return Ok(Some(ReadinessPath::MemberNotReady));
        }
        let Some(formation) = self.bound_formation(state, unit, formation)? else {
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
        if let Some(queries) = self.current_placement_queries {
            return queries
                .queries
                .placement_context(state, queries.definitions)
                .and_then(|context| {
                    context.unit_record_placement(
                        state,
                        unit,
                        coord,
                        c6_core::movement_shore::PlacementRequest::shore_query(false),
                    )
                })
                .map(|result| result == 1)
                .map_err(Error::MovementPlacement);
        }
        if let Some(queries) = self.placement_queries {
            return queries
                .unit_record_placement(
                    state,
                    unit,
                    coord,
                    c6_core::movement_shore::PlacementRequest::shore_query(false),
                )
                .map(|result| result == 1)
                .map_err(Error::MovementPlacement);
        }
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
        // The original scan uses only the subject's layer. Reuse its ordered
        // candidate filtering instead of treating occupancy in any layer as an
        // unavailable query. Foreign/visibility/strength dependencies stay typed.
        if crate::movement_placement_state::best_defender(state, unit, coord)
            .map_err(Error::MovementPlacement)?
            .is_some()
        {
            return Err(Error::Provider(
                "adjacent placement selected defender continuation unavailable",
            ));
        }
        // The native clan/bribe stage belongs only to the valid foreign-owned,
        // non-spy/non-trader territory branch. That branch is explicitly
        // unsupported above. Own/unowned destinations reach no civilization,
        // tribe, clan or bribe query, even with a nonnegative stored tribe ID.
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

#[cfg(test)]
mod movement_reporting_tests {
    use super::*;
    fn report(event_hash: u32, raw_q8: Option<i32>) -> MovementReport {
        MovementReport {
            event_hash,
            identity: ComponentId {
                owner: 2,
                component_type: 1,
                id: 123,
            },
            raw_q8,
        }
    }
    #[test]
    fn scalar_wire_roundtrip_has_native_signed_fraction_and_rounding() {
        for (raw, expected) in [
            (0, 0),
            (257, 257),
            (-1, -1),
            (16_777_217, 16_777_216),
            (i32::MAX, i32::MIN),
        ] {
            let (p, snapshot) =
                movement_publication(report(MOVES_REMAINING_CHANGED, Some(raw))).unwrap();
            assert!(snapshot);
            assert_eq!(p.host_raw_q8, Some(expected));
        }
        let (zero, _) = movement_publication(report(MOVES_REMAINING_CHANGED, Some(0))).unwrap();
        assert_eq!(zero.wire_float_bits, Some(0));
    }
    #[test]
    fn identity_exhausted_and_restored_skip_snapshot_and_reject_wrong_shapes() {
        let (cleared, snapshot) = movement_publication(report(MOVES_EXHAUSTED, None)).unwrap();
        assert_eq!(cleared.host_event, 26);
        assert!(!snapshot);
        assert_eq!(cleared.host_raw_q8, None);
        let (restored, snapshot) =
            movement_publication(report(MOVEMENT_RESTORED, Some(768))).unwrap();
        assert_eq!(restored.host_event, 27);
        assert!(!snapshot);
        assert_eq!(restored.host_raw_q8, Some(768));
        assert!(movement_publication(report(MOVES_EXHAUSTED, Some(0))).is_err());
        assert!(movement_publication(report(MOVES_REMAINING_CHANGED, None)).is_err());
        assert!(movement_publication(report(0, None)).is_err());
    }
}

#[cfg(test)]
mod segment_complete_tests {
    use super::*;
    #[test]
    fn preserves_both_host_words_and_requires_actual_argument_shape() {
        let report = IdentityReport {
            event: OPERATION_SEGMENT_COMPLETE,
            identity: ComponentId {
                owner: 0,
                component_type: 1,
                id: 14155776,
            },
            subtype: 0,
            first: Some(0x8374d954u32 as i32),
            second: Some(0),
        };
        assert_eq!(segment_complete_payload(report).unwrap(), (-2089494188, 0));
        assert_eq!(
            segment_complete_payload(IdentityReport {
                first: Some(i32::MIN),
                second: Some(i32::MAX),
                ..report
            })
            .unwrap(),
            (i32::MIN, i32::MAX)
        );
        for bad in [
            IdentityReport {
                first: None,
                ..report
            },
            IdentityReport {
                second: None,
                ..report
            },
            IdentityReport {
                subtype: 1,
                ..report
            },
            IdentityReport {
                event: OPERATION_STARTED,
                ..report
            },
        ] {
            assert!(segment_complete_payload(bad).is_err());
        }
    }
}
