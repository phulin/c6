# Linux movement-cost native comparison

The prepared-fact Rust movement-cost helper matches all **1,536 genuine adjacent
plot queries** in the pinned Linux XP2 fixture. The independent checker also
matches all **13,824 native phase states**, including each phase's hard minimum,
soft maximum, regular cost and exit addition. There were no scalar, accumulator
or formation-ignore mismatches. No arithmetic correction was needed.

This accepts the bounded arithmetic boundary over observed query facts. Rust
world/rules/query reconstruction is a separate comparison. Movement legality,
executed MoveTo actions, complete gameplay state, complete RNG ownership and
turns remain unimplemented or unaccepted; this is not full movement/game parity.

## Identities and retained controls

The [result artifact](../evidence/linux/movement-cost-results.json) and
[offline checker](../tools/linux/movement_cost_results.py) pin:

- XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
  and the matching Linux host/dependency/rules/configuration admission profile.
- Native helper source `a969de3`, including the independently reviewed
  [XP2 dynamic Map correction](linux-movement-cost-xp2-correction.md), and its
  [separate immutable contract](../evidence/linux/movement-cost-probe-xp2-contract.json).
- Frozen Rust candidate
  `d4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718`, with
  its `cb6113b` source manifest. New working-tree code was not substituted for it.
- Complete native cost corpus
  `0314b8153d2d6e0bb63f70ec22fb4513988fa0a3bfa08c6d9da237c22241657f` and
  fresh fact-only control `b6f4243ee2fde1ae020e55cf5e5e3e853c87b7c268dcd5fada88393254bef276`.

The checker verifies the recorded normal lifecycle/cleanup, both archived source
sets and runner hashes, exact candidate/corpus/contract identities and the actual
retry configuration delta. The earlier 180-second cost attempt timed out after
588 complete rows; its [incomplete prefix](../evidence/linux/movement-cost-timeout-1.json)
remains preserved without a fabricated final summary. The successful retry
changed only the external deadline to 900 seconds and the recording runner hash.
It kept all cases, native calls, identity/quiescence guards and the candidate.
The helper's instrumented elapsed time is not a native movement benchmark.

Fresh control facts and all five definition registries match the cost profile.
In each process, four internal and two outer Core saves are exactly identical:
80,657,462 bytes, SHA-256
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`.
Ten explicitly named within/across-profile raw snapshot comparisons report zero
differing bytes and zero masked bytes. Both observed 32-byte RNG objects remain
unchanged. These observations do not cover every unserialized object/cache,
PCG/temporary RNG or all affected state. The separately recorded 89,198-byte
Save→Load failure remains unresolved and is not treated as a permitted tolerance.

## Actual exercised cases and limits

The scan deliberately stops after 256 formations and reports truncation. It
preserves every one of their six adjacent queries, covering owners 0–4 and native
land/sea/air domains. Observed cases include:

| Actual observation | Cases |
| --- | ---: |
| Hill destinations | 323 |
| Present destination feature definitions | 193 |
| City destinations | 51 |
| Route pairs | 887 |
| River crossings / bridged crossings | 32 / 17 |
| Slowed-by-ZOC queries | 35 |
| Land→water / water→land | 36 / 20 |
| Remaining moves below / equal / above base | 828 / 588 / 120 |
| Terrain / feature / river ignore predicates | 6 / 2 / 6 |

Route costs include actual binary32 values 0.25, 0.5, 0.75 and 1. Six queries
retain the genuine formation-layer sentinel -1. Original missing-layer lookups
and signed-negative grid-index guards return the container's owned empty vector;
no layer was fabricated or case dropped. This establishes a supported cost query,
not that the formation is placed or can legally execute MoveTo.

This corpus has no pillaged-route endpoint, escort-mobility or ignore-shores
example. Counts of free-embark districts or water transitions do not by themselves
establish coverage of every shoreline branch. Nonfinite/out-of-range route floats,
integer overflow, alternate flags/selectors, portals and configuration variants
still need their own original boundary fixtures. The existing synthetic Rust
regressions do not count as native coverage for those missing cases.

## Reproduction

Raw corpora, twelve Core snapshots and source/runtime archives remain local under
`../c6-ghidra/evidence/linux-movement-cost/`, outside the held private upload scope.
The exact checker invocation uses:

```
python3 tools/linux/movement_cost_results.py \
  --native "$COST/facts.jsonl" --control "$CONTROL/facts.jsonl" \
  --candidate "$FROZEN/candidate" --source-manifest "$FROZEN/source-manifest.json" \
  --contract evidence/linux/movement-cost-probe-xp2-contract.json \
  --native-runtime "$COST/runtime.json" --control-runtime "$CONTROL/runtime.json" \
  --prior-timeout-runtime "$PRIOR/runtime.json" \
  --supervisor-timing-change "$COST/supervisor-timing-change.json" \
  --private-candidate-output "$COST/candidate-comparison.jsonl" \
  --snapshot-pair control_outer "$CONTROL/save.bin" "$CONTROL/repeat.bin" \
  --snapshot-pair control_before_control "$CONTROL/before.bin" "$CONTROL/control.bin" \
  --snapshot-pair control_before_after "$CONTROL/before.bin" "$CONTROL/after.bin" \
  --snapshot-pair control_after_repeat "$CONTROL/after.bin" "$CONTROL/after-repeat.bin" \
  --snapshot-pair cost_outer "$COST/save.bin" "$COST/repeat.bin" \
  --snapshot-pair cost_before_control "$COST/before.bin" "$COST/control.bin" \
  --snapshot-pair cost_before_after "$COST/before.bin" "$COST/after.bin" \
  --snapshot-pair cost_after_repeat "$COST/after.bin" "$COST/after-repeat.bin" \
  --snapshot-pair fresh_before "$CONTROL/before.bin" "$COST/before.bin" \
  --snapshot-pair fresh_after "$CONTROL/after.bin" "$COST/after.bin" \
  --output evidence/linux/movement-cost-results.json
```

Here `COST` is `../c6-ghidra/evidence/linux-movement-cost/native-cost-xp2-2`,
`CONTROL` is its sibling `native-control-xp2-3`, `PRIOR` is sibling
`native-cost-xp2-1`, and `FROZEN` is
`target/frozen/movement-cost-probe/d4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718`.
Only the frozen Rust program runs; the checker never loads original game code.
