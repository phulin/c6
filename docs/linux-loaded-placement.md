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
