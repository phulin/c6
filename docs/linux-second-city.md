# Second city and an ordinary round

One retained Rust Session founds a second city for an existing major player,
accepts external research and production choices, and advances both cities
through one ordinary round to the next player decision. This is a bounded
Linux gameplay profile; arbitrary actions and complete Civ VI turns remain
incomplete.

The fixture starts with Washington and an available Settler placed at a legal
second-city site. The external FoundCity request creates Baltimore at `[16, 7]`
for the same owner. This comparison exercises founding and its following round;
it does not reproduce the Settler's earlier production or movement. A separate
[joined expansion comparison](linux-produced-settler-expansion.md) now produces
the Settler through upkeep, moves it legally and founds its city on one Session.

Use the same `Session::from_initialized` owner for `found_city`, research,
`choose_building`, unit orders and `advance(Request::EndTurn { .. })`. Select
Monument externally for Baltimore and Washington, issue the existing Warrior's
Fortify order, and supply every reached player and NPC policy response. The
comparison records activation and deactivation policy responses as well as
legal queued unit orders. Native AI selection does not supply missing choices.

The second founding preserves the existing capital, city identities and roster
order. Ordinary naming excludes names already used and consumes the current
GameSync stream's weighted draw before marking Baltimore used. City-center
construction extracts the reached Olives resource through the current owner's
resource manager, including luxury redistribution and ResourceChanged
subscribers. Territory acquisition uses the retained visibility and modifier
owners. Both consumed Settlers remain delayed until ordinary round-end cleanup.

At turn 2, player 0 receives `DecisionReady`. Both cities have 512 Q8 stored
food. Monument progress is 1,024 Q8 in Washington and 768 Q8 in Baltimore;
technology progress is 768 Q8 and civic progress is 408 Q8. The comparison checks
typed affected gameplay state, core processors, global religion, represented
requirements, growth modifiers and blackboards, relationship modifiers, used
city names, the logical territory-query identity counter and known GameSync/libc
RNG. It reads native AFTER observations only after the simulated round returns.

The development and optimized comparisons use public `93bf8228` and private
`1aa277b8`, with
configuration
`bf10b8daab2d07f586a2324a0c80ec35cc548c68175270eee74f146444a2435b`.
Private `c6-re/evidence/linux/second-city-session-13-dev.json` and
`second-city-session-14-release.json` record the exact target, rules, committed
sources, canonical input hashes, commands, profiles and results.

One optimized replay measured 11.10 ms for FoundCity and 19.33 ms for the
ordinary-round requests. The round measurement includes applying recorded
external orders and policy responses. These single observations exclude fixture
import, provider setup, external policy selection latency, native AFTER
comparison and output serialization; they are not full-game performance claims.

The existing initial-capital FoundCity regression also passes through the
retained Session on public `93bf8228` and private `9ae0dc3c`. Its three pending
decisions remain external. Private
`c6-re/evidence/linux/second-city-initial-founding-regression-release.json`
retains that result and the preceding stale-adapter/input failures.

Raw typed component differences remain recorded; the complete gameplay and RNG
inventory is unfinished. This case does not qualify continent-specific or
duplicate city-name candidates, exhausted-name fallback, artifact extraction,
positive road-network traversal or arbitrary reached callback branches. Unknown
effects return explicit errors with their completed prefix preserved. Callers
must inspect failures and pending decisions before treating an action or round
as complete.
