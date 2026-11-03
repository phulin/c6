//! Source-boundary check over admitted rules/retained PlayerTechs plus explicitly
//! synthetic graph cases. No original code or callback is executed.
#[path = "loaded_actions_review_load.rs"]
mod fixture;
use c6_core::notification_lifecycle::{ObjectId, PoolId};
use c6_core::unit::HashValue;
use c6_data::loaded_actions::{choose_tech as ct, notifications, resources::Resources};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args_os().nth(1).ok_or("workspace root")?);
    let private = root.parent().ok_or("parent")?.join("c6-ghidra/evidence");
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
    let _snapshot = bound.admit_observed_snapshot(&mut state)?;
    let profile = c6_data::RulesProfile::LinuxXp2ActivatedDiagnostic;
    let rules = c6_data::Rules::load_profile(
        profile,
        private.join("linux-rules/mod-activation-xp2-constructor-final-1"),
        &profile.source_identity()?,
    )?;
    let report = std::fs::read(
        private.join("linux-rules/mod-activation-xp2-constructor-final-1/report.json"),
    )?;
    let definitions = ct::Definitions::from_rules(&state, &rules, &bound, &report)?;
    let notifications = notifications::Definitions::from_rules(&rules, &bound, &report)?;
    let mut retained = Vec::new();
    for p in &state.retained().players {
        let q = definitions.player(&state, p.requested_owner, None)?;
        retained.push(serde_json::json!({"owner":p.requested_owner,"queue_hashes":p.techs.research_queue_hashes,"researching_native_index":definitions.researching_tech(&p.techs),"city_count":q.city_count,"human":q.human,"needed_without_native_graph":format!("{:?}",q.is_needed(true))}));
    }
    let benchmark = retained
        .iter()
        .find(|p| p["owner"] == 0)
        .ok_or("benchmark owner")?
        .clone();
    let trace_path =
        std::path::PathBuf::from(std::env::args_os().nth(2).ok_or("owner trace path")?);
    let trace = std::fs::read(trace_path)?;
    assert!(definitions
        .source_read_graph(
            &state,
            b"invalid trace",
            c6_core::unit::Limits {
                max_bytes: 1_000_000,
                max_items: 10_000
            }
        )
        .is_err());
    let graph_input = definitions.source_read_graph(
        &state,
        &trace,
        c6_core::unit::Limits {
            max_bytes: 1_000_000,
            max_items: 10_000,
        },
    )?;
    assert_eq!(graph_input.source(), ct::GraphSource::SourceReadNoRebuild);
    let source_witness = graph_input.source_witness().ok_or("graph source witness")?;
    assert_eq!(source_witness.snapshot_sha256, state.binding().core_sha256);
    let projected_owner = graph_input
        .projected_owner()
        .ok_or("retained complete graph")?;
    let graph_projection = serde_json::json!({"nodes":projected_owner.nodes().len(),"iteration":projected_owner.iteration().len(),"source_scope":"ReadValidate/nonemptyPostLoad; not final native runtime admission","owner_range":source_witness.owner_range,"raw_costs_retained":true,"fallback_for_missing_node":"unknown"});
    let source_needed = definitions
        .player(&state, 0, Some(&graph_input))?
        .is_needed(true)?;
    let mut benchmark = benchmark;
    benchmark["needed_with_source_read_graph"] = serde_json::json!(source_needed);
    let created = ct::create_auto(
        &notifications,
        true,
        Some(false),
        ct::CreatorBinding::ChooseTech,
        Some(ct::LogicalAllocation::Fresh(ObjectId(77))),
        PoolId(99),
        13,
    )?
    .ok_or("created")?;
    assert_eq!(created.record.type_hash, ct::TYPE_HASH);
    assert_eq!(created.record.factory_type, ct::TYPE_HASH);
    assert_eq!(
        created.record.id,
        c6_core::notifications::ComponentId {
            owner: 13,
            component_type: 38,
            id: u32::MAX
        }
    );
    assert_eq!(created.record.location, [-9999, -9999]);
    assert_eq!(created.record.native_188, -1);
    assert_eq!(created.record.native_18c, 0);
    assert!(
        !created.record.is_dismissed
            && !created.record.native_199
            && created.record.native_19a
            && created.record.native_19b
            && !created.record.native_19c
            && !created.record.native_19d
            && !created.record.has_been_activated
    );
    assert!(
        created.record.message.base.is_empty()
            && created.record.summary.base.is_empty()
            && created.record.parameters.entries.is_empty()
    );
    assert_eq!(created.live.lock_count, 0);
    assert_eq!(created.default_blocking, ct::DEFAULT_BLOCKING);
    assert_eq!(created.group_hash, 0xb6fa1284);
    let severity = created.record.severity;
    let mut prior = created.record.clone();
    prior.id.id = 123;
    prior.component.component_type = 7;
    prior.is_dismissed = true;
    prior.native_18c = 99;
    prior.native_19d = true;
    prior.native_190 = -7;
    prior.message.base = b"discarded".to_vec();
    let reused = ct::create_auto(
        &notifications,
        true,
        Some(false),
        ct::CreatorBinding::ChooseTech,
        Some(ct::LogicalAllocation::Reused {
            object: ObjectId(77),
            prior: &prior,
        }),
        PoolId(99),
        13,
    )?
    .ok_or("reused")?;
    assert_eq!(reused.record.id.id, 123);
    assert_eq!(reused.record.component.component_type, 7);
    assert_eq!(reused.record.component.owner, -1);
    assert_eq!(reused.record.component.id, u32::MAX);
    assert_eq!(reused.record.native_190, 1);
    assert_eq!(reused.record.native_18c, 0);
    assert!(
        !reused.record.is_dismissed
            && !reused.record.native_19d
            && reused.record.message.base.is_empty()
    );
    assert_eq!(prior.id.id, 123);
    assert!(prior.is_dismissed);
    assert!(ct::create_auto(
        &notifications,
        false,
        None,
        ct::CreatorBinding::Unavailable,
        None,
        PoolId(99),
        13
    )?
    .is_none());
    assert!(ct::create_auto(
        &notifications,
        true,
        Some(true),
        ct::CreatorBinding::Unavailable,
        None,
        PoolId(99),
        13
    )?
    .is_none());
    for creator in [
        ct::CreatorBinding::Unavailable,
        ct::CreatorBinding::BaseFallback,
    ] {
        assert!(ct::create_auto(
            &notifications,
            true,
            Some(false),
            creator,
            None,
            PoolId(99),
            13
        )
        .is_err());
    }
    assert!(ct::create_auto(
        &notifications,
        true,
        None,
        ct::CreatorBinding::ChooseTech,
        None,
        PoolId(99),
        13
    )
    .is_err());
    assert!(ct::create_auto(
        &notifications,
        true,
        Some(false),
        ct::CreatorBinding::ChooseTech,
        None,
        PoolId(99),
        13
    )
    .is_err());
    assert!(ct::create_auto(
        &notifications,
        true,
        Some(false),
        ct::CreatorBinding::ChooseTech,
        Some(ct::LogicalAllocation::Failed),
        PoolId(99),
        13
    )
    .is_err());
    let mut techs = state
        .retained()
        .players
        .first()
        .ok_or("player")?
        .techs
        .clone();
    techs.has_tech.pairs = (0..definitions.technology_count())
        .map(|i| HashValue {
            definition_hash: definitions.technology_hash(i).unwrap(),
            value: false,
        })
        .collect();
    techs.boost_triggered = techs.has_tech.clone();
    techs.research_queue_hashes.clear();
    let hash0 = definitions.technology_hash(0).ok_or("technology0")?;
    let graph = ct::GameTechs::from_complete_native(
        state.binding().clone(),
        vec![0, 1],
        vec![
            ct::TechNode {
                native_type: 0,
                active: true,
                prerequisites: vec![],
            },
            ct::TechNode {
                native_type: 1,
                active: true,
                prerequisites: vec![0],
            },
        ],
    )?;
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: None,
        human: None,
        graph: None,
    };
    assert!(q.is_needed(false)?);
    assert!(!q.can_research(-1)?);
    assert!(!q.can_research(i32::MAX)?);
    let q = ct::Research {
        city_count: Some(0),
        ..q
    };
    assert!(!q.is_needed(true)?);
    let q = ct::Research {
        city_count: Some(1),
        human: Some(false),
        ..q
    };
    assert!(!q.is_needed(true)?);
    let q = ct::Research {
        human: Some(true),
        ..q
    };
    assert!(q.is_needed(true).is_err());
    let q = ct::Research {
        graph: Some(&graph),
        ..q
    };
    assert!(q.is_needed(true)?);
    assert!(q.can_research(0)?);
    assert!(!q.can_research(1)?);
    techs.research_queue_hashes = vec![hash0];
    assert_eq!(definitions.researching_tech(&techs), 0);
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: Some(1),
        human: Some(true),
        graph: Some(&graph),
    };
    assert!(!q.is_needed(true)?);
    techs.has_tech.pairs[0].value = true;
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: Some(1),
        human: Some(true),
        graph: Some(&graph),
    };
    assert!(q.is_needed(true)?);
    assert!(!q.can_research(0)?);
    assert!(q.can_research(1)?);
    let hidden = rules
        .gameplay()
        .table("Technologies_XP2")
        .ok_or("XP2")?
        .rows()
        .find(|r| r.integer("HiddenUntilPrereqComplete").ok() == Some(1))
        .ok_or("hidden technology")?
        .text("TechnologyType")?;
    let hidden_hash = c6_data::registry::linux_type_hash(hidden.as_bytes());
    let hi = (0..definitions.technology_count())
        .find(|i| definitions.technology_hash(*i) == Some(hidden_hash))
        .ok_or("hidden index")?;
    let hidden_graph = ct::GameTechs::from_complete_native(
        state.binding().clone(),
        vec![hi as i32],
        vec![
            ct::TechNode {
                native_type: 0,
                active: true,
                prerequisites: vec![],
            },
            ct::TechNode {
                native_type: 1,
                active: true,
                prerequisites: vec![],
            },
            ct::TechNode {
                native_type: hi as i32,
                active: true,
                prerequisites: vec![0, 1],
            },
        ],
    )?;
    techs
        .has_tech
        .pairs
        .iter_mut()
        .for_each(|p| p.value = false);
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: Some(1),
        human: Some(true),
        graph: Some(&hidden_graph),
    };
    assert!(!q.revealed(hi as i32)?);
    assert!(!q.can_research(hi as i32)?);
    techs.has_tech.pairs[0].value = true;
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: Some(1),
        human: Some(true),
        graph: Some(&hidden_graph),
    };
    assert!(q.revealed(hi as i32)?);
    assert!(!q.can_research(hi as i32)?);
    techs.has_tech.pairs[1].value = true;
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: Some(1),
        human: Some(true),
        graph: Some(&hidden_graph),
    };
    assert!(q.can_research(hi as i32)?);
    // Hidden reveal's boost short circuit is separate from all-prerequisite legality.
    techs
        .has_tech
        .pairs
        .iter_mut()
        .for_each(|p| p.value = false);
    techs.boost_triggered.pairs[hi].value = true;
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: Some(1),
        human: Some(true),
        graph: Some(&hidden_graph),
    };
    assert!(q.revealed(hi as i32)?);
    assert!(!q.can_research(hi as i32)?);
    let repeat_index = rules
        .gameplay()
        .table("Technologies")
        .ok_or("technology table")?
        .rows()
        .position(|r| r.integer("Repeatable").ok() == Some(1))
        .ok_or("repeatable technology")?;
    techs.has_tech.pairs[repeat_index].value = true;
    let repeat_graph = ct::GameTechs::from_complete_native(
        state.binding().clone(),
        vec![repeat_index as i32],
        vec![ct::TechNode {
            native_type: repeat_index as i32,
            active: true,
            prerequisites: vec![],
        }],
    )?;
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: Some(1),
        human: Some(true),
        graph: Some(&repeat_graph),
    };
    assert!(q.more_to_research()?);
    assert!(q.can_research(repeat_index as i32)?);
    let empty_graph = ct::GameTechs::from_complete_native(state.binding().clone(), vec![], vec![])?;
    techs.has_tech.pairs.clear();
    let q = ct::Research {
        definitions: &definitions,
        techs: &techs,
        city_count: Some(1),
        human: Some(true),
        graph: Some(&empty_graph),
    };
    assert!(!q.is_needed(true)?); // Native MoreToResearch false avoids CanResearch.
    let q = ct::Research {
        graph: Some(&graph),
        ..q
    };
    assert!(q.has_tech(0).is_err());
    println!(
        "{}",
        serde_json::json!({"schema":"c6-linux-choose-tech-notification-check-v1","original_execution":false,"technology_definitions":definitions.technology_count(),"created_severity":severity,"created_default_blocking":created.default_blocking,"created_group_hash":created.group_hash,"benchmark":benchmark,"game_techs_projection":graph_projection,"retained_players":retained,"synthetic_graph_cases_pass":true,"logical_creation_cases_pass":true,"native_factory_pool_or_graph_admitted":false,"complete_state_actions_rng_turns":false})
    );
    Ok(())
}
