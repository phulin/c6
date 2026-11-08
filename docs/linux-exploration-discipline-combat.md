# Retained exploration: Discipline combat and upkeep

One persistent Rust Session can retain the supported exploration and
[city-development sequence](linux-exploration-city-development.md), make two
externally selected Scout moves, attack an original barbarian Scout under the
newly active Discipline policy, and advance an ordinary round to player 0's
next external decision. Complete Civ VI gameplay state, arbitrary actions and
arbitrary full turns remain incomplete.

The original player 0 Scout `196610` starts this continuation at (14, 21).
External MoveTo requests take it to (15, 20) and (15, 19), with an ordinary round
after each move. Both original legal moves cost 256 Q8 movement. Immediate post-move typed
state and GameSync comparisons match before either round restores movement.
It then attacks
the surviving original barbarian Scout `63:131073` at (16, 18), whose earlier
river-flood damage is still 36. The adjacent melee request uses modifiers 16.
Discipline contributes +5 strength, reported by the actual versus-barbarian
query as 1280 Q8. The existing Roosevelt and difficulty combat modifiers retain
their actual requirements, parameters and application state.

The attacker takes 23 damage and deals 42, leaving the defender at 78 damage.
Both survive. Immediate post-attack typed Map, Units, Formations, rosters,
represented Player prefixes and GameSync state match independently of the
following round. At turn 24, player 0 is `DecisionReady`; the attacker remains
at (15, 19), with 23 damage, 768 Q8 movement and one attack. The defender remains
at (16, 18), with 78 damage. Research, production, unit orders and every reached
human/NPC policy response remain external decisions.

The same canonical effect, requirement, lifecycle, history, territory and sight
owners survive the entire prefix and combat. Known Player strength effects are
decoded when the initial owner is assembled. Combat reuses live default-defeat
requirements instead of importing their earlier values again. The full combat
runtime retains the prefix's handler-aware promotion query and qualified
promotion reports. Damage, attack and formation report routes each have one
owner. No unit, damage, movement or RNG setup is reset after BEFORE.

The optimized comparison uses public `e2bb54d4` and private `9aaddc28`, on the
[pinned Linux target](targets.md), configuration
`443f506f340ee792d1717accaff5c9f63ef935bb3b58cad9774718c1619b9b57`.
Private `c6-re/evidence/linux/exploration-discipline-combat-release-1.json`
records the source, canonical input and executable hashes, command, profile,
actual result and scoped timings. Native AFTER is comparison only. Terminal
typed gameplay, core processors, represented requirements, the wonder prefix,
and known GameSync/libc state match. Final GameSync is 787 calls, seed
4134406316, reset count 0. The attack's two recorded GameSync draws have bound
12 and values 4 and 11.

The prior 20-round city-development profile and existing surviving Warrior
melee profile also pass with this source. In one optimized replay, the melee
request took 1.44 ms; recorded round requests across all 23 retained rounds
totaled 520.06 ms. Attack timing includes synchronous gameplay callbacks and
mandatory aftermath. Round timing covers recorded advance/resume/research and
handoff requests. Both exclude fixture import, rule/provider setup, comparison,
external policy selection and JSON serialization. Round timing additionally
excludes explicit SkipTurn orders. These do not measure complete arbitrary turns.

This qualifies one surviving original Scout-versus-barbarian Scout attack and
its reached ordinary upkeep in this retained profile. It does not qualify a
barbarian kill, positive kill boost, camp clearance, arbitrary combat aftermath,
or a complete runtime/RNG inventory. Raw typed/full-action equality remains
false for preserved property-name transport annotations and source-qualified
policy/presentation differences. The earlier city-development and exploration
restrictions continue to apply.
