# International trade routes

The persistent Rust `Session::trade_route` accepts an explicit destination for a
bounded peaceful route between two major-owned cities. The policy supplies the
route choice; the retained order continues through ordinary upkeep. This extends
the existing domestic path and lifecycle owners and does not implement complete
trade, arbitrary actions or complete turns.

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

Native initiation and ordinary-turn comparison is pending. The implementation's
source-qualified boundary and synthetic checks do not establish complete runtime
state, a complete RNG inventory or full-game parity.
