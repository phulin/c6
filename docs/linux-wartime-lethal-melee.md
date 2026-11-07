# Lethal Unit melee followed by an ordinary round

One persistent Rust Session can start the supported foreign trade route,
declare surprise war, execute an externally selected Warrior attack that kills
the major-player defender, and advance one ordinary round to player 0's next
external decision. Complete Civ VI gameplay, arbitrary actions and arbitrary
full turns remain incomplete.

This Linux XP2 profile extends the [surviving Unit melee case](linux-wartime-unit-melee.md).
Both Warriors are created by the original engine. Before capture, the original
`Unit::SetDamage` gives the defender 90 damage. The attacker moves from (17, 9)
into the defender's plot at (18, 9), taking 16 damage. Combat marks the defender
dead and delayed, removes its location, and retains its roster entry until the
ordinary round's delayed cleanup. At turn 2, the defender is absent and the
attacker has 2 moves and 1 attack available. Player 0 is `DecisionReady`;
research, production, unit orders and every NPC policy response remain external.

Use `melee_attack::stock::RetainedDamage` for the retained death composition.
It borrows the same creation collections alongside combat collections and uses
the existing live default-defeat requirement queries. The canonical effect,
requirement and collection owners survive the attack and round. UnitCombat
subscriptions and dead-unit modifier owners are destroyed in source order;
death war weariness, kill bookkeeping and the global units-killed counter run
before the next policy boundary. Initial subtype-wrapper ownership and the
units-killed counter come from the same BEFORE snapshot. Native AFTER values
serve only as comparison targets.

The actual combat boost listeners reject this Warrior-versus-Warrior pairing:
the relevant kill rows require other unit types or barbarian kills. This case
does not qualify positive research boosts. The reached `UnitKilled` achievement
observer and stock reporting output are omitted only under their established
headless qualification; gameplay callbacks and RNG remain in scope.

The optimized comparison uses public `df27477b` and private `6e547687`, with
configuration
`05a912507809a6095eb02c8009870486fe3f56e0d7cb52135ba1065915b376cf`.
Private `c6-re/evidence/linux/wartime-lethal-melee-session-7-release.json`
records the exact source, target, rules, canonical input and executable hashes,
command, profile and actual result. Represented trajectory, terminal typed
gameplay, core processors, known GameSync/libc RNG, requirements/modifiers and
opinion graph match. The global units-killed counter is 1, proposal counter
7839 and shared requirement counter 19001; no proposal remains active.

In one optimized replay, the melee request took 4.36 ms and the ordinary round
requests took 27.44 ms. Melee timing includes synchronous gameplay callbacks
and mandatory aftermath. Round timing covers Session advance/resume/research
and recorded handoffs. These single observations exclude fixture import,
provider setup, requirement metadata joining, initial trade/war commands,
comparison, external policy selection and JSON serialization.

The comparison is bounded to this original-engine-initialized wounded Warrior
profile and one ordinary wartime round on the [pinned Linux target](targets.md).
It inherits the surviving case's forecast, diplomacy and unsupported-branch
restrictions. Other unit types, stacked or hidden targets, active military
operations, positive unimplemented rewards or boosts, and arbitrary combat
aftermath remain outside the qualified profile. Complete runtime and RNG
inventory coverage is not established. Raw full-action equality remains false
for the preserved policy/presentation fields and intermediate unsaved District
observations.
