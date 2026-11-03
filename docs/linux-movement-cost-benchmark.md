# Linux movement arithmetic benchmark

This release benchmark measures the accepted Linux XP2 movement-cost arithmetic over already prepared per-edge facts. It measures neither world/runtime queries nor a movement action or turn. Complete gameplay state, action execution and turns remain unimplemented. The accepted comparison covers 1,536 real directed edges from a deliberately truncated 256-formation scan; all rows, including the six layer `-1` rows, remain in the benchmark.

The two modes reset an accumulator, apply the rules and obtain the final cost. `zoc_aware` also applies the hard minimum using the supplied original ZOC predicate. `no_zoc` omits that prelude. Each timed edge uses `black_box` on its typed input and complete returned accumulator/cost, and adds the cost to a wrapping checksum. Reported time includes iteration, typed input copying, barriers and checksum overhead. It is the repeated, resident-corpus workload on this host, without CPU pinning or an original-code speed comparison.

JSON reading, typed preparation, native expected-output validation, formation aggregation checks, configuration checks, hashing and building occur outside the cost loops. Preparation and validation durations are reported separately. Registry lookup, world resolution, save/codec work, native execution, movement legality, state mutation, RNG, player policies and turns are excluded. Facts do not establish the exactness of their future runtime resolver. No full-turn throughput may be inferred.

`tools/linux/movement_cost_benchmark.py` requires the complete accepted comparison report and its explicit SHA-256. It checks native/control/contract/source-manifest identities, immutable candidate sources, the accepted unchanged arithmetic source, the successful native/control/state gates and every output from the frozen candidate. It builds a distinct release binary and records its hash, source hashes, command, compiler, environment and machine. The benchmark binary validates every native final accumulator/full/noZOC output and formation aggregate before warmup or timing. Both timed evaluators are checked directly against native expected values; aggregate timed checksums are checked by the driver against the frozen candidate results. Incomplete or mismatching corpora fail before timing.

From the repository root, use the complete private corpora and the accepted report hash:

```sh
python3 tools/linux/movement_cost_benchmark.py \
  --native ../c6-ghidra/evidence/linux-movement-cost/native-cost-xp2-2/facts.jsonl \
  --control ../c6-ghidra/evidence/linux-movement-cost/native-control-xp2-3/facts.jsonl \
  --accepted-report evidence/linux/movement-cost-results.json \
  --accepted-report-sha256 e7db3c366a762e30dc761e8c4507f127505745d34a0f6910f1489c4d415d8458 \
  --contract evidence/linux/movement-cost-probe-xp2-contract.json \
  --candidate target/frozen/movement-cost-probe/d4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718/candidate \
  --source-manifest target/frozen/movement-cost-probe/d4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718/source-manifest.json \
  --rounds 10000 --samples 7 --warmup-rounds 1000 \
  --output evidence/linux/movement-cost-benchmark.json
```

The frozen candidate is never rebuilt or relabeled. The new benchmark artifact must independently pass its validations. The command pins the accepted report from commit `2548852`; the checked-in benchmark report records that SHA independently. Each sample contains 15,360,000 cost evaluations. Timings are observations, not acceptance thresholds. Native phase acceptance and raw state evidence remain in `movement-cost-results.json`; unchanged known streams do not establish a complete RNG inventory. The separate recorded 89,198-byte Save→Load failure remains unresolved, with no unknown bytes masked or excluded.

The recorded run measured median **23.81 ns/edge** without the ZOC prelude and **24.33 ns/edge** with it (seven samples per mode). This is prepared arithmetic including the loop/barrier/checksum overhead described above, on the recorded AMD EPYC 7642 48-Core Processor host. The distinct benchmark executable SHA-256 is `8af1e339671e18eb4cff8da8b6e32a43a0f28e6c4866fcb5e26abd43ef0fc452`. All 1,536 cases validated before timing; neither result is full runtime query or turn throughput.
