# Bounded Linux movement-world fact comparison

The reviewed frozen world candidate `63d15a41…` matches **1,536 of 1,536 sampled
edges** from corrected original Linux fact control3 in all three comparison
groups, with zero mismatches and zero unsupported rows. This is a typed-wire
movement-fact comparison using named actual runtime inputs. No native cost
functions ran in this control, and complete state/actions/turns remain
unimplemented. It is not full movement or simulator acceptance.

The [result](../evidence/linux/movement-world-control-3.json) is separate from the
[preparation freeze](../evidence/linux/movement-world-adapter.json), which correctly
retains `comparison: null` for the earlier unexecuted checkpoint. The public
[binding](../evidence/linux/movement-world-control-3-binding.json) records exact
artifact identities and original serializer ownership.

## Compared inputs and results

| Group | Equal | Mismatched | Unavailable |
| --- | ---: | ---: | ---: |
| Typed Unit vectors/member lookup and Formation facts | 1,536 | 0 | 0 |
| Saved hashes/route bytes to actual native definition identities | 1,536 | 0 | 0 |
| Plot and edge query facts within the documented resolver boundary | 1,536 | 0 | 0 |

The checker decodes all **1 Map, 1,262 Unit and 1,250 Formation** original
serializer-owned records from the actual Core snapshot. Native discovery selected
256 formations, 257 distinct resolved members and 1,086 distinct plots, with
1,542 member observations across the 1,536 edges. The formation limit truncated
discovery; this does not cover every formation or gameplay branch.

The actual native-order definition inputs contain 17 terrains, 31 features,
42 improvements, five routes and 28 districts. Corrected export revision
`xp2-map-and-terrain-water-2` supplies actual Terrain::IsWater values. No names,
SQL row IDs, SQL order, water defaults or case answers replace those properties.

Actual native district lookup/type/completion observations remain **runtime side
inputs**, covering 1,282 resolved district endpoints and 298 distinct districts.
Actual signed i16 validDomains masks are required at 257 water-with-attributes
endpoints, spanning 122 distinct plots. The candidate computes domain validity
from those raw masks; it does not reconstruct them from saved Plot bytes.
Unreachable domain-valid placeholders on land or absent attributes are outside
that predicate comparison. No Save bytes are excluded or masked.

Observed classes include 887 route pairs, 32 river edges, 17 bridge edges,
51 city destinations, 323 hill destinations, 193 feature destinations and
56 water transitions. Six cases have multiple members. Escort mobility, shore
ignore, absent-member and stale-generation cases are absent from this sample;
their source-only/static evidence is not promoted to native fixture coverage.
All sampled edges are adjacent, so nonadjacent/nonempty portals remain untested.
The candidate does not resolve base/max budgets, global parameters or ZOC here,
and no original scalar cost/rule comparison is claimed by this fact-only result.

## Exact stream and execution boundary

The actual before/control/after/repeat and two outer original Core saves are
all **80,657,462 bytes**, SHA-256 `ff31f4c5…`. Their bytes are identical to the
previously checked canonical stream associated with original serializer trace
`52872108…`. The pinned XP2 module identity also matches. This permits reuse of
those exact original owner boundaries; similar IDs, counts or names would not.
The accepted Unit/Map codec and review reports are linked by the binding. No
Core range is guessed from byte patterns or fixture identity values.

The complete native JSONL is SHA-256 `b6f4243e…`, configuration profile
`dbe92853…`, and the closed runtime report is `c175591a…`. The offline reproducer
checks the complete configuration digest, original host/module identities,
closed execution status, all six raw Core files, both known RNG objects, original
trace identity, and the immutable candidate/checker/source snapshot before
comparing. It invokes only the frozen Rust candidate and Python byte readers.
The original native execution belongs to the separately reviewed runner.

Reproduce without building or running original code:

```sh
python3 tools/linux/movement_world_results.py --output target/linux/movement-world-control-3.json
```

Private original inputs must be present at the binding's recorded locations.
Per-case comparison diagnostics remain in a content-addressed local target path;
the public report contains hashes, counts and the first problem rows (none).
The frozen source and candidate from `0db5d51` remain unchanged. Original arithmetic
artifacts remain separate and are not rebuilt or relabeled by this check.

Six equal Core streams and two unchanged known RNG objects do not establish
complete live state, complete RNG coverage, legality, actions or turns. The
separately recorded **89,198-byte Save→Load failure remains unresolved and
unmasked**. Full native differential acceptance and gameplay parity remain false.
