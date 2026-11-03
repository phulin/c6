# Closed expanded Linux movement query comparison

The frozen Rust candidate agrees with all 7,461 ordered query result rows from the closed expanded Linux cost profile. Independent arithmetic replay agrees with all 67,149 native accumulator states across nine phases per row. This extends the measured query corpus; complete gameplay state, action execution and turns remain unimplemented.

The profiles contain 1,250 formations reached while scanning 1,262 units, zero skipped formations, and `truncated=false`. The checker retains all recorded cases, actual formation layers and genuine sentinels. It does not fabricate edges for null neighbors. Detailed coverage classification belongs to the separate expanded corpus audit.

`tools/linux/movement_cost_expanded_results.py` verifies the immutable accepted cache report and checker, shared replay helper, reviewed cache sources, runner, expansion plan and review, Linux target contract, frozen candidate/helper, candidate source manifest and referenced source hashes. It verifies each closed runtime's canonical configuration, 60 archived source files, archived runner, integrated preload, standalone configuration and candidate manifest, facts identity, and normal framework/client cleanup.

The entire expanded configuration is constructed from its corresponding accepted cached configuration by changing only `lower_core_movement_cost.max_formations` from 256 to 4096. Expanded control and cost configurations differ only by mode. No helper, preload or candidate refresh is admitted. The fresh control's ordered facts and five registries equal the expanded cost observations after projecting only named process addresses and cost outputs. Both old 1,536-case prefixes match the expanded profiles; the cost prefix comparison retains every cost scalar and phase.

The checker executes only the SHA-gated frozen Rust candidate `d4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718`, never original game code. It checks every candidate row's exact keys, ordered owner/formation/unit/direction identifiers, six equality flags, full and no-ZOC outputs, and the complete final summary. Raw output remains private in the closed expanded cost directory. No Rust or native component is rebuilt.

All six actual 80,657,462-byte CoreSave streams in each of the four cached/expanded profiles compare directly against the accepted cached cost `before.bin`: 24 pairs, zero differing bytes and zero masked bytes. All have SHA-256 `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`. These observations establish retained serialized bytes for these queries; they do not establish complete affected-state ownership or state semantics.

The full raw 32 bytes of each known Game and Map Random32 object are unchanged before/after within each process. Known seed, call/reset/lock counters and the raw sync byte agree across the four profiles. Vptrs and padding remain unclassified, retained in raw within-process comparisons; no cross-process vptr gameplay assertion is made. Complete gameplay RNG inventory remains unresolved.

Reproduce the closed offline comparison from the repository root:

```sh
python3 -m py_compile tools/linux/movement_cost_expanded_results.py
python3 tools/linux/movement_cost_expanded_results.py \
  --private-candidate-output /home/phulin/c6-ghidra/evidence/linux-movement-cost/native-expanded-cost-1/expanded-candidate-comparison.jsonl
```

The public result is `evidence/linux/movement-cost-expanded-results.json`; it contains hashes, counts, exact configuration differences and retained state/RNG proof, with no raw game facts. Validation also rejected ten mutations: unrelated timeout, extra configuration key, wrong formation bound, dropped result row, reordered rows, wrong identifier, false equality flag, wrong output scalar, extra result field and wrong summary count. These checks modified copies in memory.

This acceptance covers resolved movement query arithmetic and the recorded state/RNG observations. It establishes no movement legality, action execution, complete state, complete RNG, turn equivalence or native query speed. Native AI policies cannot fulfill external player decisions. The separate raw Save→Load failure of 89,198 bytes remains unresolved; unknown bytes are neither masked nor excluded.
