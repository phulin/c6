# Slinger kill, Archery boost and an ordinary round

One retained Rust Session starts the supported foreign trade route, declares
surprise war, executes an externally selected Slinger ranged kill with its
positive Archery boost, then advances one ordinary round to the next external
decision. Complete Civ VI gameplay, arbitrary actions and arbitrary full
turns remain incomplete.

The original engine initializes the Slinger and major-player Warrior and gives
the defender 90 damage before the BEFORE snapshot. The explicit RangeAttack
uses modifiers 0. The Slinger remains at its origin without receiving damage;
the defender dies and loses its location while retaining its delayed roster
entry until ordinary cleanup. Every research, production, unit and NPC policy
choice remains external.

Use `ranged_attack::stock::RetainedDamage` to retain the canonical creation and
combat collections, live default-defeat requirements and death continuations.
`Session::range_attack` loans its current research runtime for synchronous
combat callbacks and restores that same owner before returning. The stock
combat boost continuation uses the existing research manager path; no second
Session or state import is constructed.

The qualifying kill listener checks the attacking unit type and opposing death.
In the native comparison, Archery (index 6) changes from unboosted with zero
progress to boosted with 4,864 Q8 progress, exactly 19 science. Its acquired flag
stays false; the empty research queue and zero overflow remain unchanged by the
kill. The stock semantic notice carries source 0 and definition percent 40;
native Q8 rounding determines the science grant. No kill counter belongs to this
trigger.

The positive technology path sets its boost flag, publishes the semantic boost
report and notice, dispatches current boost subscribers, then adds science with
native Q8 arithmetic and checks research agreements. Signal subscribers see the
old progress and the new flag. The external research queue is preserved unless
actual acquisition requires its existing completion path. Positive unsupported
EraScore, quest and other reached continuations fail explicitly.

The ordinary round retains the Slinger’s reached wartime forecast queries.
The supported two-turn, single-unit land search uses its range of 1 and maximum
movement of 2, preserves source-ordered combat proposal creation and destruction,
and leaves scoring and player choices external. Other ranged forecast profiles
remain unsupported.

The optimized Linux comparison uses public `74bf57be` and private `b83c28c3`,
configuration
`768d098310224cb06379af8e72f59edc534b5f9e3c6a2b72d8e05b62ea8634af`.
Private `c6-re/evidence/linux/slinger-archery-session-5-release.json` records the
exact source, target, rules, canonical inputs, executable hash, command, profile
and actual result. Full typed native technology state matches after the attack
and round. Represented trajectory, terminal typed gameplay, core processors,
known GameSync/libc RNG, requirements/modifiers and opinion graph match. The
endpoint is player 0 `DecisionReady` at turn 2; the defender is absent, and the
Slinger has 512 Q8 movement and one attack. The global units-killed counter is
1, proposal counter 7839 and shared requirement counter 18993; no proposal
remains active.

One optimized replay measured 4.38 ms for the ranged request, including its
synchronous gameplay callbacks and mandatory aftermath, and 29.05 ms for the
ordinary-round Session requests. These single observations exclude fixture
import, provider setup, input reading, external policy selection, initial
trade/war requests, checkpoint comparison and JSON serialization.

The profile inherits the [lethal melee case](linux-wartime-lethal-melee.md)'s
bounded diplomacy and ordinary-round scope. Arbitrary combat arrangements,
positive unimplemented rewards, EraScore or quest completion, arbitrary
acquisition and full runtime/RNG inventory remain outside the qualified profile.
The library requires caller-initialized typed state, matching rules and current
services; it does not yet provide general game initialization or C ABI gameplay.

Raw full-action equality remains false for preserved policy/presentation fields
and intermediate unsaved District observations. Complete runtime and RNG
inventory coverage is not established. This representative wounded-major kill
qualifies the positive Archery path; it does not qualify arbitrary research
boosts or combat aftermath.

The public `session::runtime::Builder` now supplies reusable typed runtime
composition for this retained sequence. It joins common research/player
services, canonical factory owners and stock combat/formation callbacks, then
moves the update/research/notification owners into Session. Importers supply
initialized typed state, definitions and registrations; native capture decoding
remains private. The same API also passes the lethal Warrior retained-round
regression. See [runtime construction](architecture.md#supported-gameplay-runtime-construction).

The constructor replay uses public `0d4e84c3` and private `e82efd86`, the same
native Slinger inputs and unchanged target/configuration above. Private receipts
`evidence/linux/public-runtime-slinger-release.json` and
`evidence/linux/public-runtime-lethal-regression-release.json` retain exact
source/input/executable identities, commands and actual results. Represented
trajectory, terminal gameplay, known RNG, requirements/modifiers and opinion
graph remain equal; raw full trajectory remains unequal for the existing
preserved exclusions. No original engine was launched for these replays.

One release observation measured 498.20 ms for stock runtime construction,
typed feature binding and the Session owner join, excluding earlier fixture,
rules and query import. Separately, the Slinger request took 4.33 ms and the
ordinary-round requests took 29.15 ms with the request exclusions above. These
single observations are not a full-game benchmark. The public workspace tests,
clippy, C ABI smoke and embedding example run from temporary committed public
build inputs with no private sibling.
