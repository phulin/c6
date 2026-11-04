//! A small computed SkipTurn transaction over authoritative typed state.
//!
//! Planning changes only owned actor/Formation records and a deferred logical
//! script delta. Unknown semantic effects refuse without changing live state.
//! This finite profile is not complete action/native parity or Engine::apply.
use super::{
    arena::ActorRecords, handlers, reporting, Error, Event, HandlerAuthority, IdentityReport,
    PhaseContext, Stage,
};
use c6_core::{
    core_player::initialization::WireBinding,
    formation_initialization::{FormationRuntime, UNIT_COMPONENT_TYPE},
    loaded_state::{ActionStamp, LoadedState, ObservedSnapshotWitness},
    map::UnitReference,
    script_event_registry::{
        CachedExecutionState, FactoryExecutionState, InternExecutionState, RegistryError,
        RegistryExecutionPlan, ScriptEventRegistry,
    },
    skip_turn::legality::{ManagerQueries, ManagerQueryState, ScriptTestAllResult},
    unit::{Formation, Unit},
    unit_operation_interruption::Handler,
};

/// Unsupported simulation or stale prepared state. These events describe
/// planning order only; no live callback/effect prefix has been committed.
#[derive(Debug)]
pub struct Refusal {
    pub error: Error,
    pub planned_trace: Vec<Event>,
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SkipTurn preparation refused: {} ({} planned events)",
            self.error,
            self.planned_trace.len()
        )
    }
}
impl std::error::Error for Refusal {}

/// A modeled native rejection is an applied outcome, not an unsupported error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    ActivationRejected,
    QueuedBusy,
    Started,
    Continued,
    HandlerAbsent,
}
fn disposition(trace: &[Event]) -> Disposition {
    if trace
        .iter()
        .any(|e| matches!(e, Event::AddHandlerAbsent(_)))
    {
        Disposition::HandlerAbsent
    } else if trace.iter().any(|e| matches!(e, Event::Legality(false))) {
        Disposition::ActivationRejected
    } else if trace
        .iter()
        .any(|e| matches!(e, Event::Stage(Stage::Continue)))
    {
        Disposition::Continued
    } else if trace
        .iter()
        .any(|e| matches!(e, Event::Stage(Stage::Start)))
    {
        Disposition::Started
    } else {
        Disposition::QueuedBusy
    }
}

/// Logical initialized ScriptSystem identity, not a native pointer or OS handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptSystemIdentity(pub u64);

/// Owned initial/current script semantics. Construction checks consistency only;
/// actual capture/initialization attribution belongs to the input adapter.
pub struct ScriptState {
    binding: WireBinding,
    origin: Option<ObservedSnapshotWitness>,
    identity: ScriptSystemIdentity,
    registry: ScriptEventRegistry,
    execution: CachedExecutionState,
    factory: FactoryExecutionState,
    intern: Option<InternExecutionState>,
}
impl ScriptState {
    #[allow(clippy::too_many_arguments)]
    pub fn from_supplied(
        binding: WireBinding,
        origin: Option<ObservedSnapshotWitness>,
        identity: ScriptSystemIdentity,
        registry: ScriptEventRegistry,
        execution: CachedExecutionState,
        factory: FactoryExecutionState,
        intern: Option<InternExecutionState>,
    ) -> Self {
        Self {
            binding,
            origin,
            identity,
            registry,
            execution,
            factory,
            intern,
        }
    }
    pub fn registry(&self) -> &ScriptEventRegistry {
        &self.registry
    }
    pub fn execution(&self) -> &CachedExecutionState {
        &self.execution
    }
    pub fn factory(&self) -> &FactoryExecutionState {
        &self.factory
    }
    pub fn intern(&self) -> Option<&InternExecutionState> {
        self.intern.as_ref()
    }
    fn matches(&self, state: &LoadedState) -> bool {
        self.binding == *state.binding() && self.origin.as_ref() == state.observed_snapshot()
    }
}

/// Concrete immutable definitions and input authorities for this command.
pub struct Inputs<'a, 'd, 'capture> {
    pub operation: &'a super::Inputs<'a, 'd>,
    pub handlers: &'a handlers::BoundHandlers<'capture>,
    pub reports: &'a reporting::Definitions,
    pub movement: &'a crate::unit_movement_state::Inputs<'a>,
    pub report_scope: reporting::Scope,
}

/// Computed records and required deferred effects. There is deliberately no
/// public state-phase commit: aftermath must also be supported before advance.
pub struct StatePhasePlan {
    stamp: ActionStamp,
    binding: WireBinding,
    before: ActorRecords,
    after: ActorRecords,
    script: Option<RegistryExecutionPlan>,
    script_identity: Option<ScriptSystemIdentity>,
    trace: Vec<Event>,
    publications: Vec<reporting::Publication>,
    snapshots: Vec<reporting::SnapshotObservation>,
}
impl StatePhasePlan {
    /// Whether this plan still belongs to this exact owned World and its
    /// unchanged initialization/gameplay generation. Script deltas are checked
    /// separately by their exclusive commit guard.
    pub fn is_current(&self, state: &LoadedState) -> bool {
        self.stamp == state.action_stamp()
    }
    pub fn before(&self) -> &ActorRecords {
        &self.before
    }
    pub fn after(&self) -> &ActorRecords {
        &self.after
    }
    pub fn trace(&self) -> &[Event] {
        &self.trace
    }
    pub fn publications(&self) -> &[reporting::Publication] {
        &self.publications
    }
    /// Readonly query branches computed while projecting synchronous reports.
    pub fn snapshot_observations(&self) -> &[reporting::SnapshotObservation] {
        &self.snapshots
    }
}

#[must_use = "a prepared SkipTurn must be committed or deliberately discarded"]
pub struct PreparedSkipTurn {
    phase: StatePhasePlan,
    diplomacy: c6_core::diplomacy_drain::Plan,
}
/// The source-supported finite transition was computed and committed. These
/// publications remain a presentation projection, not delivered subscribers.
pub struct AppliedSkipTurn {
    pub disposition: Disposition,
    pub trace: Vec<Event>,
    pub publications: Vec<reporting::Publication>,
    pub snapshots: Vec<reporting::SnapshotObservation>,
}

struct ScriptQueries<'a> {
    state: &'a LoadedState,
    scripts: Option<&'a ScriptState>,
    pending: Option<RegistryExecutionPlan>,
    error: Option<RegistryError>,
    held: bool,
}
impl ManagerQueries<Unit> for ScriptQueries<'_> {
    type Error = &'static str;
    type Parameters = ();
    type Player = ();
    type Game = ();
    type Script = ScriptSystemIdentity;
    type EngineUtility = ();
    type Args = Vec<i32>;
    type Handler = ();
    type Plot = ();
    type Results = ();
    #[allow(unused_variables)]
    fn allow_tactical_commands(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        owner: i32,
    ) -> Result<bool, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn player(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        owner: i32,
    ) -> Result<Self::Player, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn turn_active(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn processing_automation(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<i32, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn player_id(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<i32, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn game(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::Game, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn player_human(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn paused(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        game: Self::Game,
    ) -> Result<bool, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn runtime_id(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        hash: u32,
    ) -> Result<i32, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn handler(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<Option<Self::Handler>, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn can_be_interrupted(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        handler: Self::Handler,
    ) -> Result<bool, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn operation_disabled(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<bool, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn script_system(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Option<Self::Script>, Self::Error> {
        let scripts = self
            .scripts
            .ok_or("initialized logical ScriptSystem state unavailable")?;
        if !scripts.matches(self.state) {
            return Err("ScriptSystem initial-context binding mismatch");
        }
        Ok(Some(scripts.identity))
    }
    #[allow(unused_variables)]
    fn args_begin(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::Args, Self::Error> {
        let mut args = Vec::new();
        args.try_reserve_exact(3)
            .map_err(|_| "script argument resource limit")?;
        Ok(args)
    }
    #[allow(unused_variables)]
    fn args_push_int(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        args: &mut Self::Args,
        value: i32,
    ) -> Result<(), Self::Error> {
        if args.len() >= 3 {
            return Err("unsupported script argument shape");
        }
        args.push(value);
        Ok(())
    }
    #[allow(unused_variables)]
    fn engine_utility(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::EngineUtility, Self::Error> {
        Ok(())
    }
    #[allow(unused_variables)]
    fn has_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<bool, Self::Error> {
        Ok(self.held)
    }
    #[allow(unused_variables)]
    fn release_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        self.held = false;
        Ok(())
    }
    #[allow(unused_variables)]
    fn get_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        self.held = true;
        Ok(())
    }
    #[allow(unused_variables)]
    fn script_call_test_all(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        script: Self::Script,
        args: &mut Self::Args,
        event: &'static str,
        initial_value: bool,
    ) -> Result<ScriptTestAllResult, Self::Error> {
        let scripts = self
            .scripts
            .ok_or("initialized logical ScriptSystem state unavailable")?;
        if script != scripts.identity || self.pending.is_some() {
            return Err("unsupported repeated or changed ScriptSystem call");
        }
        let arguments: [i32; 3] = args
            .as_slice()
            .try_into()
            .map_err(|_| "script argument shape")?;
        if arguments != [s.unit.e_owner, s.unit.id as i32, s.operation.runtime_id] {
            return Err("script argument identity");
        }
        match scripts.registry.plan_test_all(
            event,
            arguments,
            &scripts.execution,
            &scripts.factory,
            scripts.intern.as_ref(),
        ) {
            Ok(plan) => {
                let result = plan.result();
                self.pending = Some(plan);
                Ok(ScriptTestAllResult {
                    called: result.is_some(),
                    value: result.unwrap_or(initial_value),
                })
            }
            Err(error) => {
                self.error = Some(error);
                Err("script semantic frontier")
            }
        }
    }
    #[allow(unused_variables)]
    fn args_end(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        args: Self::Args,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    #[allow(unused_variables)]
    fn unit_plot(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Option<Self::Plot>, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
    #[allow(unused_variables)]
    fn handler_can_start(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        handler: Self::Handler,
        plot: Self::Plot,
        visible_only: bool,
        results: Option<&mut Self::Results>,
    ) -> Result<bool, Self::Error> {
        Err("World-backed query must use loaded action context")
    }
}

struct Staged<'a, 'd, 'capture> {
    state: &'a LoadedState,
    reference: UnitReference,
    inputs: &'a super::Inputs<'a, 'd>,
    authority: HandlerAuthority<'a, 'capture>,
    unit: Unit,
    formation: Formation,
    formation_runtime: FormationRuntime,
    reports: reporting::Provider<'a>,
    scripts: ScriptQueries<'a>,
}
impl PhaseContext for Staged<'_, '_, '_> {
    fn actor(&self) -> Result<&Unit, Error> {
        Ok(&self.unit)
    }
    fn actor_mut(&mut self) -> Result<&mut Unit, Error> {
        Ok(&mut self.unit)
    }
    fn formation(&self) -> Result<&Formation, Error> {
        Ok(&self.formation)
    }
    fn formation_mut(&mut self) -> Result<&mut Formation, Error> {
        Ok(&mut self.formation)
    }
    fn set_evaluating(&mut self, value: bool) -> Result<(), Error> {
        self.formation_runtime
            .set_evaluating_operation(&self.formation, value)
            .map_err(|_| Error::Unsupported("staged Formation identity"))
    }
    fn local_player(&self) -> i32 {
        self.state.retained().core.local_player
    }
    fn current_turn(&self) -> i32 {
        self.state.retained().core.current_turn
    }
    fn resolve_handler(&mut self, id: i32) -> Result<Option<Handler>, Error> {
        self.inputs
            .handlers
            .handler(c6_core::messages::HandlerId(id))
            .map_err(Error::Registry)
    }
    fn can_start(&mut self, id: i32) -> Result<bool, Error> {
        super::can_start_for_actor(
            self.state,
            &mut self.unit,
            self.inputs,
            &mut self.scripts,
            id,
            self.authority,
        )
    }
    fn has_moved(&mut self) -> Result<bool, Error> {
        let input = self
            .inputs
            .movement
            .ok_or(Error::Unsupported("actual MaxMoves inputs unavailable"))?;
        crate::unit_movement_state::unit_has_moved(self.state, &self.unit, input)
            .map_err(Error::Movement)
    }
    fn deliver_report(
        &mut self,
        report: IdentityReport,
        trace: &mut Vec<Event>,
    ) -> Result<(), Error> {
        trace.push(Event::Report(report));
        self.reports.send_overlay_report(
            self.state,
            self.reference,
            &self.unit,
            &self.formation,
            report,
        )
    }
}

/// Read-only preparation of the actual shared state machine. Unknown effects
/// return the completed planning trace but do not mutate World or script state.
pub fn plan_state_phase<'a, 'd, 'capture>(
    state: &'a LoadedState,
    reference: UnitReference,
    inputs: Inputs<'a, 'd, 'capture>,
    scripts: Option<&'a ScriptState>,
) -> Result<StatePhasePlan, Refusal> {
    let mut trace = vec![
        Event::Stage(Stage::ExecuteLookup),
        Event::Stage(Stage::Preflight),
    ];
    let result = (|| {
        if reference.component_id().component_type != UNIT_COMPONENT_TYPE {
            return Err(Error::Unsupported(
                "SkipTurn requires Unit component identity",
            ));
        }
        let before = ActorRecords::capture(state, reference).map_err(|_| {
            Error::Unsupported("canonical typed actor/Formation records unavailable")
        })?;
        let authority = HandlerAuthority::RuntimeBinding(inputs.handlers);
        let (identity, id) = super::preflight(state, reference, inputs.operation, authority)?;
        if identity.component_type != UNIT_COMPONENT_TYPE {
            return Err(Error::Unsupported("Unit runtime component identity"));
        }
        let formation_runtime = state
            .formation_runtime(reference.player(), before.formation.id)
            .map_err(Error::State)?
            .clone();
        let mut reports =
            reporting::Provider::new(inputs.reports, inputs.movement, inputs.report_scope);
        reports.bind_skip_turn_handler(state, inputs.handlers, inputs.operation.operations)?;
        let mut staged = Staged {
            state,
            reference,
            inputs: inputs.operation,
            authority,
            unit: before.unit.clone(),
            formation: before.formation.clone(),
            formation_runtime,
            reports,
            scripts: ScriptQueries {
                state,
                scripts,
                pending: None,
                error: None,
                held: true,
            },
        };
        if let Err(error) = super::run_state_phase(&mut staged, identity, id, &mut trace) {
            return Err(staged
                .scripts
                .error
                .take()
                .map(Error::ScriptRegistry)
                .unwrap_or(error));
        }
        if staged.formation_runtime.evaluating_operation() {
            return Err(Error::Unsupported(
                "unfinished Formation operation evaluation",
            ));
        }
        let script_identity = staged
            .scripts
            .pending
            .as_ref()
            .map(|_| scripts.expect("planned scripts").identity);
        let after = ActorRecords::new(reference, staged.unit, staged.formation)
            .map_err(|_| Error::Unsupported("staged actor/Formation identity changed"))?;
        Ok((
            before,
            after,
            staged.scripts.pending,
            script_identity,
            staged.reports.pending_publications().to_vec(),
            staged.reports.snapshot_observations().to_vec(),
        ))
    })();
    match result {
        Ok((before, after, script, script_identity, publications, snapshots)) => {
            Ok(StatePhasePlan {
                stamp: state.action_stamp(),
                binding: state.binding().clone(),
                before,
                after,
                script,
                script_identity,
                trace,
                publications,
                snapshots,
            })
        }
        Err(error) => Err(Refusal {
            error,
            planned_trace: trace,
        }),
    }
}

/// Qualify the initial reducer's query effects for the finite stock XP2
/// source-initialized family. Factory/type identity alone does not authenticate
/// a native object: the transaction's stock initialization/consumer scope must
/// be established by its input adapter. No omitted runtime field is filled here.
fn check_initial_blocking_query_effects(
    records: &[c6_core::notifications::Notification],
) -> Result<(), Error> {
    for record in records {
        if record.factory_type != record.type_hash {
            return Err(Error::Unsupported(
                "initial notification factory/type binding mismatch",
            ));
        }
        match record.factory_type {
            // Base fallback and PrideMomentRecorded all bind constant-true
            // phase, Base expiry/blocking and scalar severity. Those queries
            // only read state, including the unknown default-blocking scalar.
            0xcb5eed15 | 0x9efbc217 | 0xffad0d16 | 0xba4af0fb | 0xe90d955a | 0x458dcd81
            | 0x0c19b649 => {}
            _ => {
                return Err(Error::Unsupported(
                    "initial notification subtype query effects",
                ))
            }
        }
    }
    Ok(())
}

/// Compute outgoing diplomacy and qualify the initial notification reduction.
/// This finite transaction uses pinned stock factory initialization and the
/// caller-admitted stock consumer scope. The pure Base/Pride reduction may be
/// omitted without materializing its UI value; Update and any final changed
/// report still require their own effect plan before a transaction can commit.
fn plan_aftermath(
    state: &LoadedState,
    owner: i32,
    trace: &mut Vec<Event>,
) -> Result<c6_core::diplomacy_drain::Plan, Error> {
    trace.push(Event::Stage(Stage::Diplomacy));
    let diplomacy = state.plan_pending_diplomacy().map_err(Error::State)?;
    trace.push(Event::Stage(Stage::Notifications));
    let collection = state
        .player_notifications()
        .map_err(Error::State)?
        .collection(owner)
        .map_err(|e| Error::State(c6_core::loaded_state::Error::PlayerNotifications(e)))?;
    check_initial_blocking_query_effects(&collection.records)?;
    if !super::notifications::turn_complete(state, owner)
        .map_err(|_| Error::Unsupported("actual Processor turn-complete input"))?
    {
        return Err(Error::Unsupported("NotificationManager.SendAutoNotify ordered six-subtype query/callback effects require semantic planning"));
    }
    if !collection.records.is_empty() {
        // Query purity does not imply a zero result or absence of the setter's
        // changed-local report. No action is accepted through this frontier.
        return Err(Error::Unsupported(
            "notification final blocking/store/report effects require semantic planning",
        ));
    }
    if super::notifications::owner_blocking(state, owner)
        .map_err(|_| Error::Unsupported("actual owner blocking input"))?
        != 0
    {
        return Err(Error::Unsupported(
            "owner blocking store/report requires semantic planning",
        ));
    }
    Ok(diplomacy)
}

/// Prepare a whole direct-Core SkipTurn only for the supported semantic profile.
/// Refusal after an otherwise computed actor phase still leaves every live field
/// and logical script object unchanged. No native result/certificate is consumed.
pub fn prepare<'a, 'd, 'capture>(
    state: &'a LoadedState,
    reference: UnitReference,
    inputs: Inputs<'a, 'd, 'capture>,
    scripts: Option<&'a ScriptState>,
) -> Result<PreparedSkipTurn, Refusal> {
    let mut phase = plan_state_phase(state, reference, inputs, scripts)?;
    let diplomacy = match plan_aftermath(state, reference.player(), &mut phase.trace) {
        Ok(plan) => plan,
        Err(error) => {
            return Err(Refusal {
                error,
                planned_trace: phase.trace,
            })
        }
    };
    Ok(PreparedSkipTurn { phase, diplomacy })
}
impl PreparedSkipTurn {
    pub fn records(&self) -> (&ActorRecords, &ActorRecords) {
        (&self.phase.before, &self.phase.after)
    }
    pub fn commit(
        self,
        state: &mut LoadedState,
        scripts: Option<&mut ScriptState>,
    ) -> Result<AppliedSkipTurn, Refusal> {
        let PreparedSkipTurn { phase, diplomacy } = self;
        let current = phase.is_current(state);
        let mut trace = phase.trace;
        let result = (|| {
            if !current {
                return Err(Error::State(
                    c6_core::loaded_state::Error::ActorCommitConflict,
                ));
            }
            if *state.binding() != phase.binding {
                return Err(Error::ConfigurationIdentity);
            }
            if let Some(plan) = phase.script {
                let script = scripts.ok_or(Error::Unsupported(
                    "prepared ScriptSystem state unavailable",
                ))?;
                if !script.matches(state) || Some(script.identity) != phase.script_identity {
                    return Err(Error::Unsupported("prepared ScriptSystem context changed"));
                }
                let actor = state
                    .prepare_unit_formation_diplomacy_commit(
                        phase.before.reference,
                        phase.stamp,
                        &phase.before.unit,
                        &phase.before.formation,
                        phase.after.unit,
                        phase.after.formation,
                        diplomacy,
                    )
                    .map_err(Error::State)?;
                let registry = script
                    .registry
                    .prepare_commit(
                        &mut script.execution,
                        &mut script.factory,
                        script.intern.as_mut(),
                        plan,
                    )
                    .map_err(Error::ScriptRegistry)?;
                actor.commit();
                registry.commit();
            } else {
                state
                    .prepare_unit_formation_diplomacy_commit(
                        phase.before.reference,
                        phase.stamp,
                        &phase.before.unit,
                        &phase.before.formation,
                        phase.after.unit,
                        phase.after.formation,
                        diplomacy,
                    )
                    .map_err(Error::State)?
                    .commit();
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                let disposition = disposition(&trace);
                trace.push(Event::Stage(Stage::Committed));
                Ok(AppliedSkipTurn {
                    disposition,
                    trace,
                    publications: phase.publications,
                    snapshots: phase.snapshots,
                })
            }
            Err(error) => Err(Refusal {
                error,
                planned_trace: trace,
            }),
        }
    }
}

#[cfg(test)]
mod notification_initial_effect_tests {
    use super::*;
    use c6_core::notifications::Notification;

    #[test]
    fn stock_initial_queries_do_not_require_ui_runtime_values() {
        let records: Vec<_> = [
            0xcb5eed15, 0x9efbc217, 0xffad0d16, 0xba4af0fb, 0xe90d955a, 0x458dcd81, 0x0c19b649,
        ]
        .into_iter()
        .map(|hash| Notification {
            factory_type: hash,
            type_hash: hash,
            // Expiry/severity/dismissal affect the UI result, not query purity.
            severity: i32::MIN,
            native_188: i32::MAX,
            is_dismissed: false,
            ..Notification::default()
        })
        .collect();
        check_initial_blocking_query_effects(&records).unwrap();
        assert_eq!(records[0].severity, i32::MIN);
    }

    #[test]
    fn subtype_override_or_factory_mismatch_is_not_qualified() {
        let mut record = Notification {
            factory_type: 0x0c19b649,
            type_hash: 0xcb5eed15,
            ..Notification::default()
        };
        assert!(matches!(
            check_initial_blocking_query_effects(&[record.clone()]),
            Err(Error::Unsupported(
                "initial notification factory/type binding mismatch"
            ))
        ));
        record.factory_type = 0x12345678;
        record.type_hash = record.factory_type;
        assert!(matches!(
            check_initial_blocking_query_effects(&[record]),
            Err(Error::Unsupported(
                "initial notification subtype query effects"
            ))
        ));
    }
}
