# Ordinary melee against a city

The persistent `Session::melee_attack` supports an ordinary unembarked land
melee attack against a major-owned city-center District when the attacker and
city survive, or the supported noncapital capture branch is reached. The
survivor path includes the bounded Ancient Walls profile below; the compared
capture profile is unwalled. The external policy supplies the MoveTo request.
Complete Civ VI state, arbitrary combat and complete turns remain unimplemented.

Use the shared [melee setup](linux-melee-combat.md), provide
`melee_attack::city::Definitions` in `execution::GenerateRules::city`, and retain
the same combat, sight, script, requirement and effect owners. Placement queries
need current city-domain masks, civilization levels and civic activity. Bind
both formation combat and District definitions to the stock reporting provider
with `bind_combat_placement_queries`; notification upkeep also needs the current
technology and civic graphs. City report publication requires a same-origin
`city::ReportScope` qualified against the actual synchronous consumers.

Resolution runs the ordered temporary combat proposal and its callbacks, three
GameSync damage draws, Unit and District damage, experience, formation movement,
attack budget, war weariness, combat callbacks, history and notification upkeep.
The third damage draw occurs even without walls. Failures retain their executed
prefix and freeze the Session. Attacker death, special wall prevention modifiers
and other unsupported branches return explicit errors.

The retained Linux comparison attacks a noncapital Japanese city while its
owner retains another city. The attacker receives 18 damage, the city receives
50 damage, and the attacker gains 3 XP, exhausts movement and its attack, and
clears its operation queue. The complete request reaches
`HeadlessMessagesProcessed` and matches the compared gameplay state, callbacks,
history, visibility and ordered GameSync draws (bound 12, values 4, 7, 8).

Native AI targeting is excluded. `CityAttacked` only changes a targeting tally
and consumes no RNG; the library emits a typed `CityAttackPlanningObservation`.
The comparison retains the raw discrepancy (attacker tally for the defender:
0 versus native 2250) and compares every other Targets field. This exclusion
provides no permission to omit gameplay or RNG from AI-named services.

The comparison used public commit `35d7a71dd74c5bbbe489cdbb0de335a01882eb84`,
private adapter commit `90f6a9b59781c5b85c732ca7d802679caa195403`, and the
[pinned Linux target](targets.md). Configuration SHA-256 was
`bb849332b058a0db07d09a4351a31d76fc14d6847205314a48dd2b01585aad98`;
gameplay database SHA-256 was
`c880adaedcfd6da7808302b3b76bb9cad7d3188b35ae78e780cff7b56e04477e`.
Private receipt `evidence/linux/city-melee-survivor-session.json` retains the
original/candidate source pins, canonical inputs, executable hash, command,
results and earlier failures.

One Cargo release measurement was **5.529 ms** for the request, combat callbacks
and notification aftermath in this three-city fixture. It excludes import,
service construction and comparison. This is one bounded action measurement,
not complete-game throughput or a native speedup claim.

A second retained Linux case completes lethal capture of that noncapital city
while both players retain their capitals. The Warrior takes 12 damage and gains
10 XP, enters the city, exhausts movement and its attack, and clears its operation
queue. Capture destroys the old City and reconstructs the transferred City using
the same territory, sight, script, modifier and requirement owners. Territory,
population, current culture/loyalty, diplomacy, callback lifetimes, history and
notification blocking match the compared native state. All five ordered GameSync
draws match: bounds 12/12/12/1/3, values 4/7/8/0/2, ending at call 609 and seed 3853515706.

The human capture queues a semantic disposition at the native callback point,
then finishes melee cleanup. `Session::pending_city_disposition` exposes the
external choice after completion; the pending state survives rewrapping and gates
later Session actions. The accepted case stops at this decision boundary.
Separate bounded [Keep City](linux-city-keep.md) and [Raze City](linux-city-raze.md)
transitions resolve an initialized completed choice. Return and Liberate remain
unimplemented; this capture comparison establishes the pending boundary. A
separate [capture-to-Keep sequence](linux-city-keep.md) now validates both actions
on one persistent Session, reusing the live owners without importing another
state or injecting a completed decision.

Capture needs current founding/transfer services, complete affected modifier and
lifetime inputs, and stock City placement/query services. Postcapture queries read
the same territory owner after capture releases its mutable loan. Native policy
selection and source-qualified human CityBuild planner payload are omitted;
logical identities, contract owners, trade state and the synchronous off-turn
completion flag still match. Raw planner differences remain in the private result.

The capture comparison used public commit
`d102d9a17d29151717d8a5b429365d0abdc4e3aa` and private adapter commit
`ff424e82130142928562b6efb0e0b5ba8c82a0d9`, with configuration SHA-256
`b75ae33b3899262740bb50dc1b4d9d894ad2b28aa4c26f8d0370e1f2d8068144`.
Private receipt `evidence/linux/city-melee-capture-session.json` retains the closed
native fixture, complete input/result hashes, exact source identities, commands,
profiles and earlier failures. This is one supported capture profile; arbitrary
cities, capture through walls, capitals, effects, disposition execution and a complete gameplay
RNG inventory remain outside its acceptance.

One Cargo release measurement for the capture request was **11.877 ms**
(11,876,796 ns). This single sample covers action execution, reached capture/combat
callbacks and notification aftermath. Initialization, rules/fixture import,
service construction, postaction diagnostic queries and comparison are outside
the timer. It is not a full-turn throughput or native speedup measurement.

The ordinary Ancient Walls survivor profile now follows the same persistent
Session path. Two retained Linux Warrior attacks match the compared gameplay
state, callback-qualified effects, history, visibility and ordered GameSync
state:

| Initial outer damage | Final outer damage | Garrison damage | Attacker damage | XP |
| --- | --- | --- | --- | --- |
| 0 / 100 | 6 / 100 | 1 | 20 | 3 |
| 99 / 100 | 100 / 100 | 44 | 20 | 3 |

Both requests reach `HeadlessMessagesProcessed`, exhaust movement and the attack,
and clear the operation queue. The three GameSync draws retain bounds 12/12/12,
values 4/7/8, and final call count 607 with seed 3059301340. The same external
policy omission for `CityAttacked` targeting tallies applies; raw differences
remain in the private comparison.

Damage generation uses the precombat wall fraction. Resolution applies attacker
retaliation, then positive outer damage and its synchronous publication, then
garrison damage and its publication. The exhausting hit generates outer damage
6 and an unclamped total of 105; the District clamps it to 100 without adding
that excess to garrison damage. The generated defense type remains the
precombat `OUTER_DEFENSES` value (`0x6da56a3d`) on that hit. This enum is qualified
against the pinned Linux source; these captures do not directly transport the
native Results scalar.

These cases cover an ordinary Warrior, Ancient Walls, a surviving attacker and
city, and the reached stock callbacks. Attacker death, special wall prevention
modifiers, religion-dependent wall attack and new siege-support mechanics remain
unsupported or outside this comparison. Capture through walls, arbitrary combat
and complete state/action/turn equivalence are not established.

The optimized comparisons used public commit
`4454ef4c1e653cbe946762f04de881d329f606a8` and private commit
`8e5758dd9ffbadc7c116d4840c14e78d37d649aa`, reusing the same executable for both
fixtures. Private receipts `evidence/linux/city-walls-melee-session.json` and
`evidence/linux/city-walls-exhaust-melee-session.json` retain exact source pins,
canonical input hashes, build profile, commands and actual results. Native
capture receipts separately bind the producer pins and pinned Linux target.
Configuration SHA-256 values were
`f9a14e0d90bfa7da8c9b3145c7251b5c6e4b84c428283d1ba25dc560eeae409a` and
`44e14452f052a7525821b5be93c47454fd6353a939e234799a77381aac9bb48b`;
both used gameplay database SHA-256
`c880adaedcfd6da7808302b3b76bb9cad7d3188b35ae78e780cff7b56e04477e`.

Single Cargo release action measurements were **7.007 ms** and **7.188 ms** for
intact and exhausting walls. These cover request execution, reached combat
callbacks and notification aftermath, excluding import, service construction and
comparison. They are bounded action samples, not full-turn throughput or native
speedup measurements.
