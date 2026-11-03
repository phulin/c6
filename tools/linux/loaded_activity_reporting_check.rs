//! Actual raw handler adapter and source-registration queue comparison.
//! Runs the real first synchronous reporting consumer; later consumers fail explicitly.
#![allow(unused_variables)]
#[path = "loaded_actions_review_load.rs"]
mod fixture;
use c6_core::messages::{HandlerId, OperationHash};
use c6_core::skip_turn::legality::{ManagerQueries, ManagerQueryState, ScriptTestAllResult};
use c6_core::unit::{Unit, MOVE_TO_HASH, SKIP_TURN_HASH};
use c6_data::loaded_actions::{
    self as actions, handlers::Capture, resources::Resources, Error, IdentityReport,
};
struct MissingScripts;
impl ManagerQueries<Unit> for MissingScripts {
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
        panic!("actual wrapper must derive allow_tactical_commands from owned inputs")
    }
    fn player(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        owner: i32,
    ) -> Result<Self::Player, Self::Error> {
        panic!("actual wrapper must derive player from owned inputs")
    }
    fn turn_active(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        panic!("actual wrapper must derive turn_active from owned inputs")
    }
    fn processing_automation(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<i32, Self::Error> {
        panic!("actual wrapper must derive processing_automation from owned inputs")
    }
    fn player_id(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<i32, Self::Error> {
        panic!("actual wrapper must derive player_id from owned inputs")
    }
    fn game(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::Game, Self::Error> {
        panic!("actual wrapper must derive game from owned inputs")
    }
    fn player_human(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        player: Self::Player,
    ) -> Result<bool, Self::Error> {
        panic!("actual wrapper must derive player_human from owned inputs")
    }
    fn paused(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        game: Self::Game,
    ) -> Result<bool, Self::Error> {
        panic!("actual wrapper must derive paused from owned inputs")
    }
    fn runtime_id(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        hash: u32,
    ) -> Result<i32, Self::Error> {
        panic!("actual wrapper must derive runtime_id from owned inputs")
    }
    fn handler(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<Option<Self::Handler>, Self::Error> {
        panic!("actual wrapper must derive handler from owned inputs")
    }
    fn can_be_interrupted(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        handler: Self::Handler,
    ) -> Result<bool, Self::Error> {
        panic!("actual wrapper must derive can_be_interrupted from owned inputs")
    }
    fn operation_disabled(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        runtime_id: i32,
    ) -> Result<bool, Self::Error> {
        panic!("actual wrapper must derive operation_disabled from owned inputs")
    }
    fn script_system(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Option<Self::Script>, Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn args_begin(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::Args, Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn args_push_int(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        args: &mut Self::Args,
        value: i32,
    ) -> Result<(), Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn engine_utility(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Self::EngineUtility, Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn has_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<bool, Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn release_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn get_game_core_lock(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        utility: Self::EngineUtility,
    ) -> Result<(), Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn script_call_test_all(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        script: Self::Script,
        args: &mut Self::Args,
        event: &'static str,
        initial_value: bool,
    ) -> Result<ScriptTestAllResult, Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn args_end(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        args: Self::Args,
    ) -> Result<(), Self::Error> {
        Err("genuine script/lock/args backend unavailable")
    }
    fn unit_plot(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
    ) -> Result<Option<Self::Plot>, Self::Error> {
        panic!("actual wrapper must derive unit_plot from owned inputs")
    }
    fn handler_can_start(
        &mut self,
        s: &mut ManagerQueryState<'_, Unit, Self::Parameters>,
        handler: Self::Handler,
        plot: Self::Plot,
        visible_only: bool,
        results: Option<&mut Self::Results>,
    ) -> Result<bool, Self::Error> {
        panic!("actual wrapper must derive handler_can_start from owned inputs")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use c6_core::unit_initialization::InitializationInputs;
    use c6_data::{unit_initialization_definitions::Definitions, Rules, RulesProfile};
    let root = std::path::PathBuf::from(std::env::args_os().nth(1).ok_or("workspace root")?);
    let private = root
        .parent()
        .ok_or("workspace parent")?
        .join("c6-ghidra/evidence");
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
    let snapshot = bound.admit_observed_snapshot(&mut state)?;
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
    let report_definitions =
        actions::reporting::Definitions::from_rules(&state, &rules, &bound, &rules_runtime)?;
    let movement_definitions = c6_data::unit_movement_state::Definitions::from_rules(
        &state,
        &rules,
        &bound,
        &rules_runtime,
    )?;
    let movement = c6_data::unit_movement_state::Inputs {
        definitions: &movement_definitions,
    };
    let relation_definitions =
        c6_data::game_relations::Definitions::from_rules(&state, &rules, &bound, &rules_runtime)?;
    let relation_trace = std::fs::read(
        std::env::args_os()
            .nth(2)
            .ok_or("uncompressed relation trace")?,
    )?;
    relation_definitions.install_attributed_snapshot(
        &mut state,
        &relation_trace,
        c6_core::unit::Limits::default(),
    )?;
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
    assert_eq!(
        (
            state.statistics().players,
            state.statistics().units,
            state.statistics().formations
        ),
        (43, 1262, 1250)
    );
    let capture = Capture::from_closed_capture(
        &private.join("linux-current-handler-metadata/native-3"),
        &root
            .parent()
            .ok_or("parent")?
            .join("c6-assets-linux/libGameCore_XP2.so"),
    )?;
    let authority = capture.admit(&state, &bound)?;
    let registry = authority.registry()?;
    assert_ne!(
        registry.identity().configuration_sha256,
        decode_binding
            .inputs
            .configuration_sha256
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert_eq!(state.binding(), &decode_binding);
    assert!(registry
        .saved_hash_runtime_id(OperationHash(SKIP_TURN_HASH))
        .is_err());
    let mut nonempty = 0;
    let mut source_true = 0;
    let mut source_false = 0;
    for queue in authority.initial_runtime_queues() {
        if let Some(front) = queue.nodes.first() {
            nonempty += 1;
            let handler = registry
                .handler(front.runtime_id)
                .map_err(|e| format!("observed numeric lookup: {e:?}"))?
                .ok_or("observed nonnull handler")?;
            if handler.can_be_interrupted() {
                source_true += 1;
            } else {
                source_false += 1;
            }
        }
    }
    assert_eq!(
        (
            authority.initial_runtime_queues().len(),
            nonempty,
            source_true,
            source_false
        ),
        (1262, 308, 123, 185)
    );
    assert_eq!(authority.requested_skip_turn(), HandlerId(36));
    // Select through actual typed fields/collections, never a reused actor ID.
    let actor = state
        .world()
        .players()
        .iter()
        .filter(|p| p.slot_status == Some(3))
        .find_map(|p| {
            p.unit_records().ok()?.iter().find_map(|e| {
                let u = &e.value;
                let f = p.formation(u.formation_id).ok()??;
                (u.e_automation == -1
                    && !u.b_needs_movement_completion
                    && u.k_operations.nodes.len() == 1
                    && u.k_operations.nodes[0].operation_hash == MOVE_TO_HASH
                    && [0x490cc8e1, 0x6b25012e].contains(&u.e_activity)
                    && f.record.unit_ids == [u.id])
                .then_some((p.owner(), u.id))
            })
        })
        .ok_or("actual supported actor")?;
    let reference = c6_core::map::UnitReference {
        owner: actor.0 | (1 << 16),
        id: actor.1,
    };
    let placement = bound.placement_definitions()?;
    let war = snapshot.relations();
    let inputs = actions::Inputs {
        transport: actions::TransportScope::HeadlessLocalAuthority,
        operations: bound.operations(),
        phases: bound.phases(),
        handlers: &registry,
        placement_definitions: Some(&placement),
        relations: Some(&war),
        plot_unit_limit: Some(snapshot.plot_unit_limit()),
        movement: Some(&movement),
    };
    let initial_queue = state
        .world()
        .unit(reference)?
        .ok_or("actor")?
        .record
        .k_operations
        .clone();
    let mut script = MissingScripts;
    let mut reports = actions::reporting::Provider::new(
        &report_definitions,
        &movement,
        actions::reporting::Scope::StockHeadlessNoAddedConsumers,
    );
    reports.bind_skip_turn_handler(&state, &authority, bound.operations())?;
    let failure = actions::skip_turn_state_phase_observed(
        &mut state,
        reference,
        &inputs,
        &authority,
        &mut script,
        &mut reports,
    )
    .expect_err(
        "a genuine unsupported predicate or reporting consumer must stop this source fixture",
    );
    let error = format!("{:?}", failure.error);
    assert!(matches!(
        failure.error,
        Error::Provider("genuine script/lock/args backend unavailable")
    ));
    assert_eq!(reports.pending_publications().len(), 4);
    assert!(reports.pending_publications().len() >= 2);
    assert_eq!(reports.pending_publications()[0].host_event, 35);
    assert_eq!(reports.pending_publications()[1].host_event, 70);
    assert_eq!(reports.pending_publications()[1].operation, 1477390184);
    assert_eq!(reports.pending_publications()[1].value, 1);
    let observation = reports.snapshot_observations()[0];
    assert_eq!(observation.identity.owner, 0);
    assert_eq!(observation.identity.id, 15794187);
    assert_eq!(observation.current_activity, 0x6b25012e);
    assert_eq!(observation.queue_nodes, 1);
    assert_eq!(observation.remaining_q8, 256);
    assert_eq!(
        observation.path,
        actions::reporting::ReadinessPath::MemberNotReady
    );
    let completed = IdentityReport {
        event: 0xe23b112d,
        subtype: 0,
        identity: observation.identity,
        first: Some(0x6b25012e),
        second: None,
    };
    let publication_count = reports.pending_publications().len();
    let snapshot_count = reports.snapshot_observations().len();
    let revision = state.revision();
    let mut bad = completed;
    bad.subtype = 1;
    assert!(reports.send_loaded_report(&state, reference, bad).is_err());
    bad = completed;
    bad.identity.id ^= 1;
    assert!(reports.send_loaded_report(&state, reference, bad).is_err());
    assert!(reports.drain_playback().is_err());
    assert_eq!(reports.pending_publications().len(), publication_count);
    assert_eq!(reports.snapshot_observations().len(), snapshot_count);
    assert_eq!(state.revision(), revision);
    // Independent source projection cases after recording the genuine failure.
    // These explicit fixture setups are not action continuation and their rows
    // must never be appended to partial_actual_entrypoint_trace.
    let skip_queue = state
        .world()
        .unit(reference)?
        .ok_or("actor")?
        .record
        .k_operations
        .clone();
    assert_eq!(skip_queue.nodes.len(), 1);
    assert_eq!(skip_queue.nodes[0].operation_hash, SKIP_TURN_HASH);
    let captured: serde_json::Value = serde_json::from_str(include_str!(
        "../../evidence/linux/skip-turn-state-execute-3.json"
    ))?;
    let mut isolated = Vec::new();
    let expected_host = [35, 70, 71, 67, 35, 68, 35, 70, 71];
    let expected_operation = [
        0,
        MOVE_TO_HASH as i32,
        0,
        0,
        SKIP_TURN_HASH as i32,
        SKIP_TURN_HASH as i32,
        0,
        SKIP_TURN_HASH as i32,
        0,
    ];
    let expected_value = [-1, 1, 0, 36, -1, 0, -1, 0, 0];
    for (index, row) in captured["reporting_events"]
        .as_array()
        .ok_or("original report rows")?
        .iter()
        .enumerate()
    {
        let bits = u64::from_str_radix(row["type_bits"].as_str().ok_or("type bits")?, 16)?;
        let component =
            u64::from_str_radix(row["component_bits"].as_str().ok_or("component bits")?, 16)?;
        let report = IdentityReport {
            event: bits as u32,
            subtype: (bits >> 32) as u32,
            identity: c6_core::unit::ComponentId {
                owner: component as i16,
                component_type: (component >> 16) as i16,
                id: (component >> 32) as u32,
            },
            first: row
                .get("value")
                .or_else(|| row.get("first"))
                .and_then(serde_json::Value::as_i64)
                .map(|x| x as i32),
            second: row
                .get("second")
                .and_then(serde_json::Value::as_i64)
                .map(|x| x as i32),
        };
        let actor = state.unit_mut(actor.0, actor.1)?;
        actor.k_operations = if index < 2 {
            initial_queue.clone()
        } else {
            skip_queue.clone()
        };
        if index == 2 || index == 8 {
            actor.k_operations.nodes.clear();
        }
        actor.e_activity = match index {
            0..=3 => 0x6b25012e,
            4..=5 => 0x490cc8e1,
            _ => 0x047c048b,
        };
        let mut projection = actions::reporting::Provider::new(
            &report_definitions,
            &movement,
            actions::reporting::Scope::StockHeadlessNoAddedConsumers,
        );
        projection.bind_skip_turn_handler(&state, &authority, bound.operations())?;
        projection.send_loaded_report(&state, reference, report)?;
        assert_eq!(projection.pending_publications().len(), 1);
        let publication = projection.pending_publications()[0];
        assert_eq!(publication.host_event, expected_host[index]);
        assert_eq!(publication.operation, expected_operation[index]);
        assert_eq!(publication.value, expected_value[index]);
        if publication.host_event == 35 {
            assert_eq!(publication.activity_parameters, Some([-1; 4]));
        }
        isolated.push(serde_json::json!({"captured_row_index":index,"host_event":publication.host_event,"operation":publication.operation,"value":publication.value,"activity_operation":publication.activity_operation,"activity_parameters":publication.activity_parameters,"snapshot_paths":projection.snapshot_observations().iter().map(|s|format!("{:?}",s.path)).collect::<Vec<_>>(),"context_setup":"explicit isolated current-Unit queue/activity fixture; not action continuation"}));
    }
    assert_eq!(isolated.len(), 9);
    let publications = reports.pending_activity_publications();
    assert!(state.revision() > 0);
    assert!(capture.admit(&state, &bound).is_err());
    println!(
        "{}",
        serde_json::json!({"schema":"c6-linux-loaded-scoped-ordered-reporting-check-v3","players":43,"units":1262,"formations":1250,"actual_handler_slots":63,"initialized_runtime_queues":1262,"numeric_fronts_compared":308,"known_node_fields_compared":true,"source_leaf_true_fronts":source_true,"source_leaf_false_fronts":source_false,"empty_queues":954,"authority":authority.provenance(),"decode_role_unchanged":true,"source_actor":{"owner":actor.0,"id":actor.1},"partial_actual_entrypoint_trace":actions::trace_json(&failure.trace),"synchronous_activity_publications":publications.iter().map(|p|serde_json::json!({"owner":p.identity.owner,"id":p.identity.id,"activity":p.activity})).collect::<Vec<_>>(),"exact_remaining_consumer":error,"isolated_source_projection_cases":isolated,"isolated_cases_are_action_continuation":false,"callback_capability":"immutable authoritative World view and checked addressed actor only; wider World/RNG effects unsupported","host_publication_fields_have_native_capture_comparison":false,"ordered_publications":reports.pending_publications().iter().map(|p|serde_json::json!({"host_event":p.host_event,"operation":p.operation,"value":p.value,"activity_operation":p.activity_operation})).collect::<Vec<_>>(),"snapshot_paths":reports.snapshot_observations().iter().map(|s|serde_json::json!({"activity":s.current_activity,"queue_nodes":s.queue_nodes,"path":format!("{:?}",s.path)})).collect::<Vec<_>>(),"actual_snapshot_path":{"path":format!("{:?}",observation.path),"pending_nodes":observation.queue_nodes,"remaining_q8":observation.remaining_q8,"activity":observation.current_activity},"negative_shape_identity_playback_checks":true,"successful_callback_stubs_used":false,"original_execution":false,"native_predicate_or_action_comparison":false,"full_live_affected_state_rng_parity":false})
    );
    Ok(())
}
