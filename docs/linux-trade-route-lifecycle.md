# Domestic trade-route lifetime

The persistent Rust `Session` supports a bounded domestic trade route through
normal turn progression, return and completion, ending at the next external
Trader decision. This is Linux gameplay coverage, not complete trade, turn or
full-game support.

The external policy supplies the destination to
[`Session::domestic_trade_route`](../crates/c6-data/src/loaded_actions/session_trade_route.rs).
The queued operation then advances automatically using the same current world,
route graph, movement, sight, research and callback owners as the action and
ordinary stock turns. The round and action services share the canonical trade
manager. Automatic route continuation executes the existing order; research,
production and all other player decisions remain external.

The representative comparison starts a domestic route between two owned cities
and advances 24 ordinary rounds, from turn 1 through turn 25. Trader 262147 follows
path `[502, 458, 413, 369, 324]`, completes its normal outbound/return travel and
returns to `[18, 11]`. The stored edge becomes inactive with one completed route;
its route ID and path remain intact. Completion executes trading-post and yield
callbacks. The derived operation deactivation checks the current route, finishes
movement and performs the normal report and retained operation-slot removal.
At turn 25 the Trader is awake, has 512 Q8 movement, and has no operation,
automation or controller. The next route choice is external.

This sequence also exercises ordinary unit production, city growth and culture
annexation, the six-population civic boost, reached requirement/modifier effects,
and two weather events, including a positive storm through expiry. No native AI
policy chooses research, production, unit orders or NPC actions for the simulator;
the comparison supplies its recorded external responses in order.

All 25 represented trajectory checkpoints match: typed Map, Unit and Formation
state, their rosters, the represented Player fields and GameSync state. Final
comparisons also match the route graph, independently observed district appeal,
stored storm and event history, represented requirement/modifier state, logical
allocation counters, core processors and tracked global RNG/libc state on the
[pinned Linux target](targets.md).

The raw comparisons remain unequal. Property-container wire annotations have a
separate semantic comparison; the narrowly source-qualified human defender role
and city-assignment fields are excluded as policy-planning bookkeeping while
retaining their raw differences. Intermediate wire exports do not observe
unsaved district appeal/delete fields. Native AI-owned fields, full runtime state
and the complete RNG inventory are not established by this case.

Foreign initiation and four rounds of peaceful major-to-major upkeep have
[separate bounded coverage](linux-international-trade.md). Foreign completion,
live route cancellation, new idle-path edge allocation and other unimplemented
production, modifier or event branches return explicit errors.
Completed mutations remain visible after an unsupported continuation. These
limits do not turn unsupported work into a successful empty action.
