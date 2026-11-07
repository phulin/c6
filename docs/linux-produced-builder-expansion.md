# Produced Builder serving a second city

This bounded Linux profile joins Settler production, externally ordered travel,
second-city founding, Builder production, a legal Farm and the following round
on one retained Rust `Session`. Complete Civ VI state, arbitrary actions and
full turns remain unimplemented.

The initialized fixture begins with Washington at `[18, 11]`, population two,
and a queued Settler one Q8 production unit below its actual cost of 80. The
capital also has stored Builder production one Q8 unit below its actual cost
of 50, prepared through original queue APIs before BEFORE. Earlier production
is fixture setup; this profile does not claim to simulate either full build
duration. Neither execution injects a completed unit, teleports an actor,
changes production progress after BEFORE or reloads an endpoint as input.

All requests use the same `Session::from_initialized` owner:

1. External research, civic and Fortify choices precede ordinary upkeep, which
   produces Settler `196608` and consumes the capital's population normally.
2. `Session::choose_unit` selects Builder production externally. During the
   Settler's recorded travel and ordinary rounds, upkeep creates Builder
   `262146` at the capital with three charges. Select Monument externally.
3. The Settler reaches `[16, 7]` through two `Session::move_to` requests and a
   genuine movement-restoration round. `Session::found_city` creates New York;
   select its Monument and complete its first ordinary round.
4. The produced Builder travels from `[18, 11]` through `[17, 9]` to `[17, 8]`
   using two externally ordered MoveTo requests across a genuine round. This
   featureless, resource-free tile belongs to New York; original improvement
   legality accepts a Farm there.
5. `Session::build_improvement` constructs that Farm, changing the Builder's
   charges from three to two. The following ordinary round returns the
   retained two-city state at turn 7, player 0's next decision boundary.

Every reached player decision remains external, including NPC policy responses.
Native policy selection supplies no missing choices. The action runtime,
visibility, city owners and gameplay RNG persist across this entire sequence.
Source-established unsupported branches continue to return explicit errors with
the completed prefix retained.

The corrected original Linux profile completed all six rounds against the pinned
[Linux target](targets.md), configuration
`9a0a5f1cc6ae6687c20afd3d05db780408f9b8f16b7d7ff473ec870d2f596659`.
It recorded nine external deferrals at the exact city-defender selector call,
retaining the surrounding city activation gameplay. An earlier capture's
native policy-assignment mismatch remains preserved. The development and optimized
comparisons match this corrected capture using public `b09bac8e` and private
`cd7374c5`.

Intermediate checks cover produced identity and charges; movement before and
after every MoveTo; restored movement before FoundCity; and Farm movement,
processed state and three-to-two charges. Affected typed gameplay, core
processors, global religion, represented requirements, modifiers, city names,
territory query identity and known
GameSync/libc RNG match. The retained Session returns `DecisionReady` for player
0 at turn 7. Farm's improvement-added report now preserves its temporary
contribution-query creation/destruction before visibility updates; the next
logical query counter is 20 in both executions. Raw component differences and
incomplete gameplay/RNG inventories remain explicit limits.


One optimized sample measured 130.30 ms across production request intervals:
24.00 ms for explicit actions and 106.30 ms for turn advancement. Farm
construction took 2.73 ms. These sums exclude fixture import, runtime/provider
setup, file and packet parsing, external policy selection, diagnostics,
comparison and JSON serialization. They are not full-game performance claims.
The same executable also passes the earlier four-round produced-Settler profile.

Workspace verification passes 1,830 unit tests, two doc tests and Clippy with
warnings denied. A committed public-only export builds without a private sibling.
The private Linux comparison receipts record the exact native and Rust source
pins, rules, canonical input hashes, commands, build profiles and actual results;
initial setup failures, the missing boost input, the native policy-assignment
mismatch and the missing Farm query callback remain preserved.
Private `c6-re/evidence/linux/produced-builder-expansion-session-5-dev.json`,
`produced-builder-expansion-session-6-release.json` and
`produced-builder-expansion-settler-regression-1-release.json` retain those results.

The [owned Wheat Farm and Irrigation profile](linux-resource-farm-irrigation.md)
extends this retained Session sequence with rule-compatible resource acquisition,
the research boost and the following ordinary round.
