# Independent movement query caller review

The movement-only caller and ownership guard in `bf76da7`, loader/main-thread
wiring in `f900235`, and lifecycle selector in `68e3ce3` have no source blocker
within the boundary reviewed here. This is source and synthetic execution
acceptance, not admission of an original run or native query parity. Complete
gameplay state, actions and turns remain unimplemented.

The caller uses the genuine `Context.GetGame` (`0x2c7cae0`) and original
`ICvUnknown` deletion path (`0x290bba0`). It checks the facade Game pointer,
vtable, reference count and destructor dispatch before accepting ownership and
again before release. The known EngineUtility lock/unlock dispatch is checked
before acquisition and before release. Ordinary collector returns, including
243, release the facade once; post-release checks read retained owners rather
than the freed facade. Unknown owner, binding or field states exit240 without
attempting further original calls or cleanup.

The guard has independent `movement_scope` storage. Four two-argument lifecycle
callbacks are distinct from the three-argument retained-owner guard and
five-argument locked-begin callback. Unlocked checks require owner/depth zero;
after actual acquisition, locked-begin retains the native nonzero owner token
and depth1. That token is never derived from the OS TID. The real main-entry
witness, stable initialized Controller/Context/Game and absent worker are
checked separately. No full-Manager guard, Lua/frame gate, empty-work/queue
test, Save equality, effect clearing or artificial Unit field is introduced.

The collector preserves the original manager's65-slot lookup semantics:
`GetValidPlayer` (`0x2003940`) checks validity then indexes the stored pointer
vector by player number, independently of the exported PlayerToIndex mapping.
Native iterator begin/end, dereference and increment return/use the one-word
iterator ABI; full IDs are checked using original Units.Find before queries.
`GetMaxMoves` (`0x2c27180`) returns a signed32-bit value, and `HasMoved`
(`0x2c27b50`) returns bool and internally calls GetMaxMoves. The documented
2,524 calls count the collector's direct target calls; internal calls remain
part of original execution. Neither getter filters delayed-death/unplaced
records. Null plots are valid, and the base-movement source handles a missing
definition. The collector records these conditions rather than fabricating
definitions or excluding records.

Pass0 records inputs for every stored Unit, pass1 calls MaxMoves then HasMoved
with ownership guards between targets, and pass2 records inputs again. This
does not snapshot every possible affected gameplay field. Four caller phases
also log64 known RNG bytes without an equality veto. Input comparison and
subsequent native/Rust numerical comparison remain separate checks; a changed
known RNG diagnostic is retained, not silently cleared.

The lifecycle selects begin→idle→probe→idle→identity→end before Uninitialize.
It preserves ordinary243 while doing release/teardown and discards fatal240
paths. Movement and full-Manager use observational trace closure; all earlier
profiles retain the previous unsigned trace ABI and exact source projection.
Trace207/208 still propagate when the probe succeeds; the existing nonzero
probe-result precedence remains. The new loader branch permits absent Save
paths and labels optional before/after Save failures diagnostic. It rejects
simultaneous full-Manager and movement selectors. Other mode/environment
combinations still require the runner's exact-profile gate.

## Independent checks

The [review harness](../tools/linux/unit_movement_query_review.c) includes the
actual collector, with compiler-checked mock assignments to each actual Api
function type. Locally generated executable stubs jump only into those C
mocks. No original ELF is loaded or called. Five discriminating cases check:

- 1,262 stored Units, including owner64, full IDs with high bits set,21 records
  both unplaced and delayed, nullable plots/definition, complete input passes
  before/after queries, and2,524 ordered direct calls with signed/bool results.
- A1,261-record capture returning243 without hiding the missing record or
  discarding changed known RNG state.
- Exclusive output creation refusing an existing input path with242 and no
  target call.
- An owner guard refusing between MaxMoves and HasMoved with240 and no second
  target call.
- Wrong full-ID membership refusing with240 before any target call.

Strict C warnings, GCC analyzer and UBSan apply to this harness. The existing
lifecycle test is also rerun, retaining its generated C and executable; its
legacy projections, callback signatures/order, ordinary243, fatal240, all nine
trace selectors and207/208 checks pass. These are complementary to the author's
four successes and41 targeted fatal caller/guard cases in `26832c4`; that
separate audit protects the released mock facade with PROT_NONE and checks
that pending work and RNG effects survive. The author cases are reviewed
evidence, not newly authored independent tests.

Reproduce with a fresh private directory and a fresh report path:

```sh
python3 tools/linux/unit_movement_query_review.py \
  --directory /absolute/new/private-review-directory \
  --output /absolute/new/review.json
```

The [review report](../evidence/linux/unit-movement-query-review.json) retains
production source pins, all compiled source preimages, test/archive hashes,
toolchain and dynamic dependencies, and individual logs. It also rechecks the
54 movement and21 facade/owner/lock method fingerprints against read-only
pinned XP2/host files. Fingerprints establish target identity; they do not
execute or validate getter results.

Coherent runner/CLI/supervisor source and fresh profile admission remain to be
reviewed. Actual module/dependency/rules/input identity, launch isolation,
terminal process disposal and complete affected-state/RNG evidence are not
established by this packet. The sole runner retains original execution
ownership. No original execution or push occurred here.
