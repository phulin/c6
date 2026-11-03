# Actual Linux ScriptSystem semantics and the next state bytes

The [source/closed-byte audit](../tools/linux/script_state_semantics_audit.py) narrows the actual native-2 scripting result: the selected protected body reached its **callable method path**, returned normally, and obtained the requested **nil Lua result**. This is distinct from a false Lua result, the wrapper's false return/out `00`, and the complete Manager's true AL. It establishes neither empty callback storage nor a no-op backend. No Rust provider or replayed result is added; complete state/actions/RNG/turns remain unsupported.

## Null ASL owner is not an unlocked mutex

The exact Linux host's `InitializeCriticalSection` allocates **40 separate bytes** with malloc and stores that pointer at critical-section `+0x18`. It constructs a recursive pthread mutex using type 1; the object is pointed storage, not inline bytes inside the captured 64-byte critical section. Native-2's pointer `0x779bca0` is outside the static Lua lock at `0x6ec9c70`. Those 40 bytes were not captured.

Original Enter increments bookkeeping at `+8`, executes original pthread try/block locking, increments recursion at `+0xc`, and on a 0→1 transition stores the result of `ASL::GetCurrentThreadHANDLE` at `+0x10`. That helper reads its TLS numeric ID, searches the actual registry under its own registry mutex and returns NULL when no entry is found; a stored NULL can also return NULL. There is no requirement that every native main thread have a nonnull ASL handle. Original Leave decrements recursion, clears the pointer at zero, decrements bookkeeping and unlocks the pointed pthread mutex.

Thus native-2's owner-pointer zero with recursion one is compatible with the original locking implementation. It must not become “missing lock,” an OS TID, or a prerequisite to register a thread merely to inspect it. The blocking pthread-lock return is not checked by Enter before updating bookkeeping; recursion one alone also does not prove its return status or underlying mutex ownership. The existing normal original Enter/Leave PCs, actual pointed-mutex identity and main-thread/context facts remain separate evidence. **The observer must not call GetCurrentThreadHANDLE or another lock/getter merely to inspect it.**

## What closed native-2 actually proves

At host PC `0x36b3c44`, the current state/context registers equal the selected CCall's captured state and context. The retained actual parent chain includes that original CCall continuation `0x3bf2308`, followed by concrete CallTestAll, utility and Manager continuations. The pinned `vm_call_internal` C-call return and original protected body supply a narrow source attribution, without predicting a complete HKS stack. The old raw producer's generic “unattributed” event is preserved unchanged; this new analysis does not pretend it recorded an actual closure.

The original body reaches this PC only after global/event table checks and successful `lua_isfunction`. It then pushes ScriptSystemArgs, calls the function for one result and assigns non-nil and truthiness bytes to the retained context. The recorded protected return is status 0/success 1. Context byte `+0x18` is zero before and after; byte `+0x19` starts as raw uninitialized 99 and ends zero. Never reinterpret that initial byte as a valid C++ bool. Together with the attributed callable path and source stores, the final flag means the actual requested result was nil. A false Lua boolean would be non-nil and produce a different flag/Manager veto path.

The installed TestAll function could be built-in, overwritten, or a Lua closure. Built-in lTestAll returns nil when it visits no nonempty callback array, but its installed identity and actual shared group storage were not captured. Nil alone therefore proves no empty registry, absent callback, lack of prior lookup/indexer effects, or absence of arbitrary method effects. Save/RNG endpoint equality supplies none of those missing meanings.

## Exact missing bytes and minimal natural window

All three captured outer state headers have top equal to base, after the original protected stack is restored or before it is created. Their global-state and global-table pointers are available, but their pointees are not. There is **no fresh resolved-state header or top[-1] TValue**, so the actual installed callable and its closure/upvalues cannot be recovered retrospectively.

The minimal next passive observation is the already naturally reached `0x36b3c44`, after the original lookups and function check, before original PushScriptSystemArgs. Retain exact selected state/context, original caller/return/RSP identities and the actual CCall ancestor's unwound stack/register facts; unknown internal VM/GC frames remain unresolved. Copy only bounded readable memory while all threads are stopped, without any new engine/Lua/API/getter/lock invocation:

- Fresh actual-state header and the 16-byte HksObject at actual top minus 16; raw C/Lua closure/environment/upvalues and source identity.
- Actual raw globals/event/method tables and metatables. For proven built-in TestAll, actual shared groups, local callback arrays, native iteration/objlen/holes and every raw slot. Existing offline HKS layout readers already support this shallow reconstruction; unsupported/ambiguous storage must retain partial bytes and remain unsupported.
- The live concrete ScriptSystemArgs object and its active 24-byte cells. Source inspection closes its late inline-vector metadata: **object `+0x1a0` data pointer, `+0x1a8` u32 capacity, `+0x1b0` u32 size**. It is not a conventional three-pointer vector at object `+0x10`. Signed-int cells have low16 type 2 and payload i32 at cell `+8`; all other bytes remain raw.
- The existing Lua lock's separate pointed 40-byte pthread object. Keep real pthread return/owner proof separate; do not infer it from ASL NULL.
- Raw collector-prefix/control bytes only where safely bounded and source-closed. GC classification remains unresolved; no flag is an idle proof. Lua Method/prototype/code and transitive environments still need further contracts if actual callback closures require them.

This is an opt-in passive extension proposal, not a new capture or runtime admission. A fresh reviewed source/profile is required, and closed native-2 remains immutable. The eventual Rust provider must consume actual registration/method/behavior state and preserve lookup/allocation/callback/RNG effects; replaying native-2's nil/false/success as a constant is unsupported. The historical 89,198-byte raw Save→Load difference remains unclassified and unmasked; serialization equality is not an action gate.

## Implemented opt-in passive collector (not yet captured)

[script_state_snapshot.py](../tools/linux/script_state_snapshot.py) implements the proposed read-only extension in the existing hardware observer. A plan without `script_snapshot` keeps its old frame projection and raw protocol. Closed native-2 replays using its retained source preimages and reproduces the committed raw report unchanged. Its sealed files are not modified or reinterpreted as a newly decoded registry.

The fresh static plan must set `observation_mode: "raw_unresolved"` and add this key (substitute the exact collector source SHA):

```json
{"script_snapshot":{"schema":"c6-linux-script-natural-snapshot-v1","phases":["acquired","resolved","before_release"],"byte_budget":16777216,"max_items":65536,"source_sha256":"<exact collector SHA-256>"}}
```

Bind that source in the prepared configuration's recursive source inventory and include all eight `ARGUMENT_BODIES` exported by the collector as host `live_bodies`, copied into the reviewed stack packet. Existing original module, main-thread, facade/guard, selected Manager, actual return-slot, code/mapping and all-stop checks remain active. The collector adds no breakpoint lane, target call, lock, field write or forced lookup. Source/ownership/ABI refusal stays separate from optional diagnostic incompleteness. A fresh sole-runner profile and independent review are still required; this implementation is not an extra permission requirement.

At acquired/before-release, actual RSP must equal the retained selected CCall entry minus 24 and actual r14/r15 must retain that context/protected body; acquired also checks actual state in rbx. At resolved, current r14/rbx must match the selected context/state, and an actual unwound CCall continuation at `0x3bf2308` must expose that same entry-minus-24 RSP, context, body and state in its callee-saved registers. Unavailable registers or mismatches emit an unattributed diagnostic and copy no heap. Unknown intervening HKS/startup frames do not imply GC zero or block bounded copying when these actual selected-call facts exist. The original generic raw producer's unresolved event remains visible separately.

Each selected observation copies independent bounded sections for current state/collector prefix, Lua lock plus pointed mutex, live arguments, raw named-event graph, and separate current singleton roots. At resolved it also decodes the actual top[-1] closure; a proven built-in TestAll includes its shared groups, native callback order/holes and raw weak-mode metatables. Current singleton roots are never substituted for the actual retained CCall state. Unsupported storage, unreadable detail, overlapping-byte disagreement or exhausted budget produces an explicitly partial private image and section errors; successfully copied independent facts survive. Diagnostic completeness does not supply an empty-registry result or replace/suppress the original predicate. Artifact/unsafe-source failures still follow the existing runner refusal/disposal protocol.

Images live in fresh exclusive `capture/script-state/snapshot-NNNN-PHASE` directories. Their manifest retains the configuration, actual selected context/state/argument pointers, collector identity, exact segment hashes, partial status and unresolved GC. Replay only captured bytes with:

```sh
python3 tools/linux/script_state_snapshot.py --image /private/snapshot/image.json --output /tmp/decoded-script.json
```

The [source audit](../evidence/linux/script-state-snapshot-source.json) records 14 focused collector regressions, 31 existing adapter cases, eight newly pinned argument-layout bodies, and unchanged closed native-2 report replay. The collector preserves raw pthread mutex bytes and never interprets ASL NULL as unlocked or establishes a pthread return/owner status. Its copied graph is a stopped byte image, not proof of GC consistency, complete callback effects or transitive Lua Method/code behavior. Those actual bytes must guide the next semantic implementation; an observed nil result is never a Rust constant backend.
