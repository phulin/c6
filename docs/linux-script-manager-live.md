# Passive scripting observation inside genuine Manager CanStart

The [observer](../tools/linux/script_manager_observer.py) source-closes a passive window reached **after the original Lua lock acquisition** during the concrete Linux Manager predicate. It adds bounded private image capture using the existing reviewed HKS decoder. It has not run against the original process. Full Manager acceptance, callback effects, complete gameplay state/actions/RNG/turns remain unsupported. The historical 89,198-byte raw Save→Load failure remains unclassified and unmasked.

The [source report](../evidence/linux/script-manager-live-source.json) gates the complete target XP2 and Linux host hashes, 30 original method bodies, protected-call edges and the actual negative Lua stack-index access. Original code is read only. Raw disassembly and heap images are excluded from this public report. Earlier genuine AppLuaScriptSystem creation is established by the [VM lifecycle audit](linux-script-vm-lifecycle.md); neither that creation nor a nonnull global proves successful initialization or callback absence.

## Concrete path and source windows

Manager `XP2+0x2da20f0` reaches its script call with return continuation `XP2+0x2da22a0`. The concrete utility wrapper releases an already-held core lock before dispatch; its dispatch PC is `XP2+0x2ce79e9`. At that PC the original Manager continuation is at `rsp+56`, retained ScriptSystem is `r13`, event name is `r12`, arguments are `r15`, out pointer is `r14`, and `bl` retains whether the core lock was originally held. The observer requires the genuine concrete ScriptSystem vptr and original event bytes.

Concrete host ScriptSystem tail-forwards to GameEventLibrary::CallTestAll. At CCallWithErrorHandling entry `0x3bf22e0`, `[rsp]` must be `0x36b40e1`, `[rsp+64]` must be `XP2+0x2ce79ff`, `rdi` is main state, `rsi` is protected body `0x36b3bc0` and `rdx` is context. Context holds event, TestAll string, args and result flags. The observer checks the actual args identity. CCall pushes 24 bytes, retains state in `rbx`, body in `r15`, context in `r14`, then invokes the original critical-section Enter at `0x3bf22f5`.

| PC | Observation |
| --- | --- |
| Host `0x3bf22fa` | Original Enter returned normally; original protected body and lookups have not run. Snapshot actual roots, raw Lua lock, collector prefix and raw named registration state. |
| Host `0x36b3c44` | The original protected body already performed ordinary GameEvents/event/TestAll lookup and validated a function. Capture the actual stack function as well as raw named state. |
| Host `0x3bf2308` | Original protected call returned; retain signed status before error handling. |
| Host `0x3bf2329` | Protected body/error handling completed, before original Leave. Capture state again and retain protected-success bit. |
| Host `0x3bf2335` | Original Leave returned; retain raw lock bytes. |
| XP2 `+0x2ce79ff` | Concrete script method returned; retain return bit and actual out byte before utility reacquisition. |

The acquired/resolved/completed snapshots describe distinct states. GameEvents.__index may allocate or register during the **original** lookups. Raw absent keys before lookup cannot stand in for its actual resolution. Source `hksi_lua_isfunction(-1)` reads the HksObject at `[state+0x48]-16`, bounded below by `[state+0x50]`; the resolved snapshot copies that exact value. Even a metamethod-returned uncached method is retained. Proven built-in TestAll closures additionally supply their actual shared-group table, native stable iteration order, callback-array lengths/slots and shallow callback closure/environment/upvalue identities. Overridden functions remain overridden. Callback mutation can change subsequent iteration; immutable image order does not establish live execution order or effects.

## Runner integration contract

Instantiate `Observer(mapped_read, private_emit, gamecore_base, configuration_sha256, private_directory)` only after the runner verifies exact on-disk and mapped target identities, unchanged live body bytes, source/profile identity and its existing ordinary lower-Core fixture. This preparation neither grants native execution nor creates a profile. `mapped_read` must reject unreadable/unmapped spans; every observer request is separately bounded to the u64 address space and at most 2,621,440 bytes. A snapshot permits 65,536 entries and 16 MiB total requested bytes. All reads copy memory; there are no Lua/native/getter/lock/thread-identity calls.

The runner passes `observe(event, regs, thread, provenance)` and arms the returned next sites. Registers include `pc,rsp,rax,rbx,r12,r13,r14,r15,rdi,rsi,rdx`. At dispatch provenance must contain the exact configuration hash, scope `genuine_lower_core_full_Manager`, `all_threads_stopped=true`, and source-reviewed `active_lua_parent_frames=0`, `active_gc_parent_frames=0`. These are runner obligations, not facts inferred from idle flags. Preserve the frame/unwind evidence supporting them. Subsequent checkpoints require the same thread and exact retained frame/stack identities.

Start with dispatch, then CCall entry, then acquired. After acquired arm resolved **and** protected_return: missing/nonfunction lookup naturally skips resolved. After resolved arm protected_return alone; continue through before_release, released and wrapper_return, then dispatch. This needs at most two simultaneous hardware lanes. Use the existing outer-drain pattern: breakpoint stop callbacks queue facts only, mutate lanes after stop unwinds. No software patching, FinishBreakpoint, forced Lua loading, new lock acquisition, GC/frame step or callback invocation for inspection. Missing calls, stops, unwind, overridden backend and partial capture are unavailable observations and never empty callbacks.

The observer retains all 64 raw Lua critical-section bytes, including signed acquisition/recursion counts, owner **ASL ThreadHANDLE pointer**, and mutex pointer. It checks consistent nonzero ownership after the observed original Enter but does not equate that owner with the core controller's u32 platform thread ID or debugger OS TID. It never calls GetCurrentThreadHANDLE. Original acquisition flow and matching release remain required; counter bytes alone prove neither ownership acquisition nor GC idleness.

Heap images and decoded closure records go to a private directory outside this checkout. Overlapping reads must agree, merged segment files are hashed, and successful captures are checked by reconstructing named-event state entirely from the immutable copied image. Decoder failures preserve partial segments plus explicit failure provenance. A failed graph, null root or budget limit is never classified as absent callbacks. Method bytecode/debug/prototype graphs, transitive environments, complete VM/GC state, all affected gameplay state and callback/RNG effects remain separate work.

## Current admission gap

The sole runner confirms the current-handler metadata profile only reads fields and applies the three-argument idle guard. It never invokes the full Manager predicate and therefore cannot reach these windows. A separate full-Manager predicate profile in the same genuine lower-Core fixture is the next implementation/admission step. It must supply the real Manager receiver, Unit, operation, optional Plot/results, exact visible-only flag, caller lock lifecycle and existing scripting context; preserve before/control/after/repeat Save and known RNG controls because callbacks may mutate state. No GUI, saved-Begin/account-security reroute or forced Lua initialization is required or authorized here.

Reproduce source-only checks:

```sh
python3 tools/linux/script_manager_observer.py --self-test
python3 tools/linux/script_manager_observer.py --output target/linux/script-manager-live-source.json
```
