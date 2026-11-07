# Produced Settler expansion on one Session

One retained Rust `Session` now joins ordinary Settler production, externally
ordered movement, second-city founding and the following ordinary round. This
is a bounded Linux gameplay profile; complete Civ VI state, arbitrary actions
and full turns remain unimplemented.

The original fixture starts at turn 1 with Washington at `[18, 11]`, population
two and a legally queued Settler one Q8 production unit below its actual cost
of 80. Original population and production APIs prepare this initial state
before BEFORE. The captured sequence does not include the earlier capital
founding or the production prefix. No completed Settler is supplied as Rust
input, and neither execution reloads state, injects a unit or teleports after
BEFORE.

Every request operates on the same `Session::from_initialized` owner:

1. Apply external research/civic choices and Fortify, then complete the ordinary
   round. City upkeep produces Settler `196608` at `[18, 11]`, consumes population
   and restores the new unit's normal movement.
2. Select Monument for the capital. An external `move_to` reaches `[17, 9]` and
   consumes all 512 Q8 movement. Complete another ordinary round to restore it.
3. A second external `move_to` reaches `[16, 7]` and again consumes all movement.
   Native FoundCity is not legal on arrival; another genuine round restores
   movement before the next founding request.
4. Call `found_city`, creating New York for the same major player. Select its
   Monument externally and complete the following ordinary round. At turn 5,
   player 0 receives `DecisionReady` with both cities retained.

Research, production, unit orders and every reached player/NPC policy response
remain external. Native AI selection does not supply missing decisions. The
comparison checks each MoveTo's native position and movement before and after
the Rust call, and checks restored movement before FoundCity. Native AFTER state
is opened only after the complete Rust sequence returns.

The joined path retains production population/overflow, unit creation and later
cleanup owners; current sight and geographic naming; the discovered volcano's
source-established non-Toa observer branch; named-volcano scheduling draws;
reached NPC era callbacks; weighted city naming; city-center Olives extraction;
and both cities' normal upkeep. New York is named from the ongoing GameSync
stream rather than the separate second-city fixture's earlier RNG state.

Compared affected typed gameplay, core processors, global religion, represented
requirements, growth modifiers/blackboards, relationship modifiers, used names,
logical territory-query identity and known GameSync/libc RNG match the pinned
[Linux target](targets.md). The development and optimized comparisons use
public `f94c08c0` and private `38619eea`, with configuration
`364b776b661c7be9903fad450a4e38f0537c6649e0da06d2d2bc29e795634db5`.
Private `c6-re/evidence/linux/produced-settler-expansion-session-5-dev.json` and
`produced-settler-expansion-session-6-release.json` record the exact committed
sources, executable, canonical inputs, command, profile and actual result.
Earlier setup and reached-branch failures remain retained with their original
inputs. Workspace tests and Clippy pass from the compared public source in a
standalone export without a private sibling.

One optimized sample measured a sum of 70.73 ms across production request
intervals: 15.73 ms for explicit actions and 55.00 ms for turn advancement.
FoundCity took 10.63 ms; the two MoveTo requests took 1.58 and 1.15 ms. These
intervals exclude fixture import, provider/runtime setup, file/packet parsing,
external policy selection, diagnostics, comparison and JSON serialization.
They are sums of measured calls, not continuous wall time or full-game
performance claims.

Complete gameplay and RNG inventories remain unfinished. Raw wire annotations,
AI/control and other recorded differences remain visible; this is not a
full-action or arbitrary-turn equivalence claim. The checked volcano observer
branch excludes Maori Toa. Unknown subscribers and positive volcano status
setters remain explicit unsupported transitions. A zero scheduling draw retains
its pending activation/deactivation so retry cannot consume that draw again.
Other unsupported naming, connectivity, resource, modifier and event branches
continue to return errors with their completed prefix preserved.
