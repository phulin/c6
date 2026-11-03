//! Full-entrypoint source fixtures over strictly decoded original records.
//! Observed snapshot controls plus synthetic handler assertions/callbacks test orchestration;
//! they are not native observations or gameplay callback implementations.
#![allow(unused_variables)]
#[path = "loaded_actions_review_load.rs"]
mod fixture;
use c6_core::loaded_state::LoadedState;
use c6_core::skip_turn::legality::{ManagerQueries, ManagerQueryState, ScriptTestAllResult};
use c6_core::unit::{ComponentId, Unit, MOVE_TO_HASH, SKIP_TURN_HASH};
use c6_core::unit_operation_aftermath::{self as aftermath, Blocking, Diplomacy, Notifications};
use c6_data::loaded_actions::{self as actions, Error as ActError, IdentityReport, Reports};
use std::cell::RefCell;
use std::rc::Rc;
type Error = aftermath::Error;
#[derive(Clone, Copy)]
enum Mode {
    Accept,
    Activity,
    Identity,
    Veto,
    DamageVeto,
    OldVeto(i32),
    QueueVeto,
}
struct Script {
    mode: Mode,
    calls: usize,
    log: Vec<&'static str>,
}
impl ManagerQueries<Unit> for Script {
    type Error = &'static str;
    type Parameters = ();
    type Player = ();
    type Game = ();
    type Script = ();
    type EngineUtility = ();
    type Args = Vec<i32>;
    type Handler = ();
    type Plot = ();
    type Results = ();
    fn allow_tactical_commands(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        owner: i32,
    ) -> Result<bool, Self::Error> {
        panic!("wrapper must derive allow_tactical_commands from loaded state")
    }
    fn player(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        owner: i32,
    ) -> Result<Self::Player, Self::Error> {
        panic!("wrapper must derive player from loaded state")
    }
    fn turn_active(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        panic!("wrapper must derive turn_active from loaded state")
    }
    fn processing_automation(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<i32, Self::Error> {
        panic!("wrapper must derive processing_automation from loaded state")
    }
    fn player_id(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<i32, Self::Error> {
        panic!("wrapper must derive player_id from loaded state")
    }
    fn game(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::Game, Self::Error> {
        panic!("wrapper must derive game from loaded state")
    }
    fn player_human(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        panic!("wrapper must derive player_human from loaded state")
    }
    fn paused(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        game: Self::Game,
    ) -> Result<bool, Self::Error> {
        panic!("wrapper must derive paused from loaded state")
    }
    fn runtime_id(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        hash: u32,
    ) -> Result<i32, Self::Error> {
        panic!("wrapper must derive runtime_id from loaded state")
    }
    fn handler(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<Option<Self::Handler>, Self::Error> {
        panic!("wrapper must derive handler from loaded state")
    }
    fn can_be_interrupted(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        handler: Self::Handler,
    ) -> Result<bool, Self::Error> {
        panic!("wrapper must derive can_be_interrupted from loaded state")
    }
    fn operation_disabled(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<bool, Self::Error> {
        panic!("wrapper must derive operation_disabled from loaded state")
    }
    fn script_system(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Option<Self::Script>, Self::Error> {
        self.log.push("script_system");
        Ok(Some(()))
    }
    fn args_begin(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::Args, Self::Error> {
        self.log.push("args_begin");
        Ok(vec![])
    }
    fn args_push_int(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        args: &mut Self::Args,
        value: i32,
    ) -> Result<(), Self::Error> {
        args.push(value);
        Ok(())
    }
    fn engine_utility(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::EngineUtility, Self::Error> {
        self.log.push("utility");
        Ok(())
    }
    fn has_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<bool, Self::Error> {
        self.log.push("has_lock");
        Ok(true)
    }
    fn release_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        self.log.push("release");
        Ok(())
    }
    fn get_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        self.log.push("acquire");
        Ok(())
    }
    fn script_call_test_all(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        script: Self::Script,
        args: &mut Self::Args,
        event: &'static str,
        initial_value: bool,
    ) -> Result<ScriptTestAllResult, Self::Error> {
        self.calls += 1;
        self.log.push("script");
        assert_eq!(event, "CanStartOperation");
        assert!(!initial_value);
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], s.unit.e_owner);
        assert_eq!(args[1], s.unit.id as i32);
        match self.mode {
            Mode::Accept => {}
            Mode::Activity => s.unit.e_activity = c6_core::skip_turn::HOLD,
            Mode::Identity => s.unit.id ^= 0x10000,
            Mode::Veto => {}
            Mode::DamageVeto => s.unit.i_damage = s.unit.i_damage.wrapping_add(1),
            Mode::OldVeto(turn) => s.unit.k_operations.nodes[0].push_turn = turn,
            Mode::QueueVeto => {
                let node = s.unit.k_operations.nodes[0].clone();
                s.unit.k_operations.nodes.push(node);
            }
        }
        Ok(ScriptTestAllResult {
            called: true,
            value: !matches!(
                self.mode,
                Mode::Veto | Mode::DamageVeto | Mode::OldVeto(_) | Mode::QueueVeto
            ),
        })
    }
    fn args_end(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        args: Self::Args,
    ) -> Result<(), Self::Error> {
        self.log.push("args_end");
        Ok(())
    }
    fn unit_plot(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Option<Self::Plot>, Self::Error> {
        panic!("wrapper must derive unit_plot from loaded state")
    }
    fn handler_can_start(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        handler: Self::Handler,
        plot: Self::Plot,
        visible_only: bool,
        results: Option<&mut Self::Results>,
    ) -> Result<bool, Self::Error> {
        panic!("wrapper must derive handler_can_start from loaded state")
    }
}
struct After<'a> {
    state: &'a mut LoadedState,
    log: Rc<RefCell<Vec<&'static str>>>,
    sessions: Vec<aftermath::Session<()>>,
}
impl Diplomacy for After<'_> {
    type Player = i32;
    type Statement = ();
    fn ever_alive(&self) -> Vec<i32> {
        assert_eq!(self.state.statistics().players, 43);
        self.log.borrow_mut().push("diplomacy");
        vec![]
    }
    fn player_id(&self, p: i32) -> i32 {
        p
    }
    fn is_ai(&self, p: i32) -> bool {
        panic!("not a native diplomacy provider")
    }
    fn open_sessions(&mut self) -> &mut Vec<aftermath::Session<()>> {
        &mut self.sessions
    }
    fn process_statement(&mut self, s: ()) -> Result<(), Error> {
        Err(Error::Unsupported(
            "source fixture has no statement backend",
        ))
    }
}
impl Notifications for After<'_> {
    type Object = usize;
    type Cursor = usize;
    type Manager = usize;
    type Configuration = usize;
    type Collection = usize;
    type Definitions = usize;
    fn configuration(&mut self) -> Result<Self::Configuration, Error> {
        panic!("unreached source notification method configuration")
    }
    fn definitions(&mut self) -> Result<Self::Definitions, Error> {
        panic!("unreached source notification method definitions")
    }
    fn collection(
        &mut self,
        manager: Self::Manager,
        owner: i32,
        edit: bool,
    ) -> Result<Self::Collection, Error> {
        panic!("unreached source notification method collection")
    }
    fn accepts_notifications(&mut self, manager: Self::Manager, owner: i32) -> Result<bool, Error> {
        panic!("unreached source notification method accepts_notifications")
    }
    fn definition_count(&mut self, definitions: Self::Definitions) -> Result<usize, Error> {
        panic!("unreached source notification method definition_count")
    }
    fn definition_is_auto(
        &mut self,
        definitions: Self::Definitions,
        index: usize,
    ) -> Result<bool, Error> {
        panic!("unreached source notification method definition_is_auto")
    }
    fn definition_hash(
        &mut self,
        definitions: Self::Definitions,
        index: usize,
    ) -> Result<u32, Error> {
        panic!("unreached source notification method definition_hash")
    }
    fn create_auto(
        &mut self,
        manager: Self::Manager,
        hash: u32,
        owner: i32,
    ) -> Result<Option<Self::Object>, Error> {
        panic!("unreached source notification method create_auto")
    }
    fn redundant(
        &mut self,
        collection: Self::Collection,
        object: Self::Object,
    ) -> Result<bool, Error> {
        panic!("unreached source notification method redundant")
    }
    fn release(&mut self, object: Self::Object) -> Result<(), Error> {
        panic!("unreached source notification method release")
    }
    fn send(&mut self, object: Self::Object) -> Result<(), Error> {
        panic!("unreached source notification method send")
    }
    fn add(&mut self, collection: Self::Collection, object: Self::Object) -> Result<(), Error> {
        panic!("unreached source notification method add")
    }
    fn play_by_cloud(&mut self, configuration: Self::Configuration) -> Result<bool, Error> {
        panic!("unreached source notification method play_by_cloud")
    }
    fn manager_game_local_player(&mut self, manager: Self::Manager) -> Result<Option<i32>, Error> {
        panic!("unreached source notification method manager_game_local_player")
    }
    fn owner_turn_active(&mut self, owner: i32) -> Result<bool, Error> {
        panic!("unreached source notification method owner_turn_active")
    }
    fn network_multiplayer(&mut self, configuration: Self::Configuration) -> Result<bool, Error> {
        panic!("unreached source notification method network_multiplayer")
    }
    fn find_auto_activate(
        &mut self,
        collection: Self::Collection,
        threshold: i32,
    ) -> Result<Option<Self::Object>, Error> {
        panic!("unreached source notification method find_auto_activate")
    }
    fn locked(&mut self, object: Self::Object) -> Result<bool, Error> {
        panic!("unreached source notification method locked")
    }
    fn activate(&mut self, object: Self::Object, argument: bool) -> Result<(), Error> {
        panic!("unreached source notification method activate")
    }
    fn begin(&mut self, collection: Self::Collection) -> Result<Self::Cursor, Error> {
        panic!("unreached source notification method begin")
    }
    fn end(&mut self, collection: Self::Collection) -> Result<Self::Cursor, Error> {
        panic!("unreached source notification method end")
    }
    fn object(&mut self, cursor: Self::Cursor) -> Result<Self::Object, Error> {
        panic!("unreached source notification method object")
    }
    fn next(&mut self, current: Self::Cursor) -> Result<Self::Cursor, Error> {
        panic!("unreached source notification method next")
    }
    fn dismissed(&mut self, object: Self::Object) -> Result<bool, Error> {
        panic!("unreached source notification method dismissed")
    }
    fn expired(&mut self, object: Self::Object) -> Result<bool, Error> {
        panic!("unreached source notification method expired")
    }
    fn dismiss(&mut self, object: Self::Object) -> Result<(), Error> {
        panic!("unreached source notification method dismiss")
    }
    fn cleanup(&mut self, collection: Self::Collection) -> Result<(), Error> {
        panic!("unreached source notification method cleanup")
    }
    fn valid_for_phase(&mut self, object: Self::Object) -> Result<bool, Error> {
        panic!("unreached source notification method valid_for_phase")
    }
    fn blocking(&mut self, object: Self::Object) -> Result<i32, Error> {
        panic!("unreached source notification method blocking")
    }
    fn severity(&mut self, object: Self::Object) -> Result<i32, Error> {
        panic!("unreached source notification method severity")
    }
}
impl Blocking for After<'_> {
    type Owner = i32;
    fn notification_manager(&mut self, edit: bool) -> Result<Self::Manager, Error> {
        self.log.borrow_mut().push("notifications");
        Err(Error::Unsupported(
            "intentional missing notification backend",
        ))
    }
    fn owner_id(&mut self, owner: Self::Owner) -> Result<i32, Error> {
        panic!("unreached source notification method owner_id")
    }
    fn turn_complete(&mut self, owner_id: i32) -> Result<bool, Error> {
        panic!("unreached source notification method turn_complete")
    }
    fn owner_blocking(&mut self, owner: Self::Owner) -> Result<i32, Error> {
        panic!("unreached source notification method owner_blocking")
    }
    fn store_owner_blocking(&mut self, owner: Self::Owner, blocking: i32) -> Result<(), Error> {
        panic!("unreached source notification method store_owner_blocking")
    }
    fn local_player(&mut self) -> Result<i32, Error> {
        panic!("unreached source notification method local_player")
    }
    fn blocking_changed(&mut self, event: u32, old: i32, new: i32) -> Result<(), Error> {
        panic!("unreached source notification method blocking_changed")
    }
}

#[derive(Clone, Copy)]
enum ReportMode {
    Collect,
    FailAdded,
    OperationAtAdded,
    HoldAtStart,
}
struct ReportFixture {
    mode: ReportMode,
    calls: usize,
}
impl Reports for ReportFixture {
    fn send(&mut self, u: &mut Unit, r: IdentityReport) -> Result<i32, ActError> {
        self.calls += 1;
        match (self.mode, r.event) {
            (ReportMode::FailAdded, 0x15c35863) => {
                u.i_damage = u.i_damage.wrapping_add(1);
                return Err(ActError::Provider("intentional report failure"));
            }
            (ReportMode::OperationAtAdded, 0x15c35863) => u.e_activity = 0x490cc8e1,
            (ReportMode::HoldAtStart, 0x3d4006f2) => u.e_activity = c6_core::skip_turn::HOLD,
            _ => {}
        }
        Ok(0)
    }
}
fn script(mode: Mode) -> Script {
    Script {
        mode,
        calls: 0,
        log: vec![],
    }
}
fn report_fixture(mode: ReportMode) -> ReportFixture {
    ReportFixture { mode, calls: 0 }
}
fn restore(state: &mut LoadedState, u: &Unit, f: &c6_core::unit::Formation) {
    *state.unit_mut(u.e_owner, u.id).unwrap() = u.clone();
    let (formation, _) = state
        .formation_with_runtime_mut(u.e_owner, u.formation_id)
        .unwrap();
    *formation = f.clone();
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use c6_core::map::UnitReference;
    use c6_core::messages::{HandlerId, OperationHash};
    use c6_core::unit_initialization::InitializationInputs;
    use c6_core::unit_operation_interruption::{
        InterruptionTarget, ObservationIdentity, ObservedHandler, ObservedHashResolution,
        ObservedRegistry,
    };
    use c6_data::loaded_actions::resources::Resources;
    use c6_data::loaded_actions::{Event, Inputs, Stage, TransportScope};
    use c6_data::unit_initialization_definitions::Definitions;
    use c6_data::{Rules, RulesProfile};
    let root = std::path::PathBuf::from(std::env::args_os().nth(1).expect("workspace root"));
    let private = root.parent().unwrap().join("c6-ghidra/evidence");
    let mut state = fixture::load_state(
        &private.join("linux-military-initialization/native-3/candidate-paths-v2.json"),
    )?;
    let resources = Resources::from_closed_captures(
        &private.join("linux-placement-metadata/native-1"),
        &private.join("linux-unit-operation-manager-metadata/native-1"),
    )?;
    let target =
        std::fs::read(private.join("linux-military-initialization/native-3/runtime.json"))?;
    let bound = resources.bind(&state, &target)?;
    let decode_binding = state.binding().clone();
    let mut snapshot = bound.admit_observed_snapshot(&mut state)?;
    assert_eq!(state.binding(), &decode_binding);
    assert_eq!(
        state.observed_snapshot().unwrap().source_role,
        c6_core::core_player::initialization::WireRole::SavedOutputCore
    );
    assert_eq!(
        state
            .world()
            .players()
            .iter()
            .filter(|p| p.slot_status == Some(3))
            .count(),
        1
    );
    assert!(bound.admit_observed_snapshot(&mut state).is_err());
    let original_war = snapshot.relations().war_types[0];
    snapshot.war_types_mut()[0] = original_war ^ 1;
    assert_eq!(snapshot.relations().war_types[0], original_war ^ 1);
    snapshot.war_types_mut()[0] = original_war;
    let profile = RulesProfile::LinuxXp2ActivatedDiagnostic;
    let rules = Rules::load_profile(
        profile,
        private.join("linux-rules/mod-activation-xp2-constructor-final-1"),
        &profile.source_identity()?,
    )?;
    let rules_runtime = std::fs::read(
        private.join("linux-rules/mod-activation-xp2-constructor-final-1/report.json"),
    )?;
    let definitions = Definitions::from_rules(&rules, &bound, &rules_runtime)?;
    let identities = state.binding().inputs.clone();
    state.initialize_rosters(&InitializationInputs {
        identities: &identities,
        units: bound.unit_definitions()?,
        abilities: definitions.abilities(),
        terrains: definitions.terrains(),
        features: definitions.features(),
        great_person_classes: definitions.great_person_classes(),
        operations: bound.operation_lookup(),
    })?;
    state.initialize_formations()?;
    let movement_definitions = c6_data::unit_movement_state::Definitions::from_rules(
        &state,
        &rules,
        &bound,
        &rules_runtime,
    )?;
    let movement_inputs = c6_data::unit_movement_state::Inputs {
        definitions: &movement_definitions,
        alliances: None,
    };
    let local = state.retained().core.local_player;
    let p = state.world().player(local)?;
    let actor = p
        .unit_records()?
        .iter()
        .filter_map(|s| {
            let u = &s.value;
            if u.x_moves_remaining <= 0
                || u.e_automation != -1
                || u.b_needs_movement_completion
                || u.k_operations.nodes.len() != 1
                || u.k_operations.nodes[0].operation_hash != MOVE_TO_HASH
                || u.k_espionage.e_operation != u32::MAX
                || u.i_fortify_turns > 0
                || ![0x490cc8e1, 0x6b25012e].contains(&u.e_activity)
                || c6_core::movement_world::plot_at(state.world().map(), u.k_location).is_err()
            {
                return None;
            }
            if c6_data::unit_movement_state::unit_max_moves(&state, u, &movement_inputs)
                .ok()
                .is_none_or(|m| m <= 0 || m.wrapping_shl(8) <= 1)
            {
                return None;
            }
            let f = p.formation(u.formation_id).ok()??;
            (f.record.unit_ids == [u.id]).then(|| (u.clone(), f.record.clone()))
        })
        .next()
        .expect("concrete placed singleton MoveTo actor");
    let (baseline, formation) = actor;
    let mut stale = ComponentId {
        owner: baseline.e_owner as i16,
        component_type: 1,
        id: baseline.id,
    };
    stale.id ^= 0x10000;
    let r = UnitReference::from_component_id(stale);
    let configuration = state
        .binding()
        .inputs
        .configuration_sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    // Synthetic slot assertions exercise API/source control flow only. They are
    // deliberately NOT authentic live slot observations or native admission.
    let slots = [Some(ObservedHandler {
        runtime_id: HandlerId(0),
        operation_hash: OperationHash(SKIP_TURN_HASH),
        interruption_target: InterruptionTarget::LiveSlotObservation(0x2d95660),
    })];
    let hashes = [ObservedHashResolution {
        hash: OperationHash(SKIP_TURN_HASH),
        runtime_id: Some(HandlerId(0)),
    }];
    let handlers = ObservedRegistry::new(
        ObservationIdentity {
            gamecore_sha256: "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa",
            configuration_sha256: &configuration,
            capture_sha256: "1111111111111111111111111111111111111111111111111111111111111111",
        },
        &slots,
        &hashes,
    )
    .unwrap();
    let placements = bound.placement_definitions()?;
    let relations = snapshot.relations();
    let inputs = Inputs {
        transport: TransportScope::HeadlessLocalAuthority,
        movement: None,
        operations: bound.operations(),
        phases: bound.phases(),
        handlers: &handlers,
        placement_definitions: Some(&placements),
        relations: Some(&relations),
        plot_unit_limit: Some(snapshot.plot_unit_limit()),
    };
    let mut cases = vec![];
    assert!(state.is_pristine());
    let before_revision = state.revision();
    let actor_copy = state.unit_mut(local, baseline.id)?.clone();
    *state.unit_mut(local, baseline.id)? = actor_copy;
    assert!(state.revision() > before_revision);
    assert!(!state.is_pristine());
    assert!(bound.admit_observed_snapshot(&mut state).is_err());
    cases.push("once-only snapshot controls, owned mutable war facts and restored-byte stale re-admission rejection");
    // Busy old MoveTo must clear first; the only script query is activation.
    state.unit_mut(local, baseline.id)?.i_operation_timer = 7;
    let mut q = script(Mode::Accept);
    let mut reports = report_fixture(ReportMode::Collect);
    let trace =
        actions::skip_turn_state_phase(&mut state, r, &inputs, &mut q, &mut reports).unwrap();
    assert_eq!(q.calls, 1);
    assert_eq!(reports.calls, 9);
    assert_eq!(
        trace
            .iter()
            .filter_map(|e| if let Event::Report(r) = e {
                Some(r.event)
            } else {
                None
            })
            .collect::<Vec<_>>(),
        [
            0xe23b112d, 0xa60328c5, 0x383669f1, 0x15c35863, 0xe23b112d, 0x3d4006f2, 0xe23b112d,
            0xa60328c5, 0x383669f1
        ]
    );
    let u = state.world().unit(r)?.unwrap().record;
    assert!(u.k_operations.nodes.is_empty());
    assert_eq!(u.i_operation_timer, 0);
    assert_eq!(u.e_activity, c6_core::skip_turn::HOLD);
    assert_eq!(
        q.log,
        [
            "script_system",
            "args_begin",
            "utility",
            "has_lock",
            "utility",
            "release",
            "script",
            "utility",
            "acquire",
            "args_end"
        ]
    );
    cases.push("busy old MoveTo clears before single Manager/script activation query");
    // Native Execute missing-player/Unit returns before Add and all aftermath.
    for missing in [
        UnitReference::from_component_id(ComponentId {
            owner: 61,
            component_type: 1,
            id: baseline.id,
        }),
        UnitReference::from_component_id(ComponentId {
            owner: local as i16,
            component_type: 1,
            id: u32::MAX,
        }),
    ] {
        assert!(state.world().unit(missing)?.is_none());
        let mut q = script(Mode::Accept);
        let mut reports = report_fixture(ReportMode::Collect);
        let trace = actions::diagnose_skip_turn::<_, _, After<'_>, _>(
            &mut state,
            missing,
            &inputs,
            &mut q,
            &mut reports,
            |_| panic!("missing Unit must skip aftermath factory"),
            local,
            20,
        )
        .unwrap();
        assert!(trace.iter().any(|e| matches!(e, Event::UnitAbsent { .. })));
        assert_eq!((q.calls, reports.calls), (0, 0));
        assert!(!trace.contains(&Event::Stage(Stage::Clear)));
    }
    cases.push("invalid mapped player and missing Unit return before Add or aftermath factory");
    // A proven null requested handler is read after native Clear, not before it.
    restore(&mut state, &baseline, &formation);
    let null_slots = [None];
    let null_handlers = ObservedRegistry::new(handlers.identity(), &null_slots, &hashes).unwrap();
    let null_inputs = Inputs {
        transport: TransportScope::HeadlessLocalAuthority,
        movement: None,
        operations: bound.operations(),
        phases: bound.phases(),
        handlers: &null_handlers,
        placement_definitions: Some(&placements),
        relations: Some(&relations),
        plot_unit_limit: Some(snapshot.plot_unit_limit()),
    };
    let log = Rc::new(RefCell::new(vec![]));
    let next_log = Rc::clone(&log);
    let mut q = script(Mode::Accept);
    let mut reports = report_fixture(ReportMode::Collect);
    let fail = actions::diagnose_skip_turn(
        &mut state,
        r,
        &null_inputs,
        &mut q,
        &mut reports,
        |state| After {
            state,
            log: next_log,
            sessions: vec![],
        },
        local,
        20,
    )
    .unwrap_err();
    assert!(matches!(
        fail.error,
        ActError::Aftermath(Error::Unsupported(
            "intentional missing notification backend"
        ))
    ));
    assert!(fail.trace.contains(&Event::AddHandlerAbsent(0)));
    assert!(!fail.trace.contains(&Event::Stage(Stage::Insert)));
    assert_eq!((q.calls, reports.calls), (0, 3));
    assert_eq!(*log.borrow(), ["diplomacy", "notifications"]);
    assert!(state
        .world()
        .unit(r)?
        .unwrap()
        .record
        .k_operations
        .nodes
        .is_empty());
    cases.push("observed null handler preserves prior Clear and still enters post-Add aftermath");
    // Mutated definition-dependent wire storage cannot consume stale sidecars.
    restore(&mut state, &baseline, &formation);
    state.unit_mut(local, baseline.id)?.e_type_index ^= 1;
    let mut q = script(Mode::Accept);
    let mut reports = report_fixture(ReportMode::Collect);
    let fail =
        actions::skip_turn_state_phase(&mut state, r, &inputs, &mut q, &mut reports).unwrap_err();
    assert!(matches!(
        fail.error,
        ActError::State(c6_core::loaded_state::Error::RuntimeDependenciesChanged)
    ));
    assert_eq!((q.calls, reports.calls), (0, 0));
    assert!(!fail.trace.contains(&Event::Stage(Stage::Clear)));
    cases.push(
        "changed definition-dependent wire state rejects stale Unit runtime before execution",
    );
    // Known unsupported shape is rejected before callbacks and mutation.
    restore(&mut state, &baseline, &formation);
    state.unit_mut(local, baseline.id)?.k_operations.nodes[0].operation_hash = SKIP_TURN_HASH;
    let before = state.world().unit(r)?.unwrap().record.clone();
    let before_revision = state.revision();
    let mut q = script(Mode::Accept);
    let mut reports = report_fixture(ReportMode::Collect);
    let fail =
        actions::skip_turn_state_phase(&mut state, r, &inputs, &mut q, &mut reports).unwrap_err();
    assert!(matches!(
        fail.error,
        ActError::Kernel(c6_core::skip_turn::Error::UnsupportedQueue)
    ));
    assert_eq!(
        fail.trace,
        [
            Event::Stage(Stage::ExecuteLookup),
            Event::Stage(Stage::Preflight)
        ]
    );
    assert_eq!(state.world().unit(r)?.unwrap().record, &before);
    assert_eq!(state.revision(), before_revision);
    assert_eq!((q.calls, reports.calls), (0, 0));
    cases.push("preflight Unsupported has no callbacks or owned-record mutation");
    // A report error preserves actual prefix writes and stops before activation.
    restore(&mut state, &baseline, &formation);
    let damage = baseline.i_damage;
    let mut q = script(Mode::Accept);
    let mut reports = report_fixture(ReportMode::FailAdded);
    let fail =
        actions::skip_turn_state_phase(&mut state, r, &inputs, &mut q, &mut reports).unwrap_err();
    assert!(matches!(
        fail.error,
        ActError::Provider("intentional report failure")
    ));
    assert_eq!(q.calls, 0);
    let u = state.world().unit(r)?.unwrap().record;
    assert_eq!(u.i_damage, damage.wrapping_add(1));
    assert_eq!(u.k_operations.nodes[0].operation_hash, SKIP_TURN_HASH);
    assert!(!fail.trace.contains(&Event::Stage(Stage::Activation)));
    cases.push("report failure retains clear/insertion/callback mutation before activation");
    // A permitted activity callback must not route native Continue through Start.
    restore(&mut state, &baseline, &formation);
    let mut q = script(Mode::Accept);
    let mut reports = report_fixture(ReportMode::OperationAtAdded);
    let continued =
        actions::skip_turn_state_phase(&mut state, r, &inputs, &mut q, &mut reports).unwrap();
    assert_eq!(q.calls, 0);
    assert!(continued.contains(&Event::Stage(Stage::Continue)));
    assert!(!continued.contains(&Event::Stage(Stage::Start)));
    assert!(state
        .world()
        .unit(r)?
        .unwrap()
        .record
        .k_operations
        .nodes
        .is_empty());
    assert!(!state
        .formation_runtime(local, baseline.formation_id)?
        .evaluating_operation());
    cases.push(
        "report-selected OPERATION takes inherited SkipTurn Continue without Manager/Start query",
    );
    // Script scratch mutations are retained or returned explicitly, not discarded.
    for mode in [Mode::Activity, Mode::Identity, Mode::DamageVeto, Mode::Veto] {
        restore(&mut state, &baseline, &formation);
        let mut q = script(mode);
        let mut reports = report_fixture(ReportMode::Collect);
        let fail = actions::skip_turn_state_phase(&mut state, r, &inputs, &mut q, &mut reports)
            .unwrap_err();
        assert_eq!(q.calls, 1);
        match mode {
            Mode::Identity => match fail.error {
                ActError::ActorIdentityChanged(actor) => {
                    assert_eq!(actor.id, baseline.id ^ 0x10000);
                    assert_eq!(state.world().unit(r)?.unwrap().record.id, baseline.id);
                }
                _ => panic!("identity mutation not returned"),
            },
            Mode::Activity => {
                assert!(matches!(fail.error, ActError::Unsupported(_)));
                assert_eq!(
                    state.world().unit(r)?.unwrap().record.e_activity,
                    c6_core::skip_turn::HOLD
                );
            }
            Mode::DamageVeto => {
                assert!(matches!(fail.error, ActError::Unsupported(_)));
                assert_eq!(
                    state.world().unit(r)?.unwrap().record.i_damage,
                    baseline.i_damage.wrapping_add(1)
                );
            }
            Mode::Veto => assert!(matches!(fail.error, ActError::Unsupported(_))),
            _ => unreachable!(),
        }
    }
    cases.push(
        "script actor effects, identity payload and missing HasMoved false continuation explicit",
    );
    // With actual MaxMoves inputs, a genuine Manager veto follows the native
    // HasMoved/push-turn branch and then returns to the normal aftermath path.
    let veto_inputs = Inputs {
        transport: TransportScope::HeadlessLocalAuthority,
        movement: Some(&movement_inputs),
        operations: bound.operations(),
        phases: bound.phases(),
        handlers: &handlers,
        placement_definitions: Some(&placements),
        relations: Some(&relations),
        plot_unit_limit: Some(snapshot.plot_unit_limit()),
    };
    for (moved, old_turn) in [(true, None), (false, None), (true, Some(i32::MIN))] {
        restore(&mut state, &baseline, &formation);
        let max = c6_data::unit_movement_state::unit_max_moves(
            &state,
            state.world().unit(r)?.unwrap().record,
            &movement_inputs,
        )?;
        state.unit_mut(local, baseline.id)?.x_moves_remaining =
            max.wrapping_shl(8).wrapping_sub(i32::from(moved));
        let log = Rc::new(RefCell::new(vec![]));
        let next_log = Rc::clone(&log);
        let mut q = script(old_turn.map_or(Mode::Veto, Mode::OldVeto));
        let mut reports = report_fixture(ReportMode::Collect);
        let fail = actions::diagnose_skip_turn(
            &mut state,
            r,
            &veto_inputs,
            &mut q,
            &mut reports,
            |state| After {
                state,
                log: next_log,
                sessions: vec![],
            },
            local,
            20,
        )
        .unwrap_err();
        assert!(matches!(
            fail.error,
            ActError::Aftermath(Error::Unsupported(
                "intentional missing notification backend"
            ))
        ));
        assert_eq!(q.calls, 1);
        assert_eq!(*log.borrow(), ["diplomacy", "notifications"]);
        assert!(fail.trace.contains(&Event::Legality(false)));
        assert!(!fail.trace.contains(&Event::Stage(Stage::Start)));
        assert_eq!(
            state
                .world()
                .unit(r)?
                .unwrap()
                .record
                .k_operations
                .nodes
                .len(),
            usize::from(moved && old_turn.is_none())
        );
        assert_eq!(
            fail.trace.contains(&Event::Stage(Stage::Delete)),
            !(moved && old_turn.is_none())
        );
    }
    cases.push("activation veto uses actual HasMoved and signed current/old push turns to retain or pop before aftermath");
    // A veto callback can change the actual queue; unsupported dispatch shapes
    // remain in place and stop before guessed activation or aftermath.
    restore(&mut state, &baseline, &formation);
    let mut q = script(Mode::QueueVeto);
    let mut reports = report_fixture(ReportMode::Collect);
    let fail = actions::skip_turn_state_phase(&mut state, r, &veto_inputs, &mut q, &mut reports)
        .unwrap_err();
    assert!(matches!(fail.error, ActError::Unsupported(_)));
    assert_eq!(q.calls, 1);
    assert_eq!(
        state
            .world()
            .unit(r)?
            .unwrap()
            .record
            .k_operations
            .nodes
            .len(),
        2
    );
    assert!(!fail.trace.contains(&Event::Stage(Stage::Delete)));
    cases.push(
        "veto callback queue-shape mutation is retained and stops before unsupported continuation",
    );
    // Start's synchronous activity reread suppresses a redundant HOLD report.
    restore(&mut state, &baseline, &formation);
    let mut q = script(Mode::Accept);
    let mut reports = report_fixture(ReportMode::HoldAtStart);
    actions::skip_turn_state_phase(&mut state, r, &inputs, &mut q, &mut reports).unwrap();
    assert_eq!(q.calls, 1);
    assert_eq!(reports.calls, 8);
    cases.push(
        "full entrypoint rereads Start subscriber activity before equality-suppressed assignment",
    );
    // Busy empty queue inserts but performs no legality/script query. Add returns,
    // then diagnose must run aftermath even though the operation did not Start.
    restore(&mut state, &baseline, &formation);
    let u = state.unit_mut(local, baseline.id)?;
    u.k_operations.nodes.clear();
    u.e_activity = 0x6b25012e;
    u.i_operation_timer = 7;
    let log = Rc::new(RefCell::new(vec![]));
    let after_log = Rc::clone(&log);
    let mut q = script(Mode::Accept);
    let mut reports = report_fixture(ReportMode::Collect);
    let fail = actions::diagnose_skip_turn(
        &mut state,
        r,
        &inputs,
        &mut q,
        &mut reports,
        |state| After {
            state,
            log: after_log,
            sessions: vec![],
        },
        local,
        20,
    )
    .unwrap_err();
    assert!(matches!(
        fail.error,
        ActError::Aftermath(Error::Unsupported(
            "intentional missing notification backend"
        ))
    ));
    assert_eq!(q.calls, 0);
    assert_eq!(reports.calls, 1);
    assert_eq!(*log.borrow(), ["diplomacy", "notifications"]);
    assert!(fail.trace.contains(&Event::Stage(Stage::Diplomacy)));
    assert!(fail.trace.contains(&Event::Stage(Stage::Notifications)));
    let u = state.world().unit(r)?.unwrap().record;
    assert_eq!(u.i_operation_timer, 7);
    assert_eq!(u.k_operations.nodes.len(), 1);
    assert_eq!(u.k_operations.nodes[0].operation_hash, SKIP_TURN_HASH);
    cases.push(
        "busy empty insertion returns to real diagnose aftermath order; missing provider errors",
    );
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"source_only_full_entrypoint_cases":cases,"concrete_players":state.statistics().players,"concrete_units":state.statistics().units,"resolved_actor_owner":baseline.e_owner,"resolved_actor_id":baseline.id,"requested_stale_id":r.id,"genuine_callbacks_supplied":false,"native_execution":false,"native_parity":false,"full_affected_state_equality":false})
        )?
    );
    Ok(())
}
