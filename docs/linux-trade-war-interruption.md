# War interruption of international trade

One persistent Rust Session can initiate a supported major-to-major foreign
route and interrupt it by an external surprise-war decision. Both actions use
the same current state, callback owners and RNG; the candidate imports no native
AFTER state. A separately compared [wartime continuation](linux-wartime-rounds.md)
advances three ordinary rounds on that same Session. This page describes bounded action coverage, not complete Civ VI gameplay,
arbitrary actions, full turns or a complete RNG inventory.

The ordered war subscriber uses
[`ending_stock::interrupt_for_war`](../crates/c6-data/src/loaded_actions/trade_route_actions/ending_stock.rs).
It removes both directed player pairs before cleaning up either route. The
existing edge, route ID and path remain; the edge becomes inactive and its
shared removal counter (`routes_completed`) increments even on cancellation.
Cancellation creates no trading posts and awards no completion era score or
trading-post History moment. Passing-city contributions, the origin's active
route count, diplomatic visibility and reached yield callbacks update normally.

An in-transit Trader relocates through the existing Placement, sight and
UnitMoved gameplay owners. Native relocation mode zero skips location validation
and sends coordinate reporting, including CompleteMovement, through the same
shared tail. It then clears the retained operation slots and finishes movement.
The Trader returns awake with an empty queue and zero movement. The current
player remains under external control; cancellation does not grant another move
or select the next route.

The Linux case starts at `[18, 11]`, targets the foreign capital at `[16, 7]`, and
observes the active Trader at `[18, 10]`. Surprise war returns it to `[18, 11]`.
The actual edge is inactive, with started/removal counts `1/1`, and both trading
posts remain absent. The route manager's next real route ID remains `1`.
Compared Unit/Formation/Map and Player gameplay fields, diplomacy, grievances,
requirements and receiver fields, GlobalAI, district appeal, core processors and
observed gameplay RNG/libc match. No native policy selects an action or turn.

The optimized comparison uses public `512d906a` and private `b174c733` on the
[pinned Linux target](targets.md). Private receipts in `c6-re` are
`evidence/linux/trade-war-session-11-release.json` and
`evidence/linux/trade-war-no-route-baseline-release.json`. They retain exact
source, rules/configuration, canonical inputs and hashes, executable, commands,
profile and actual results. The original interruption capture uses configuration
`0d4398893d5be8476c3601f9446bb30282ab2fbe2ba467d52b81b8d34adc4710`.

Raw full-action equality remains false. Native diplomatic evaluation leaves
score/proposal differences that are source-qualified policy output; reached
military/war knowledge, gameplay callbacks and RNG remain implemented. The
foreign-known-capital and one-owned-luxury query branches have separate source
closure and current-state guards. Native also retains one disposable numerical
query route while Rust retains none. That raw count difference is preserved:
the native record has ID `-1` and no Trader, allocates no real route or graph ID,
and has empty calculation, finalization and clear subscriber rings. Its cache
storage is omitted at the headless query boundary. Non-default query bonuses or
added subscribers are unsupported.

The current interruption boundary allows at most one active route in each
direction between the two players; multiple-edge native cleanup order remains
unsupported before removal. Attached resources, active deals, teammate expansion,
foreign governors, shared sight, territorial expulsion and other unimplemented
Placement/modifier/callback branches remain strict errors. Arbitrary live route
cancellation is not covered. Completed mutations survive an unsupported later
continuation, and callers must not treat that result as a completed action.
