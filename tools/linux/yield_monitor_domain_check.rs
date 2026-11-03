//! Offline real owned-state admission/query check; no original execution.
#[path = "loaded_actions_review_load.rs"]
mod fixture;
use c6_data::loaded_actions::reporting::yield_monitor as monitor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args_os().nth(1).ok_or("root")?);
    let trace = std::fs::read(std::env::args_os().nth(2).ok_or("owner trace")?)?;
    let mut state = fixture::load_state(&root.parent().ok_or("parent")?.join(
        "c6-ghidra/evidence/linux-military-initialization/native-3/candidate-paths-v2.json",
    ))?;
    assert!(monitor::warm_route_domain(&state).is_err());
    let initial_revision = state.revision();
    let mut bad = trace.clone();
    bad[0] ^= 1;
    assert!(monitor::install_trade_graph_snapshot(&mut state, &bad, Default::default()).is_err());
    assert!(state.trade_graph_source().is_none());
    assert_eq!(state.revision(), initial_revision);
    monitor::install_trade_graph_snapshot(&mut state, &trace, Default::default())?;
    let before = monitor::warm_route_domain(&state)?;
    assert_eq!(
        (before.active_routes, before.queried_path_nodes),
        (182, 6193)
    );
    assert!(monitor::install_trade_graph_snapshot(&mut state, &trace, Default::default()).is_err());
    assert_eq!(monitor::warm_route_domain(&state)?, before);
    let (owner, id) = state
        .world()
        .players()
        .iter()
        .find_map(|player| {
            player
                .unit_records()
                .ok()?
                .first()
                .map(|unit| (player.owner(), unit.value.id))
        })
        .ok_or("unit")?;
    // A mutable Unit borrow increments the conservative revision. This check
    // demonstrates current graph authority, not unchanged monitor inputs.
    let activity = state
        .world()
        .players()
        .iter()
        .find(|p| p.owner() == owner)
        .ok_or("owner")?
        .unit(id)?
        .ok_or("unit")?
        .record
        .e_activity;
    state.unit_mut(owner, id)?.e_activity = activity;
    let after = monitor::warm_route_domain(&state)?;
    assert!(after.revision > before.revision);
    assert_eq!((after.active_routes, after.queried_path_nodes), (182, 6193));
    let source = state.trade_graph_source().ok_or("source")?;
    println!(
        "{}",
        serde_json::json!({
            "schema":"c6-linux-yield-monitor-owned-domain-v1",
            "actual_players":state.statistics().players,
            "actual_units":state.statistics().units,
            "graph_frame":source.owner_range,
            "graph_frame_bytes":source.decoded_prefix_bytes,
            "active_routes":before.active_routes,
            "actual_route_city_forward_path_nodes":before.queried_path_nodes,
            "before_revision":before.revision,
            "after_unit_borrow_revision":after.revision,
            "invalid_trace_atomic":true,
            "duplicate_install_rejected":true,
            "native_code_executed":false,
            "native_live_lookup_comparison":false,
            "cache_freshness_admitted":false,
            "monitor_omission_authorized":false,
            "numerical_yield_answers_replayed":false,
            "complete_action_acceptance":false
        })
    );
    Ok(())
}
