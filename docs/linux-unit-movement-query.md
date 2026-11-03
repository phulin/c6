# Direct Linux Unit movement query fixture

The standalone fixture prepares direct original `Unit.GetMaxMoves` (`0x2c27180`)
and `Unit.HasMoved` (`0x2c27b50`) comparisons against the concrete owned-state
[provider](linux-unit-movement-state.md). **Original execution and direct query
comparison are pending.** Historical source-only results remain associated with
their original source versions. This packet does not establish full gameplay,
action, turn, RNG or native load equivalence.

`native/linux/unit_movement_query_fixture.[ch]` exposes one capture operation.
The sole original runner must supply an actually retained Game, its borrowed
Context and a new, separately reviewed three-argument ownership guard. The
fixture does not acquire a facade, enter the existing Manager fixture, alias
its profile/globals, execute an action, drain a queue, Save or load a game.
The caller must pin the XP2 module, matching host/dependencies, complete rules,
DLC/configuration/input identity, OS main-thread witness, actual native lock
owner token/depth and absence of a competing worker. A function-pointer type is
not evidence that an implementation satisfies that guard contract. Caller and
guard implementation, lifetime and build identities remain execution blockers.
There is no idle-queue or Save-equality acceptance gate.

The capture uses the established real Player manager and Unit container
iterator ABI from `placement_metadata.c`, with its own local state. Every Unit
must resolve under its actual owner and full ID through original Find. No
placement or delayed-death filtering occurs. Source `GetMaxMoves` has no such
filter; its GetPlot result may be null, in which case plot contributions are
skipped. `GetBaseMoves(NO_DOMAIN)` checks its definition pointer and returns
zero if absent. The collector represents that pointer as null rather than
inventing a definition. The current candidate expects initialized definitions
for all 1,262 records; any disagreement fails input agreement explicitly.

Three passes enumerate all stored records. Pass0 exports query inputs, pass1
calls each genuine Unit getter, and pass2 repeats the inputs. The two output
files use distinct schemas and exclusive creation; desired answers never enter
the native input file. Inputs include all 65 actual manager slots/mapping and
resolved controls, current turn and movement globals, current Player Diplomacy
modifier triples/timings, embarked movement modifiers, suzerains, the complete
initialized alliance value/changed-turn vector, and each Unit's actual identity,
location, movement fields, activity, raw/effective embarkation, runtime domain,
definition index/hash/BaseMoves and resolved plot owner/cost. Plot cost is an
independent native observation: Rust derives its own value from actual Map,
District and definitions and compares it; it never accepts the observed cost as
an effective-stat input. The same applies to effective embarkation.

The native API/field audit is
[evidence/linux/unit-movement-query-abi.json](../evidence/linux/unit-movement-query-abi.json).
Its raw disassembly stays private. Reviewing actual `IsEmbarked` at `0x2c12ca0`
found an omitted predicate: activity hash `0x5ff1bbdf` suppresses the raw embarked
flag. `GetActivityType` at `0x265b300` reads Unit+`0x290`. The new
`unit_is_embarked` helper preserves this exact predicate in base movement and
clear-terrain bonuses. Focused tests cover the exceptional activity with either
raw flag and ordinary activities including neighboring hashes. Other loaded
reporting/placement adapters contain no separate raw Unit `b_embarked` read;
Formation's own embarkation inputs remain a different contract.

## Candidate and comparison

`tools/linux/unit_movement_query_check.py prepare --source-commit COMMIT
--directory ABSOLUTE_NEW_DIAGNOSTICS_DIRECTORY` archives the complete committed
repository before building. It retains that archive and its extracted source,
Cargo.lock, dependency library/executable identities, toolchain versions, static
native ABI report and all actual fixture argument identities. It builds the
Rust comparator from that committed archive and freezes `candidate.jsonl`
(inputs) separately from `candidate-answers.jsonl`. The Rust executable opens
no native answer file and reuses the genuine admitted loaded fixture,
506-table rules/resources, original controls as a *reference projection*, and
owned Game.DiplomaticRelations. It computes actual movement queries from those
inputs. The native runner must not feed candidate answers into game state.
No new live authority is inferred from the older control capture.

After the sole runner executes the reviewed acquisition/guard integration, use:

```sh
python3 tools/linux/unit_movement_query_check.py compare \
  --directory ABSOLUTE_FROZEN_DIRECTORY \
  --inputs ORIGINAL_INPUTS_JSONL --answers ORIGINAL_ANSWERS_JSONL \
  --profile-sha256 NEW_NATIVE_PROFILE_SHA256 --output NEW_COMPARISON_JSON
```

Comparison requires complete pass0/pass2 equality, the same global/RNG witness
in all three passes, and complete agreement with independently derived candidate
inputs before matching any answer. Identity is owner/full ID plus recorded
container ordinal, never pointer identity. Expected coverage is all 1,262 Units,
including the 21 known unplaced/delayed-death records when source-valid. There
are 2,524 direct target calls; HasMoved also internally calls GetMaxMoves.
Mismatches and incomplete capture are errors, with original files retained;
no exclusion is silently counted as passing. The comparator's positive output
is numerical query agreement pending external execution-admission review,
not proof of authentic execution or full affected-state parity. Its 64-byte
known RNG witness does not claim complete gameplay RNG coverage. Query inputs
are not a complete mutation projection or a Save-equivalence test.

`test` exercises duplicate identities, answer/input separation, incomplete
capture, changed input rejection and mismatched answers using explicitly
artificial parser cases; those tests are never native observations. GCC strict
warnings/analyzer and Rust focused tests validate the preparation boundary.
The remaining execution steps belong to `reference_reuse`: freeze a reviewed
independent caller/guard and complete new profile, retain exact linked helper
and runtime/source preimages, run after the primary Manager profile closes,
then retain original inputs/answers and compare against this frozen candidate.
No original process was launched to prepare this packet.
