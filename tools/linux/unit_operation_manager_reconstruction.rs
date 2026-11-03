//! Strict source-state reconstruction candidate; never executes original code.
use c6_core::core_player::PlayerHeader;
use c6_core::unit::{Limits, PlotCoord, Unit};
use c6_core::world::{Player, World};
use c6_data::unit_operation_manager::{
    OperationDefinition, OperationDefinitions, PhaseDefinitions, RuntimeOperationId, WorldPlayer,
};
use c6_data::{Rules, RulesProfile};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const GAME: &str = "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa";
const HOST: &str = "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4";
const CORE: &str = "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884";
const TRACE: &str = "528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae";
type Result<T> = std::result::Result<T, String>;
fn require(ok: bool, reason: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(reason.into())
    }
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn identity(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    path: String,
    bytes: usize,
    sha256: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    collection_index: u32,
    hash: u32,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    hash: u32,
    runtime_id: i32,
    handler_present: bool,
    handler_id: Option<i32>,
    handler_hash: Option<u32>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Controls {
    pause_raw: u8,
    automation_player_raw: i32,
    script_system_present: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerSlot {
    query: i32,
    mapped_index: i32,
    stored_player_present: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitMember {
    ordinal: u32,
    id: u32,
    wire: Blob,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerInput {
    query: i32,
    slot_status: i32,
    header: Blob,
    units: Vec<UnitMember>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema: String,
    gamecore_sha256: String,
    host_sha256: String,
    configuration_sha256: String,
    profile_sha256: String,
    core_sha256: String,
    trace_sha256: String,
    map: Blob,
    rules_directory: String,
    operation_definitions: Vec<Definition>,
    registrations: Vec<Registration>,
    configured_phase_hash: u32,
    current_phase: i32,
    controls: Controls,
    player_slots: Vec<PlayerSlot>,
    players: Vec<PlayerInput>,
}
fn contained(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    require(
        !relative.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_))),
        "unsafe input path",
    )?;
    let joined = root
        .join(path)
        .canonicalize()
        .map_err(|e| format!("input path: {e}"))?;
    require(joined.starts_with(root), "input symlink escapes root")?;
    Ok(joined)
}
fn blob(root: &Path, input: &Blob, maximum: usize) -> Result<Vec<u8>> {
    require(
        input.bytes <= maximum && identity(&input.sha256),
        "blob extent/identity",
    )?;
    let path = contained(root, &input.path)?;
    require(path.is_file(), "blob is not a regular file")?;
    let bytes = fs::read(path).map_err(|e| format!("blob read: {e}"))?;
    require(
        bytes.len() == input.bytes && sha(&bytes) == input.sha256,
        "blob digest/length mismatch",
    )?;
    Ok(bytes)
}
fn header(bytes: &[u8]) -> Result<PlayerHeader> {
    require(bytes.len() == 98, "Player47 header must exhaust 98 bytes")?;
    let mut at = 0;
    let word = |at: &mut usize| {
        let value = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap());
        *at += 4;
        value
    };
    require(word(&mut at) == 47, "Player header version")?;
    let owner = word(&mut at) as i32;
    let starting_plot = PlotCoord {
        x: word(&mut at) as i32,
        y: word(&mut at) as i32,
    };
    let ordered_scalars = std::array::from_fn(|_| word(&mut at) as i32);
    let mut ordered_flags = [false; 6];
    for value in &mut ordered_flags {
        require(bytes[at] <= 1, "noncanonical header bool")?;
        *value = bytes[at] != 0;
        at += 1;
    }
    let value = PlayerHeader {
        owner,
        starting_plot,
        ordered_scalars,
        ordered_flags,
        handicap_hash: word(&mut at),
        district_hash: word(&mut at),
        project_hash: word(&mut at),
        final_scalar: word(&mut at) as i32,
    };
    require(at == bytes.len(), "header tail")?;
    Ok(value)
}
fn reconstruct(input: Input, root: &Path) -> Result<Value> {
    require(
        input.schema == "c6-linux-unit-operation-manager-input-v1",
        "input schema",
    )?;
    require(
        input.gamecore_sha256 == GAME && input.host_sha256 == HOST,
        "target identity",
    )?;
    require(
        input.core_sha256 == CORE && input.trace_sha256 == TRACE,
        "unverified Core/owner trace pair",
    )?;
    require(
        identity(&input.configuration_sha256) && input.profile_sha256 == input.configuration_sha256,
        "configuration/profile identity",
    )?;
    require(
        input.operation_definitions.len() <= 65536 && !input.operation_definitions.is_empty(),
        "definition extent",
    )?;
    let entries: Vec<_> = input
        .operation_definitions
        .iter()
        .map(|d| OperationDefinition {
            collection_index: d.collection_index,
            hash: d.hash,
        })
        .collect();
    let definitions =
        OperationDefinitions::from_native_order(&entries).map_err(|e| e.to_string())?;
    require(
        input.registrations.len() == entries.len(),
        "registration coverage",
    )?;
    let mut definition_rows = Vec::new();
    for (i, (entry, registration)) in entries.iter().zip(&input.registrations).enumerate() {
        require(registration.hash == entry.hash, "registration order/hash")?;
        if registration.runtime_id != -1 {
            require(
                registration.runtime_id >= 0
                    && (registration.runtime_id as usize) < definitions.count(),
                "registration ID extent",
            )?;
            require(
                definitions.find_index(entry.hash)
                    == Some(RuntimeOperationId(registration.runtime_id)),
                "registration ID/definition mismatch",
            )?;
        }
        if registration.handler_present {
            require(
                registration.runtime_id >= 0
                    && registration.handler_id == Some(registration.runtime_id)
                    && registration.handler_hash == Some(entry.hash),
                "actual handler identity mismatch",
            )?;
        } else {
            require(
                registration.handler_id.is_none() && registration.handler_hash.is_none(),
                "absent handler has fields",
            )?;
        }
        let numeric = definitions.find_index(i as u32).map_or(-1, |v| v.0);
        let hash = definitions.find_index(entry.hash).map_or(-1, |v| v.0);
        definition_rows.push(json!({"ordinal":i,"collection_index":entry.collection_index,"hash":entry.hash,"numeric_find_index":numeric,"hash_find_index":hash,"numeric_hash_to_index":numeric,"hash_hash_to_index":hash}));
    }
    require(
        input.player_slots.len() == 65,
        "complete 65-slot roster required",
    )?;
    let mut slot_rows = Vec::new();
    let mut resolved_queries = Vec::new();
    for (query, slot) in input.player_slots.iter().enumerate() {
        require(slot.query == query as i32, "player slot order/duplicate")?;
        let valid = slot.mapped_index != -1;
        let resolved = valid && slot.stored_player_present;
        if resolved {
            resolved_queries.push(slot.query);
        }
        slot_rows.push(json!({"query":slot.query,"mapping_valid":valid,"stored_player_present":slot.stored_player_present,"resolved_player_present":resolved}));
    }
    require(
        resolved_queries.len() == 43
            && input
                .players
                .iter()
                .map(|p| p.query)
                .eq(resolved_queries.iter().copied()),
        "complete ordered 43-player coverage",
    )?;
    let map_bytes = blob(root, &input.map, 64 * 1024 * 1024)?;
    let map = c6_core::map::Map::decode(&map_bytes, c6_core::map::Limits::default())
        .map_err(|e| format!("map: {e}"))?;
    let mut players = Vec::new();
    let mut headers = Vec::new();
    for p in &input.players {
        let h = header(&blob(root, &p.header, 98)?)?;
        require(h.owner == p.query, "Player owner/header mismatch")?;
        headers.push(h);
        let mut player = Player::new(p.query, None, None).map_err(|e| e.to_string())?;
        player.slot_status = Some(p.slot_status);
        players.push(player);
    }
    let mut world = World::new(map, players).map_err(|e| e.to_string())?;
    world.player_mapping = Some(std::array::from_fn(|i| input.player_slots[i].mapped_index));
    world.player_slot_present = Some(std::array::from_fn(|i| {
        input.player_slots[i].stored_player_present
    }));
    let mut player_rows = Vec::new();
    let mut unit_rows = Vec::new();
    let mut all_ids = BTreeSet::new();
    for (p, h) in input.players.iter().zip(&headers) {
        let player = WorldPlayer::resolve(&world, p.query, Some(h)).map_err(|e| e.to_string())?;
        require(p.units.len() <= 65536, "unit roster bound")?;
        let mut slots = BTreeSet::new();
        for (ordinal, member) in p.units.iter().enumerate() {
            require(
                member.ordinal as usize == ordinal
                    && member.id != u32::MAX
                    && slots.insert(member.id & 65535)
                    && all_ids.insert((p.query, member.id)),
                "Unit ordinal/identity/slot duplicate",
            )?;
            let wire = blob(root, &member.wire, 64 * 1024 * 1024)?;
            let unit = Unit::decode(&wire, Limits::default()).map_err(|e| format!("Unit: {e}"))?;
            require(
                unit.e_owner == p.query && unit.id == member.id,
                "Unit wire/native membership identity",
            )?;
            let disabled = definitions
                .restore_unit_disabled(&unit)
                .map_err(|e| e.to_string())?;
            require(
                unit.encode(Limits::default()).map_err(|e| e.to_string())? == wire,
                "Unit changed/noncanonical",
            )?;
            unit_rows.push(json!({"owner":p.query,"ordinal":ordinal,"id":unit.id,"size":disabled.values().len(),"default_raw":0,"values_raw":disabled.values().iter().map(|v|if *v {"01"}else{"00"}).collect::<String>(),"status":"supported_fresh_read"}));
        }
        player_rows.push(json!({"query":p.query,"actual_id":player.id(),"human":player.is_human().map_err(|e|e.to_string())?,"saved_turn_active_raw":u8::from(player.is_turn_active().map_err(|e|e.to_string())?),"unit_count":p.units.len()}));
    }
    require(
        unit_rows.len() == 1262,
        "complete ordered 1262-unit coverage required",
    )?;
    let rules_path = contained(root, &input.rules_directory)?;
    let profile = RulesProfile::LinuxXp2ActivatedDiagnostic;
    let rules = Rules::load_profile(
        profile,
        rules_path,
        &profile.source_identity().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let phase = match PhaseDefinitions::from_rules(&rules, input.configured_phase_hash) {
        Ok(phases) => {
            let rows:Vec<_>=phases.slots().iter().enumerate().map(|(i,p)| {let p=p.expect("from_rules creates nonnull phases");json!({"slot":i,"rowid":p.row_id.0,"type_hash":p.phase_type_hash,"phase_order":p.phase_order,"active_segment_present":p.active_segment.is_some(),"segment_rowid":p.active_segment.map(|s|s.row_id.0),"segment_flags_raw":p.active_segment.map(|s|s.flags)})}).collect();
            // Permission is source-derived only; metadata helper does not query
            // native permission and cannot validate this as native parity.
            let permission = match phases.allow_tactical_commands(input.current_phase, 0) {
                Ok(v) => json!({"status":"source_only","value":v}),
                Err(e) => json!({"status":"unsupported","reason":e.to_string()}),
            };
            json!({"status":"supported_unique_order","slots":rows,"current_phase":input.current_phase,"tactical_permission":permission})
        }
        Err(e) => {
            json!({"status":"unsupported","reason":e.to_string(),"current_phase":input.current_phase})
        }
    };
    let controls = c6_data::unit_operation_manager::LiveControlFields::new(
        input.controls.automation_player_raw,
        input.controls.pause_raw,
    )
    .map_err(|e| e.to_string())?;
    Ok(
        json!({"schema":"c6-linux-unit-operation-manager-reconstruction-v1","configuration_sha256":input.configuration_sha256,"profile_sha256":input.profile_sha256,"core_sha256":input.core_sha256,"trace_sha256":input.trace_sha256,"operation_definitions":definition_rows,"player_slots":slot_rows,"players":player_rows,"units":unit_rows,"phases":phase,"controls":{"automation_player_raw":controls.player_processing_automation,"pause_raw":u8::from(controls.is_paused()),"script_system_present":input.controls.script_system_present,"script_callback_support":"unsupported_not_inspected"},"complete_player_slots":65,"resolved_players":43,"units_reconstructed":1262,"expected_outputs_used_as_inputs":false,"native_execution":false,"native_predicate_comparison":false,"full_manager_support":false,"complete_state_actions_rng_turns":false}),
    )
}
fn main() {
    let run = || -> Result<Value> {
        let args: Vec<_> = std::env::args_os().collect();
        require(args.len() == 2, "usage: candidate INPUT.json")?;
        let path = Path::new(&args[1])
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let raw = fs::read(&path).map_err(|e| e.to_string())?;
        require(raw.len() <= 64 * 1024 * 1024, "request byte cap")?;
        let input: Input =
            serde_json::from_slice(&raw).map_err(|e| format!("strict input: {e}"))?;
        reconstruct(input, path.parent().ok_or("request directory")?)
    };
    match run() {
        Ok(value) => println!("{value}"),
        Err(e) => {
            eprintln!("unsupported/malformed manager reconstruction: {e}");
            std::process::exit(2);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_protocol_rejects_expected_targets_unknown_and_duplicate_fields() {
        let raw = r#"{"path":"x","bytes":1,"sha256":"a","values_raw":"00"}"#;
        assert!(serde_json::from_str::<Blob>(raw).is_err());
        assert!(
            serde_json::from_str::<Blob>(r#"{"path":"x","path":"y","bytes":1,"sha256":"a"}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<Controls>(r#"{"pause_raw":0,"automation_player_raw":-1,"script_system_present":false,"live_disabled":[]}"#).is_err());
        assert!(serde_json::from_str::<Definition>(
            r#"{"collection_index":0,"hash":99,"numeric_find_index":0}"#
        )
        .is_err());
        let mut minimal = json!({
            "schema":"x","gamecore_sha256":GAME,"host_sha256":HOST,
            "configuration_sha256":GAME,"profile_sha256":GAME,
            "core_sha256":CORE,"trace_sha256":TRACE,
            "map":{"path":"map.bin","bytes":1,"sha256":GAME},
            "rules_directory":"rules","operation_definitions":[],"registrations":[],
            "configured_phase_hash":0,"current_phase":-1,
            "controls":{"pause_raw":0,"automation_player_raw":-1,"script_system_present":false},
            "player_slots":[],"players":[]
        });
        assert!(serde_json::from_value::<Input>(minimal.clone()).is_ok());
        for field in ["phases", "unit_disabled", "live_turn_active", "expected"] {
            minimal[field] = json!([]);
            let error = serde_json::from_value::<Input>(minimal.clone()).unwrap_err();
            assert!(error.to_string().contains("unknown field"));
            minimal.as_object_mut().unwrap().remove(field);
        }
    }
    #[test]
    fn header_exact_caps_versions_bools_and_signed_bits() {
        let mut raw = vec![0; 98];
        raw[..4].copy_from_slice(&47u32.to_le_bytes());
        raw[4..8].copy_from_slice(&2i32.to_le_bytes());
        raw[78] = 1;
        let h = header(&raw).unwrap();
        assert_eq!(h.owner, 2);
        assert!(h.ordered_flags[2]);
        assert!(header(&raw[..97]).is_err());
        raw.push(0);
        assert!(header(&raw).is_err());
        raw.pop();
        raw[78] = 2;
        assert!(header(&raw).is_err());
        raw[78] = 0;
        raw[0] = 46;
        assert!(header(&raw).is_err());
    }
    #[test]
    fn identity_and_relative_path_boundaries() {
        assert!(identity(GAME));
        assert!(!identity(&GAME.to_uppercase()));
        assert!(!identity("0"));
        let root = std::env::temp_dir().canonicalize().unwrap();
        for path in ["", "../outside", "/etc/passwd", "./x"] {
            assert!(contained(&root, path).is_err());
        }
    }
}
