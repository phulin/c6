//! Actual raw handler adapter and source-registration queue comparison.
//! Required gameplay callbacks remain unavailable; no successful no-op backend.
#![allow(unused_variables)]
#[path = "loaded_actions_review_load.rs"]
mod fixture;
use c6_core::messages::{HandlerId, OperationHash};
use c6_core::skip_turn::legality::{ManagerQueries, ManagerQueryState, ScriptTestAllResult};
use c6_core::unit::{Unit, MOVE_TO_HASH, SKIP_TURN_HASH};
use c6_data::loaded_actions::{
    self as actions, handlers::Capture, resources::Resources, Error, IdentityReport, Reports,
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

struct MissingReports {
    attempts: usize,
}
impl Reports for MissingReports {
    fn send(&mut self, unit: &mut Unit, report: IdentityReport) -> Result<i32, Error> {
        self.attempts += 1;
        Err(Error::Provider(
            "genuine synchronous reporting backend unavailable",
        ))
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
    let owner_trace = std::fs::read(std::env::args_os().nth(2).ok_or("attributed owner trace")?)?;
    actions::notifications::install_attributed_snapshot(
        &mut state,
        &owner_trace,
        c6_core::notifications::Limits::default(),
    )?;
    let saved_notifications = state.player_notifications()?;
    assert_eq!(saved_notifications.collections().len(), 64);
    assert_eq!(saved_notifications.collection(0)?.next_id, 49);
    assert_eq!(saved_notifications.collection(0)?.records.len(), 12);
    assert!(saved_notifications.collections()[1..]
        .iter()
        .all(|owner| owner.records.is_empty()));
    for (slot, owner) in saved_notifications.collections().iter().enumerate() {
        assert_eq!(owner.owner, slot as i32);
        for record in &owner.records {
            let first = owner
                .records
                .iter()
                .position(|r| r.id.id == record.id.id)
                .unwrap();
            assert_eq!(
                saved_notifications
                    .find_numeric_id(slot as i32, record.id.id)?
                    .unwrap()
                    .index(),
                first
            );
        }
    }
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
    let notification_definitions =
        actions::notifications::Definitions::from_rules(&rules, &bound, &rules_runtime)?;
    let notification_rows: Vec<_>=notification_definitions.ordered().iter().enumerate().map(|(index,d)|serde_json::json!({
        "collection_index":index,"row_id":d.row_id.0,"type_hash":d.type_hash,"notification_type":d.notification_type,
        "auto_activate":d.auto_activate,"auto_notify":d.auto_notify,"expires_end_of_next_turn":d.expires_end_of_next_turn,
        "expires_end_of_turn":d.expires_end_of_turn,"group_type":d.group_type,"icon":d.icon,"message":d.message,
        "severity_type":d.severity_type,"show_icon_single_player":d.show_icon_single_player,"sub_type":d.sub_type,
        "summary":d.summary,"visible_in_ui":d.visible_in_ui
    })).collect();
    for d in notification_definitions.ordered() {
        assert_eq!(notification_definitions.find(d.type_hash), Some(d));
    }
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
    let owner_queries = serde_json::json!({
        "owner":actor.0,
        "blocking":actions::notifications::owner_blocking(&state,actor.0)?,
        "turn_complete":actions::notifications::turn_complete(&state,actor.0)?,
        "previously_active_this_turn":actions::notifications::previously_active_this_turn(&state,actor.0)?
    });
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
        movement: None,
    };
    let initial_actor = state.world().unit(reference)?.ok_or("actor")?.record;
    let initial_actor_facts = serde_json::json!({
        "operation_timer":initial_actor.i_operation_timer,
        "is_busy_source_query":initial_actor.i_operation_timer > 0,
        "activity":initial_actor.e_activity,
        "moves_remaining_q8":initial_actor.x_moves_remaining,
        "automation":initial_actor.e_automation,
        "queue":initial_actor.k_operations.nodes.iter().map(|n|serde_json::json!({"hash":n.operation_hash,"push_turn":n.push_turn,"previous_activity":n.previous_activity,"previous_automation":n.previous_automation})).collect::<Vec<_>>()
    });
    let mut script = MissingScripts;
    let mut reports = MissingReports { attempts: 0 };
    let prefix_started = std::time::Instant::now();
    let failure = actions::skip_turn_state_phase_observed(
        &mut state,
        reference,
        &inputs,
        &authority,
        &mut script,
        &mut reports,
    )
    .expect_err("genuine report backend must remain unsupported");
    let prefix_elapsed_ns = prefix_started.elapsed().as_nanos();
    assert!(matches!(
        failure.error,
        Error::Provider("genuine synchronous reporting backend unavailable")
    ));
    assert_eq!(reports.attempts, 1);
    assert_eq!(
        state
            .world()
            .unit(reference)?
            .ok_or("actor")?
            .record
            .e_activity,
        0x6b25012e
    );
    let owner_range = state
        .player_notifications_source()
        .ok_or("notification source")?
        .owner_range;
    assert!(state.player_notifications()?.matches_source(
        &state.source_image()[owner_range[0]..owner_range[1]],
        c6_core::notifications::Limits::default()
    )?);
    let advanced_revision = state.revision();
    assert!(matches!(
        actions::notifications::install_attributed_snapshot(
            &mut state,
            &owner_trace,
            c6_core::notifications::Limits::default()
        ),
        Err(actions::notifications::Error::State(
            c6_core::loaded_state::Error::NotPristine
        ))
    ));
    assert_eq!(state.revision(), advanced_revision);
    // Isolated source-only setter probe after the action prefix has terminated.
    // This is not concatenated into an action continuation or native comparison.
    assert_eq!(state.retained().core.local_player, actor.0);
    let prior_blocking = actions::notifications::owner_blocking(&state, actor.0)?;
    let probe_blocking = actions::choose_tech::DEFAULT_BLOCKING;
    assert_ne!(prior_blocking, probe_blocking);
    let mut block_reports = 0;
    let error = actions::notifications::set_owner_blocking(
        &mut state,
        actor.0,
        probe_blocking,
        &mut |current, event, old, new| {
            block_reports += 1;
            assert_eq!(event, c6_core::unit_operation_aftermath::BLOCKING_CHANGED);
            assert_eq!((old, new), (prior_blocking, probe_blocking));
            assert_eq!(
                actions::notifications::owner_blocking(current, actor.0)?,
                new
            );
            Err(actions::notifications::Error::Unsupported(
                "genuine global blocking reporting backend unavailable",
            ))
        },
    )
    .expect_err("required global reporting backend remains unavailable");
    assert!(matches!(
        error,
        actions::notifications::Error::Unsupported(
            "genuine global blocking reporting backend unavailable"
        )
    ));
    assert_eq!(block_reports, 1);
    assert_eq!(
        actions::notifications::owner_blocking(&state, actor.0)?,
        probe_blocking
    );
    let same_revision = state.revision();
    actions::notifications::set_owner_blocking(
        &mut state,
        actor.0,
        probe_blocking,
        &mut |_, _, _, _| panic!("equal-value branch must not report"),
    )?;
    assert_eq!(state.revision(), same_revision);

    assert!(state.revision() > 0);
    assert!(capture.admit(&state, &bound).is_err());
    println!(
        "{}",
        serde_json::json!({"schema":"c6-linux-loaded-observed-handler-adapter-check-v1","players":43,"units":1262,"formations":1250,"actual_handler_slots":63,"owned_notification_collections":64,"owned_notification_records":12,"notification_first_duplicate_lookup_checked":true,"notification_reads_survive_unit_activity_write":true,"notification_replay_after_advancement_rejected":true,"notification_definitions":notification_rows,"initialized_runtime_queues":1262,"numeric_fronts_compared":308,"known_node_fields_compared":true,"source_leaf_true_fronts":source_true,"source_leaf_false_fronts":source_false,"empty_queues":954,"authority":authority.provenance(),"decode_role_unchanged":true,"source_actor":{"owner":actor.0,"id":actor.1},"owner_queries":owner_queries,"isolated_blocking_store_probe":{"reports_attempted":block_reports,"stored_value_retained_after_error":true,"equal_value_skips_reporting_and_revision":true,"is_action_continuation":false,"native_comparison":false},"initial_actor_facts":initial_actor_facts,"prefix_elapsed_ns":prefix_elapsed_ns,"timing_scope":"single diagnostic attempt ending at first unavailable report; excludes decode, rules, capture admission and all-roster comparisons; not full-action throughput","per_attempt_validation":"63 immutable definition/slot pairs; no module hashing or all-Unit scans","partial_actual_entrypoint_trace":actions::trace_json(&failure.trace),"required_reporting_backend_available":false,"successful_callback_stubs_used":false,"original_execution":false,"native_predicate_or_action_comparison":false,"full_live_affected_state_rng_parity":false})
    );
    Ok(())
}
