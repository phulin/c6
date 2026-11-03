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
`update_owner_blocking` functions in that order. Initial Manager rejection still
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

The adapter checks the handler observation's configuration identity against the
loaded binding. Definition orders, phase selection, placement relations and
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

After initial legality, the implementation performs Clear, old deactivation and
front deletion, insertion, fresh activation legality, activity assignment,
SkipTurn Start, and final front deletion. Timer assignment, ordered queue writes,
empty-queue reports and repeated Formation path/cache clears happen in place.
It reuses `set_activity`, `clear_path_cache`, full Manager/SkipTurn predicates and
the current-interruption/aftermath modules. Start is split at its report boundary
so an unavailable report consumer stops before the following activity write;
the existing leaf's infallible hook cannot express such a failure.

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

Reports run synchronously against the actual loaded Unit. The bounded
continuation permits activity writes; other Unit writes remain in the record
and return an error before unsupported continuation. Missing gameplay consumers
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
three pinned-record source replays pass. The ordinary test checks exact packed
signed identity bits and optional report arguments. No full loaded-world/native
SkipTurn acceptance or complete affected-state comparison is claimed.

Original target remains Linux XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, matching
host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Reference attribution remains in [reference.md](reference.md); private game
artifacts and raw reverse-engineering evidence remain outside the public repo.
