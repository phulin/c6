# Expanded Linux movement corpus preparation

The next corpus uses the existing runtime budget, `--movement-cost-max-formations 4096`, to scan all currently unit-reachable formations if they fit the supported bound. No helper, arithmetic candidate, schema, original query arguments or default 256 profile changes. This is a prepared configuration, not expanded native acceptance. Complete gameplay state, actions and turns remain unimplemented.

The [offline plan and coverage tool](../tools/linux/movement_expanded_corpus.py) pins the closed cached control/cost runtime identities and computes the exact expected configurations. The [plan](../evidence/linux/movement-expanded-corpus-plan.json) allows only `/lower_core_movement_cost/max_formations: 256 → 4096`. It retains the 900-second supervisor limit, Linux XP2 `a0b6611f…`, Linux host `106d4986…`, existing rules/DLC/mod/Core inputs, frozen helper `d13acf6c…`, frozen Rust candidate `d4dddd41…` and archived runner `c6634f7` (`9aa5e146…`). The canonical expected profiles are:

- Control: `ca3b7f99da6ccc334fc573d17e1bede01bffce3cf46f6f519ecda213756cc263`.
- Cost: `55cddc3140ad426931c6966deb945c11f557b0d86909b1417387e0ace2776736`.

The sole native runner should reproduce its accepted cached launch with the same frozen source view, add the budget argument above, and use fresh exclusive output paths under `../c6-ghidra/evidence/linux-movement-cost/native-expanded-control-1` and `native-expanded-cost-1`, outside the held private snapshot upload. Prepare and verify the control configuration against the plan, run control to normal cleanup, then prepare and run cost. Do not rebuild the helper/candidate or execute the current mutable checkout as the frozen runner. Preserve any incomplete run as a failed prefix; a larger timeout, source change or changed input needs a separate explicit configuration delta and review. All accepted old freezes and the failed 180-second run remain untouched.

Discovery still walks original players 0–63, each original unit-ID vector in native order, and deduplicates actual `(owner, formation_id)` identities. It verifies owner/container lookup, uses the genuine formation position, and asks native adjacency in direction order 0–5. It retains all returned domains and layers, including layer `-1`; no domain, shore, route, escort or ZOC filters are introduced. It does not fabricate map fields, units, budgets, decisions or policy actions. Native state remains authoritative. This does not claim enumeration of all 1,250 serialized formations or slot 64. A formation not reachable through these original unit accessors is outside this discovery path.

Success requires `status=0`, normal native cleanup and `truncated=false`; 4,096 is a bound, not the expected formation count. Border plots need not yield six neighbors. The existing summary's `skipped` aggregate combines missing units, missing formations and formations without a source plot. The current schema cannot separately identify those reasons or every zero-case formation; the offline report retains the raw aggregate and discovered-minus-observed count explicitly. It reports absent directions only for formations with emitted cases, without inventing off-map edges. A nonzero skipped count cannot establish complete placed-formation accounting. Actual owners, formations, borders, member/escort distribution, domains/layers, pillaged routes, shores and ZOC counts must come from the resulting closed facts; none are promised by this preparation.

An offline smoke check over the accepted cached corpus accounts for 1,536 rows / 256 formations and owners 0–4, with no skipped or missing-member rows. The actual layers are `-1: 6`, `0: 1,134`, `1: 288`, `2: 48`, `3: 60` rows. It contains six rows from a two-member formation, zero escort-member rows, zero source-border rows, zero pillaged-route endpoint rows, 36 land→water rows, 20 water→land rows and 35 slowed-by-ZOC rows. Those are existing baseline counts, not predictions or expanded results.

Regenerate the plan without loading original code:

```sh
python3 tools/linux/movement_expanded_corpus.py \
  --output evidence/linux/movement-expanded-corpus-plan.json
```

After both native profiles close, produce a distinct coverage report:

```sh
python3 tools/linux/movement_expanded_corpus.py \
  --control ../c6-ghidra/evidence/linux-movement-cost/native-expanded-control-1 \
  --cost ../c6-ghidra/evidence/linux-movement-cost/native-expanded-cost-1 \
  --output evidence/linux/movement-expanded-corpus-coverage.json
```

The tool validates full archived source/preload/configuration provenance, exact configuration deltas, ordered control/cost fact and registry equality, and the two observed 32-byte RNG endpoints. It reports actual coverage without running Rust or asserting arithmetic acceptance. The subsequent differential checker must compare every native phase and candidate output, preserve failing cases, and compare all six raw saves per profile without masking unknown bytes. Unchanged observed RNG objects do not establish a complete gameplay RNG inventory. The separate recorded 89,198-byte Save→Load failure remains unresolved. Even a successful expanded arithmetic comparison would cover resolved native movement queries rather than movement legality, actions or turns.
