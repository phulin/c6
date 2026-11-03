# Movement-only original query caller and ownership scope

The new `unit_movement_query_profile.c` caller and
`unit_movement_query_profile_guard.[ch]` supply the pending lifetime/owner
boundary for the [standalone Unit query fixture](linux-unit-movement-query.md).
They are source-tested preparation. **No original code was executed, no fresh
native profile was admitted, and direct native MaxMoves/HasMoved comparison
remains pending.** Complete gameplay state/actions/turns remain unimplemented.

The guard has its own static `movement_scope`. It neither calls a Manager
fixture nor accesses/aliases its guard or globals. Source contracts for the
same native Context/Game/controller/lock fields and genuine facade acquisition
are reused and fingerprinted against the pinned XP2 module and matching host.
There is no additional debugger, VM/frame, idle-queue or Save-equality gate for
this movement-getter profile. A debugger remains a separate supervisor choice.
The field scope itself does not establish a policy decision boundary or purity.

The four lifecycle selectors are two-argument functions taking module and
borrowed Context: `profile_admission_begin`, `profile_caller_idle`,
`profile_probe` and `profile_admission_end`, each prefixed
`c6_linux_unit_movement_query_`. `profile_identity` supplies base/Game/main TID.
`pre_acquisition`, `retained_owner`, `locked_end` and `released_owner` are new
three-argument callbacks taking module/Context/Game. `locked_begin` adds the
actual Core base and main-entry TID. The final header SHA is
`91bdfb0076d735e7876793e390789052b95e896ef2d99990b4fa9f3c2692fd13`.
The earlier wording-only header preimage remains historical; no callback
signature changed during that correction.

The scope requires the actual own-entry main-thread witness
`c6_linux_mod_main_thread_tid`, stable controller/exported/native Context/Game
owners, initialized objects, no competing worker and the original native lock
owner/depth. The native owner token is captured after genuine acquisition and
is not equated to the OS TID. Guards read fields using safe process-local copies;
they never invoke an original getter, lock, policy, action, queue drain or turn.
Unknown ownership exits240 immediately, before further original calls. They do
not clear or compare pending-work queues, AI counters or actor membership.

## Caller lifetime

The caller validates/copies four environment inputs:
`C6_UNIT_MOVEMENT_QUERY_PROFILE_SHA256`, `INPUTS`, `ANSWERS` and
`CANDIDATE_CORE_SHA256` (all with the same prefix). The latter is the frozen Rust
reference image ff31 identity. It is not a native field, desired getter answer,
produced-Save comparison or Save-equality gate. Inputs and answers use distinct
exclusive-create output paths in the existing collector.

The outer lifecycle establishes the movement admission scope. The caller then:

1. Checks actual GetGame/Delete/DecrementReference symbol bindings, facade
   destruction target and current EngineUtility object/virtual lock dispatch.
2. Uses genuine `Exports.Context.GetGame`, verifies its actual facade vptr,
   Native Game identity and reference count, then captures the native lock token.
3. Invokes the existing collector exactly once with the new typed retained-owner
   callback and expected coverage1,262. It supplies no Unit fields or answers.
4. Rechecks owner/facade/release dispatch and uses the original ICvUnknown release
   exactly once on ordinary returns, including nonzero collector statuses.
5. Verifies the released ownership boundary without dereferencing the freed
   facade, then returns the collector status for the outer lifecycle to retain.

Fatal ownership/dispatch failures do not attempt original cleanup. The caller
logs four actual raw owner/depth and64-byte known RNG observations around
acquisition/queries/release. These are diagnostics, not a complete RNG inventory
or equality veto; queued gameplay/RNG effects are preserved. No internal Saves,
script, Manager predicate, action, AI policy or turn call is added. The original
facade/query source closure and exact module/host/dependency/input/rules/
configuration identity must be reviewed in the sole runner's fresh profile.

## Retained source-only checks

[evidence/linux/unit-movement-query-profile-source.json](../evidence/linux/unit-movement-query-profile-source.json)
records strict C warnings, GCC analyzer and UBSan checks. Four successful mock
cases prove one acquisition/collector/release sequence, status propagation,
local environment-input lifetime, retained queued/RNG effects and no reads of
a released facade protected with `PROT_NONE`. Forty-one refusal cases check
exact fatal reasons and absence of further mock-original calls: genuine OS
wrong-thread invocation, missing/repeated scope, unreadable/extreme pointers,
worker/controller/Game changes, native token/depth changes, malformed input,
wrong bindings, unknown facade/refcount/destructor/utility dispatch and release
boundary changes. A collector fatal performs no caller cleanup.

These tests use fabricated carrier objects and locally generated trampolines
into mock C functions. They never load a game/host module or execute original
instruction bytes, and their synthetic output files are not native query
answers. Complete compiled source preimages, exact test executable, compiler,
dynamic dependency identities and per-case logs are retained in the private
candidate directory named by the report. The archive explicitly identifies its
new-file WIP relative to the recorded commit; it is not relabeled as a coherent
committed original profile. Earlier diagnostic source snapshots remain private.

The sole runner owns loader/CLI/main-witness/lifecycle integration, the exact
new profile and original execution. The integrated source must then receive
independent review before one fresh original run and source-matched Rust
comparison. Coverage must include source-valid unplaced/delayed-death records;
no invented safe fields or silent filters are allowed. Original ownership or
query observation gaps remain validation gaps. No push or original execution
was performed for this packet.
