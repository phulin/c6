# Independent loaded SkipTurn candidate review

Original candidates `b59e39d` and `15f4a90` required corrections before their
composed execution order could pass this review. Corrected action checkpoint
`c5baaeb`, resource checkpoint `44c2225`, movement dependency `1ffc015`, and
loaded-state guards `2ec623f`/`40c9775` pass the bounded source and entrypoint
checks below. This is a diagnostic composition over concrete loaded records,
not accepted native Read, a complete gameplay action, or a turn library.
`Engine::apply` still rejects unsupported gameplay actions.

The [independent evidence](../evidence/linux/loaded-actions-review.json) identifies
the exact tested sources, executable, lockfile and target dispatch checks.
The strict decode helper is separately committed as `20b2b25`; it uses the
existing native-3 artifact manifest and real capture admission API. All 43
Players, 1,262 Units and their retained owners come from the actual decoded
records. The resource/rules builders and once-admitted snapshot controls/war
storage are real inputs, rather than invented definition orders or Manager
predicate answers. No original game code executes in this review.

## Corrections found during review

The original composition invented a full Manager::CanStart query before
Clear. Native Execute `0x2d28a00` resolves its Unit and calls Manager::Add at
`0x2d28a88`; Add `0x2da1a20` constructs its node, performs the requested Clear
at `0x2da1a6d`, and then resolves the requested handler at `0x2da1a94`.
Full Manager legality belongs to Activate after insertion. A busy old MoveTo
is a concrete counterexample: Clear resets its timer, so rejecting it from
the old timer before Clear loses real native mutations and reporting.

The correction removes that query. The full entrypoint test gives a decoded
MoveTo actor a positive operation timer, supplies no old MoveTo interruption
lookup, and verifies successful Clear, nine ordered identity-report rows and
exactly one script query at activation. Busy empty-queue insertion instead
preserves its timer, makes zero script queries, and returns to the post-Add
continuation. A proven null requested handler is resolved after Clear and
also returns normally to aftermath. Invalid mapped Player/missing Unit takes
Execute's earlier return and never constructs the aftermath provider.

Review also found two distinct Activate branches that cannot be collapsed
into ordinary Start:

- An operation-added subscriber can set activity to OPERATION. Native Activate
  then invokes Continue without CanStart/Start. The bounded original SkipTurn
  specialization now follows the source-proven inherited Update/CheckComplete
  leaves, toggles Formation evaluating state, clears its path and deletes the
  node. The full entrypoint checks this branch with zero script calls.
- A false Manager result reaches Unit::HasMoved, not an unconditional return
  or generic unsupported result. HasMoved `0x2c27b50` compares remaining raw
  Q8 movement with wrapped Q8 GetMaxMoves. Activate deletes when not moved,
  or when the signed front push turn precedes current turn; otherwise it
  retains the node. Tests use actual movement-definition/state providers for
  moved, unmoved and signed-old push-turn cases, then verify normal aftermath
  entry. Missing movement inputs or unsupported callback queue shapes still
  produce explicit partial failures.

The movement provider's source evidence is recorded separately in
[its target audit](../evidence/linux/unit-movement-state-source.json) and
[loaded-record check](../evidence/linux/unit-movement-state-check.json).
Neither is an original numerical Unit query comparison. This review tests its
use in the actual action entrypoint; it does not promote those source checks
to complete native movement/state equivalence.

## Callback state and failure boundaries

Manager querying copies only the addressed Unit, while the loaded World remains
owned. Script mutations are copied back on normal false results and provider
errors. World-backed placement refuses a changed scratch actor rather than
querying the old World record as though it were current. Changed owner/ID
returns the complete mutated scratch actor in `ActorIdentityChanged`, leaving
the real record's already-performed prefix intact; it does not install a new
identity into stale recycled indices. The independent identity test verifies
both the payload and the original resolved record identity.

This conservative boundary is not an authoritative actor overlay for arbitrary
world queries. Providers needing to mutate other Players, Units, Game data or
RNG must reject the interface. Safe exclusive borrowing prevents an ordinary
mutable alias to this LoadedState during those scratch callbacks; it does not
prove arbitrary external/native provider purity.

Reports receive the actual owned Unit synchronously. Activity changes are
reread at the original boundaries, including equality suppression after Start.
Other callback writes are retained and stop unsupported continuation. Tests
verify a report error after insertion leaves the new node and callback damage
write in place, and that a veto callback producing an unsupported two-node
queue retains those nodes without guessing further dispatch.

Known unsupported shapes are checked through immutable preflight views: both
the owned record and revision remain unchanged, and no callback runs. Failures
after Clear, insertion or callback execution preserve their prefix and trace;
they are not unchanged rejections or rollback. The combined diagnostic creates
aftermath over that same resulting LoadedState, invokes diplomacy first, then
owner notification/blocking continuation. The independent aftermath probe
deliberately errors at the unavailable notification backend. Its synthetic
diplomacy list and report collector establish call order only, not genuine
pending-statement, notification, cache, scripting or RNG behavior.

## Resource lifetime and provenance

The original snapshot helper checked role/image identity but could reissue its
captured mutable facts after state mutation. Resource `44c2225` replaces it
with once-only pristine admission, verifies the full associated source image,
installs separately attributed observed controls, and transfers owned mutable
war storage. Independent tests reject second admission and admission after
mutable access even when record bytes are restored. Changed definition-dependent
wire inputs also reject stale Unit sidecars before execution.

The decoder's actual Read binding remains unchanged. The controls witness
retains its observed SavedOutput source; the associated captures are independent
processes over the same saved snapshot. They are not relabeled native-3 live
Read controls, and matching bytes do not establish all unsaved post-load state.
The mutable war sidecar must remain with its associated LoadedState and be
updated by faithful gameplay providers. Immutable definitions can be reused
only under the actual resource/configuration gate; effective war, membership,
pause or automation facts cannot be refreshed from immutable definitions.

Native low16 lookup remains intact. The entrypoint fixture deliberately uses
a stale requested generation and verifies execution/reporting on the actual
resolved actor. No generation-equality requirement, whole-World clone, default
null scripting provider or attachment flag is introduced by this review.

## Reproduction and acceptance limits

```sh
python3 tools/linux/loaded_actions_review.py \
  --output evidence/linux/loaded-actions-review.json
```

The offline runner checks source stability across compilation/execution and
records the executable hash. Thirteen finite source-case groups exercise the
full `skip_turn_state_phase` or `diagnose_skip_turn` entrypoints, rather than
only Start/report leaves. They cover the corrected command order, exact script
invocation counts and lock/argument ordering, low16 identity, missing actors,
null handlers, preflight atomicity, stale runtime inputs, callback mutations,
Continue, veto branches, partial errors and aftermath entry.

Full-entrypoint coverage is still absent for nonlocal actors, the no-script and
unheld-lock branches, script-provider error/unwind, nonbusy empty-queue Start,
dynamic registry changes, and alliance-dependent MaxMoves inputs. The fixtures
do not enumerate arbitrary subscriber mutations or complete any notification
continuation. Lower-level source tests cannot turn these gaps into whole-action
or native coverage.

Handler-slot assertions and script/report/aftermath probes remain explicitly
synthetic source fixtures. They do not supply actual live virtual binding,
script VM/lock behavior or the complete synchronous gameplay subscribers.
Nine tested identity-report rows are not the native eleven-Send callback
inventory. No-op/collector completion is never counted as full callback proof.
The [bounded local/stock metadata exclusions](linux-local-gameplay-scope-review.md)
still require local authority, no native resync and no added gameplay dirty-list
consumer; actual field mutations, reports, callbacks, projections and RNG remain
required.

Resource/context/executable hashes establish identities, not full game-state
equality. Complete initialized state, gameplay RNG inventory, transitive callback
effects, affected-state comparison, external-decision continuations, general
actions and turns remain open. Unknown save bytes and the recorded 89,198-byte
Save→Load difference remain unmasked evidence. Save writing is not an action gate.
