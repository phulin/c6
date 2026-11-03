# Loaded Unit maximum movement and HasMoved

`c6_data::unit_movement_state` computes original Unit.GetMaxMoves and HasMoved
from actual owned Unit/Player/Map fields and admitted rules definitions. SkipTurn's
activation-false path can use this query instead of Formation.has_moved or a
caller-provided effective movement answer. This implements a bounded query, not
a movement action, full SkipTurn acceptance or complete state/action/turn parity.

The target is Linux XP2
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
with the pinned matching Linux host. No original execution was performed.
[Static evidence](../evidence/linux/unit-movement-state-source.json) fingerprints
25 query/getter bodies, the diplomacy source jump table, actual Plot vtable
binding and five native definition builders. Raw original source stays in
`../c6-ghidra/evidence/linux-unit-movement-state/`; existing continuation evidence
was reused. The [offline integration check](../evidence/linux/unit-movement-state-check.json)
exercises real decoded fields without claiming native Unit query parity. The
checker retains an isolated immutable public-source candidate, hashes every
source preimage, and explicitly records the WIP dependencies at snapshot time;
concurrent checkout edits cannot change its tested candidate.

## Source query

Unit.HasMoved (`0x2c27b50`) copies the Unit's actual raw-Q8 remaining moves
(`GetMovesRemaining` at `0x213cc90`, value object+`0x4f8`), calls Unit.GetMaxMoves
(`0x2c27180`), converts its signed integer by the native wrapping left shift of
8, then performs a **strict signed comparison**. Remaining equal to maximum
means false; remaining above maximum also means false. No Formation flag or
Formation movement budget enters this predicate.

GetMaxMoves computes signed wrapping additions in this order:

1. `GetBaseMoves(NO_DOMAIN)` (`0x2c27990`). With an initialized definition, use
   actual Unit.BaseMoves except when runtime domain is not 1, the Unit is
   embarked, and its CombatModifiers.fight_while_embarked is false. That branch
   uses actual `MOVEMENT_WHILE_EMBARKED_BASE` plus the owner's retained
   Player.Units.embarked_movement_modifier. No CanEmbark predicate is needed for
   this particular NO_DOMAIN query branch.
2. Add the Unit's saved MovementModifiers.extra_moves.
3. Add current Player.Diplomacy diplomatic movement modifiers in stored order.
4. If its actual plot exists, add clear-terrain-start movement when **not
   embarked and Plot.GetMovementCost()==1**. Then add the actual friendly- or
   enemy-territory-start movement for the corresponding diplomatic category.

The Plot virtual slot `+0x20` is **GetMovementCost, not a PlotType enum**:
actual vtable relocation `0x3ae23c0` resolves to `0x2bc9510`. This prevents a
terrain-name or numeric-enum approximation. City plots use actual
CITY_MOVEMENT_COST. Otherwise a valid Terrain contributes MovementCost, then
resolved Feature and Improvement contribute MovementChange in that order. Missing
Terrain returns INT_MAX immediately. Missing Feature/Improvement contributes
nothing. No route, hills flag, improvement-pillage check or lower-bound clamp is
introduced by this method. The city predicate resolves the actual District using
its owner/type and low16 recycled slot, remaps its saved definition hash through
native order, then checks the native city-center hash. Generation mismatch alone
does not invalidate an otherwise resolved slot.

Map lookup reuses the existing native CoordRange arithmetic for both axes. Either
coordinate equal to -9999 returns no plot before wrapping. Out-of-map coordinates
after normalization return no plot. The native negative-exact-multiple wrapping
behavior is retained. Plot ownership comes from CityAttributes.player through
GetOwningCity/GetOwner; the owning-city ID bits and improvement owner are separate.

## Diplomatic timing and territory

`GetCurrentDiplomaticMovementModifier` (`0x2aaf540`) selects these saved
last_war_and_city_turns indices through the original table at `0x2f59940`:

| Native source value | Saved timing index |
| --- | --- |
| 1 | 2 |
| 2 | 0 |
| 3 | 1 |
| 4 | 3 |
| 5 | 4 |
| 6 | 6 |
| 8 | 5 |

Source7 and values outside1..8 contribute zero. A selected last timestamp must
be positive, and `wrapping_i32(last + turns_active) >= actual_current_turn`.
Expiry equality is inclusive. Accepted extra_moves accumulate with signed
wrapping arithmetic; the method does not remove expired records. All inputs
come from current mutable retained Diplomacy and current turn, rather than a
cached buff answer.

`GetDiplomaticTerritory` (`0x2bed4f0`) returns no category for NO_PLAYER/null
plot, neutral category2 for unowned plot, friendly category1 for own land,
land whose owner's actual Influence.suzerain is this player, or allied land.
Other owned land is category3, independently of whether the two players are at
war. Only categories1/3 add the corresponding Unit modifiers. WarTypes is not
a substitute for actual alliance state.

HasAllied delegates to Game.DiplomaticRelations.IsAllied (`0x2983170`), which
compares the actual initialized alliance relationship value against -1 using the
native symmetric triangular index. The query now reads the
[owned Game.DiplomaticRelations component](linux-game-relations-inputs.md) directly
from LoadedState. Its original saved hashes are initialized against admitted
War/Alliance definitions before use. There is no pre-action raw snapshot input
or global-revision rejection after unrelated Unit writes. Missing owned state
still rejects when the actual modifiers require its value.

Nonzero equal friendly/enemy modifiers need no alliance distinction after
ownership is established; zero contributions can similarly elide pure getters
without supplying invented answers. The original 1,059-query result remains a
frozen diagnostic of the earlier missing-input boundary. The new owned-input
check covers all 1,262 queries, with the native comparison limits below.

## Concrete admission and validation

`Definitions::from_rules(state, rules, bound, rules_report)` reuses the existing
Unit initialization resource gate and native-order registry builder. The pinned
activated rules report must match actual resource/configuration identities;
Unit order is additionally compared with the observed Bound Unit registry.
All five source SQL getters select complete collections ORDER BY rowid, and the
native builders establish collection indexes before append. Type hashes, native
indexes and SQL row identities remain distinct. BaseMoves, Terrain/Feature/
Improvement movement values and global parameters are actual rules values.
Unsupported numeric forms/ranges reject; no default3 or implicit zero appears.

`Inputs { definitions }` then supports:

```rust
unit_max_moves(&state, &unit, &inputs)?;
unit_has_moved(&state, &unit, &inputs)?;
```

The Unit runtime sidecar must have the actual initialized definition index/domain.
The query reads current saved modifiers directly; it does not copy the world or
require native archive dirtiness. Only the independent test fixture clones Units
for synthetic remaining-movement boundaries.

Reproduce:

```sh
python3 tools/linux/unit_movement_state_audit.py --private-output ../c6-ghidra/evidence/linux-unit-movement-state --output target/linux/unit-movement-state-source.json
python3 tools/linux/game_relations_check.py --output target/linux/game-relations-query-input-check.json
cargo test -p c6-data unit_movement_state --lib
cargo clippy -p c6-data --all-targets -- -D warnings
```

The frozen earlier [1,059-query report](../evidence/linux/unit-movement-state-check.json)
retains its 155 missing-alliance and 48 missing-Manager-slot failures. Its exact
source candidate and executable remain archived; it is not overwritten by this
closure. The new [owned-input report](../evidence/linux/game-relations-query-input-check.json)
admits the actual same-snapshot controls and owned initialized relationships:
**1,262/1,262 queries**, 844 HasMoved=true, and 2,524 strict remaining-movement
boundary checks. All 2,016 initialized War values match the independently captured
native metadata. There are 26 initialized alliances from five actual definitions.

Changed-turn words and the 74,616-byte unclassified owner suffix remain retained.
A subsequent unrelated Unit write increments the LoadedState revision while the
owned relationships and maximum-movement query remain valid. Unit wire values
are unchanged. Source-only tests cover timing enum dispatch, inclusive expiry,
ignored sources, and wrapping sums/expiry; the core relation module separately
checks triangular lookup and native definition remapping boundaries.

The sole native owner found no closed direct Unit.GetMaxMoves/HasMoved capture.
Existing movement-cost captures observe Formation maxima and are not mislabeled
as direct Unit oracle fixtures. Native boundary comparison remains unavailable;
no new original capture is requested here. Actual gameplay post-load,
subscriber effects, all gameplay RNG and full turns remain separate requirements.
