//! Offline typed-wire movement resolution. This never executes original code.
//! Current cb6113b registries omit Terrain IsWater: affected groups report
//! unsupported, never a passing world-resolver comparison.
use c6_core::city_district::DomainMask;
use c6_core::{map, movement::PlotFacts, movement_world::*, unit};
use c6_data::movement::{NativeMovementExports, RegistryExport, RegistryKind};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, path::Path};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn unsigned(v: &Value, key: &str) -> Result<u32, String> {
    v[key]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("invalid u32 {key}"))
}
fn integer(v: &Value, key: &str) -> Result<i32, String> {
    v[key]
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .ok_or_else(|| format!("invalid i32 {key}"))
}
fn boolean(v: &Value, key: &str) -> Result<bool, String> {
    v[key]
        .as_bool()
        .ok_or_else(|| format!("invalid bool {key}"))
}
fn checked_file(base: &Path, value: &Value, limit: u64) -> Result<Vec<u8>, String> {
    let path = base.join(value["path"].as_str().ok_or("missing artifact path")?);
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > limit {
        return Err("artifact exceeds resource budget".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if value["sha256"].as_str() != Some(digest(&bytes).as_str()) {
        return Err("typed wire artifact hash mismatch".into());
    }
    Ok(bytes)
}
fn coord(v: &Value) -> Result<unit::PlotCoord, String> {
    Ok(unit::PlotCoord {
        x: integer(v, "x")?,
        y: integer(v, "y")?,
    })
}
fn bool_bits(values: &[bool]) -> Value {
    json!(values.iter().map(|&b| u8::from(b)).collect::<Vec<_>>())
}
fn facts(v: PlotFacts) -> Value {
    json!({"is_city":v.is_city,"is_water":v.is_water,"has_city_attributes":v.has_city_attributes,
        "city_domain_valid":v.city_domain_valid,"terrain_cost":v.terrain_cost,"feature_change":v.feature_change,
        "improvement_change":v.improvement_change,"district_free_embark":v.district_free_embark,
        "district_complete":v.district_complete,"route":v.route.map(|r|json!({"pillaged":r.pillaged,
            "bridged":r.bridged,"movement_cost_float_bits":r.movement_cost_float_bits}))})
}

fn definition_observations_equal(
    plot: &map::Plot,
    native: &Value,
    inputs: &NativeMovementExports,
    runtime: &impl RuntimePlotInputs,
) -> Result<bool, MissingInput> {
    let district = plot
        .city_attributes
        .as_ref()
        .map(|c| runtime.district(c))
        .transpose()?
        .flatten();
    let short_index = |kind, hash| {
        inputs
            .registry(kind)
            .find_index(hash)
            .map_or(-1, |i| i as i16 as i32)
    };
    let queries = [
        short_index(RegistryKind::Terrain, plot.terrain_hash),
        short_index(RegistryKind::Feature, plot.feature_hash),
        short_index(RegistryKind::Improvement, plot.improvement_hash),
        i32::from(plot.route_type),
        district.map_or(-1, |d| d.type_index),
    ];
    let kinds = [
        RegistryKind::Terrain,
        RegistryKind::Feature,
        RegistryKind::Improvement,
        RegistryKind::Route,
        RegistryKind::District,
    ];
    let Some(observations) = native["definitions"].as_array().filter(|a| a.len() == 5) else {
        return Ok(false);
    };
    let mut equal = true;
    for ((&query, kind), observation) in queries.iter().zip(kinds).zip(observations) {
        let hash = inputs.registry(kind).type_hash(query as u32 as usize);
        equal &= observation["query"] == query as u32 && observation["present"] == hash.is_some();
        if let Some(hash) = hash {
            equal &= observation["index"] == query as u32 && observation["hash"] == hash;
        }
    }
    if let Some(city) = &plot.city_attributes {
        equal &= native["working_city_id"] == city.working_city
            && native["owning_city_id"] == city.owning_city
            && native["requested_district_id"] == city.district
            && native["district_owner"] == i32::from(city.player);
    }
    Ok(equal)
}
// Targeted actual runtime observations only; no completion/domain inference.
struct Runtime<'a> {
    map: &'a map::Map,
    from: usize,
    to: usize,
    case: &'a Value,
}
impl RuntimePlotInputs for Runtime<'_> {
    fn district(
        &self,
        city: &map::CityAttributes,
    ) -> Result<Option<DistrictInstance>, MissingInput> {
        for (index, key) in [(self.from, "from"), (self.to, "to")] {
            if self.map.plots[index].city_attributes.as_ref() != Some(city) {
                continue;
            }
            let observation = &self.case[key];
            if observation["requested_district_id"].as_u64() != Some(u64::from(city.district))
                || observation["district_owner"].as_i64() != Some(i64::from(city.player))
            {
                return Err(MissingInput::DistrictLookup {
                    player: city.player,
                    district: city.district,
                });
            }
            if observation["district_resolved"] == false {
                return Ok(None);
            }
            if observation["district_resolved"] != true {
                break;
            }
            let d = &observation["definitions"][4];
            let type_index = d["query"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .map(|n| n as i32);
            if let (Some(type_index), Some(complete)) =
                (type_index, observation["district_complete"].as_bool())
            {
                return Ok(Some(DistrictInstance {
                    type_index,
                    complete,
                }));
            }
        }
        Err(MissingInput::DistrictLookup {
            player: city.player,
            district: city.district,
        })
    }
    fn domain_valid(&self, plot: usize, domain: i32) -> Result<bool, MissingInput> {
        let key = if plot == self.from {
            "from"
        } else if plot == self.to {
            "to"
        } else {
            return Err(MissingInput::CityDomainValidity { plot, domain });
        };
        let mask = self.case[key]["valid_domains_mask"]
            .as_i64()
            .and_then(|n| i16::try_from(n).ok())
            .ok_or(MissingInput::CityDomainValidity { plot, domain })?;
        Ok(DomainMask(mask).is_valid(domain))
    }
}
fn run() -> Result<u8, String> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: c6-movement-world-candidate native.jsonl typed-fixture.json".into());
    }
    let native = fs::read(&args[0]).map_err(|e| e.to_string())?;
    if native.len() > 256 * 1024 * 1024 {
        return Err("native corpus exceeds resource budget".into());
    }
    let records = std::str::from_utf8(&native)
        .map_err(|e| e.to_string())?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let header = records.first().ok_or("empty native corpus")?;
    if header["kind"] != "header"
        || header["schema"] != "c6-linux-movement-cost-native-v1"
        || header["game_sha256"]
            != "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
        || header["host_sha256"]
            != "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4"
    {
        return Err("native schema/module identity mismatch".into());
    }
    let summary = records.last().ok_or("missing native summary")?;
    if summary["kind"] != "summary" || integer(summary, "status")? != 0 {
        return Err("native corpus incomplete/failed".into());
    }
    let fixture_bytes = fs::read(&args[1]).map_err(|e| e.to_string())?;
    if fixture_bytes.len() > 16 * 1024 * 1024 {
        return Err("fixture manifest exceeds resource budget".into());
    }
    let fixture: Value = serde_json::from_slice(&fixture_bytes).map_err(|e| e.to_string())?;
    if fixture["schema"] != "c6-linux-movement-world-fixture-v1"
        || fixture["game_sha256"] != header["game_sha256"]
        || fixture["host_sha256"] != header["host_sha256"]
        || fixture["profile_sha256"] != header["profile_sha256"]
        || fixture["native_corpus_sha256"] != digest(&native)
        || fixture["units_complete"] != true
        || fixture["formations_complete"] != true
        || !matches!(
            fixture["evidence_level"].as_str(),
            Some("native_serializer_owned" | "source_only")
        )
    {
        return Err("typed fixture provenance/identity/completeness mismatch".into());
    }
    for key in ["profile_sha256", "core_sha256", "trace_sha256"] {
        if !fixture[key].as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }) {
            return Err(format!("missing checked identity {key}"));
        }
    }
    let exports = records
        .iter()
        .filter(|v| v["kind"] == "registry")
        .map(|v| serde_json::from_value::<RegistryExport>(v.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let inputs = NativeMovementExports::from_records(&exports).map_err(|e| e.to_string())?;
    let complete = inputs.complete();
    if complete.is_ok() && header["probe_contract_revision"] != "xp2-map-and-terrain-water-2" {
        return Err("complete movement properties require the separately reviewed original water export revision".into());
    }
    let missing_properties = complete.as_ref().err().map(ToString::to_string);
    let definitions = complete
        .as_ref()
        .ok()
        .map(|d| d.as_core())
        .transpose()
        .map_err(|e| e.to_string())?;
    let base = Path::new(&args[1]).parent().unwrap_or(Path::new("."));
    let map = map::Map::decode(
        &checked_file(base, &fixture["map"], 64 * 1024 * 1024)?,
        map::Limits::default(),
    )
    .map_err(|e| format!("typed Map: {e:?}"))?;
    let mut units = BTreeMap::new();
    for file in fixture["units"]
        .as_array()
        .ok_or("missing complete typed Units")?
    {
        let unit = unit::Unit::decode(
            &checked_file(base, file, 16 * 1024 * 1024)?,
            unit::Limits::default(),
        )
        .map_err(|e| format!("typed Unit: {e:?}"))?;
        let movement = UnitMovement::from_wire(
            &unit,
            inputs.registry(RegistryKind::Terrain),
            inputs.registry(RegistryKind::Feature),
        )
        .map_err(|e| e.to_string())?;
        if units
            .insert(
                (unit.e_owner, (unit.id & 0xffff) as u16),
                (unit.id, movement, unit.formation_id),
            )
            .is_some()
        {
            return Err("duplicate typed Unit native slot".into());
        }
    }
    let mut formations = BTreeMap::new();
    for file in fixture["formations"]
        .as_array()
        .ok_or("missing complete typed Formations")?
    {
        let formation = unit::Formation::decode(
            &checked_file(base, file, 16 * 1024 * 1024)?,
            unit::Limits::default(),
        )
        .map_err(|e| format!("typed Formation: {e:?}"))?;
        if formations
            .insert((formation.owner, (formation.id & 0xffff) as u16), formation)
            .is_some()
        {
            return Err("duplicate typed Formation native slot".into());
        }
    }
    let cases: Vec<_> = records.iter().filter(|v| v["kind"] == "case").collect();
    if cases.is_empty() || cases.len() != unsigned(summary, "cases")? as usize {
        return Err("empty/inconsistent case count".into());
    }
    let mut mismatches = 0;
    let mut unsupported = 0;
    for case in cases {
        let owner = integer(case, "owner")?;
        let id = unsigned(case, "formation_id")?;
        let formation = formations
            .get(&(owner, (id & 0xffff) as u16))
            .ok_or("native Formation absent from complete typed fixture")?;
        let from = coord(&case["from"])?;
        let to = coord(&case["to"])?;
        let (from_index, from_plot) = plot_at(&map, from).map_err(|e| e.to_string())?;
        let (to_index, to_plot) = plot_at(&map, to).map_err(|e| e.to_string())?;
        let terrain = inputs
            .registry(RegistryKind::Terrain)
            .find_index(to_plot.terrain_hash)
            .map_or(-1, |i| i as i16 as i32);
        let feature = inputs
            .registry(RegistryKind::Feature)
            .find_index(to_plot.feature_hash)
            .map_or(-1, |i| i as i16 as i32);
        let resolved = resolve_formation(formation, terrain, feature, |owner, id| {
            Ok(if id == u32::MAX {
                None
            } else {
                units.get(&(owner, (id & 0xffff) as u16)).map(|(_, u, _)| u)
            })
        })
        .map_err(|e| e.to_string())?;
        let actual_formation = json!({"ignore_terrain":resolved.ignore_terrain,"ignore_feature":resolved.ignore_feature,
            "ignore_rivers":resolved.ignore_rivers,"ignore_shores":resolved.ignore_shores,"can_embark":resolved.can_embark});
        let mut equal = actual_formation == case["formation"]
            && formation.id == id
            && id != u32::MAX
            && formation.domain == integer(case, "formation_domain")?
            && formation.location == from
            && formation.moves_remaining_raw == integer(&case["budget"], "remaining")?
            && terrain == integer(case, "terrain_ignore_index")?
            && feature == integer(case, "feature_ignore_index")?;
        let selected_id = unsigned(case, "unit_id")?;
        let selected = units
            .get(&(owner, (selected_id & 0xffff) as u16))
            .ok_or("selected Unit missing from typed registry")?;
        equal &= selected_id != u32::MAX
            && selected.0 == unsigned(case, "actual_unit_id")?
            && selected.2 != u32::MAX
            && (selected.2 & 0xffff) == (id & 0xffff);
        let members = case["members"]
            .as_array()
            .ok_or("missing observed formation members")?;
        equal &= members.len() == formation.unit_ids.len();
        for (position, m) in members.iter().enumerate() {
            let requested = unsigned(m, "id")?;
            equal &= formation.unit_ids.get(position) == Some(&requested);
            let actual = if requested == u32::MAX {
                None
            } else {
                units.get(&(owner, (requested & 0xffff) as u16))
            };
            equal &= actual.is_some() == boolean(m, "found")?;
            if let Some((actual_id, u, _)) = actual {
                equal &= *actual_id == unsigned(m, "actual_unit_id")?
                    && bool_bits(&u.invalid_terrain) == m["invalid_terrain_bits"]
                    && bool_bits(&u.ignore_terrain) == m["ignore_terrain_bits"]
                    && bool_bits(&u.ignore_feature) == m["ignore_feature_bits"]
                    && u.ignores_terrain(terrain).map_err(|e| e.to_string())?
                        == boolean(m, "terrain")?
                    && u.ignores_feature(feature).map_err(|e| e.to_string())?
                        == boolean(m, "feature")?
                    && u.ignore_rivers == boolean(m, "rivers")?
                    && u.ignore_shores == boolean(m, "shores")?
                    && u.escort_mobility == boolean(m, "escort")?;
            }
        }
        let unit_equal = equal;
        let mut missing = missing_properties.clone();
        let mut plot_equal = Value::Null;
        let runtime = Runtime {
            map: &map,
            from: from_index,
            to: to_index,
            case,
        };
        let identity_result = (|| {
            Ok::<bool, MissingInput>(
                definition_observations_equal(from_plot, &case["from"], &inputs, &runtime)?
                    && definition_observations_equal(to_plot, &case["to"], &inputs, &runtime)?,
            )
        })();
        let mut identity_equal = Value::Null;
        match identity_result {
            Ok(result) => {
                equal &= result;
                identity_equal = json!(result);
            }
            Err(error) => missing = Some(error.to_string()),
        }
        if let Some(definitions) = &definitions {
            let attempt = (|| {
                let from_facts = resolve_plot(
                    from_plot,
                    from_index,
                    formation.domain,
                    definitions,
                    &runtime,
                )?;
                let to_facts =
                    resolve_plot(to_plot, to_index, formation.domain, definitions, &runtime)?;
                let edge = resolve_edge(&map, from, to, from_facts, to_facts)?;
                let compare = |actual: PlotFacts, native: &Value| {
                    facts(actual).as_object().unwrap().iter().all(|(k, v)| {
                        // Domain-valid false is an unreachable placeholder on land/without attrs.
                        (k == "city_domain_valid"
                            && (!actual.is_water || !actual.has_city_attributes))
                            || native[k] == *v
                    })
                };
                Ok::<bool, MissingInput>(
                    compare(from_facts, &case["from"])
                        && compare(to_facts, &case["to"])
                        && case["edge"]["river_crossing"] == edge.river_crossing
                        && case["edge"]["adjacent"] == edge.adjacent
                        && case["edge"]["source_portal_has_exit_at_destination"]
                            == edge.source_portal_has_exit_at_destination
                        && case["edge"]["bridge_between"]
                            == (edge.river_crossing
                                && from_facts.route.is_some_and(|r| r.bridged && !r.pillaged)
                                && to_facts.route.is_some_and(|r| r.bridged && !r.pillaged)),
                )
            })();
            match attempt {
                Ok(result) => {
                    equal &= result;
                    plot_equal = json!(result);
                }
                Err(error) => missing = Some(error.to_string()),
            }
        }
        if !equal {
            mismatches += 1;
        }
        if missing.is_some() {
            unsupported += 1;
        }
        println!(
            "{}",
            json!({"kind":"world_case_comparison","owner":owner,"formation_id":id,"direction":case["direction"],
            "typed_member_and_formation_equal":unit_equal,"equal":equal,"plot_and_edge_equal":plot_equal,"definition_identities_equal":identity_equal,"unsupported":missing,"native_acceptance":false})
        );
    }
    println!(
        "{}",
        json!({"kind":"world_comparison_summary","evidence_level":fixture["evidence_level"],
        "native_corpus_sha256":digest(&native),"typed_fixture_sha256":digest(&fixture_bytes),"profile_sha256":header["profile_sha256"],
        "core_sha256":fixture["core_sha256"],"trace_sha256":fixture["trace_sha256"],"compared":unsigned(summary,"cases")?,
        "mismatches":mismatches,"unsupported":unsupported,"runtime_inputs":"actual targeted district observations and raw domain masks; reconstruction not claimed",
        "complete_affected_state":false,"complete_rng_inventory":false,"native_differential_acceptance":false,"full_movement_parity":false,
        "raw_save_load_different_bytes_retained":89198,"unknown_bytes_masked":0})
    );
    Ok(if mismatches > 0 {
        1
    } else if unsupported > 0 {
        2
    } else {
        0
    })
}
fn main() {
    match run() {
        Ok(code) => std::process::exit(i32::from(code)),
        Err(error) => {
            eprintln!("world candidate rejected input: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_only_definition_identity_check_does_not_need_or_invent_water() {
        let records=[("terrain",100), ("feature",200), ("improvement",300), ("route",400), ("district",500)]
            .into_iter().map(|(name,hash)|serde_json::from_value::<RegistryExport>(json!({
                "kind":"registry","name":name,"count":1,"entries":[{"query":0,"index":0,"hash":hash}]
            })).unwrap()).collect::<Vec<_>>();
        let inputs = NativeMovementExports::from_records(&records).unwrap();
        assert!(inputs.complete().is_err());
        let mut plot = map::Plot::decode(&[0; 55], 6, map::Limits::default()).unwrap();
        plot.terrain_hash = 100;
        plot.feature_hash = 200;
        plot.improvement_hash = 300;
        let mut observed = json!({"definitions":[
            {"query":0,"present":true,"index":0,"hash":100},
            {"query":0,"present":true,"index":0,"hash":200},
            {"query":0,"present":true,"index":0,"hash":300},
            {"query":0,"present":true,"index":0,"hash":400},
            {"query":4294967295u32,"present":false}
        ]});
        assert!(
            definition_observations_equal(&plot, &observed, &inputs, &NoRuntimePlotInputs).unwrap()
        );
        observed["definitions"][0]["hash"] = json!(999);
        assert!(
            !definition_observations_equal(&plot, &observed, &inputs, &NoRuntimePlotInputs)
                .unwrap()
        );
        observed["definitions"][0]["hash"] = json!(100);
        plot.route_type = -1;
        assert!(
            !definition_observations_equal(&plot, &observed, &inputs, &NoRuntimePlotInputs)
                .unwrap()
        );
    }
}
