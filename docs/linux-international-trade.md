# International trade routes

The persistent Rust [`Session::trade_route`](../crates/c6-data/src/loaded_actions/session_trade_route.rs)
supports a bounded peaceful route between two major-owned cities through normal
expiry and the next external Trader decision. The external policy supplies the
destination and all other player decisions. The route, ordinary upkeep and
completion share the Session's canonical gameplay, callback and RNG owners.
Arbitrary trade actions and complete turns remain unimplemented.

Admission uses current city identities, civilization levels, contact and war
relationships, both participants' trade restrictions, revealed city plots,
capacity and the active route graph. Cold pathfinding uses the international
land-route restriction for a foreign destination. Initiation retains route IDs,
ordered paths, Trader endpoints, foreign city passing counters and reached stock
callbacks. Diplomatic visibility and its source remain gameplay state;
route-start research boosts use the existing research continuation.

Current city and upkeep queries calculate foreign district yields, transportation
Gold, trading-post Gold and supported city/player/route modifiers with native Q8
arithmetic and multiplier order. Completion makes the edge inactive, increments
its completion count, constructs both trading posts, removes passing-route
contributions, runs the reached visibility/yield callbacks and removes the Trader
operation. The Trader returns awake to its origin with movement available; the
next route choice remains external.

Foreign post history uses the existing HandleMoment continuation. In the
representative two-major case the new post reaches the world-first moment at
Kyoto on turn 24: moment ID 3, five era-score points, and History count 3→4.
The actual first-post technology and civic reward counts are both zero, and the
received-reward flags stay clear. Positive unreceived random boost rewards remain
explicitly unsupported before their flag writes.

Ordinary upkeep preserves reached diplomatic opinion trackers, shared subject
sets, affected-object lists, logical IDs, references and callback order. This
includes final-reference retirement of the initial ten-turn contact requirement
and Bushido's current military/Faith/Culture calculation and tracker reapplication.
Encountered-unit flags update both current and retained Player views before
callbacks. Foreign-territory healing and district occupation use current units
and diplomacy. The reached hurricane preserves its footprint, movement, lifetime,
naming draws and empty-target/coastal damage checks on the observed RNG streams.

The Linux comparison initiates a route from `[18, 11]` to `[16, 7]`, supplies
48 external major-player handoffs and the recorded NPC responses, then advances
24 full rounds through turn 25. All 25 represented trajectory checkpoints match.
Final comparisons match typed gameplay components and rosters, the route graph,
independently observed district appeal, opinion modifiers and requirement graph,
History and current era-score vectors, visibility, grievances, climate and stored
events, logical counters, core processors and observed gameplay RNG/libc state.
The empty Trader queue is corroborated by the inactive completed edge, both
constructed posts, origin location and current external-control readiness.

The optimized comparison uses public `f8244d02` and private `82d9853b` on the
[pinned Linux target](targets.md). Its private receipt is
`evidence/linux/trade-international-completion-session-15-release.json` in `c6-re`;
it records the target/rules/configuration, canonical inputs and hashes, executable,
commands, profile and actual result. Earlier four-round receipts
`trade-international-session-21.json` and `trade-international-session-22-release.json`
remain the accepted ongoing-route observations. The first completion capture's
missing NPC startup join remains recorded in
`international-trade-completion-native-1-limits.json`; the accepted completion
comparison uses corrected `native-2`.

Raw wire equality and full-action flags remain false. Property-container wire
annotations and narrowly source-qualified human defender-policy residue retain
their raw differences. Intermediate wire checkpoints omit unsaved district
appeal/delete fields; final gameplay observations are compared separately.
Nine captured human purchase-hook DEFER calls are retained as policy observations:
the exact native human guard returns before purchase, and the hook already skips
policy selection and its RNG. Nonhuman purchase requests remain strict.

This does not establish complete runtime state, a complete RNG inventory or
full-game parity. [War-triggered interruption](linux-trade-war-interruption.md) has
separate bounded coverage. Other live cancellation, new idle-path edge allocation, occupied naval
storm targets, positive eligible era Gossip delivery and other unimplemented
production, modifier, resource or event branches return explicit errors while
retaining completed mutations. The domestic convenience method remains available.
