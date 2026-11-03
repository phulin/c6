//! Strict offline placement-count reconstruction. Native answers are not inputs.
use c6_core::{
    map::{self, Slot},
    movement_world::DefinitionTable,
    skip_turn::{
        legality,
        placement::{self, Inputs, UnitProperties, WarRelations},
    },
    unit::{self, Formation, Unit},
    world::{CompleteSlots, Player, World},
};
use c6_data::registry::LinuxDefinitionHashRegistry;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};
const GAME: &str = "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa";
const HOST: &str = "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4";
fn digest(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn require(value: bool, reason: &str) -> Result<(), Box<dyn std::error::Error>> {
    if value {
        Ok(())
    } else {
        Err(reason.into())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    path: String,
    bytes: usize,
    sha256: String,
}
impl Blob {
    fn read(&self, root: &Path, budget: &mut usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        require(
            self.bytes <= 128 * 1024 * 1024 && self.bytes <= *budget,
            "blob byte budget",
        )?;
        let path = Path::new(&self.path);
        require(
            path.components().all(|v| matches!(v, Component::Normal(_))),
            "blob path must be relative normal components",
        )?;
        let actual = root.join(path).canonicalize()?;
        require(actual.starts_with(root), "blob escapes input directory")?;
        let mut data = Vec::new();
        fs::File::open(actual)?
            .take(self.bytes as u64 + 1)
            .read_to_end(&mut data)?;
        require(
            data.len() == self.bytes && digest(&data) == self.sha256,
            "blob identity differs",
        )?;
        *budget -= self.bytes;
        Ok(data)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    ordinal: usize,
    id: u32,
    slot: u32,
    actual_owner: i32,
    wire: Blob,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Membership {
    count: usize,
    complete: bool,
    entries: Vec<Member>,
}
impl Membership {
    fn check(&self, owner: i32) -> Result<(), Box<dyn std::error::Error>> {
        require(
            self.complete && self.count == self.entries.len() && self.count <= 65536,
            "incomplete membership",
        )?;
        for (i, m) in self.entries.iter().enumerate() {
            require(
                m.ordinal == i
                    && m.actual_owner == owner
                    && m.id != u32::MAX
                    && m.slot == m.id & 65535,
                "member ordering/identity/slot",
            )?;
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerInput {
    owner: i32,
    slot_status: i32,
    team: i32,
    alive: bool,
    units: Membership,
    formations: Membership,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    ordinal: usize,
    index: usize,
    hash: u32,
    domain_hash: u32,
    formation_class_hash: u32,
    make_trade_route: bool,
    spy: bool,
    combat: i32,
    range: i32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definitions {
    count: usize,
    complete: bool,
    entries: Vec<Definition>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Relations {
    player_count: i32,
    war_types: Vec<i32>,
    complete: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    gamecore_sha256: String,
    host_sha256: String,
    configuration_sha256: String,
    profile_sha256: String,
    map: Blob,
    player_mapping: Vec<i32>,
    player_slot_present: Vec<bool>,
    players: Vec<PlayerInput>,
    unit_definitions: Definitions,
    relations: Relations,
    plot_unit_limit: i32,
}
fn count_row(
    identity: (&str, i32, usize, u32),
    anchor: unit::PlotCoord,
    layer: Option<u32>,
    direction: i32,
    plot: Option<unit::PlotCoord>,
    count: Option<u32>,
) -> Value {
    let (kind, owner, ordinal, id) = identity;
    let mut v = json!({"kind":"count_result","member_kind":kind,"owner":owner,"ordinal":ordinal,"id":id,
  "anchor":[anchor.x,anchor.y],"anchor_valid":placement::coordinate_valid(anchor),"layer":layer.map_or(-1,|v|v as i32),
  "direction":direction,"plot_present":plot.is_some()});
    if let Some(p) = plot {
        v["plot"] = json!([p.x, p.y]);
        v["bool_argument"] = json!(false);
        v["count"] = json!(count.expect("present plot has computed count"));
    }
    v
}
fn run(
    request: Request,
    root: &Path,
    config: &str,
    profile: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    require(
        request.schema == "c6-linux-placement-inputs-v1"
            && request.gamecore_sha256 == GAME
            && request.host_sha256 == HOST,
        "unsupported input schema/module",
    )?;
    require(
        valid_sha(config)
            && valid_sha(profile)
            && request.configuration_sha256 == config
            && request.profile_sha256 == profile,
        "external configuration/profile gate differs",
    )?;
    require(
        request.player_mapping.len() == 65
            && request.player_slot_present.len() == 65
            && request.players.len() <= 65,
        "player slot dimensions",
    )?;
    require(
        request.unit_definitions.complete
            && request.unit_definitions.count == request.unit_definitions.entries.len()
            && request.unit_definitions.count <= 1_000_000,
        "definition collection incomplete",
    )?;
    for (i, d) in request.unit_definitions.entries.iter().enumerate() {
        require(
            d.ordinal == i && d.index == i,
            "native definition order/index",
        )?;
    }
    let registry = LinuxDefinitionHashRegistry::from_native_order(
        request
            .unit_definitions
            .entries
            .iter()
            .map(|d| d.hash)
            .collect(),
    );
    let properties: Vec<_> = request
        .unit_definitions
        .entries
        .iter()
        .map(|d| {
            Some(UnitProperties {
                domain_hash: d.domain_hash,
                formation_class_hash: d.formation_class_hash,
                make_trade_route: d.make_trade_route,
                spy: d.spy,
                combat: d.combat,
                range: d.range,
            })
        })
        .collect();
    let definitions = DefinitionTable::new(&registry, &properties)?;
    require(
        request.relations.complete && (0..=65).contains(&request.relations.player_count),
        "war relation collection unavailable",
    )?;
    let n = request.relations.player_count as usize;
    require(
        request.relations.war_types.len() == n * n.saturating_sub(1) / 2,
        "war relation vector is incomplete",
    )?;
    let relations = WarRelations {
        player_count: request.relations.player_count,
        war_types: &request.relations.war_types,
    };
    let mut byte_budget = 256 * 1024 * 1024;
    let map_bytes = request.map.read(root, &mut byte_budget)?;
    let map = map::Map::decode(&map_bytes, map::Limits::default())?;
    require(
        map.encode(map::Limits::default())? == map_bytes,
        "Map wire not exact current transport",
    )?;
    let mut players = Vec::new();
    let mut record_controls = Vec::new();
    let mut previous_owner = None;
    for p in &request.players {
        require(
            (0..65).contains(&p.owner) && previous_owner.is_none_or(|v| v < p.owner),
            "players not in native query order",
        )?;
        previous_owner = Some(p.owner);
        require(
            request.player_mapping[p.owner as usize] != -1
                && request.player_slot_present[p.owner as usize],
            "imported Player contradicts valid actual slot",
        )?;
        p.units.check(p.owner)?;
        p.formations.check(p.owner)?;
        let mut units = Vec::new();
        let mut formations = Vec::new();
        for m in &p.units.entries {
            let b = m.wire.read(root, &mut byte_budget)?;
            let v = Unit::decode(&b, unit::Limits::default())?;
            require(
                v.id == m.id && v.e_owner == p.owner && v.encode(unit::Limits::default())? == b,
                "Unit wire/owner/id mismatch",
            )?;
            record_controls.push((false, p.owner, m.slot, m.wire.sha256.clone()));
            units.push(Slot {
                index: m.slot as i32,
                value: v,
            });
        }
        for m in &p.formations.entries {
            let b = m.wire.read(root, &mut byte_budget)?;
            let v = Formation::decode(&b, unit::Limits::default())?;
            require(
                v.id == m.id && v.owner == p.owner && v.encode(unit::Limits::default())? == b,
                "Formation wire/owner/id mismatch",
            )?;
            record_controls.push((true, p.owner, m.slot, m.wire.sha256.clone()));
            formations.push(Slot {
                index: m.slot as i32,
                value: v,
            });
        }
        let mut player = Player::from_lookup_slots(
            p.owner,
            Some(CompleteSlots { entries: units }),
            Some(CompleteSlots {
                entries: formations,
            }),
        )?;
        player.slot_status = Some(p.slot_status);
        player.team = Some(p.team);
        player.is_alive = Some(p.alive);
        players.push(player);
    }
    for (owner, (&mapping, &present)) in request
        .player_mapping
        .iter()
        .zip(&request.player_slot_present)
        .enumerate()
    {
        if mapping != -1 && present {
            require(
                request.players.iter().any(|p| p.owner == owner as i32),
                "valid nonnull Player omitted",
            )?;
        }
    }
    let mut world = World::new(map, players)?;
    world.player_mapping = Some(
        request
            .player_mapping
            .try_into()
            .map_err(|_| "mapping length")?,
    );
    world.player_slot_present = Some(
        request
            .player_slot_present
            .try_into()
            .map_err(|_| "slot presence length")?,
    );
    let inputs = Inputs {
        world: &world,
        definitions: Some(&definitions),
        relations: Some(&relations),
        plot_unit_limit: Some(request.plot_unit_limit),
    };
    let mut counts = Vec::new();
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    for player in world.players() {
        for (ordinal, entry) in player.unit_records()?.iter().enumerate() {
            let u = &entry.value;
            let layer = placement::unit_layer(&world, u)?;
            let index = registry
                .find_index(u.e_type_index)
                .map_or(u32::MAX, |v| v as u32);
            let binding = registry.find_index(index);
            let formation = player.formation(u.formation_id)?.map(|v| v.resolved_id);
            bindings.push(json!({"kind":"unit_binding_result","owner":player.owner(),"ordinal":ordinal,"id":u.id,"definition_index":binding,"formation_id":formation}));
            let mut candidate = u.clone();
            let mut queries = Inputs { ..inputs };
            let handler = legality::skip_turn_can_start(&mut candidate, &mut queries)
                .map_err(|e| format!("handler {}/{}: {e}", u.e_owner, u.id));
            require(candidate == *u, "query mutated Unit")?;
            handlers.push(match handler {Ok(result)=>json!({"kind":"handler_result","owner":player.owner(),"ordinal":ordinal,"id":u.id,"result":result,"native_compared":false}),Err(error)=>json!({"kind":"handler_result","owner":player.owner(),"ordinal":ordinal,"id":u.id,"unsupported":error,"native_compared":false})});
            for (i, plot) in placement::query_plots(world.map(), u.k_location)?
                .into_iter()
                .enumerate()
            {
                let count = plot
                    .map(|p| inputs.unit_count(u, placement::units_at(&world, layer, p)?))
                    .transpose()?;
                counts.push(count_row(
                    ("unit", player.owner(), ordinal, u.id),
                    u.k_location,
                    layer,
                    i as i32 - 1,
                    plot,
                    count,
                ));
            }
        }
        for (ordinal, entry) in player.formation_records()?.iter().enumerate() {
            let f = &entry.value;
            let layer = placement::formation_layer(&world, f)?;
            for (i, plot) in placement::query_plots(world.map(), f.location)?
                .into_iter()
                .enumerate()
            {
                let count = plot
                    .map(|p| inputs.formation_count(f, placement::units_at(&world, layer, p)?))
                    .transpose()?;
                counts.push(count_row(
                    ("formation", player.owner(), ordinal, f.id),
                    f.location,
                    layer,
                    i as i32 - 1,
                    plot,
                    count,
                ));
            }
        }
    }
    require(
        digest(&world.map().encode(map::Limits::default())?) == request.map.sha256,
        "Map query state changed",
    )?;
    for (formation, owner, slot, hash) in record_controls {
        let p = world.player(owner)?;
        let bytes = if formation {
            p.formation(slot)?
                .ok_or("missing checked Formation")?
                .record
                .encode(unit::Limits::default())?
        } else {
            p.unit(slot)?
                .ok_or("missing checked Unit")?
                .record
                .encode(unit::Limits::default())?
        };
        require(digest(&bytes) == hash, "owned query record changed")?;
    }
    Ok(
        json!({"schema":"c6-linux-placement-count-results-v1","gamecore_sha256":GAME,"configuration_sha256":config,"profile_sha256":profile,
  "count_results":counts,"unit_binding_results":bindings,"handler_results":handlers,"typed_input_state_unchanged":true,
  "native_execution":false,"native_handler_comparison":false,"complete_state_actions_turns":false}),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    require(
        args.len() == 4,
        "usage: c6-linux-placement-count-candidate INPUT_JSON CONFIGURATION_SHA256 PROFILE_SHA256",
    )?;
    let path = Path::new(&args[1]).canonicalize()?;
    let root = path.parent().ok_or("input parent")?;
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    require(bytes.len() <= 16 * 1024 * 1024, "JSON input resource limit")?;
    let request: Request = serde_json::from_slice(&bytes)?;
    println!("{}", run(request, root, &args[2], &args[3])?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Value {
        json!({"schema":"c6-linux-placement-inputs-v1","gamecore_sha256":GAME,"host_sha256":HOST,
        "configuration_sha256":"0".repeat(64),"profile_sha256":"1".repeat(64),
        "map":{"path":"map.bin","bytes":0,"sha256":"0".repeat(64)},"player_mapping":[],"player_slot_present":[],"players":[],
        "unit_definitions":{"count":0,"complete":true,"entries":[]},"relations":{"player_count":0,"war_types":[],"complete":true},"plot_unit_limit":1})
    }
    #[test]
    fn schema_rejects_oracle_answers_unknown_and_duplicate_fields() {
        let mut value = request();
        value["expected_counts"] = json!([]);
        assert!(serde_json::from_value::<Request>(value).is_err());
        let text = request().to_string();
        let duplicate = format!("{},\"plot_unit_limit\":1}}", text.trim_end_matches('}'));
        assert!(serde_json::from_str::<Request>(&duplicate).is_err());
        let mut value = request();
        value.as_object_mut().unwrap().remove("player_slot_present");
        assert!(serde_json::from_value::<Request>(value).is_err());
    }
    #[test]
    fn payload_paths_cannot_escape_or_use_absolute_files() {
        for path in ["../other.bin", "/tmp/other.bin", "x/../../other.bin"] {
            let blob = Blob {
                path: path.into(),
                bytes: 0,
                sha256: digest(&[]),
            };
            assert!(blob.read(Path::new("/tmp"), &mut 1).is_err());
        }
    }
}
