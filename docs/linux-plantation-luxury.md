# Plantation and luxury amenities

This bounded Linux profile extends the persistent produced-Builder `Session` with an Irrigation-gated Plantation on an owned luxury resource, followed by ordinary upkeep. Complete Civ VI state, arbitrary actions and full turns remain unimplemented.

Select `builder_improvement::Kind::Plantation` when preparing improvement definitions, then submit the selected improvement through `Session::build_improvement`. The bounded legality path requires the current Irrigation prerequisite, a visible compatible resource listed by the admitted rules, and an owned feature-free flat unimproved plot. Plantation has no terrain-only placement rows. Its fourteen compatible resource rows include Bananas and thirteen luxuries; equivalent rule-listed resources use the same implementation. Hills, features, replacements and unsupported positive callbacks remain explicit unsupported branches.

Construction reuses ordinary placement, sight, citizen reassignment, worker activation contribution queries, tile yields, housing, Builder movement and charge effects. Its resource callback updates the canonical player's actual owned amount, runs luxury allocation, and dispatches retained resource callbacks and reporting in order. Amenities are derived from those actual allocations, including other cities affected by redistribution. The implementation does not introduce a second state owner or assign allocation counters. All player decisions remain external policy boundaries.

The private native fixture prepares a previously unavailable luxury and acquires Pottery and Irrigation through original APIs before BEFORE. After that boundary, the sequence uses ordinary production, movement, second-city founding, Builder construction and the following upkeep round. It injects no completed unit, position, production progress or research progress after BEFORE. A separate synthetic legality case verifies the same plot is permitted with Irrigation and denied without it; the compared Session is unchanged.

The pinned [original Linux engine](targets.md) completed six rounds with Tobacco at `[17,8]`, a produced Builder spending one charge (`3→2`), and the following ordinary upkeep round. It retained the existing activation, deactivation and unit external-policy guards plus nine city-defender deferrals. Native and development Rust observations agree:

| State | Before construction | After Plantation / after following round |
| --- | --- | --- |
| Owned Tobacco | 0 | 1 |
| Capital assigned luxuries | 1 | 2 |
| Second city assigned luxuries | 1 | 2 |
| Capital amenities | 3 | 4 |
| Second city amenities | 1 | 2 |
| Amenities from luxuries, each city | 1 | 2 |

Development comparison passes at public `2bf1d7e4` / private `0ab2c0f2`: the Session reaches turn 7 / player 0 `DecisionReady`, with every action comparison complete, affected represented gameplay, core processors and known GameSync/libc RNG matching. Full final typed resource state matches for every represented player. The isolated missing-Irrigation legality check also passes.

Worked Tobacco contributes faith. Ordinary upkeep now retains the canonical saturating faith balance, the actual `OnFaithEarned` Script callback and current faith report/cache effects. The same BEFORE image supplies the real signal registrations and absent Script event; unknown subscribers remain explicit errors. The source-qualified headless boundary omits presentation affordability notices without creating a forced purchase decision. The preserved initial replay failures did not count as completed-round acceptance.

Only Tobacco is exercised natively here. The other compatible rule rows are source-derived coverage. Features, hills, replacement, last-charge removal in this sequence, arbitrary positive subscribers, religious Hero purchase queries and complete gameplay/RNG inventories remain outside this profile.

Optimized comparison passes at public `ee056fd1` / private `0ab2c0f2`. The same executable also passes the earlier produced-Builder, produced-Settler and resource-Farm profiles. The original capture used public `563f2704` / private `99aab2c6`, configuration `bc184eca47fa57f0fd0a883836e461f24a1a6d882d38f0c80f14f03dbfebed3d`. Canonical capture `artifacts/linux-plantation-luxury/native-1`, the 89-input manifest referencing the original 88 inputs plus the faith packet, exact source pins, commands, profiles and actual results remain in the private workspace. Receipts include `plantation-luxury-session-3-dev.json` and `plantation-luxury-session-4-release.json` (closing receipt commit `cee785a3`).

One optimized sample measured 1.913 ms for the Plantation request, 20.127 ms across explicit action requests and 87.285 ms for six rounds of upkeep, totaling 107.412 ms. These intervals exclude fixture import, provider/runtime setup, capture/build, packet/file parsing and validation, observations, comparison, external policy selection and JSON serialization. They measure this bounded sequence.

Closing public checks pass 1,831 unit tests and two doc tests (`2bf1d7e4`), plus workspace Clippy with warnings denied, the existing C smoke check and a temporary committed-public-only build (`ee056fd1`). The latter commit only collapses an equivalent nested guard for Clippy. Public production has no dependency on the original engine or private siblings.
