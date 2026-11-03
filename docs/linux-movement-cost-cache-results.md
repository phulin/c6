# Movement admission-cache comparison

The distinct [offline checker](../tools/linux/movement_cost_cache_results.py) compares the reviewed movement admission cache with the accepted frozen Linux XP2 movement query fixture. It leaves the [older acceptance report](../evidence/linux/movement-cost-results.json) and its supervisor-only retry checker unchanged. The [cache result](../evidence/linux/movement-cost-cache-results.json) passes: all 1,536 ordered query rows and 13,824 phase states match; the frozen candidate reports zero mismatches. Both native profiles closed normally before this offline comparison.

Run from the repository root after the separate native runner has closed both archives:

```sh
python3 tools/linux/movement_cost_cache_results.py \
  --private-candidate-output /home/phulin/c6-ghidra/evidence/linux-movement-cost/native-cache-cost-1/cache-candidate-comparison.jsonl \
  --output evidence/linux/movement-cost-cache-results.json
```

Defaults select `native-cache-cost-1`, `native-cache-control-1`, `native-cost-xp2-2` and `native-control-xp2-3` under the private `c6-ghidra/evidence/linux-movement-cost` directory. The checker never loads original code or builds a candidate. It executes only the hash-pinned frozen Rust candidate against the already closed cost corpus. Original facts, saves, RNG endpoints and candidate output remain local; the public result records identities and comparison summaries.

The admission gate constructs the entire expected cache configuration from the hash-pinned accepted cost configuration. It replaces only the exact reviewed loader/lifecycle/audit sources, runner, integrated preload and explicit cache revision metadata. Every other configuration value must match. Fresh cache control and cost configurations may differ only in movement mode. The report retains every configuration change, all four complete archived source manifests, verified archived source counts and hashes, and the full control-baseline configuration diff. Normal framework/client cleanup, canonical configuration hashes, actual archived preload bytes, all archived sources, frozen contract/helper/candidate, and immutable candidate source revision are required.

The comparison retains all 1,536 rows in order, all 256 formations, genuine sentinels and six layer `-1` rows. It compares facts and five registries with both controls; cost outputs and all nine phase states with accepted cost; and independently replays 13,824 phase states before running the frozen candidate. Only explicit process-address metadata is projected from facts. No saved bytes are projected or masked. All six Save streams from each of the four profiles are compared byte-for-byte with the accepted cost pre-query stream, and both observed Random32 objects are compared as raw 32-byte before/after endpoints within each process. Across all four profiles it also compares the known owned Random32 seed, calls, resets and locks fields at offsets +8/+12/+16/+20 and the raw sync byte at +24. Native vptr addresses are retained in that within-process comparison; they are not treated as cross-process gameplay values.

This boundary establishes query/helper behavior invariance under the reviewed harness change if every check passes. It makes no native engine or turn speed claim. Scan truncation remains true, movement actions and legality remain unimplemented, complete affected-state and gameplay RNG ownership remain unproven, and the separate 89,198-byte raw Save-to-Load failure remains unresolved and unmasked. The project still lacks complete gameplay state, action execution and turns; bounded movement query evidence does not supply those systems.

Validation passed with Python syntax compilation, the complete offline comparison, and negative checks rejecting an unrelated configuration edit, an altered reviewed source hash and changed raw RNG bytes. Every one of the 24 unmasked Save comparisons reports zero differing bytes; each of the four profiles retains six streams with SHA-256 `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`. Both observed Random32 objects have identical raw endpoints within each process. Public RNG metadata records endpoint hashes; original endpoint bytes remain in the private corpora.

Known owned RNG fields also match across all four profiles: Game seed/calls/resets/locks `872261551/922992/0/0`, sync byte `1`; Map `0/0/1/0`, sync byte `0`. Vptr and padding bytes remain unclassified and retained; no unknown bytes are masked.
