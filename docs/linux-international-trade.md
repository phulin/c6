# International trade routes

The persistent Rust [`Session::trade_route`](../crates/c6-data/src/loaded_actions/session_trade_route.rs)
supports initiation and four ordinary rounds of a peaceful route between two
major-owned cities. The external policy supplies the destination and all other
player decisions. The retained route order advances through the existing domestic
path and lifecycle owners. Coverage remains bounded; arbitrary trade actions and
complete turns are not implemented.

Admission uses current city identities, civilization levels, contact and war
relationships, both participants' trade restrictions, revealed city plots,
capacity and the active route graph. Cold pathfinding uses the international
land-route restriction for a foreign destination. Initiation retains route IDs,
ordered paths, Trader endpoints, foreign city passing counters and reached stock
callbacks on the same gameplay and RNG owners. Diplomatic visibility and its
source are canonical gameplay state; route-start research boosts use the existing
research continuation.

Current city and upkeep queries calculate foreign district yields, transportation
Gold, trading-post Gold and the supported city/player/route modifiers with native
Q8 arithmetic and multiplier order. Unsupported positive modifier/resource,
canal/wonder-domain, cultural-dominance and other unimplemented branches return
explicit errors. Foreign route completion and cancellation remain outside the
validated ongoing scope; an unsupported continuation retains its applied prefix.
The domestic convenience method remains available.

Ordinary upkeep preserves reached diplomatic opinion trackers and their shared
subject sets, affected-object lists, logical IDs, references and callback order.
This includes NPC subjects that initialize and then fail their requirements.
Encountered-unit flags update both their current runtime and retained Player
views before callbacks. Foreign-territory healing and district occupation use
current units and diplomacy. Era Gossip preserves its occurrence and reads the
current visibility history and fallback; this case exercises excluded recipients.

The Linux comparison initiates a route from `[18, 11]` to the other major's city
at `[16, 7]`, then supplies eight external player handoffs across turns 1–5.
All five represented trajectory checkpoints match. Final comparisons match typed
gameplay components, the route graph, independently observed district appeal,
all 36 captured opinion modifiers and their requirement graph, encountered-unit
flags, visibility history and fallback, grievance cells, climate and historical
events, logical counters, core processors and the observed gameplay RNG/libc
state. The route remains active at the destination; completion is not claimed.

Both development and optimized comparisons pass this bounded scope on the
[pinned Linux target](targets.md). The private comparison receipts are
`evidence/linux/trade-international-session-21.json` (public `89be514c`, private
`d7a39e85`) and `evidence/linux/trade-international-session-22-release.json`
(public `43526bf5`, private `957e26ee`) in `c6-re`. They retain the native rules and
configuration identity, canonical input hashes, executable identity, commands
and actual results. The focused requirement-set ordering/lifetime suite passes
12 tests.

Raw wire equality and full-action flags remain false. Property-container wire
annotations and narrowly source-qualified human defender-policy residue retain
their raw differences. Intermediate wire checkpoints omit unsaved district
appeal/delete fields; their final gameplay observations are compared separately.
This does not establish complete runtime state, a complete RNG inventory or
full-game parity. Positive eligible era Gossip delivery, hostile aircraft
rebasing and other unimplemented continuations remain explicit errors.
