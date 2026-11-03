# Loaded-world SkipTurn diagnostic

`c6_data::loaded_actions` now composes actual loaded records with the existing
Linux Manager predicate, current-handler interruption leaves, SkipTurn handler
predicate, queue/activity/timer/Formation changes and in-place post-Add aftermath.
It is a diagnostic execution candidate, not a complete or accepted action.
Complete gameplay state, action execution, RNG coverage and turns remain open;
`Engine::apply` still gates gameplay actions.

`skip_turn_state_phase` changes the addressed records in `LoadedState`; it does
not replace or clone the World, discard retained components, change Player human
status, supply a precomputed legality answer, or manufacture archive bindings.
`diagnose_skip_turn` then creates an explicit aftermath provider over the same
resulting LoadedState and calls the existing `process_pending` and
`update_owner_blocking` functions in that order. A supported early Activate return still
runs the post-Add continuation. Each genuine player choice remains external;
the existing aftermath decision errors propagate without running native policy.

## Inputs and actual mutations

The consuming [loaded-state adapter](linux-world-state.md) must first retain the
Core/Player inputs and initialize the Unit/Formation sidecars. Control inputs
are explicit attributed live facts. Saved header flags remain saved provenance;
this adapter uses bound live turn-active controls rather than silently promoting
all saved flags to live state.

| Dependency | Concrete use |
| --- | --- |
| Actual `World`, configuration and loaded controls | Manager validity/nonnull membership, Player identity/human status, turn-active, pause and automation-processing reads |
| Retained Core | Current turn, local Player and actual phase processor |
| Native-order operation definitions and actual Unit runtime | Original FindIndex semantics over the live disabled-operation vector; no repeated saved-array restoration after gameplay |
| Observed handler registry | Saved hash translation and actual retained current-handler interruption leaf; missing observations remain errors |
| Selected phase definitions | Actual tactical permission at the saved current-phase index |
| Placement definitions, Game war relations and plot limit | Existing state-backed layered friendly-count and full SkipTurn CanStart |
| Script/argument/lock provider | Existing Manager ordering, retained ScriptSystem handle and utility refetch semantics; no default null script or successful callback |
| Synchronous report provider | Original report arguments and ordering, including activity equality suppression and subsequent activity rereads |
| Diplomacy/notification provider over the resulting state | Real pending statements, configuration/manager/collection receiver semantics, notification callbacks, owner blocking and RNG effects |

The compatibility entrypoint checks the handler observation's configuration
identity against the loaded binding. The
[actual captured-handler entrypoint](linux-loaded-handler-adapter.md) instead
uses typed `BoundHandlers` authority: it preserves the distinct actual observer
identity and requires source-registered IDs, matched initial queues, pristine
associated-snapshot admission and actual resource equivalence. Definition orders, phase selection, placement relations and
providers must still be admitted for that actual configuration. Existing
standalone captures cannot acquire native3 live provenance merely because a
profile name or saved output resembles it. In particular, a nonnull script
pointer does not establish callback behavior.

Known shape restrictions are checked before callbacks: no automation or pending
MoveTo movement completion, an empty queue or one supported MoveTo node, supported
initial activity, a resolved singleton Formation and no evaluating operation.
The requested and resolved recycled IDs retain native low16 lookup behavior;
request generation equality is not manufactured. ComponentID owner/type words
remain packed in the request, and the sidecar supplies actual report identity.

The actual Execute lookup first resolves through the World Manager mapping.
An absent/invalid Unit returns before Add and before aftermath. A found Unit
then performs Clear, old deactivation and front deletion, post-Clear handler
lookup/CheckInsert, insertion, activation at its original boundary, activity
assignment, SkipTurn Start, and final front deletion. A proven null requested
handler slot returns after Clear without inserting, then runs the normal
aftermath. No extra full Manager query runs before Clear. A busy Unit with an old MoveTo
therefore has its timer reset before activation; an initially empty busy Unit
keeps its timer and returns from Activate before querying legality. The
OPERATION/Continue activation branch follows the actual inherited SkipTurn
virtual leaves: clear the Formation path, set evaluating=true, invoke the
no-effect Update/TimerComplete leaves where required, let CheckComplete set
delete=true, execute the actual no-pending-completion branch, set evaluating=false
and PopFront. Unit and Formation movement values do not change that deletion
result. A pending movement completion remains an explicit partial dependency.
A false activation CanStart result uses the concrete
[Unit MaxMoves/HasMoved provider](linux-unit-movement-state.md), including actual rules, Unit modifiers and Player diplomacy. It pops
the front when the Unit has not moved or its signed push turn precedes the
actual current turn; otherwise it retains the node. Missing movement/alliance
inputs remain precise partial failures. No Formation `has_moved` flag supplies
this Unit predicate.

Timer assignment, ordered queue writes,
empty-queue reports and repeated Formation path/cache clears happen in place.
It reuses `set_activity`, `clear_path_cache`, full Manager/SkipTurn predicates and
the current-interruption/aftermath modules. Start is split at its report boundary
so an unavailable report consumer stops before the following activity write;
the existing leaf's infallible hook cannot express such a failure.

## Concrete captured resources

`loaded_actions::resources::Resources::from_closed_captures` consumes the actual
closed placement/Manager `native-1` facts and validates their admitted terminal
runtime, configuration-file and fact hashes. It reads **116 actual Unit
definitions**, **63 native-order operation definitions** and the four observed
selected phase slots. No expected CanStart/count result or saved byte mask enters
this adapter. The original disabled vectors remain comparison targets; this
loader does not install those observed outputs instead of initializing state.

`Resources::bind` checks the target runtime's canonical configuration identity
against the loaded binding, then compares actual module/host assets, content
gate, saved configuration, DLC descriptors and observed activated mods/components.
These values match for the closed resource captures and the existing native3
runtime even though their observer profile hashes differ. Immutable definitions
can therefore be used conditionally in that admitted resource configuration.
This check does not authenticate a new execution or establish an unchanged live
rules database after unsupported callbacks/configuration changes.

Mutable war relations, Manager membership and Player/control facts are installed
through `Bound::admit_observed_snapshot(&mut LoadedState)` exactly once. It checks
the complete **80,657,462-byte** original image against the associated snapshot
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`,
the Bound destination binding and core's pristine/one-shot guards. A mutable
borrow or action advances the revision, even if the caller later restores the
same bytes; the old captured facts cannot then be admitted again. Existing
controls/initialization also block replay.

The actual decoder framing role stays unchanged. A separate observed-snapshot
witness records the SavedOutput source role and its capture/runtime/facts/
configuration identities. This is an explicitly attributed observed-snapshot
diagnostic, not a claim that native3's Read process supplied these mutable
controls or reproduced all Read effects. Different full images, including
`a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa`,
fail the snapshot gate. No broad role-admission change is made.

The one-shot method installs 43 actual Player configurations, preserving the
one-human roster and captured pause=0/automation=-1 values. It transfers a
nonclone `SnapshotState` owning the 2,016-element mutable WarTypes vector.
Subsequent `relations()` accesses that owned current state; the API exposes no
reusable original-controls or original-war-vector issuance. Faithful diplomacy
providers must retain and update this same sidecar with the LoadedState.
Provenance names both independent placement/Manager capture artifacts and their
shared saved-snapshot association, with `native3_live_state_acceptance=false`.
None of these mutable facts is exposed through the cross-profile immutable
definition view.

The resource adapter supplies Unit domain/placement and operation/phase inputs.
The [Unit initialization definition builder](linux-unit-initialization-definitions.md)
supplies the source-backed ability/terrain/feature/GreatPerson collection orders
from the admitted actual rules. Runtime handler virtual slots, scripting
callbacks and diplomacy/notification behavior still require their own genuine
providers; none is fabricated by this resource adapter.

## Diagnostic boundaries and partial failures

The scripting provider currently receives the sole mutable addressed Unit
scratch and referenced request. Exclusive borrowing prevents a safe mutable
alias to this LoadedState while the provider runs. A fresh Unit is copied back
after Manager return, including a false result or provider error; the World is
never cloned. A script that changes the Unit before world-backed placement
returns a precise partial-execution error after retaining that mutation, because
the current World view cannot overlay an authoritative scratch actor. Changed
logical identity returns the complete mutated actor in `ActorIdentityChanged`
rather than installing it into inconsistent recycled storage. This bounded
interface cannot claim arbitrary gameplay callback effects on other Units,
Players, Game state or RNG. A future live action context must support those
shared reads/writes and redirect actor lookups to its authoritative actor.

Reports run synchronously through a scoped `ReportingContext`: providers read
the authoritative LoadedState and may mutate only the addressed actor through
`actor_mut()`. The guard compares that addressed Unit, permits activity writes,
and retains other actor changes before returning a partial error. It does not
claim to scan or validate arbitrary changes elsewhere in the World. The context
prevents arbitrary safe providers from acquiring other mutable World components.
This is an explicitly partial diagnostic interface; genuine whole-World/RNG
callbacks require a future versioned mutation context and remain unsupported
until their effects can be retained and executed. Missing gameplay consumers
must return errors, never silently become empty subscriptions. A collector is
useful for a diagnostic trace but cannot discharge reporting obligations.
The actual stock reporting/UnitFlag result is scoped to verified stock inputs
and no added dirty-list gameplay consumers; this adapter still requires its real
gameplay callbacks. UI dirty-vector acceptance is not a gameplay gate.

The explicit `HeadlessLocalAuthority` transport scope follows the pinned
[Unit/Formation archive consumer evidence](linux-load-archive-gameplay-scope.md).
Native dirty-pointer baselines, tree ordering and registration acceptance are
excluded from this local boundary. Actual setter writes and gameplay reports
remain. Other archive families, custom consumers and gameplay-mutating native
`loadDelta`/snapshot resynchronization are outside this profile. The code passes
false only to the lower leaves' optional transport registration; it never labels
unknown wrappers as attached or invents a clean baseline.

Every failure includes the completed prefix trace and leaves real mutations and
provider effects in place. It is not rollback and must not be reported as an
unchanged rejection. The combined function's `DiagnosticComplete` means the
supplied provider chain returned; it does not certify full action acceptance.
Native diagnostic logging backends remain outside this bounded source provider
profile; the typed trace is not a native service comparison.
Request decoding/TurnUnready, unrestricted callbacks, whole-action native
comparison, remaining gameplay state and full RNG inventory remain gaps.

`trace_json` emits report rows using the existing native/kernel `type_bits`,
`component_bits`, `first` and `second` shape, plus diagnostic stage and legality
rows. Both failed prefixes and completed candidates can therefore be compared
without treating collection as report delivery. No save writer or binary-equal
save gate is added. The original raw **89,198-byte** Save→Load difference stays
unclassified and unmasked.

## Checks

```sh
python3 tools/linux/loaded_skip_turn_continuation_audit.py --output target/linux/loaded-skip-turn-continuation.json
cargo test -p c6-data loaded_actions
cargo test -p c6-data loaded_actions -- --ignored
cargo clippy -p c6-data --lib --tests -- -D warnings
```

The ignored tests read the existing private `native-execute-3/before.bin`
fixture, checking its complete Core hash and selected Unit hash against the
public original metadata before decoding. Override the path with
`C6_LOADED_ACTION_BEFORE_CORE`. They execute no original code. They check Start's
synchronous activity reread/equality suppression; retained callback mutations
and stop on unsupported/error; and assignment-before-report failure. These
three pinned-record source replays pass. Two additional closed-resource tests
check actual cross-profile reuse, changed-resource rejection, full-byte
associated-snapshot validation and preservation of the separate decoder framing
role. Pristine/replay rejection and full state-phase/aftermath behavior are
covered by the [independent full-entrypoint review](linux-loaded-actions-review.md). The ordinary test checks exact packed
signed identity bits and optional report arguments. The independent review uses actual decoded state and captured controls/war
inputs with explicit synthetic handler/script/report probes, and deliberately
stops at the unavailable notification backend. No full loaded-world/native
SkipTurn acceptance or complete affected-state comparison is claimed.

Original target remains Linux XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, matching
host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Reference attribution remains in [reference.md](reference.md); private game
artifacts and raw reverse-engineering evidence remain outside the public repo.

`diagnose_skip_turn_observed_owned_aftermath` continues the same action algorithm
through `LoadedState::process_pending_diplomacy` before constructing its required
notification provider. The Game.Diplomacy.Manager owner retains stable ever-alive
handles and live sessions; each outgoing statement is removed before the supplied
processor runs against the same authoritative LoadedState. Missing manager state
or a missing statement/decision backend returns a partial failure with the
executed action trace. The method does not infer empty sessions from saved bytes
or transplant the historical execute-3 runtime. An admitted current dynamic
witness is still required before that owner can be installed. Notification Update
and complete gameplay/RNG comparison remain required.
