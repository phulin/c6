//! Diagnostic loaded-world SkipTurn composition, never full action acceptance.
//! Native Execute/TurnUnready, unrestricted script/report effects and full-world
//! parity remain open. Only the addressed Unit is copied for legality borrowing;
//! queue, activity, timer and Formation writes below change the loaded records.
use crate::unit_operation_manager::{
    OperationDefinitions, PhaseDefinitions, RuntimeOperationId, WorldPlayer,
};
use c6_core::{
    loaded_state::{Error as StateError, LoadedState},
    map::UnitReference,
    messages::{HandlerId, OperationHash},
    skip_turn::{
        self,
        legality::{self, ManagerOperation, ManagerQueries, ManagerQueryError, ManagerQueryState},
        placement,
    },
    unit::{ComponentId, OperationNode, OperationParameters, Unit, MOVE_TO_HASH, SKIP_TURN_HASH},
    unit_operation_aftermath as aftermath,
    unit_operation_interruption::{Handler, ObservedRegistry},
};

pub mod arena;
pub mod choose_tech;
pub mod city_range_attack;
pub mod command_units;
pub mod configuration;
pub mod culture_notifications;
pub mod handlers;
pub mod notifications;
pub mod reporting;

const OPERATION: i32 = 0x490cc8e1;
const AWAKE: i32 = 0x6b25012e;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    ExecuteLookup,
    Preflight,
    Clear,
    Insert,
    Activation,
    Continue,
    Start,
    Delete,
    Diplomacy,
    Notifications,
    DiagnosticComplete,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityReport {
    pub event: u32,
    pub subtype: u32,
    pub identity: ComponentId,
    pub first: Option<i32>,
    pub second: Option<i32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    Stage(Stage),
    Legality(bool),
    Report(IdentityReport),
    UnitAbsent { owner: i32, id: u32 },
    AddHandlerAbsent(i32),
}
/// Stable report rows share the existing kernel/native differential shape.
/// Stage and legality rows retain diagnostic boundaries; reporting rows alone
/// must never be interpreted as delivered-subscriber or whole-action parity.
pub fn trace_json(trace: &[Event]) -> serde_json::Value {
    serde_json::Value::Array(trace.iter().map(|event|match event {
        Event::Stage(stage)=>serde_json::json!({"kind":"stage","stage":format!("{stage:?}")}),
        Event::Legality(allowed)=>serde_json::json!({"kind":"legality","allowed":allowed}),
        Event::UnitAbsent {owner,id}=>serde_json::json!({"kind":"execute_unit_absent","owner":owner,"id":id}),
        Event::AddHandlerAbsent(id)=>serde_json::json!({"kind":"add_handler_absent","runtime_id":id}),
        Event::Report(r)=>{
            let component=(u64::from(r.identity.id)<<32)|(u64::from(r.identity.component_type as u16)<<16)|u64::from(r.identity.owner as u16);
            let ty=(u64::from(r.subtype)<<32)|u64::from(r.event);
            serde_json::json!({"kind":"report","type_bits":format!("{ty:016x}"),"component_bits":format!("{component:016x}"),"first":r.first,"second":r.second})
        }
    }).collect())
}
#[derive(Debug)]
pub enum Error {
    State(StateError),
    World(c6_core::world::MissingInput),
    Manager(crate::unit_operation_manager::Error),
    Placement(placement::Error),
    Registry(c6_core::unit_operation_interruption::Error),
    HandlerObservation(handlers::Error),
    Kernel(skip_turn::Error),
    Movement(crate::unit_movement_state::Error),
    Provider(&'static str),
    Aftermath(aftermath::Error),
    MissingUnit,
    ActorIdentityChanged(Box<Unit>),
    ConfigurationIdentity,
    Unsupported(&'static str),
}
#[derive(Debug)]
pub struct Failure {
    pub error: Error,
    pub trace: Vec<Event>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loaded SkipTurn diagnostic: {self:?}")
    }
}
impl std::error::Error for Error {}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({} completed trace entries)",
            self.error,
            self.trace.len()
        )
    }
}
impl std::error::Error for Failure {}

/// Facts must describe this loaded context. Definition collection order and
/// observed handler identities are independent authorities, never SQL ordinals.
pub enum TransportScope {
    /// Conditional stock Unit/Formation profile: no native resync or additional
    /// dirty-list gameplay consumer. This does not exclude gameplay reports.
    HeadlessLocalAuthority,
}
pub struct Inputs<'a, 'd> {
    pub transport: TransportScope,
    pub operations: &'a OperationDefinitions,
    pub phases: &'a PhaseDefinitions,
    pub handlers: &'a ObservedRegistry<'a>,
    pub placement_definitions:
        Option<&'a c6_core::movement_world::DefinitionTable<'d, Option<placement::UnitProperties>>>,
    pub relations: Option<&'a placement::WarRelations<'a>>,
    pub plot_unit_limit: Option<i32>,
    pub movement: Option<&'a crate::unit_movement_state::Inputs<'a>>,
}

/// A scoped diagnostic reporting boundary. This is deliberately narrower than
/// the eventual full gameplay callback context: the World is readable, and only
/// the addressed actor is mutable. Broader World/RNG effects remain unsupported.
mod report_context {
    use super::*;
    pub struct ReportingContext<'a> {
        state: &'a mut LoadedState,
        reference: UnitReference,
    }
    impl<'a> ReportingContext<'a> {
        pub(super) fn new(state: &'a mut LoadedState, reference: UnitReference) -> Self {
            Self { state, reference }
        }
        pub fn state(&self) -> &LoadedState {
            self.state
        }
        pub fn reference(&self) -> UnitReference {
            self.reference
        }
        pub fn actor_mut(&mut self) -> Result<&mut Unit, Error> {
            unit(self.state, self.reference)
        }
    }
}
pub use report_context::ReportingContext;
/// Required synchronous identity-report delivery. The supported-write guard
/// checks the addressed Unit only: activity changes may continue; other actor
/// changes are retained and return a partial error. The scoped context prevents
/// arbitrary safe providers from acquiring other mutable World components.
/// Real callbacks requiring broader gameplay/RNG writes need a future versioned
/// mutation context and must report unsupported here, never omit those effects.
/// Collecting reports alone is diagnostic evidence, not subscriber closure.
pub trait Reports {
    fn send(&mut self, unit: &mut Unit, report: IdentityReport) -> Result<i32, Error>;
    fn send_loaded(
        &mut self,
        context: &mut ReportingContext<'_>,
        report: IdentityReport,
    ) -> Result<i32, Error> {
        self.send(context.actor_mut()?, report)
    }
}

#[derive(Clone, Copy)]
enum HandlerAuthority<'a, 'capture> {
    DecoderBinding,
    RuntimeBinding(&'a handlers::BoundHandlers<'capture>),
}
struct Queries<'a, 'd, 'q, 'h, 'capture, Q> {
    state: &'a LoadedState,
    inputs: &'a Inputs<'a, 'd>,
    external: &'q mut Q,
    authority: HandlerAuthority<'h, 'capture>,
}
type QueryState<'a> = ManagerQueryState<'a, Unit, ()>;
impl<Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>> ManagerQueries<Unit>
    for Queries<'_, '_, '_, '_, '_, Q>
{
    type Error = Error;
    type Parameters = ();
    type Player = i32;
    type Game = ();
    type Script = Q::Script;
    type EngineUtility = Q::EngineUtility;
    type Args = Q::Args;
    type Handler = Handler;
    type Plot = usize;
    type Results = ();
    fn allow_tactical_commands(
        &mut self,
        _: &mut QueryState<'_>,
        owner: i32,
    ) -> Result<bool, Error> {
        self.inputs
            .phases
            .from_processor(&self.state.retained().core.phase_processor, owner)
            .map_err(Error::Manager)
    }
    fn player(&mut self, _: &mut QueryState<'_>, owner: i32) -> Result<i32, Error> {
        Ok(WorldPlayer::resolve(self.state.world(), owner, None)
            .map_err(Error::Manager)?
            .id())
    }
    fn turn_active(&mut self, _: &mut QueryState<'_>, player: i32) -> Result<bool, Error> {
        self.state.turn_active(player).map_err(Error::State)
    }
    fn processing_automation(&mut self, _: &mut QueryState<'_>) -> Result<i32, Error> {
        self.state.processing_automation().map_err(Error::State)
    }
    fn player_id(&mut self, _: &mut QueryState<'_>, player: i32) -> Result<i32, Error> {
        Ok(player)
    }
    fn game(&mut self, _: &mut QueryState<'_>) -> Result<(), Error> {
        Ok(())
    }
    fn player_human(&mut self, _: &mut QueryState<'_>, player: i32) -> Result<bool, Error> {
        WorldPlayer::resolve(self.state.world(), player, None)
            .map_err(Error::Manager)?
            .is_human()
            .map_err(Error::Manager)
    }
    fn paused(&mut self, _: &mut QueryState<'_>, _: ()) -> Result<bool, Error> {
        Ok(self.state.pause_storage().map_err(Error::State)? != 0)
    }
    fn runtime_id(&mut self, _: &mut QueryState<'_>, hash: u32) -> Result<i32, Error> {
        match self.authority {
            HandlerAuthority::DecoderBinding => self
                .inputs
                .handlers
                .saved_hash_runtime_id(OperationHash(hash))
                .map(|id| id.0)
                .map_err(Error::Registry),
            HandlerAuthority::RuntimeBinding(source) => source
                .runtime_id(hash)
                .map(|id| id.0)
                .map_err(Error::HandlerObservation),
        }
    }
    fn handler(&mut self, _: &mut QueryState<'_>, id: i32) -> Result<Option<Handler>, Error> {
        self.inputs
            .handlers
            .handler(HandlerId(id))
            .map_err(Error::Registry)
    }
    fn can_be_interrupted(
        &mut self,
        _: &mut QueryState<'_>,
        handler: Handler,
    ) -> Result<bool, Error> {
        Ok(handler.can_be_interrupted())
    }
    fn operation_disabled(&mut self, s: &mut QueryState<'_>, id: i32) -> Result<bool, Error> {
        let runtime = self
            .state
            .unit_runtime(s.unit.e_owner, s.unit.id)
            .map_err(Error::State)?;
        if runtime.operations_disabled.len() != self.inputs.operations.count() {
            return Err(Error::Unsupported(
                "live disabled-operation definition count",
            ));
        }
        Ok(self
            .inputs
            .operations
            .find_index(id as u32)
            .is_some_and(|RuntimeOperationId(i)| runtime.operations_disabled[i as usize]))
    }
    fn script_system(&mut self, s: &mut QueryState<'_>) -> Result<Option<Q::Script>, Error> {
        self.external.script_system(s).map_err(Error::Provider)
    }
    fn args_begin(&mut self, s: &mut QueryState<'_>) -> Result<Q::Args, Error> {
        self.external.args_begin(s).map_err(Error::Provider)
    }
    fn args_push_int(
        &mut self,
        s: &mut QueryState<'_>,
        a: &mut Q::Args,
        v: i32,
    ) -> Result<(), Error> {
        self.external
            .args_push_int(s, a, v)
            .map_err(Error::Provider)
    }
    fn engine_utility(&mut self, s: &mut QueryState<'_>) -> Result<Q::EngineUtility, Error> {
        self.external.engine_utility(s).map_err(Error::Provider)
    }
    fn has_game_core_lock(
        &mut self,
        s: &mut QueryState<'_>,
        u: Q::EngineUtility,
    ) -> Result<bool, Error> {
        self.external
            .has_game_core_lock(s, u)
            .map_err(Error::Provider)
    }
    fn release_game_core_lock(
        &mut self,
        s: &mut QueryState<'_>,
        u: Q::EngineUtility,
    ) -> Result<(), Error> {
        self.external
            .release_game_core_lock(s, u)
            .map_err(Error::Provider)
    }
    fn get_game_core_lock(
        &mut self,
        s: &mut QueryState<'_>,
        u: Q::EngineUtility,
    ) -> Result<(), Error> {
        self.external
            .get_game_core_lock(s, u)
            .map_err(Error::Provider)
    }
    fn script_call_test_all(
        &mut self,
        s: &mut QueryState<'_>,
        script: Q::Script,
        a: &mut Q::Args,
        event: &'static str,
        initial: bool,
    ) -> Result<legality::ScriptTestAllResult, Error> {
        self.external
            .script_call_test_all(s, script, a, event, initial)
            .map_err(Error::Provider)
    }
    fn args_end(&mut self, s: &mut QueryState<'_>, a: Q::Args) -> Result<(), Error> {
        self.external.args_end(s, a).map_err(Error::Provider)
    }
    fn unit_plot(&mut self, s: &mut QueryState<'_>) -> Result<Option<usize>, Error> {
        Ok(
            c6_core::movement_world::plot_at(self.state.world().map(), s.unit.k_location)
                .ok()
                .map(|(index, _)| index),
        )
    }
    fn handler_can_start(
        &mut self,
        s: &mut QueryState<'_>,
        h: Handler,
        _: usize,
        _: bool,
        _: Option<&mut ()>,
    ) -> Result<bool, Error> {
        if self
            .state
            .world()
            .player(s.unit.e_owner)
            .map_err(Error::World)?
            .unit(s.unit.id)
            .map_err(Error::World)?
            .is_none_or(|u| u.record != s.unit)
        {
            return Err(Error::Unsupported(
                "script changed Unit before world-backed placement; effects retained",
            ));
        }
        if h.identity().operation_hash != OperationHash(SKIP_TURN_HASH) {
            return Err(Error::Unsupported("requested handler is not SkipTurn"));
        }
        // An installed owned component is the only live authority. Never use
        // an older independently mutable captured vector after this handoff.
        let owned_relations = if self.state.game_relations_source().is_some() {
            let owner = self.state.game_relations().map_err(Error::State)?;
            let war = owner
                .war()
                .ok_or(Error::Unsupported("owned Game war types not initialized"))?;
            Some(placement::WarRelations {
                player_count: owner.player_count(),
                war_types: war.values(),
            })
        } else {
            None
        };
        let mut q = placement::Inputs {
            world: self.state.world(),
            definitions: self.inputs.placement_definitions,
            relations: owned_relations.as_ref().or(self.inputs.relations),
            plot_unit_limit: self.inputs.plot_unit_limit,
        };
        legality::skip_turn_can_start(s.unit, &mut q).map_err(Error::Placement)
    }
}

fn can_start<Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>>(
    state: &mut LoadedState,
    reference: UnitReference,
    inputs: &Inputs<'_, '_>,
    external: &mut Q,
    id: i32,
    authority: HandlerAuthority<'_, '_>,
) -> Result<bool, Error> {
    let mut unit = state
        .world()
        .unit(reference)
        .map_err(Error::World)?
        .ok_or(Error::MissingUnit)?
        .record
        .clone();
    let identity = (unit.e_owner, unit.id);
    let mut operation = ManagerOperation {
        runtime_id: id,
        directive: -1,
        parameters: (),
        flags: 0,
    };
    let answer = legality::manager_can_start(
        &mut unit,
        &mut operation,
        None,
        false,
        None,
        &mut Queries {
            state,
            inputs,
            external,
            authority,
        },
    );
    if (unit.e_owner, unit.id) != identity {
        return Err(Error::ActorIdentityChanged(Box::new(unit)));
    }
    *state
        .unit_mut(i32::from(reference.component_id().owner), reference.id)
        .map_err(Error::State)? = unit;
    if operation.runtime_id != id || operation.directive != -1 || operation.flags != 0 {
        return Err(Error::Unsupported(
            "script changed request outside diagnostic profile; Unit effects retained",
        ));
    }
    answer.map_err(|e| match e {
        ManagerQueryError::Provider(e) => e,
        ManagerQueryError::MissingRequestedHandler(_) => {
            Error::Unsupported("missing requested handler")
        }
    })
}

fn unit_ref(state: &LoadedState, r: UnitReference) -> Result<&Unit, Error> {
    Ok(state
        .world()
        .unit(r)
        .map_err(Error::World)?
        .ok_or(Error::MissingUnit)?
        .record)
}
fn unit(state: &mut LoadedState, r: UnitReference) -> Result<&mut Unit, Error> {
    state
        .unit_mut(i32::from(r.component_id().owner), r.id)
        .map_err(Error::State)
}
#[cfg(test)]
fn deliver_report(
    unit: &mut Unit,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
    report: IdentityReport,
) -> Result<i32, Error> {
    trace.push(Event::Report(report));
    let mut expected = unit.clone();
    let value = reports.send(unit, report)?;
    // This diagnostic kernel proves synchronous activity rereads. Broader
    // callback writes remain in the real record, but cannot silently continue
    // through a kernel whose node/formation receivers have narrower coverage.
    expected.e_activity = unit.e_activity;
    if unit != &expected {
        return Err(Error::Unsupported(
            "report changed Unit outside activity; effects retained",
        ));
    }
    Ok(value)
}
#[cfg(test)]
fn report_unit(
    unit: &mut Unit,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
    report: IdentityReport,
) -> Result<(), Error> {
    deliver_report(unit, reports, trace, report).map(|_| ())
}

fn report(
    state: &mut LoadedState,
    r: UnitReference,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
    report: IdentityReport,
) -> Result<(), Error> {
    let mut expected = unit_ref(state, r)?.clone();
    trace.push(Event::Report(report));
    reports.send_loaded(&mut ReportingContext::new(state, r), report)?;
    let actual = unit_ref(state, r)?;
    expected.e_activity = actual.e_activity;
    if actual != &expected {
        return Err(Error::Unsupported(
            "report changed Unit outside activity; effects retained",
        ));
    }
    Ok(())
}

#[cfg(test)]
struct StartReports<'a, R> {
    reports: &'a mut R,
    trace: &'a mut Vec<Event>,
    error: Option<Error>,
}
#[cfg(test)]
impl<R: Reports> skip_turn::StartEffects for StartReports<'_, R> {
    fn mark_activity_dirty(&mut self, _: &Unit, _: u32) {
        unreachable!("diagnostic local-authority profile omits transport dirty registration")
    }
    fn send(&mut self, u: &mut Unit, r: skip_turn::IdentityIntReport) -> i32 {
        let report = IdentityReport {
            event: r.event_hash,
            subtype: r.subtype_hash,
            identity: r.identity,
            first: Some(r.value),
            second: None,
        };
        match deliver_report(u, self.reports, self.trace, report) {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(e);
                0
            }
        }
    }
}
#[cfg(test)]
fn activity_unit(
    unit: &mut Unit,
    id: ComponentId,
    value: i32,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
) -> Result<(), Error> {
    let mut e = StartReports {
        reports,
        trace,
        error: None,
    };
    skip_turn::set_activity(unit, id, value, false, &mut e).map_err(Error::Kernel)?;
    e.error.map_or(Ok(()), Err)
}
/// Split the pure setter leaf at its only reporting boundary, then deliver
/// synchronously through the authoritative World after dropping the Unit borrow.
/// The internal recorder represents no subscriber or native reporting result.
struct LoadedActivityBoundary(Option<skip_turn::IdentityIntReport>);
impl skip_turn::StartEffects for LoadedActivityBoundary {
    fn mark_activity_dirty(&mut self, _: &Unit, _: u32) {
        unreachable!("local transport registration omitted")
    }
    fn send(&mut self, _: &mut Unit, report: skip_turn::IdentityIntReport) -> i32 {
        self.0 = Some(report);
        0
    }
}
fn activity(
    state: &mut LoadedState,
    r: UnitReference,
    id: ComponentId,
    value: i32,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
) -> Result<(), Error> {
    let mut boundary = LoadedActivityBoundary(None);
    skip_turn::set_activity(unit(state, r)?, id, value, false, &mut boundary)
        .map_err(Error::Kernel)?;
    if let Some(report) = boundary.0 {
        self::report(
            state,
            r,
            reports,
            trace,
            IdentityReport {
                event: report.event_hash,
                subtype: report.subtype_hash,
                identity: report.identity,
                first: Some(report.value),
                second: None,
            },
        )?;
    }
    Ok(())
}
fn start_loaded(
    state: &mut LoadedState,
    r: UnitReference,
    identity: ComponentId,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
) -> Result<(), Error> {
    report(
        state,
        r,
        reports,
        trace,
        IdentityReport {
            event: 0x3d4006f2,
            subtype: SKIP_TURN_HASH,
            identity,
            first: Some(0),
            second: None,
        },
    )?;
    activity(state, r, identity, skip_turn::HOLD, reports, trace)
}

#[cfg(test)]
fn start_unit(
    unit: &mut Unit,
    identity: ComponentId,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
) -> Result<(), Error> {
    // Split Start at its synchronous report boundary: the existing leaf's
    // infallible hook cannot stop before SetActivityType on provider error.
    report_unit(
        unit,
        reports,
        trace,
        IdentityReport {
            event: 0x3d4006f2,
            subtype: SKIP_TURN_HASH,
            identity,
            first: Some(0),
            second: None,
        },
    )?;
    activity_unit(unit, identity, skip_turn::HOLD, reports, trace)
}

fn deactivate(
    state: &mut LoadedState,
    r: UnitReference,
    id: ComponentId,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
) -> Result<(), Error> {
    if unit_ref(state, r)?.k_operations.nodes.is_empty() {
        return Ok(());
    }
    if unit_ref(state, r)?.e_activity == OPERATION {
        activity(state, r, id, AWAKE, reports, trace)?;
    }
    unit(state, r)?.i_operation_timer = 0;
    Ok(())
}
struct NoTransport;
impl skip_turn::FormationEffects for NoTransport {
    fn mark_dirty(&mut self, _: &c6_core::unit::Formation, _: u32) {
        unreachable!()
    }
}
fn clear_path(state: &mut LoadedState, r: UnitReference) -> Result<(), Error> {
    let fid = unit_ref(state, r)?.formation_id;
    let (formation, _) = state
        .formation_with_runtime_mut(i32::from(r.component_id().owner), fid)
        .map_err(Error::State)?;
    skip_turn::clear_path_cache(formation, false, false, &mut NoTransport);
    Ok(())
}
fn pop(
    state: &mut LoadedState,
    r: UnitReference,
    id: ComponentId,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
) -> Result<(), Error> {
    let local = state.retained().core.local_player;
    let u = unit_ref(state, r)?;
    let hash = u
        .k_operations
        .nodes
        .first()
        .ok_or(Error::Unsupported("report consumer removed queue front"))?
        .operation_hash;
    let ready = u.e_owner == local
        && u.e_automation == -1
        && legality::can_move(u)
        && u.k_operations.nodes.len() <= 1
        && [OPERATION, AWAKE].contains(&u.e_activity);
    report(
        state,
        r,
        reports,
        trace,
        IdentityReport {
            identity: id,
            event: 0xa60328c5,
            subtype: 0,
            first: Some(hash as i32),
            second: Some(i32::from(ready)),
        },
    )?;
    deactivate(state, r, id, reports, trace)?;
    let nodes = &mut unit(state, r)?.k_operations.nodes;
    if nodes.is_empty() {
        return Err(Error::Unsupported("report consumer removed queue front"));
    }
    nodes.remove(0);
    if unit_ref(state, r)?.k_operations.nodes.is_empty() {
        clear_path(state, r)?;
    }
    if unit_ref(state, r)?.k_operations.nodes.is_empty() {
        report(
            state,
            r,
            reports,
            trace,
            IdentityReport {
                identity: id,
                event: 0x383669f1,
                subtype: 0,
                first: None,
                second: None,
            },
        )?;
    }
    Ok(())
}

/// Original bounded SkipTurn Manager::Continue. Actual handler +0x28 is
/// inherited Update (false/no effects); +0x30 is CheckComplete (delete=true).
/// Thus both movement branches reach deletion, without the moved/blocked report.
fn continue_skip_turn(
    state: &mut LoadedState,
    r: UnitReference,
    reports: &mut impl Reports,
    trace: &mut Vec<Event>,
) -> Result<(), Error> {
    let fid = unit_ref(state, r)?.formation_id;
    clear_path(state, r)?;
    let (formation, runtime) = state
        .formation_with_runtime_mut(r.player(), fid)
        .map_err(Error::State)?;
    runtime
        .set_evaluating_operation(formation, true)
        .map_err(|_| Error::Unsupported("Continue Formation identity"))?;
    // The original pure Unit/Formation CanMove/IsBusy reads only decide whether
    // the inherited no-effect Update is called. CheckComplete always sets delete.
    let formation_can_move = formation.moves_remaining_raw > 0;
    let _call_update = (legality::can_move(unit_ref(state, r)?) || formation_can_move)
        && !legality::is_busy(unit_ref(state, r)?);
    // Inherited TimerComplete is false/no effects. Original Unit completion has
    // a true branch with gameplay reporting; retain that explicit dependency.
    if unit_ref(state, r)?.b_needs_movement_completion {
        return Err(Error::Kernel(skip_turn::Error::MovementCompletionPending));
    }
    let (formation, runtime) = state
        .formation_with_runtime_mut(r.player(), fid)
        .map_err(Error::State)?;
    runtime
        .set_evaluating_operation(formation, false)
        .map_err(|_| Error::Unsupported("Continue Formation identity"))?;
    // Both actual movement branches delete. No Start/CanStart call runs here.
    let identity = state
        .unit_runtime(r.player(), r.id)
        .map_err(Error::State)?
        .archive
        .identity();
    trace.push(Event::Stage(Stage::Delete));
    pop(state, r, identity, reports, trace)
}

/// Execute a diagnostic Manager/Add state phase in place. Error includes the
/// completed prefix trace and DOES NOT roll back state or provider effects.
/// The external Manager provider is used only for script/lock/argument methods;
/// all player, phase, interruption, disabled, plot and handler reads above derive
/// from actual loaded state and the supplied definitions. It must reject unknown
/// scripting state, rather than returning a fabricated null ScriptSystem.
///
/// Local-authority transport dirtiness is excluded; network loadDelta/resync is
/// unsupported. Reports remain synchronous obligations, even for stock profiles.
pub fn skip_turn_state_phase<Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>>(
    state: &mut LoadedState,
    r: UnitReference,
    inputs: &Inputs<'_, '_>,
    external: &mut Q,
    reports: &mut impl Reports,
) -> Result<Vec<Event>, Failure> {
    skip_turn_state_phase_impl(
        state,
        r,
        inputs,
        external,
        reports,
        HandlerAuthority::DecoderBinding,
    )
}
/// Same diagnostic algorithm with checked runtime handler/queue inputs.
/// The historical name does not authenticate a capture: input adapters must
/// establish actual origin separately. Supplied identities are never relabeled
/// as the destination's native Read process or as hash-lookup observations.
pub fn skip_turn_state_phase_observed<
    Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>,
>(
    state: &mut LoadedState,
    r: UnitReference,
    inputs: &Inputs<'_, '_>,
    source: &handlers::BoundHandlers<'_>,
    external: &mut Q,
    reports: &mut impl Reports,
) -> Result<Vec<Event>, Failure> {
    skip_turn_state_phase_impl(
        state,
        r,
        inputs,
        external,
        reports,
        HandlerAuthority::RuntimeBinding(source),
    )
}
fn skip_turn_state_phase_impl<Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>>(
    state: &mut LoadedState,
    r: UnitReference,
    inputs: &Inputs<'_, '_>,
    external: &mut Q,
    reports: &mut impl Reports,
    authority: HandlerAuthority<'_, '_>,
) -> Result<Vec<Event>, Failure> {
    let mut trace = vec![Event::Stage(Stage::ExecuteLookup)];
    let result = (|| {
        if state.world().unit(r).map_err(Error::World)?.is_none() {
            trace.push(Event::UnitAbsent {
                owner: r.player(),
                id: r.id,
            });
            return Ok(());
        }
        trace.push(Event::Stage(Stage::Preflight));
        let id = match authority {
            HandlerAuthority::DecoderBinding => {
                let observed = inputs.handlers.identity();
                let configuration: String = state
                    .binding()
                    .inputs
                    .configuration_sha256
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                if configuration != observed.configuration_sha256 {
                    return Err(Error::ConfigurationIdentity);
                }
                inputs
                    .handlers
                    .saved_hash_runtime_id(OperationHash(SKIP_TURN_HASH))
                    .map_err(Error::Registry)?
                    .0
            }
            HandlerAuthority::RuntimeBinding(source) => {
                source
                    .validate_execution(state, inputs.handlers, inputs.operations)
                    .map_err(Error::HandlerObservation)?;
                source.requested_skip_turn().0
            }
        };
        let u = unit_ref(state, r)?;
        let runtime = state.unit_runtime(r.player(), r.id).map_err(Error::State)?;
        let identity = runtime.archive.identity();
        if i32::from(identity.owner) != u.e_owner || identity.id != u.id {
            return Err(Error::Kernel(skip_turn::Error::IdentityMismatch));
        }
        if u.e_automation != -1 {
            return Err(Error::Kernel(skip_turn::Error::UnsupportedAutomation));
        }
        if u.b_needs_movement_completion && !u.k_operations.nodes.is_empty() {
            return Err(Error::Kernel(skip_turn::Error::MovementCompletionPending));
        }
        if u.k_operations.nodes.len() > 1
            || u.k_operations.nodes.first().is_some_and(|n| {
                n.operation_hash != MOVE_TO_HASH
                    || !matches!(n.parameters, OperationParameters::MoveTo { .. })
            })
        {
            return Err(Error::Kernel(skip_turn::Error::UnsupportedQueue));
        }
        if ![OPERATION, AWAKE].contains(&u.e_activity)
            || (u.k_operations.nodes.is_empty() && u.e_activity == OPERATION)
        {
            return Err(Error::Kernel(skip_turn::Error::UnsupportedInitialActivity));
        }
        let (fid, uid, owner) = (u.formation_id, u.id, u.e_owner);
        let f = state
            .world()
            .player(r.player())
            .map_err(Error::World)?
            .formation(fid)
            .map_err(Error::World)?
            .ok_or(Error::State(StateError::Missing(
                c6_core::loaded_state::MissingInput::Formation {
                    owner: r.player(),
                    id: fid,
                },
            )))?
            .record;
        let fr = state
            .formation_runtime(r.player(), fid)
            .map_err(Error::State)?;
        if f.owner != owner || f.unit_ids != [uid] {
            return Err(Error::Kernel(skip_turn::Error::FormationMismatch));
        }
        if fr.evaluating_operation() {
            return Err(Error::Unsupported("Formation is evaluating an operation"));
        }
        trace.push(Event::Stage(Stage::Clear));
        if !unit_ref(state, r)?.k_operations.nodes.is_empty() {
            deactivate(state, r, identity, reports, &mut trace)?;
            pop(state, r, identity, reports, &mut trace)?;
            clear_path(state, r)?;
        }
        // Actual Add resolves the requested handler after Clear. A proven null
        // slot returns normally, preserving Clear effects and running aftermath.
        let Some(handler) = inputs
            .handlers
            .handler(HandlerId(id))
            .map_err(Error::Registry)?
        else {
            trace.push(Event::AddHandlerAbsent(id));
            return Ok(());
        };
        if handler.identity().operation_hash != OperationHash(SKIP_TURN_HASH) {
            return Err(Error::Unsupported(
                "requested handler is not SkipTurn after Clear; prior effects retained",
            ));
        }
        trace.push(Event::Stage(Stage::Insert));
        let turn = state.retained().core.current_turn;
        let u = unit(state, r)?;
        u.k_operations.nodes.push(OperationNode {
            operation_hash: SKIP_TURN_HASH,
            directive: -1,
            flags: 0,
            push_turn: turn,
            previous_activity: u.e_activity,
            previous_automation: u.e_automation,
            parameters: OperationParameters::None,
        });
        report(
            state,
            r,
            reports,
            &mut trace,
            IdentityReport {
                identity,
                event: 0x15c35863,
                subtype: 0,
                first: Some(id),
                second: None,
            },
        )?;
        trace.push(Event::Stage(Stage::Activation));
        // Original Activate returns before querying CanStart for a busy Unit.
        // Nonempty Clear above zeroes its timer; empty Clear preserves it.
        if legality::is_busy(unit_ref(state, r)?) {
            return Ok(());
        }
        if unit_ref(state, r)?.e_activity == OPERATION {
            trace.push(Event::Stage(Stage::Continue));
            continue_skip_turn(state, r, reports, &mut trace)?;
            return Ok(());
        }
        let admitted = can_start(state, r, inputs, external, id, authority)?;
        trace.push(Event::Legality(admitted));
        if !admitted {
            let actor = unit_ref(state, r)?;
            if actor.k_operations.nodes.len() != 1
                || actor.k_operations.nodes[0].operation_hash != SKIP_TURN_HASH
                || !matches!(
                    actor.k_operations.nodes[0].parameters,
                    OperationParameters::None
                )
            {
                return Err(Error::Unsupported(
                    "script changed activation queue outside singleton SkipTurn; effects retained",
                ));
            }
            let movement=inputs.movement.ok_or(Error::Unsupported("Activate CanStart false requires actual Unit MaxMoves/HasMoved inputs; prior effects retained"))?;
            let actor = unit_ref(state, r)?;
            let moved = crate::unit_movement_state::unit_has_moved(state, actor, movement)
                .map_err(Error::Movement)?;
            let push_turn = actor
                .k_operations
                .nodes
                .first()
                .ok_or(Error::Unsupported("activation queue front unavailable"))?
                .push_turn;
            if !moved || push_turn < state.retained().core.current_turn {
                trace.push(Event::Stage(Stage::Delete));
                pop(state, r, identity, reports, &mut trace)?;
            }
            return Ok(());
        }
        let next = if legality::can_move(unit_ref(state, r)?) {
            OPERATION
        } else {
            skip_turn::HOLD
        };
        activity(state, r, identity, next, reports, &mut trace)?;
        trace.push(Event::Stage(Stage::Start));
        start_loaded(state, r, identity, reports, &mut trace)?;
        trace.push(Event::Stage(Stage::Delete));
        pop(state, r, identity, reports, &mut trace)?;
        Ok(())
    })();
    match result {
        Ok(()) => Ok(trace),
        Err(error) => Err(Failure { error, trace }),
    }
}

/// Compose the actual state phase with the existing in-place aftermath. The
/// factory must borrow this same resulting LoadedState, preserving retained
/// receivers and all gameplay/RNG effects; no whole-world Clone is required.
/// DiagnosticComplete means this supplied provider chain returned, not accepted
/// whole-action parity. A supported early Activate return still runs aftermath;
/// an unavailable activation-false continuation remains a partial failure.
#[allow(clippy::too_many_arguments)]
pub fn diagnose_skip_turn<'s, Q, R, W, F>(
    state: &'s mut LoadedState,
    r: UnitReference,
    inputs: &Inputs<'_, '_>,
    external: &mut Q,
    reports: &mut R,
    make_aftermath: F,
    owner: W::Owner,
    budget: usize,
) -> Result<Vec<Event>, Failure>
where
    Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>,
    R: Reports,
    W: aftermath::Diplomacy + aftermath::Blocking,
    F: FnOnce(&'s mut LoadedState) -> W,
{
    diagnose_skip_turn_impl(
        state,
        r,
        inputs,
        external,
        reports,
        make_aftermath,
        owner,
        budget,
        HandlerAuthority::DecoderBinding,
    )
}
/// Execute the same post-Add composition using checked handler/code bindings
/// and source-reconstructed queue IDs. Native attribution belongs to the input
/// adapter; this public consistency boundary supplies no hash-lookup answers.
#[allow(clippy::too_many_arguments)]
pub fn diagnose_skip_turn_observed<'s, Q, R, W, F>(
    state: &'s mut LoadedState,
    r: UnitReference,
    inputs: &Inputs<'_, '_>,
    source: &handlers::BoundHandlers<'_>,
    external: &mut Q,
    reports: &mut R,
    make_aftermath: F,
    owner: W::Owner,
    budget: usize,
) -> Result<Vec<Event>, Failure>
where
    Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>,
    R: Reports,
    W: aftermath::Diplomacy + aftermath::Blocking,
    F: FnOnce(&'s mut LoadedState) -> W,
{
    diagnose_skip_turn_impl(
        state,
        r,
        inputs,
        external,
        reports,
        make_aftermath,
        owner,
        budget,
        HandlerAuthority::RuntimeBinding(source),
    )
}

/// Continue the observed-handler action through the owned Game.Diplomacy.Manager
/// before constructing the notification provider. Its admitted dynamic witness
/// is required: matching saved bytes do not imply empty live sessions. Pending
/// statements are removed from the authoritative owner before the required
/// processor runs against this same LoadedState; partial effects survive errors.
/// The notification factory must borrow that resulting state and implement all
/// reached effects. No full-action acceptance or successful backend is supplied.
#[allow(clippy::too_many_arguments)]
pub fn diagnose_skip_turn_observed_owned_aftermath<'s, Q, R, P, W, F>(
    state: &'s mut LoadedState,
    r: UnitReference,
    inputs: &Inputs<'_, '_>,
    source: &handlers::BoundHandlers<'_>,
    external: &mut Q,
    reports: &mut R,
    statements: &mut P,
    make_notifications: F,
    owner: W::Owner,
    budget: usize,
) -> Result<Vec<Event>, Failure>
where
    Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>,
    R: Reports,
    P: c6_core::diplomacy_manager::ProcessStatement<LoadedState>,
    W: aftermath::Blocking,
    F: FnOnce(&'s mut LoadedState) -> W,
{
    let mut trace = skip_turn_state_phase_impl(
        state,
        r,
        inputs,
        external,
        reports,
        HandlerAuthority::RuntimeBinding(source),
    )?;
    if trace
        .iter()
        .any(|event| matches!(event, Event::UnitAbsent { .. }))
    {
        trace.push(Event::Stage(Stage::DiagnosticComplete));
        return Ok(trace);
    }
    let mut budget = budget;
    trace.push(Event::Stage(Stage::Diplomacy));
    if let Err(error) = state.process_pending_diplomacy(statements, &mut budget) {
        return Err(Failure {
            error: Error::Aftermath(error),
            trace,
        });
    }
    finish_notifications(make_notifications(state), owner, &mut budget, trace)
}
#[allow(clippy::too_many_arguments)]
fn diagnose_skip_turn_impl<'s, Q, R, W, F>(
    state: &'s mut LoadedState,
    r: UnitReference,
    inputs: &Inputs<'_, '_>,
    external: &mut Q,
    reports: &mut R,
    make_aftermath: F,
    owner: W::Owner,
    budget: usize,
    authority: HandlerAuthority<'_, '_>,
) -> Result<Vec<Event>, Failure>
where
    Q: ManagerQueries<Unit, Parameters = (), Error = &'static str>,
    R: Reports,
    W: aftermath::Diplomacy + aftermath::Blocking,
    F: FnOnce(&'s mut LoadedState) -> W,
{
    let mut trace = skip_turn_state_phase_impl(state, r, inputs, external, reports, authority)?;
    if trace
        .iter()
        .any(|event| matches!(event, Event::UnitAbsent { .. }))
    {
        trace.push(Event::Stage(Stage::DiagnosticComplete));
        return Ok(trace);
    }
    let mut provider = make_aftermath(state);
    let mut budget = budget;
    trace.push(Event::Stage(Stage::Diplomacy));
    if let Err(error) = aftermath::process_pending(&mut provider, &mut budget) {
        return Err(Failure {
            error: Error::Aftermath(error),
            trace,
        });
    }
    finish_notifications(provider, owner, &mut budget, trace)
}

fn finish_notifications<W: aftermath::Blocking>(
    mut provider: W,
    owner: W::Owner,
    budget: &mut usize,
    mut trace: Vec<Event>,
) -> Result<Vec<Event>, Failure> {
    trace.push(Event::Stage(Stage::Notifications));
    if let Err(error) = aftermath::update_owner_blocking(&mut provider, owner, budget) {
        return Err(Failure {
            error: Error::Aftermath(error),
            trace,
        });
    }
    trace.push(Event::Stage(Stage::DiagnosticComplete));
    Ok(trace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_trace_preserves_signed_identity_bits_and_optional_arguments() {
        let trace = trace_json(&[Event::Report(IdentityReport {
            event: 0xe23b112d,
            subtype: 0x80000001,
            identity: ComponentId {
                owner: -1,
                component_type: -2,
                id: 0xfedcba98,
            },
            first: Some(i32::MIN),
            second: None,
        })]);
        assert_eq!(
            trace,
            serde_json::json!([{"kind":"report","type_bits":"80000001e23b112d","component_bits":"fedcba98fffeffff","first":i32::MIN,"second":null}])
        );
    }
    // Synthetic wire-shaped input for report-order/mutation regressions only.
    // It makes no claim about a native initialized Unit or complete gameplay.
    pub(super) fn synthetic_unit() -> Unit {
        c6_core::unit::Unit {
            id: 0,
            e_type_index: 0,
            e_ai_type: 0,
            ai_controller: 0,
            ai_goal: c6_core::unit::PlotCoord { x: 0, y: 0 },
            k_location: c6_core::unit::PlotCoord { x: 0, y: 0 },
            e_facing_direction: 0,
            e_owner: 0,
            e_original_owner: 0,
            b_is_levied: false,
            formation_id: 0,
            e_military_formation: 0,
            e_automation: 0,
            e_activity: 0,
            i_damage: 0,
            i_fortify_turns: 0,
            i_upgrade_discount: 0,
            x_moves_remaining: 0,
            i_attacks_remaining: 0,
            i_operation_timer: 0,
            i_build_charges: 0,
            i_park_charges: 0,
            i_action_charges: 0,
            i_air_slot_modifier: 0,
            b_dominant_conquer: false,
            i_barbarian_tribe_index: 0,
            e_barbarian_player_target: 0,
            i_disaster_charges: 0,
            i_era_strength_modifier: 0,
            b_embarked: false,
            b_delayed_death: false,
            b_immobile: false,
            b_processed_turn: false,
            b_needs_movement_completion: false,
            b_eligible_to_heal: false,
            b_has_hidden_visibility: false,
            b_hidden_visibility_forfeited: false,
            b_ignore_terrain_cost: false,
            b_ignore_zoc: false,
            b_exert_zoc: false,
            b_can_see_through_features: false,
            b_can_see_through_terrain: false,
            b_attack_after_moving: false,
            i_move_after_attacking: 0,
            b_needs_attention: false,
            b_ignore_resource_maintenance: false,
            k_health_modifiers: c6_core::unit::HealthModifiers {
                i_extra_enemy_heal: 0,
                i_extra_neutral_heal: 0,
                i_extra_friendly_heal: 0,
                b_can_heal_after_action: false,
            },
            k_health_religion_modifiers: c6_core::unit::HealthReligionModifiers {
                i_extra_enemy_heal_religion: 0,
                i_extra_neutral_heal_religion: 0,
                i_extra_friendly_heal_religion: 0,
                b_can_heal_after_action_religion: false,
            },
            k_combat_modifers: c6_core::unit::CombatModifiers {
                i_versus_barbarians: 0,
                i_friendly_territory_combat_mod: 0,
                b_bypass_walls: false,
                b_enable_wall_attack: false,
                b_cannot_attack: false,
                i_flanking_bonus_modifier: 0,
                i_support_bonus_modifier: 0,
                i_per_luxury_attack_modifier: 0,
                b_advanced_pillaging: false,
                i_plunder_yield_modifier: 0,
                b_bypass_combat_units: false,
                b_no_reduction_for_damage: false,
                b_capture_combat_units: false,
                e_captured_combat_unit_type: 0,
                i_bonus_attacks: 0,
                b_conduct_raid: false,
                i_raid_bonus: 0,
                i_strength_reduction_for_damage_modifier: 0,
                i_post_combat_heal: 0,
                b_can_force_retreat: false,
                i_higher_diplo_viz_combat_mod: 0,
                i_each_diplo_viz_combat_mod: 0,
                i_adjacent_levy_combat_modifier: 0,
                i_number_allies_combat_modifier: 0,
                i_strength_reduction_insufficient_fuel: 0,
                i_per_unused_movement_combat_modifier: 0,
                i_advanced_coastal_raid: 0,
                i_combat_bonus_against_district: 0,
                b_ignore_ranged_vs_district_penalty: false,
                i_wmd_blast_resistance: 0,
                i_wmd_fallout_resistance: 0,
                i_water_damage_resistance: 0,
                i_number_holy_cities_combat_modifier: 0,
                i_number_military_policies_combat_modifier: 0,
                i_number_neighbor_combat_bonus: 0,
                b_fight_while_embarked: false,
                i_anti_air_strength_modifier: 0,
                i_combat_strength_modifier: 0,
                i_ranged_strength_modifier: 0,
                i_bombard_strength_modifier: 0,
                i_direct_loyalty_damage: 0,
            },
            k_movement_modifiers: c6_core::unit::MovementModifiers {
                i_extra_moves: 0,
                i_clear_terrain_start_movement: 0,
                b_enter_foreign_lands: false,
                b_ignore_rivers: false,
                b_ignore_cliffs: false,
                b_ignore_shores: false,
                b_escort_mobility: false,
                i_friendly_territory_start_movement: 0,
                i_enemy_territory_start_movement: 0,
                b_can_paradrop: false,
                i_jump_range: 0,
            },
            k_sight_modifiers: c6_core::unit::SightModifiers {
                i_sight_range: 0,
                e_can_see_hidden_type: 0,
            },
            k_attack_range_modifiers: c6_core::unit::AttackRangeModifiers { i_attack_range: 0 },
            k_sight_plots: Vec::new(),
            ai_abilities_possessed: Vec::new(),
            ab_invalid_terrain_types: Vec::new(),
            ab_ignore_terrain_types: Vec::new(),
            ab_ignore_feature_types: Vec::new(),
            ai_bonus_points_per_kill: Vec::new(),
            ai_bonus_points_per_kill_strength: Vec::new(),
            ai_bypass_walls_promotion_class: Vec::new(),
            ai_enable_wall_attack_promotion_class: Vec::new(),
            k_experience: c6_core::unit::Experience {
                b_can_promote: false,
                b_experience_locked: false,
                i_experience: 0,
                i_level: 0,
                i_stored_promotions: 0,
                i_promote_no_finish_moves: 0,
                x_general_experience_modifier: 0,
                ab_promotions_earned: Vec::new(),
                s_veteran_name: Vec::new(),
                b_can_promote_notify_needed: false,
                x_attacking_experience_modifier_percent: 0,
            },
            k_great_person: c6_core::unit::GreatPerson {
                e_individual: 0,
                e_class: 0,
                i_action_charges: 0,
            },
            k_religion: c6_core::unit::Religion {
                i_religious_strength: 0,
                e_religion_type: 0,
                i_spread_charges: 0,
                i_religious_heal_charges: 0,
                i_nw_deferred_charges: 0,
                e_initiation_yield: 0,
                i_initiation_amount: 0,
                i_evict_percent: 0,
                i_foreign_spread_modifier: 0,
                b_converts_barbarians: false,
                b_no_foreign_spread: false,
                b_relic_upon_death: false,
                b_land_victory_spread: false,
                i_land_victory_spread: 0,
                i_plunder_improvement_faith_yield: 0,
                i_plunder_district_faith_yield: 0,
                a_initiation_yield_population: Vec::new(),
                e_relic_source: 0,
            },
            k_rock_band: c6_core::unit::RockBand {
                b_max_level: false,
                b_max_promotions: false,
                i_level: 0,
                i_album_sales: 0,
                i_post_tourism_bomb_foreign_loyalty: 0,
                i_modified_rockband_level_national_park: 0,
                i_modified_tourism_bomb_value_national_park: 0,
                i_modified_rockband_level_natural_wonder: 0,
                i_modified_tourism_bomb_value_natural_wonder: 0,
                i_modified_tourism_bomb_player_range: 0,
                i_modified_tourism_bomb_value_at_range: 0,
                i_modified_tourism_bomb_convert_city: 0,
                i_modified_tourism_bomb_value_target_peace: 0,
                a_modified_yield_per_tourism_bomb: Vec::new(),
                ai_concert_plots: Vec::new(),
                ai_modified_rockband_level_district: Vec::new(),
                ai_modified_rockband_level_improvement: Vec::new(),
                ai_modified_tourism_bomb_value_district: Vec::new(),
                ai_modified_tourism_bomb_value_improvement: Vec::new(),
            },
            k_trade: c6_core::unit::Trade {
                k_origin_trade_city: c6_core::unit::ComponentId {
                    owner: 0,
                    component_type: 0,
                    id: 0,
                },
                k_destination_trade_city: c6_core::unit::ComponentId {
                    owner: 0,
                    component_type: 0,
                    id: 0,
                },
                v_domain_plunder_immunities: [0; 5],
            },
            k_archaeology: c6_core::unit::Archaeology {
                k_home_city_id: 0,
                e_home_building: 0,
                i_artifact_index: 0,
                i_great_work_index: 0,
                b_can_extract_sea_artifacts: false,
                b_choosing_artifact: false,
            },
            k_espionage: c6_core::unit::Espionage {
                a_espionage_bonuses: Vec::new(),
                a_espionage_time_reductions: Vec::new(),
                e_operation: 0,
                i_operation_end_turn: 0,
                k_operation_end_destination: c6_core::unit::PlotCoord { x: 0, y: 0 },
                i_establish_in_city_time_reduction: 0,
                i_escape_boost: 0,
                i_boost_all_spies: 0,
                i_nerf_all_enemy_spies: 0,
                i_visibility_reduction_all_cities: 0,
                b_counter_spy_entire_city: false,
                i_counter_spy_adjacent_level_boost: 0,
                e_escape_route: 0,
                b_escaping: false,
                e_stolen_boost: 0,
                i_siphoned_gold: 0,
                i_stolen_great_work_index: 0,
            },
            k_attributes: c6_core::unit::variants::VariantMap {
                scalars: Vec::new(),
                arrays: Vec::new(),
            },
            k_operations: c6_core::unit::OperationQueue { nodes: Vec::new() },
            k_operations_disabled: Vec::new(),
            k_hero: c6_core::unit::Hero {
                e_class: 0,
                k_origin_city_id: c6_core::unit::ComponentId {
                    owner: 0,
                    component_type: 0,
                    id: 0,
                },
            },
            k_properties: c6_core::unit::variants::TypedVariantMap {
                header: Default::default(),
                entries: Vec::new(),
            },
        }
    }
    struct Subscriber {
        mode: u8,
        calls: usize,
    }
    impl Reports for Subscriber {
        fn send(&mut self, u: &mut Unit, _: IdentityReport) -> Result<i32, Error> {
            self.calls += 1;
            match self.mode {
                0 => {
                    u.e_activity = skip_turn::HOLD;
                    Ok(19)
                }
                1 => {
                    u.i_damage = u.i_damage.wrapping_add(1);
                    Err(Error::Provider("unclosed report consumer"))
                }
                2 => {
                    u.i_damage = u.i_damage.wrapping_add(1);
                    Ok(23)
                }
                _ => Err(Error::Provider("unavailable report backend")),
            }
        }
    }
    fn identity(u: &Unit) -> ComponentId {
        ComponentId {
            owner: u.e_owner as i16,
            component_type: 1,
            id: u.id,
        }
    }
    #[test]
    fn loaded_report_activity_reread_and_equal_value_suppression() {
        let mut u = synthetic_unit();
        let id = identity(&u);
        u.e_activity = AWAKE;
        let mut subscriber = Subscriber { mode: 0, calls: 0 };
        let mut trace = Vec::new();
        start_unit(&mut u, id, &mut subscriber, &mut trace).unwrap();
        assert_eq!(u.e_activity, skip_turn::HOLD);
        assert_eq!(subscriber.calls, 1); // start subscriber already wrote HOLD
        assert_eq!(
            trace,
            vec![Event::Report(IdentityReport {
                event: 0x3d4006f2,
                subtype: SKIP_TURN_HASH,
                identity: id,
                first: Some(0),
                second: None
            })]
        );
    }
    #[test]
    fn loaded_report_error_retains_effect_and_stops_before_activity() {
        for mode in [1, 2] {
            let mut u = synthetic_unit();
            let id = identity(&u);
            u.e_activity = AWAKE;
            let damage = u.i_damage;
            let mut subscriber = Subscriber { mode, calls: 0 };
            let mut trace = Vec::new();
            let error = start_unit(&mut u, id, &mut subscriber, &mut trace).unwrap_err();
            assert!(matches!(error, Error::Provider(_) | Error::Unsupported(_)));
            assert_eq!(u.i_damage, damage.wrapping_add(1));
            assert_eq!(u.e_activity, AWAKE);
            assert_eq!(subscriber.calls, 1);
            assert_eq!(trace.len(), 1);
        }
    }
    #[test]
    fn loaded_activity_assignment_precedes_failed_report() {
        let mut u = synthetic_unit();
        let id = identity(&u);
        u.e_activity = AWAKE;
        let mut subscriber = Subscriber { mode: 3, calls: 0 };
        let mut trace = Vec::new();
        assert!(matches!(
            activity_unit(&mut u, id, skip_turn::HOLD, &mut subscriber, &mut trace),
            Err(Error::Provider(_))
        ));
        assert_eq!(u.e_activity, skip_turn::HOLD);
        assert_eq!(subscriber.calls, 1);
        assert!(matches!(
            trace[0],
            Event::Report(IdentityReport {
                event: 0xe23b112d,
                first: Some(skip_turn::HOLD),
                ..
            })
        ));
    }
}
