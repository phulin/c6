# Unit melee followed by an ordinary wartime round

One retained Rust Session can start the supported foreign trade route, declare
surprise war, execute an externally requested Warrior melee attack against a
visible hostile Warrior, and advance one ordinary round to player 0's next
external decision. State, callback owners and RNG stay retained throughout.
Complete Civ VI gameplay, arbitrary actions and arbitrary full turns remain
incomplete.

This Linux XP2 profile extends the [wartime continuation](linux-wartime-rounds.md)
with two original-engine-initialized, full-health Warriors. The attacker at
(17, 9) attacks the defender at (18, 9). Both survive with 23 and 36 damage,
respectively. The ordinary round then executes reached healing, attack and
formation movement restoration, major war weariness and upkeep. Research,
production, unit orders and every human/NPC policy response remain external.
The endpoint is turn 2, player 0 `DecisionReady`.

Reached visible hostile Unit forecasts now run alongside District forecasts.
The current world supplies legal positions through the stock two-turn planning
search, including hostile Unit zone of control. Forward and reverse proposals
create and release their modifiers and requirements in source order; no native
forecast answers initialize Rust. The actual attack and subsequent callbacks
use one canonical RequirementSystem through
[`Session::with_modifier_requirement_sets`](../crates/c6-data/src/loaded_actions/session/combat.rs),
so temporary combat allocations retain their IDs across the action/round boundary.
The qualified informational combat notice is omitted at the headless output
boundary; its reached gameplay callbacks and RNG remain in scope.

The canonical initialized fixture and typed/live BEFORE observations initialize
the candidate; native AFTER observations are comparison only. On the
[pinned Linux target](targets.md), configuration
`99dc95420391675c26878d5498fcd6391778d59d9bfe4c4ad7d1867ffc4f5e9b`,
the represented sequence, terminal typed gameplay, core processors, known
GameSync/libc RNG, reached requirements/modifiers, callback IDs and opinion graph
match. The final proposal counter is 7853, shared requirement counter 19125,
and no proposal remains active.

The optimized comparison uses public `da4da322` and private `6154cbb1`.
Private `c6-re/evidence/linux/wartime-unit-melee-session-13-release.json` records
exact source/rules/input identities, executable, command, build profile and
result. The existing three-round wartime case is also checked with this source.

In one optimized replay, the melee request took 3.73 ms and the ordinary round
requests took 34.23 ms. Melee timing includes synchronous gameplay callbacks and
mandatory aftermath; round timing covers Session advance/resume/research and
recorded handoff commands. These measurements exclude fixture import, provider
setup, requirement-store metadata joining, initial trade/war commands, checkpoint
comparison, external policy selection and JSON serialization.

The supported forecast search is bounded to single-member, unembarked land
Warriors with maximum 2 moves or Scouts with maximum 3 moves, range 0 and a
two-turn search. The compared Unit targets are visible Warriors, alongside
Districts; Scout targets are implemented from source without a separate native
case. Nearby unseen remembered Units, hidden Units, stacked/improved Unit
targets, other planner shapes, positive emergency eligibility, active military
operations and executable behavior trees remain explicit unsupported branches.
The interruption profile's deal, foreign-governor, shared-sight and expulsion
restrictions remain. This comparison qualifies surviving Unit melee plus one
round. A separate [lethal major-player melee case](linux-wartime-lethal-melee.md)
qualifies defender death and cleanup through one retained ordinary round.

Raw full-action/trajectory equality remains false for preserved source-qualified
policy output and intermediate unsaved District observations. Complete runtime
and RNG inventory coverage is not established. Two small diagnostic native
traces localized search and requirement-owner failures; their libc state differs
already at BEFORE, so acceptance remains on the original canonical fixture.
