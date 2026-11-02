//! Read-only current Formation record roundtrips; never executes original code.
use c6_core::unit::{Error, Formation, Limits, OperationQueue, Unit};
use std::{collections::HashMap, hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        return Err("usage: unit_codec FORMATION_DIRECTORY ITERATIONS QUEUE_DIRECTORY UNIT_DIRECTORY RANGE_OUTPUT".into());
    }
    let iterations: usize = args[2].parse()?;
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    let mut paths: Vec<_> = std::fs::read_dir(&args[1])?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    paths.sort();
    if paths.is_empty() {
        return Err("no attributed records".into());
    }
    let mut records = Vec::new();
    let mut total_bytes = 0;
    let mut nodes = 0;
    let mut unit_ids = 0;
    let mut nonempty_paths = 0;
    let mut all_lengths = Vec::new();
    for path in paths {
        let bytes = std::fs::read(path)?;
        let formation = Formation::decode(&bytes, Limits::default())?;
        assert_eq!(formation.encode(Limits::default())?, bytes);
        // Independently predict record extent from typed fields. Native exact
        // owner boundaries supply the input; the decoder must consume it all.
        let expected = 52 + 20 * formation.last_path.len() + 4 * formation.unit_ids.len();
        assert_eq!(expected, bytes.len());
        all_lengths.push(expected);
        total_bytes += bytes.len();
        nodes += formation.last_path.len();
        unit_ids += formation.unit_ids.len();
        nonempty_paths += usize::from(!formation.last_path.is_empty());
        records.push((bytes, formation));
    }
    let mut queue_roundtrips = 0;
    let mut queue_rejections = 0;
    for entry in std::fs::read_dir(&args[3])? {
        let bytes = std::fs::read(entry?.path())?;
        match OperationQueue::decode(&bytes, Limits::default()) {
            Ok(queue) => {
                assert_eq!(queue.encode(Limits::default())?, bytes);
                queue_roundtrips += 1;
            }
            Err(Error::UnsupportedHandler(_)) => queue_rejections += 1,
            Err(error) => return Err(error.into()),
        }
    }
    let mut unit_paths: Vec<_> = std::fs::read_dir(&args[4])?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    unit_paths.sort();
    let mut unit_roundtrips = 0;
    let mut unit_fields = Vec::new();
    let mut unit_bytes = 0;
    let mut unit_records = Vec::new();
    let mut unit_index = HashMap::new();
    let mut malformed_checks = 0;
    let mut exact_item_budget = 0;
    for path in unit_paths {
        let bytes = std::fs::read(&path)?;
        let (unit, ranges) = Unit::decode_with_ranges(&bytes, Limits::default())
            .map_err(|e| format!("{}: {e}", path.display()))?;
        assert_eq!(unit.encode(Limits::default())?, bytes, "{}", path.display());
        if unit_records.is_empty() {
            for end in 0..bytes.len() {
                assert!(Unit::decode(&bytes[..end], Limits::default()).is_err());
                malformed_checks += 1;
            }
            let mut bad = bytes.clone();
            bad.push(0);
            assert_eq!(
                Unit::decode(&bad, Limits::default()),
                Err(Error::TrailingBytes)
            );
            malformed_checks += 1;
            for range in &ranges {
                if matches!(
                    range.name,
                    "m_kCombatModifers"
                        | "m_kMovementModifiers"
                        | "m_kExperience"
                        | "m_kReligion"
                        | "m_kRockBand"
                        | "m_kEspionage"
                        | "m_kOperations"
                ) {
                    let mut bad = bytes.clone();
                    bad[range.start..range.start + 4].copy_from_slice(&99i32.to_le_bytes());
                    assert!(matches!(
                        Unit::decode(&bad, Limits::default()),
                        Err(Error::Version { actual: 99, .. })
                    ));
                    malformed_checks += 1;
                }
            }
            for name in ["m_bIsLevied", "m_aiAbilitiesPossessed"] {
                let range = ranges.iter().find(|r| r.name == name).unwrap();
                let mut bad = bytes.clone();
                if name == "m_bIsLevied" {
                    bad[range.start] = 2;
                    assert_eq!(
                        Unit::decode(&bad, Limits::default()),
                        Err(Error::NoncanonicalBool(2))
                    );
                } else {
                    bad[range.start..range.start + 4].copy_from_slice(&u32::MAX.to_le_bytes());
                    assert_eq!(
                        Unit::decode(&bad, Limits::default()),
                        Err(Error::ResourceLimit)
                    );
                }
                malformed_checks += 1;
            }
            let mut low = 0;
            let mut high = Limits::default().max_items;
            while low < high {
                let mid = low + (high - low) / 2;
                match Unit::decode(
                    &bytes,
                    Limits {
                        max_items: mid,
                        ..Limits::default()
                    },
                ) {
                    Ok(_) => high = mid,
                    Err(Error::ResourceLimit) => low = mid + 1,
                    Err(e) => return Err(e.into()),
                }
            }
            exact_item_budget = low;
            let exact = Limits {
                max_bytes: bytes.len(),
                max_items: low,
            };
            assert_eq!(Unit::decode(&bytes, exact)?, unit);
            assert_eq!(unit.encode(exact)?, bytes);
            assert!(Unit::decode(
                &bytes,
                Limits {
                    max_items: low - 1,
                    ..exact
                }
            )
            .is_err());
            assert!(unit
                .encode(Limits {
                    max_items: low - 1,
                    ..exact
                })
                .is_err());
            assert_eq!(
                Unit::decode(
                    &bytes,
                    Limits {
                        max_bytes: bytes.len() - 1,
                        ..exact
                    }
                ),
                Err(Error::ResourceLimit)
            );
            assert_eq!(
                unit.encode(Limits {
                    max_bytes: bytes.len() - 1,
                    ..exact
                }),
                Err(Error::ResourceLimit)
            );
        }
        if unit_index
            .insert((unit.e_owner, unit.id), unit_records.len())
            .is_some()
        {
            return Err("duplicate native Unit key".into());
        }
        let fields: Vec<_> = ranges
            .iter()
            .filter(|r| {
                matches!(
                    r.name,
                    "m_kCombatModifers"
                        | "m_kMovementModifiers"
                        | "m_kExperience"
                        | "m_kReligion"
                        | "m_kRockBand"
                        | "m_kEspionage"
                        | "m_kOperations"
                )
            })
            .map(|r| format!("[\"{}\",{},{}]", r.name, r.start, r.end))
            .collect();
        unit_fields.push(format!("[{}]", fields.join(",")));
        unit_roundtrips += 1;
        unit_bytes += bytes.len();
        unit_records.push((bytes, unit));
    }
    std::fs::write(&args[5], format!("[{}]", unit_fields.join(",")))?;
    let start = Instant::now();
    for _ in 0..iterations {
        for (bytes, _) in &records {
            black_box(Formation::decode(black_box(bytes), Limits::default())?);
        }
    }
    let decode_ns = start.elapsed().as_nanos() / (iterations * records.len()) as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        for (_, state) in &records {
            black_box(state.encode(Limits::default())?);
        }
    }
    let encode_ns = start.elapsed().as_nanos() / (iterations * records.len()) as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        for (bytes, _) in &unit_records {
            black_box(Unit::decode(black_box(bytes), Limits::default())?);
        }
    }
    let unit_decode_ns = start.elapsed().as_nanos() / (iterations * unit_records.len()) as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        for (_, state) in &unit_records {
            black_box(state.encode(Limits::default())?);
        }
    }
    let unit_encode_ns = start.elapsed().as_nanos() / (iterations * unit_records.len()) as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        for (_, state) in &unit_records {
            let i = unit_index
                .get(black_box(&(state.e_owner, state.id)))
                .unwrap();
            black_box(&unit_records[*i].1);
        }
    }
    let unit_lookup_ns = start.elapsed().as_nanos() / (iterations * unit_records.len()) as u128;
    println!("{{\"unit_decode_ns_per_record\":{},\"unit_encode_ns_per_record\":{},\"adapter_unit_lookup_ns\":{},\"native_malformed_checks\":{},\"first_unit_exact_item_budget\":{},\"native_unit_roundtrips\":{},\"native_unit_bytes\":{},\"native_queue_roundtrips\":{},\"native_queue_unsupported_handlers\":{},\"records\":{},\"bytes\":{},\"path_nodes\":{},\"unit_ids\":{},\"nonempty_paths\":{},\"record_lengths\":{:?},\"iterations\":{},\"decode_ns_per_record\":{},\"encode_ns_per_record\":{}}}", unit_decode_ns,unit_encode_ns,unit_lookup_ns,malformed_checks,exact_item_budget,unit_roundtrips, unit_bytes, queue_roundtrips, queue_rejections, records.len(), total_bytes, nodes, unit_ids, nonempty_paths, all_lengths, iterations, decode_ns, encode_ns);
    Ok(())
}
