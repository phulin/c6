// Appended to an immutable candidate copy by placement_query_benchmark.py.
// No expected answer is opened until the original query loop has completed.
use std::hint::black_box;
use std::time::Instant;

enum BenchActor<'a> {
    Unit(&'a Unit),
    Formation(&'a Formation),
}
impl BenchActor<'_> {
    fn identity(&self) -> (i32, u32) {
        match self {
            Self::Unit(u) => (u.e_owner, u.id),
            Self::Formation(f) => (f.owner, f.id),
        }
    }
    fn layer(&self, world: &World) -> Option<u32> {
        match self {
            Self::Unit(u) => placement::unit_layer(world, black_box(u)).unwrap(),
            Self::Formation(f) => placement::formation_layer(world, black_box(f)).unwrap(),
        }
    }
    fn count(&self, inputs: &Inputs<'_, '_>, candidates: &[map::UnitReference]) -> u64 {
        match self {
            Self::Unit(u) => inputs
                .unit_count(black_box(u), black_box(candidates))
                .unwrap() as u64,
            Self::Formation(f) => inputs
                .formation_count(black_box(f), black_box(candidates))
                .unwrap() as u64,
        }
    }
}
fn samples(name: &str, cases: usize, mut sweep: impl FnMut() -> u64) -> Value {
    // Calibrate outside samples; each sample includes every real case.
    let expected = sweep();
    let mut repetitions = 1usize;
    loop {
        let start = Instant::now();
        for _ in 0..repetitions {
            black_box(sweep());
        }
        if start.elapsed().as_millis() >= 10 || repetitions >= 8192 {
            break;
        }
        repetitions *= 2;
    }
    let mut elapsed_ns = Vec::new();
    let mut checksum = 0u64;
    for _ in 0..21 {
        let start = Instant::now();
        for _ in 0..repetitions {
            checksum = checksum.wrapping_add(black_box(sweep()));
        }
        elapsed_ns.push(start.elapsed().as_nanos() as u64);
    }
    assert_eq!(
        checksum,
        expected.wrapping_mul(repetitions as u64).wrapping_mul(21)
    );
    let mut per_case: Vec<_> = elapsed_ns
        .iter()
        .map(|&ns| ns as f64 / (repetitions * cases) as f64)
        .collect();
    per_case.sort_by(f64::total_cmp);
    json!({"mode":name,"cases_per_sweep":cases,"repetitions_per_sample":repetitions,
        "samples":21,"elapsed_ns":elapsed_ns,"ns_per_case_min":per_case[0],
        "ns_per_case_p10":per_case[2],"ns_per_case_median":per_case[10],
        "ns_per_case_p90":per_case[18],"ns_per_case_max":per_case[20],
        "checksum":checksum,"checksum_per_sweep":expected})
}
fn benchmark_queries(
    world: &World,
    inputs: &Inputs<'_, '_>,
    counts: &[Value],
    bindings: &[Value],
) -> Result<Value, Box<dyn std::error::Error>> {
    let expected_path = std::env::var("C6_PLACEMENT_BENCH_EXPECTED")?;
    let expected_bytes = fs::read(expected_path)?;
    require(
        digest(&expected_bytes)
            == "bc65088165f1b57473738f6ec5f784c86b838060d3c8f2c87a8bebcd5240c882",
        "closed expected identity differs",
    )?;
    let expected: Vec<Value> = std::str::from_utf8(&expected_bytes)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    for (actual, kind, source_kind) in [
        (counts, "count_result", "count_expected"),
        (bindings, "unit_binding_result", "unit_binding_expected"),
    ] {
        let native: Vec<_> = expected
            .iter()
            .filter(|r| r["kind"] == source_kind)
            .collect();
        require(actual.len() == native.len(), "native coverage differs")?;
        for (left, right) in actual.iter().zip(native) {
            let mut right = right.clone();
            right["kind"] = json!(kind);
            require(
                *left == right,
                "native ordered result differs before timing",
            )?;
        }
    }
    let mut actors = Vec::new();
    for player in world.players() {
        for u in player.unit_records()? {
            actors.push(BenchActor::Unit(&u.value));
        }
        for f in player.formation_records()? {
            actors.push(BenchActor::Formation(&f.value));
        }
    }
    let mut queries = Vec::new();
    let mut nulls = 0;
    for actor in &actors {
        let anchor = match actor {
            BenchActor::Unit(u) => u.k_location,
            BenchActor::Formation(f) => f.location,
        };
        for plot in placement::query_plots(world.map(), anchor)? {
            if let Some(plot) = plot {
                let candidates = placement::units_at(world, actor.layer(world), plot)?;
                queries.push((actor, plot, candidates));
            } else {
                nulls += 1;
            }
        }
    }
    require(
        actors.len() == 2512 && counts.len() == 17584 && queries.len() == 17359 && nulls == 225,
        "full real corpus coverage differs",
    )?;
    let units: Vec<_> = queries
        .iter()
        .filter(|(a, _, _)| matches!(a, BenchActor::Unit(_)))
        .collect();
    let formations: Vec<_> = queries
        .iter()
        .filter(|(a, _, _)| matches!(a, BenchActor::Formation(_)))
        .collect();
    let actor_lookup = samples("actor_lookup", actors.len(), || {
        actors
            .iter()
            .map(|actor| {
                let (owner, id) = actor.identity();
                let p = black_box(world).player(black_box(owner)).unwrap();
                match actor {
                    BenchActor::Unit(_) => {
                        p.unit(black_box(id)).unwrap().unwrap().resolved_id as u64
                    }
                    BenchActor::Formation(_) => {
                        p.formation(black_box(id)).unwrap().unwrap().resolved_id as u64
                    }
                }
            })
            .sum()
    });
    let layers = samples("layer_and_target_membership", queries.len(), || {
        queries
            .iter()
            .map(|(a, p, _)| {
                placement::units_at(black_box(world), a.layer(world), black_box(*p))
                    .unwrap()
                    .len() as u64
            })
            .sum()
    });
    let unit_counts = samples("unit_count_preselected_candidates", units.len(), || {
        units
            .iter()
            .map(|(a, _, c)| a.count(black_box(inputs), c))
            .sum()
    });
    let formation_counts = samples(
        "formation_max_preselected_candidates",
        formations.len(),
        || {
            formations
                .iter()
                .map(|(a, _, c)| a.count(black_box(inputs), c))
                .sum()
        },
    );
    let complete = samples("actor_lookup_layer_membership_count", queries.len(), || {
        queries
            .iter()
            .map(|(a, p, _)| {
                let (owner, id) = a.identity();
                let player = black_box(world).player(black_box(owner)).unwrap();
                let actual = match a {
                    BenchActor::Unit(_) => {
                        BenchActor::Unit(player.unit(black_box(id)).unwrap().unwrap().record)
                    }
                    BenchActor::Formation(_) => BenchActor::Formation(
                        player.formation(black_box(id)).unwrap().unwrap().record,
                    ),
                };
                let candidates =
                    placement::units_at(world, actual.layer(world), black_box(*p)).unwrap();
                actual.count(black_box(inputs), candidates)
            })
            .sum()
    });
    Ok(
        json!({"schema":"c6-linux-placement-query-benchmark-v1", "native_count_rows_validated":counts.len(),
        "native_binding_rows_validated":bindings.len(),"actors":actors.len(),"present_plot_queries":queries.len(),
        "null_plot_positions_excluded_from_timing":nulls,"unit_queries":units.len(),"formation_queries":formations.len(),
        "modes":[actor_lookup,layers,unit_counts,formation_counts,complete],
        "input_decoding_world_construction_validation_and_harness_allocation_timed":false,
        "query_plot_generation_timed":false,"original_game_execution":false,
        "native_speed_comparison":false,"complete_state_actions_turns":false}),
    )
}
