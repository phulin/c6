# Loaded World placement comparison

This diagnostic calculates the existing placement queries from the actual
consuming `LoadedState`, rather than rebuilding Units/Formations from native
getter exports. The closed Native3 Read supplies decoding/framing evidence.
Mutable configuration and war facts retain their separately attributed ff31
SavedOutput snapshot source. Actual full-byte identity and static resource
equivalence are checked; no same-process or Native3 live-state equivalence is
claimed.

`c6-loaded-placement-candidate` takes the strict Core artifact path manifest,
its target runtime, and the closed placement/Manager resource directories.
It issues each Unit and Formation's seven placement queries in native order,
plus Unit definition/Formation bindings. The existing pure placement rules
read the owned World. The one-shot observed snapshot installs actual controls
and transfers owned mutable war state; native expected answers are not inputs.
No script callback, full Manager completion, action or native code is run.

`tools/linux/loaded_placement_results.py` checks a frozen executable/source/
checker manifest, runs that candidate, then opens the independently pinned
native expected rows. It preserves all failures and exact row differences.
This is a query comparison, not a save writer or complete World/action/RNG
acceptance. Historical capture/profile evidence remains immutable.

The frozen `70ff4a1` candidate passed the actual snapshot comparison: all
**17,584 placement rows and 1,262 Unit definition/Formation bindings** match
with zero differences. It calculated these from the imported World, not from
native getter-reconstructed Unit records. Its SHA-256 is
`e24e0a65efa1340179934234f25069c93be524bf71f52e52620ed9706c2fbdee`;
all 1,553 archived source files remained unchanged. Source archive, executable
and freeze manifest are preserved in
`target/frozen/loaded-placement/70ff4a1868a4b43c66d7a7aa6e261fbc3517709c/`.

[The comparison record](../evidence/linux/loaded-placement-comparison.json)
pins the source, checker, capture manifest, target runtime and native answer
digests. World revision remains zero after the read-only queries. Independent
review `c917914` reproduced every ordered match, verified all source/input
identities and passed twelve corruption/separation checks. Native3 Read remains decoding
evidence; the mutable facts remain their actual observed SavedOutput snapshot
view. Complete native Read, actions, turns and RNG parity remain unsupported.
The recorded 89,198-byte raw difference remains unmasked.
