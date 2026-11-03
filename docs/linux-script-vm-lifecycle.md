# Linux main Lua VM lifecycle and passive observation windows

The accepted Manager `native-1` profile **did run genuine AppLuaScriptSystem creation before Context Load**. This is now verified from the immutable captured source manifest and ordered runtime markers, rather than inferred from ScriptSystem availability. Two original lock-owned windows are source-closed during ordinary gameplay-script loading. They are phase-specific opportunities; **no snapshot at either window is yet accepted, and neither is proved equivalent to the later Manager observation boundary**. Complete state/actions/RNG/turns, full Manager scripting and current-handler interruption effects remain unsupported. The historical **89,198-byte** raw Save→Load failure remains unclassified and unmasked.

The [audit](../evidence/linux/script-vm-lifecycle.json) gates 46 complete method/body spans, original call/tail/virtual bindings, the original Lua-lock loader initializer and closed profile `aa38b5ed…e08aff` / runtime `5b68cfff…3740`. It performs no original execution or live memory access. This extends [the concrete backend](linux-script-can-start-boundary.md) and [reviewed offline HKS decoder](linux-script-hks-registry.md), whose code identity `37c9c0f` was independently reviewed in `a5edd70`.

## What the accepted profile establishes

The archived `tools/linux/mod_activation_probe.py` SHA-256 `9aff7a381f5b6a1b172b737d93b5dab18bce73f8f18d0c0541a5a477136fae6a` sets `C6_HOST_LUA='1'`. The archived `native/linux/host_services.c` SHA-256 `f958b7afb65c865c061a8164e8686bb05c2bde9bda5b3a66dbc8939b5c22e3cf` uses that flag to invoke its existing genuine Create/thread probe. Both hashes match the closed reconstruction manifest and archived files. Mutable current runner sources are not substituted for that evidence.

The pinned runtime records, in order: Lua creation entered; creation returned with main state nonnull; a genuine subthread numeric roundtrip returned one; StartupGame returned; original Context Load returned zero after consuming all 76,556,938 input bytes; and, much later, Lua teardown returned with the singleton unpublished. The runtime also contains the subthread creation/release markers. This proves the existing preparation ran and observed a main state at that earlier point. The helper explicitly invokes the original factory; it does not prove normal Civ6App::GUIInit ran. There are no later main-VM pointer, registry, GC or Lua-lock snapshots in this capture. The six raw Save controls and Manager idle checks do not add those missing observations.

## Original creation, publication and teardown

Normal host Civ6App::GUIInit calls AppLuaScriptSystem::Create at `0x2f9412e`. Create (`0x36ae980`) allocates `0x120` bytes, completes construction and only then publishes the receiver to absolute host singleton global `0x6ec9e68` at `0x36ae9b7`. The actual App constructor (`0x36ae950`) invokes LuaScriptSystem construction with GC settings 110 and 5, then installs AppLuaScriptSystem's concrete vptr. It is distinct from EngineUtility's earlier host ScriptSystem factory, which does not create the main VM.

LuaScriptSystem::Setup (`0x3bfc5e0`) takes the original global Lua lock, calls original lua_newstate, stores the result at receiver `+0x10`, sets GC parameters and invokes pInitializeMainState through lua_cpcall. That initializer opens original standard libraries and installs main-state facilities. **Setup does not test that cpcall's return status** before unlocking and continuing with GlobalData, StaticFunctions and TypeExposures registration. The constructor/factory returning a nonnull state therefore does not prove every main initializer or later gameplay registration succeeded. The factory publishes only after those constructor calls return; a snapshot during Setup's earlier locked portion is not a completed/published VM snapshot.

Destroy (`0x2fa05c0`) calls the existing singleton's deleting destructor and then zeros the singleton global. LuaScriptSystem's destructor closes its main state under the original Lua lock. A null root can thus mean not yet published, failed/partial initialization, or completed teardown depending on the recorded lifecycle phase. Do not label it an empty callback registry. Concrete ScriptSystem::CallTestAll has no singleton null check; GetState directly dereferences receiver `+0x10`. A null state likewise has no guard before the original protected-call allocation path. Those are unsafe native contexts, not false script results or legal operations.

## Actual lock and GC ownership

The main Lua lock is the target Linux host's 64-byte `Lua::ms_CriticalSection` at absolute `0x6ec9c70`. Loader init-array slot `0x535e628` points to stub `0x2ca9990`, which reaches initializer `0x2ca9960`; that initializer constructs this exact critical section and registers its destructor. These are Linux compatibility implementations in the pinned Linux host, not historical Windows execution.

| Critical-section offset | Source-closed field |
| --- | --- |
| `+8` | Signed lock count, initialized to -1; includes acquisition bookkeeping/waiters |
| `+0xc` | Signed recursion count |
| `+0x10` | Owning ASL ThreadHANDLE pointer |
| `+0x18` | Allocated pthread mutex pointer |
| `+0x20` | Spin count |

InitializeCriticalSection creates a recursive pthread mutex. Enter/Try increment acquisition bookkeeping; successful ownership increments recursion and writes the current ThreadHANDLE when recursion was zero. Leave decrements recursion, clears owner at zero and unlocks the mutex. Try checks the actual pthread trylock result. Enter's blocking path does not expose a successful return status to its caller; a future observer must retain the already executed original acquisition result/normal-path provenance and mutex initialization identity rather than treating arbitrary counter bytes as proof of a newly acquired lock.

The owner is a pointer, **not** the u32 thread ID used by the core controller lock. ASL::GetCurrentThreadHANDLE (`0x2cdf911`) itself takes a thread-registry mutex. GetCurrentThreadId can initialize a TLS ID on its first call. Neither should be invoked merely to identify the observer. Use the original acquisition's already-produced owner value and existing thread/lifecycle evidence; copy raw owner/count/mutex fields without new calls or lock operations. A zero owner field must not be silently converted into a fabricated thread identity.

ScriptSystem::CreateLuaThread, protected LoadFile/CCall, PumpGarbage and AppLuaScriptSystem::EndFrame use this global Lua lock. EndFrame's native path checks load-state activity and controller execution, then performs a genuine TryLockAccess; only success reaches lua_gc. PumpGarbage performs a synchronous GC step while owning the lock. Inspected hksi_lua_gc paths invoke incremental/nonincremental collector steps synchronously. The collector can call a registered hook, and its step methods restore prior control fields after the increment. A raw GC control word or pause flag alone is therefore not an “idle” proof, nor does this bounded source audit establish every possible external VM mutator's ownership.

The core lock is separate: the CanStart utility wrapper can release its already-held core lock around Lua and reacquire it afterward. Controller/game idle does not imply the Lua lock is held or that GC is excluded.

## Ordinary Load phases and exact candidate windows

The original facade Load calls concrete Context::Instance::Read (`0x290bca0`) through slot `+0x78`; Save forwards to Write (`0x290bbf0`) through `+0x80`. Neither facade nor these complete Read/Write bodies acquires or retains a Lua lock around its entry/return. Their transitive calls do not justify inventing a protected post-Read or post-Save interval.

Read's existing-game virtual slot `+0x38` reaches InitializeExistingGame (`0x290caa0`), which calls Scripting::Manager::LoadGameplayScripts (`0x2d4d1e0`). That method obtains the actual ScriptSystem, gets its original gameplay-script count and visits real files in order through LoadState (`0x2d4d260`). The actual count is not recorded in native-1. Null system or zero files can skip this route; no observer may force a file load or CreateLuaThread solely to manufacture a window. Source GetGameplayScriptCount uses the real framework singleton and its gameplay-script vector, not a fixed expected count.

For each legitimate file, the concrete backend has these original, already-owned intervals:

| Candidate PC | Exact phase and provenance | Limit |
| --- | --- | --- |
| Host `0x3246037`, immediately before CreateLuaThread's original UnlockAccess | Its original lock was taken at `0x3245f45`; all thread-library and GameEventLibrary registration calls have returned. Ordinary LoadState continuation is XP2 base `+0x2d4d33c`. | Current gameplay file has **not** executed. This can contain earlier files' registrations and initial factories, not the later complete registry. |
| Host `0x3bf2329`, after protected body/error handling and before CCallWithErrorHandling's original Leave | Require retained C callback identity pLoadFile `0x3bfd5ae` and direct ordinary LoadState continuation XP2 base `+0x2d4d34e`. The original lock was acquired at `0x3bf22f5`; the protected call returned through `0x3bf2308`. Record its actual success/error status. | One file's protected load has completed, but LoadState's state bookkeeping and Tuner exposure have not. Other files and later lifecycle phases may still change the registry. |
| Host `0x36af0d1`, after EndFrame's successful original TryLockAccess and before lua_gc | Only the native load-active/nonexecuting-controller/successful-trylock branch reaches it. | A useful source-owned frame interval, but the closed synchronous core-stream route does not prove an original EndFrame occurs. Do not invoke one just to inspect. |

The pLoadFile selection matters: the same generic CCall wrapper runs many unrelated/reentrant callbacks. An arbitrary CCall PC, a native lock counter or a CreateLuaThread used by earlier Tuner/setup work does not establish ordinary gameplay-Load provenance. The original CCall and concrete LoadFile wrappers tail-forward so the direct LoadState return continuation can be retained. A future observer must also establish that this ordinary Load invocation has no active/reentrant Lua or collector parent frame, rather than capturing a nested finalizer/callback as a fresh idle load.

GameEventLibrary::Register (`0x36b3ab0`) runs its protected registration body on the **main state** and installs thread-local GameEvents proxies against the shared main factory. It creates plumbing; it does not prove registered CanStartOperation callbacks are absent or that the current file has run. The current root's nonnull status, raw GameEvents absence and source phase must be reported separately.

## Minimal future passive observation proposal

The sole runner may review this preparation separately from the current Military capture. This report authorizes no execution, runner change, extra initialization, added lock acquisition, Lua/API call, frame/GC step or callback invocation to obtain observations. It does not revisit or reroute the prior saved-Begin/account-security denial.

A future profile can observe an interval **only if the ordinary authorized lifecycle naturally reaches it**:

1. Gate both module hashes and actual profile/configuration identity. Record the existing Context Load invocation identity and original entry/return phase. Retain naturally returned gameplay-script count, current ordinal/path identity and original LoadState result without making another getter call.
2. Retain the already executed original Lua-lock acquisition receiver/result and owner value, its recursion level and matching original release. Copy the full raw Lua critical-section object and mutex identity. Do not call GetCurrentThreadHANDLE, reenter/try the lock, stop GC, pin threads or call a Lua getter to inspect.
3. At the selected original PC, copy actual GOT-bound XP2 ScriptSystem/utility globals, receiver/vptr identities, main singleton/state and lifecycle ordinals. Preserve null/unpublished/overridden roots. For a nonnull state, copy its global-state pointer at `+0x10` and the relevant raw collector prefix (collector starts at global-state `+0x58`; its hook at collector `+0x120` is a pointer). Keep these bytes unclassified where no meaning is source-closed; no GC flag substitutes for the acquisition/control-flow evidence.
4. If provenance and ordinary nonreentrant ownership are established, copy the immutable raw HKS image using the [reviewed decoder protocol](linux-script-hks-registry.md). Do no VM allocation or API lookup. Distinguish raw absent/ambiguous keys, overridden methods and real built-in callback groups. Keep proprietary heap/closure/script bytes private.
5. Label the snapshot with the exact phase above. A miss/zero-file route is an unavailable observation, not an empty registry or passing script comparison. Retain before/control/after/repeat raw Save and known RNG controls separately; byte equality does not establish complete GC/state/RNG coverage.

Relating the final file's protected snapshot to a later Manager check additionally requires: actual file count/order and last-file proof; result and remaining LoadState bookkeeping; actual Tuner delegate identity/effects (ExposeStateToTuner is conditionally a delegate call); later postload, cache/update, frame/GC, registration/removal and script activity; and continuity of all main singleton/state/closure identities across the original interval. Native-1 takes Manager metadata only after Read and raw Save controls. No such registry/lock/GC continuity trace exists. These dependencies remain explicit; a last-file snapshot must not be promoted to final Manager state.

Reproduce without original execution:

```sh
python3 tools/linux/script_vm_lifecycle_audit.py --self-test
python3 tools/linux/script_vm_lifecycle_audit.py --output target/linux/script-vm-lifecycle.json
```

Three source-window tests reject wrong callback/continuation/PC identity and retain the pre-file versus post-file phase distinction. The audit rechecks original bytes plus closed source/runtime identities. No native window hit, new passive image, callback execution or FullManager acceptance is claimed.
