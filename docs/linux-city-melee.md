# Ordinary melee against a city

The persistent `Session::melee_attack` supports an ordinary unembarked land
melee attack against a major-owned city-center District when the attacker and
city survive and the city has no outer defenses. The external policy supplies
the MoveTo request. Complete Civ VI state, arbitrary combat and complete turns
remain unimplemented.

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
prefix and freeze the Session. Lethal city capture remains under integration and
has no native acceptance yet; walls, attacker death and other unsupported
branches return explicit errors.

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
