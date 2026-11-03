//! Read-only preflight over the accepted strict state projection. No native code.
use c6_core::{
    map::{Map, Slot},
    movement_world::DefinitionTable,
    skip_turn::{
        legality::{self, SkipTurnQueries},
        placement::{self, Inputs, UnitProperties, WarRelations},
    },
    unit::{Formation, Unit},
    world::{CompleteSlots, Player, World},
};
use c6_data::registry::LinuxDefinitionHashRegistry;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
fn hash(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn int(v: &Value) -> i32 {
    i32::try_from(v.as_i64().expect("signed native integer")).expect("i32")
}
fn word(v: &Value) -> u32 {
    u32::try_from(v.as_u64().expect("unsigned native integer")).expect("u32")
}
fn blob(root: &Path, v: &Value) -> Vec<u8> {
    let b = fs::read(root.join(v["path"].as_str().unwrap())).unwrap();
    assert_eq!(b.len() as u64, v["bytes"].as_u64().unwrap());
    assert_eq!(hash(&b), v["sha256"].as_str().unwrap());
    b
}
struct Logged<'a, 'd> {
    inputs: Inputs<'a, 'd>,
    calls: Vec<&'static str>,
    count: Option<u32>,
}
impl SkipTurnQueries for Logged<'_, '_> {
    type Error = placement::Error;
    fn is_human(&mut self, u: &mut Unit) -> Result<bool, Self::Error> {
        self.calls.push("IsHuman");
        self.inputs.is_human(u.e_owner)
    }
    fn friendly_units_of_type_count(&mut self, u: &mut Unit) -> Result<u32, Self::Error> {
        self.calls.push("FormationFriendlyCount(false)");
        let n = self.inputs.friendly_count(u)?;
        self.count = Some(n);
        Ok(n)
    }
    fn plot_unit_limit(&mut self, _: &mut Unit) -> Result<i32, Self::Error> {
        self.calls.push("PLOT_UNIT_LIMIT");
        self.inputs
            .plot_unit_limit
            .ok_or(placement::Error::PlotUnitLimit)
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 3);
    let path = Path::new(&a[1]);
    let root = path.parent().unwrap();
    let input: Value = serde_json::from_slice(&fs::read(path)?)?;
    let output: Value = serde_json::from_slice(&fs::read(&a[2])?)?;
    let map = Map::decode(&blob(root, &input["map"]), Default::default())?;
    let mut players = Vec::new();
    for p in input["players"].as_array().unwrap() {
        let owner = int(&p["owner"]);
        let mut units = Vec::new();
        let mut forms = Vec::new();
        for e in p["units"]["entries"].as_array().unwrap() {
            let u = Unit::decode(&blob(root, &e["wire"]), Default::default())?;
            assert_eq!(u.e_owner, owner);
            assert_eq!(u.id, word(&e["id"]));
            units.push(Slot {
                index: int(&e["slot"]),
                value: u,
            });
        }
        for e in p["formations"]["entries"].as_array().unwrap() {
            let f = Formation::decode(&blob(root, &e["wire"]), Default::default())?;
            assert_eq!(f.owner, owner);
            assert_eq!(f.id, word(&e["id"]));
            forms.push(Slot {
                index: int(&e["slot"]),
                value: f,
            });
        }
        let mut v = Player::from_lookup_slots(
            owner,
            Some(CompleteSlots { entries: units }),
            Some(CompleteSlots { entries: forms }),
        )?;
        v.slot_status = Some(int(&p["slot_status"]));
        v.team = Some(int(&p["team"]));
        v.is_alive = Some(p["alive"].as_bool().unwrap());
        players.push(v);
    }
    let mut world = World::new(map, players)?;
    world.player_mapping = Some(
        input["player_mapping"]
            .as_array()
            .unwrap()
            .iter()
            .map(int)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    );
    world.player_slot_present = Some(
        input["player_slot_present"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_bool().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    );
    let defs = input["unit_definitions"]["entries"].as_array().unwrap();
    let registry = LinuxDefinitionHashRegistry::from_native_order(
        defs.iter().map(|v| word(&v["hash"])).collect(),
    );
    let properties: Vec<_> = defs
        .iter()
        .map(|v| {
            Some(UnitProperties {
                domain_hash: word(&v["domain_hash"]),
                formation_class_hash: word(&v["formation_class_hash"]),
                make_trade_route: v["make_trade_route"].as_bool().unwrap(),
                spy: v["spy"].as_bool().unwrap(),
                combat: int(&v["combat"]),
                range: int(&v["range"]),
            })
        })
        .collect();
    let table = DefinitionTable::new(&registry, &properties)?;
    let wars: Vec<_> = input["relations"]["war_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(int)
        .collect();
    let relations = WarRelations {
        player_count: int(&input["relations"]["player_count"]),
        war_types: &wars,
    };
    let params = Inputs {
        world: &world,
        definitions: Some(&table),
        relations: Some(&relations),
        plot_unit_limit: Some(int(&input["plot_unit_limit"])),
    };
    let mut frozen = BTreeMap::new();
    for r in output["handler_results"].as_array().unwrap() {
        assert!(r.get("unsupported").is_none());
        assert_eq!(r["native_compared"], false);
        assert!(frozen
            .insert(
                (
                    int(&r["owner"]),
                    r["ordinal"].as_u64().unwrap(),
                    word(&r["id"])
                ),
                r["result"].as_bool().unwrap()
            )
            .is_none());
    }
    let mut rows = Vec::new();
    let mut visited = BTreeSet::new();
    for p in world.players() {
        for (ordinal, e) in p.unit_records()?.iter().enumerate() {
            let u = &e.value;
            let key = (u.e_owner, ordinal as u64, u.id);
            assert!(visited.insert(key));
            let mut current = u.clone();
            let mut q = Logged {
                inputs: Inputs { ..params },
                calls: vec![],
                count: None,
            };
            let computed = legality::skip_turn_can_start(&mut current, &mut q)?;
            assert_eq!(current, *u);
            assert_eq!(frozen.get(&key), Some(&computed));
            let human = params.is_human(u.e_owner)?;
            let early = human
                && (u.i_fortify_turns > 0 || [0x047c048b, 0x024a74eb].contains(&u.e_activity));
            assert_eq!(
                q.calls,
                if human && !early {
                    vec![
                        "IsHuman",
                        "FormationFriendlyCount(false)",
                        "PLOT_UNIT_LIMIT",
                    ]
                } else {
                    vec!["IsHuman"]
                }
            );
            let plot = placement::query_plots(world.map(), u.k_location)?[0];
            let layer = placement::unit_layer(&world, u)?;
            let occupancy = placement::layered_units(&world, u)?;
            if layer.is_none() {
                assert!(occupancy.is_empty());
                if q.count.is_some() {
                    assert_eq!(q.count, Some(0));
                }
            }
            let formation = p.formation(u.formation_id)?;
            rows.push(json!({"owner":u.e_owner,"ordinal":ordinal,"id":u.id,"is_human":human,"slot_status":p.slot_status,
   "location":[u.k_location.x,u.k_location.y],"coordinate_valid":placement::coordinate_valid(u.k_location),"current_plot_present":plot.is_some(),"layer":layer.map_or(-1,|v|v as i32),
   "activity":u.e_activity,"fortify_turns":u.i_fortify_turns,"espionage_operation":u.k_espionage.e_operation,"delayed_death":u.b_delayed_death,
   "formation_resolved":formation.map(|v|v.resolved_id),"layered_occupancy_len":occupancy.len(),"provider_calls":q.calls,"friendly_count":q.count,"result":computed,"unit_unchanged":true}));
        }
    }
    assert_eq!(visited.len(), frozen.len());
    println!(
        "{}",
        json!({"schema":"c6-linux-skip-turn-handler-preflight-values-v1","rows":rows,"all_frozen_results_equal":true,"all_source_provider_orders_equal":true,"all_units_unchanged":true,"original_execution":false,"native_handler_comparison":false})
    );
    Ok(())
}
