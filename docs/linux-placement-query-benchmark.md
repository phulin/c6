# Linux placement query benchmark

This measures implemented Rust lookup and placement-count queries using all
actors and current/neighbor positions from closed Linux capture `native-1`.
It does not measure gameplay actions, turns, save loading, native execution, or
native speedup. Complete gameplay state, actions and turns remain unsupported.

The separate benchmark builds core/data and the strict candidate loader from
committed source `301f9e9d188f6a5ac815914218d0380f51b25938`, archive SHA-256
`560bb94b98da8a187118e6cbc82f60d279e6d008c12884bd07a1b878c1b6b559`.
The builder generates a new binary from that candidate source plus the committed
[timing fragment](../tools/linux/placement_query_benchmark.rs), inserting its call
after count/binding reconstruction. It preserves the frozen comparison binary and
configuration/profile. The native expected file is opened after reconstruction;
every ordered count and binding row must match before any timing starts.

The strict input contains actual Config/war/definition facts and owner-attributed
Map/Unit/Formation records. No count, binding or handler answer enters that input.
Coverage is 1,262 Units, 1,250 Formations, 17,584 count-position rows, 1,262 binding
rows, 17,359 actual counts and 225 null-plot positions. Null positions are validated
but excluded from per-count timing. All current and adjacent present positions
participate, including duplicates and absent-layer cases.

| Mode | One timed case |
| --- | --- |
| `actor_lookup` | World Player lookup and Unit/Formation recycled-slot lookup, once per actor |
| `layer_and_target_membership` | Subject layer discovery and target cell lookup, once per present position |
| `unit_count_preselected_candidates` | Unit count with borrowed subject/candidate slice prepared in advance |
| `formation_max_preselected_candidates` | Formation member lookup and maximum of Unit counts with prepared candidates |
| `actor_lookup_layer_membership_count` | Actor lookup, layer discovery, target membership and count, once per present position |

Input reading, decoding, world/index construction, validation, harness allocation,
coordinate generation and JSON output are outside timed loops. The production
query APIs used inside these loops borrow state without allocating. The complete
query mode repeats subject/layer lookup per position; the original candidate
instead shares layer discovery across an actor's seven positions. The modes
therefore describe separate API costs and must not be added into a candidate
runtime estimate.

Each mode warms and calibrates to at least 10 ms per sample (subject to an 8,192
sweep cap), then takes 21 samples covering the complete mode corpus each sweep.
Black-box inputs/results and checked aggregate checksums resist optimization.
The evidence keeps raw sample durations, repetitions, median, p10/p90 and extrema.
These are repeated warm-cache queries on one fixture and one host, with shared
host scheduling noise; they do not establish broader scaling or workload costs.
Timing includes loop, black-box and checksum overhead.

The [first result](../evidence/linux/placement-query-benchmark-native-1.json) used
Rust 1.98.1 on an AMD EPYC 7642 host. It validated every native count/binding row
before measuring 8,648 Unit and 8,711 Formation present-position queries.
After removing the benchmark metadata, its complete parsed output equals the
accepted frozen candidate output, including unchanged typed-state controls.

| Mode | Median ns/case | p10–p90 ns/case |
| --- | ---: | ---: |
| Actor lookup | 17.64 | 17.49–18.25 |
| Layer and target membership | 34.74 | 34.45–35.83 |
| Unit count with prepared candidates | 52.81 | 49.78–55.27 |
| Formation maximum with prepared candidates | 71.50 | 70.82–75.64 |
| Actor lookup, layer, membership and count | 117.16 | 114.89–125.77 |

Count/filter work and Formation member traversal cost more than the separate
actor lookup in this fixture. These component timings are evidence for where to
investigate if placement becomes material in a larger implemented workload; they
do not justify changing exact query behavior or predicting action/turn throughput.

Reproduce using a create-only build directory and output report:

```sh
python3 tools/linux/placement_query_benchmark.py \
  --harness-commit HARNESS_COMMIT \
  --input PRIVATE_STRICT_INPUT_JSON --input-sha256 INPUT_FILE_SHA256 \
  --expected ../c6-ghidra/evidence/linux-placement-metadata/native-1/expected-counts.jsonl \
  --comparison-report evidence/linux/placement-count-native-1.json \
  --directory target/linux/placement-query-benchmark-NEW \
  --output evidence/linux/placement-query-benchmark-NEW.json
```

Use the accepted durable checker's strict projection and independently check its
input identity. Paths can differ between projections; the supplied SHA-256 is of
the exact JSON file including any final newline. Builds are locked/offline release
builds with incremental compilation and inherited Rust flags/wrappers disabled.
The runner also checks strict release clippy for the generated binary. Reports
contain hashes, counts and timing only; proprietary input/output remains private.

Handler rows remain native-uncompared, complete affected-state/RNG coverage is
unavailable, and the separate raw Save→Load 89,198-byte failure remains unresolved
and unmasked. A timing result adds no acceptance beyond the checked query subset.
