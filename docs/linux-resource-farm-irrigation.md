# Owned Wheat Farm and Irrigation

This bounded Linux profile extends the [produced Builder expansion](linux-produced-builder-expansion.md): an actual produced Builder farms owned Wheat, then ordinary upkeep continues in the same retained Rust `Session`. Complete Civ VI state, arbitrary actions and full turns remain unimplemented.

The initialized fixture uses original resource-builder APIs before BEFORE to place one legal Wheat resource at `[17,8]`, with count one. Production preparation also precedes BEFORE. After that boundary, external policies order ordinary production, movement, second-city founding, Builder travel, Farm construction and the following round. No completed unit, position, production progress or research progress is injected after BEFORE.

`Session::build_improvement` retains the existing Farm placement, sight, worker allocation, housing/yield and charge effects. The bounded legality path accepts owned, featureless, flat first placement when the captured rules list the resource as valid for a Farm and it is visible. The admitted rules list Rice and Wheat; Wheat is exercised natively. Features, hills, replacements and unknown positive callbacks remain explicit unsupported branches.

The constructed callbacks use the owner's current revealed resource layer when checking the resource-dependent improvement boost. A matching trigger borrows the Session's canonical research runtime, notifications and callbacks; it does not assign research progress directly. The same canonical stock resource backend changes owned Wheat by the plot count, runs luxury allocation and dispatches the retained resource signal order. Worker activity changes dispatch the same improvement contribution-query lifecycle as ordinary upkeep, retaining current and adjacent query identities and callback ownership. Unknown positive callbacks and unsupported monopoly state remain explicit errors with the completed prefix retained. All player decisions remain external.

The original pinned [Linux engine](targets.md) completed this six-round sequence with produced Builder `262146`, three-to-two Farm charges and the following ordinary round. The native fixture retained nine city-defender external deferrals and the established activation, deactivation and unit deferral guards. Its configuration is `0061b2497e1b61c75969bfa0f4d1ea7905b32c9d7b35bf4da1969ecab6bc067f`.

Development and optimized comparisons pass with public `d59e8da0` and private `6003a097`. The original capture used public `d05bef49` and private `86d7bc03`. The same retained Rust state records the following transitions; its post-Farm values persist through the following round and match native final AFTER:

| State | Native BEFORE / Rust before Farm | Rust after Farm / both final AFTER |
| --- | --- | --- |
| Irrigation boost triggered | false | true |
| Irrigation research progress, raw Q8 | 0 | 4,864 |
| Wheat ownership slots | `[0,0,0]` | `[1,0,0]` |

The Builder spends its movement and one charge normally. A citizen changes from not working the Farm to working it, and represented affected gameplay, core processors, requirements, modifier state, city names, territory and known GameSync/libc RNG match after the following round. Both executions retain next logical plot-yield query identity 21. The Session returns player 0's `DecisionReady` boundary at turn 7.

The boost uses the existing once-only research kernel; its repeated-trigger synthetic test passes, and the boost remains set through subsequent upkeep. This sequence does not exercise a second native resource Farm trigger. Native Rice coverage, last-charge removal in this produced-Builder sequence and complete gameplay/RNG inventories remain outside this comparison.

The initial missing boost-input failure and the worker-activation query mismatch remain preserved. The latter was a missing reached callback, corrected by reusing ordinary upkeep's canonical lifecycle; no counter assignment or comparison exclusion was added. The sealed native capture is retained once at private `artifacts/linux-resource-farm-irrigation/native-1`. The retry reuses 87 unchanged inputs and reprojects only the research packet from the same BEFORE image. Exact source pins, input hashes, commands, profiles and actual results are retained in private receipts, including `resource-farm-irrigation-session-3-dev.json` and `resource-farm-irrigation-session-4-release.json`. The same optimized executable also passes the earlier resource-free produced-Builder profile and four-round produced-Settler profile.

One optimized sample measured 2.304 ms for the Farm request, 20.917 ms across explicit action requests and 86.476 ms for six rounds of upkeep, totaling 107.393 ms. These intervals exclude fixture import, provider/runtime setup, capture/build time, file and packet parsing, diagnostic observations, comparison, external policy selection and JSON serialization. They are not full-game performance claims.

Closing workspace verification passes 1,831 unit tests, two doc tests, Clippy with warnings denied and the C interface smoke check. A temporary export of the committed public Cargo/crates/header inputs also builds without private siblings, reusing the shared target and removing its temporary sources afterward. Public production has no dependency on the original engine or private fixtures.
