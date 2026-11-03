# Linux CanStartOperation script backend boundary

The captured ScriptSystem is the concrete Linux host `GameCore::ScriptSystem`, and its argument/event backend is now fingerprinted in the [static audit](../evidence/linux/script-can-start-boundary.json). **Full Manager CanStart remains unsupported.** The capture contains no callback registration/state snapshot or callback effects; a present ScriptSystem cannot be treated as an empty registry. The current-operation handler's CanBeInterrupted identity/effects also remain a separate full Manager provider obligation. Complete gameplay state, actions, all RNG and turns remain unimplemented. The existing **89,198-byte** raw Save→Load failure remains unclassified and unmasked.

This audit reads the pinned XP2 `a0b6611f…1c1faa` and matching host `106d4986…fe74f4`, and rechecks closed profile `aa38b5ed…e08aff` / Manager `native-1` runtime `5b68cfff…3740`. It executes no original code. Host ELF symbols are absolute ET_EXEC virtual addresses, without a subtraction of `0x400000`. The two raw fact passes are identical; their global row contains receiver `0x96c9dd0`, vptr `0x53a63a8`, and `script_callbacks_inspected=false`.

## Concrete receiver and arguments

The observed vptr is the address point of the host `GameCore::ScriptSystem` vtable (`0x53a6398+0x10`). Its slots `+0x38`, `+0x48` and `+0x80` bind original `CreateArgs` (`0x32461b0`), `DestroyArgs` (`0x3246270`) and `CallTestAll` (`0x32463d0`). The [Manager orchestration](linux-unit-operation-manager.md) retains the initially resolved system for the call but refetches the current XP2 system for argument construction/destruction. The actual host implementations clarify those boundaries:

- CreateArgs allocates `0x1b8` bytes from the current TempHeap, falls back to Platform::Malloc, then constructs `ScriptSystemArgs` with its parent system pointer at `+8`. The vector at `+0x10` has inline capacity 16 of 24-byte TypedVariants. The signed integer push (`0x3245810`, args slot `+0x20` at vptr `0x53a6300`) appends a TypedVariant set from the original i32 and returns the appended zero-based index. Manager pushes owner, Unit ID bits as signed i32, and current runtime operation ID in that order.
- DestroyArgs calls the argument object's non-deleting virtual destructor, then tests ownership against the current TempHeap and deallocates there or through Platform::Free. It does not use the system receiver to select the allocator. The destructor handles strings, user objects and managed table variants individually; three integer values do not invoke user destructors. This does not establish allocator/exception/GC parity for a Rust replacement.
- Concrete ScriptSystem::CallTestAll does not read its receiver. It obtains the current `LuaSystem::LuaScriptSystem` singleton from host `0x6ec9e68`, reads its main lua_State at `+0x10`, and forwards to `LuaSupport::GameEventLibrary::CallTestAll` (`0x36b40b0`). Retaining the original ScriptSystem still remains the native Manager contract; arbitrary overrides cannot be replaced by this concrete behavior.

## Lock direction correction

The host utility constructor installs address point `0x53a6c90`. The closed capture logs successful construction of the genuine host EngineUtility, but records neither the actual XP2 utility receiver pointer nor its live vptr. These source bindings therefore identify the concrete backend, while a passive utility read remains required to verify the actual receiver at the observation boundary.

| XP2 wrapper slot | Concrete host method | Effect |
| --- | --- | --- |
| `+0x30` | HasGameCoreLock `0x32b3da0` | Read whether current thread owns active controller lock |
| `+0x38` | ReleaseGameCoreLock `0x32b3d60` | Release one held lock level |
| `+0x40` | GetGameCoreLock `0x32b3d40` | Acquire one lock level |

Thus the concrete XP2 wrapper **releases an already-held lock before Lua and reacquires it afterward**. Earlier generic “lock/unlock” descriptions in the Manager report do not establish the concrete direction. It fetches current utility separately for the query, conditional release and conditional reacquisition. Each host method refetches the active controller. Missing controller gives false/no operation. Controller HasLock compares current thread ID with `+0x4a0`; release decrements `+0x4a4`, clears owner at zero and leaves the critical section at `+0x460`. Acquire tries/enters that section, increments the count and writes current thread ID; it also contains timing/diagnostic behavior. This is not a disposable boolean provider or proof that the main fixture held the lock. No exception-safe native relock path is invented.

## Named event storage and dispatch

GameEventLibrary builds a protected C-call context with event `CanStartOperation`, method `TestAll`, args pointer, and an initially false “non-nil result” flag. Its protected body (`0x36b3bc0`) gets global `GameEvents`, looks up the event and its `TestAll` method with ordinary `lua_getfield`, pushes the arguments and calls the method for one result. A non-nil result sets the flag, and Lua truthiness supplies the result value. The outer wrapper returns that flag and writes the caller's out-value only when the protected call succeeded and the flag is set. A missing/non-table event, missing/non-function method or nil result leaves caller out-value unchanged and returns false. Do not substitute “callbacks ran” for this return bit: an overwritten method can return a value without invoking built-in callbacks.

Ordinary event lookup is not passive. `CreateLuaOnlyEventFactory` (`0x36b4190`) installs global `GameEvents` with a metatable whose `__index` is `lEventFactoryIndexer` (`0x36b4270`). The indexer raw-looks up a per-name shared group table in its upvalue, creates one when absent (with weak values via `__mode="v"`), calls `GameEvent::lCreate`, and caches the resulting event table. Merely evaluating `GameEvents.CanStartOperation` can therefore allocate/register state. `GetEventFactory`, `Count` and `TestAll` are not read-only inspection substitutes.

`GameEvent::lCreate` (`0x36b2b60`) creates a local callback array, appends that array to the shared event group table, and registers methods. Each Add/Count/Remove closure captures its local callback array in upvalue 1; each Call/TestAll/TestAny closure captures the shared event group table in upvalue 1. `RegisterMethod` (`0x36b3960`) installs the closure by rawset. Add (`0x36b2d70`) appends at local array objlen+1 without a callback-type check. Consequently the named event's Add/Count upvalue alone does not describe all TestAll registrations.

The built-in `lTestAll` (`0x36b3540`) traverses the shared table in actual `lua_next` order, skips non-table values, and captures each callback array's objlen once on reaching it. It reads raw numeric slots 1 through that length, creates a new Lua thread for each call, installs the callback's environment and copies the arguments to that thread. Successful protected callback results are ANDed by Lua truthiness, starting true. Nil/false are false; zero and empty strings are true. A protected callback failure does not alter the aggregate and iteration continues. There is no false-result short circuit. If at least one nonempty callback group was visited, the built-in returns the aggregate; otherwise it returns nil. Live changes to group storage, array slots, closure environments and callback state remain observable; replacing this with a flattened immutable callback vector is not source-backed.

These are contracts for the built-in implementations. The actual globals, event table, methods and upvalues may be overwritten, and none was inspected in native-1. A built-in method's address does not prove that it is the installed named method. No Rust module or aggregate simulator is added because the actual registration identity, callback effects, protected error handling, VM state/GC and mutation ordering have no admitted native fixture here. Lua `math.random` and `Game.Random` both remain required wherever used; neither is excluded by this audit. Native AI policy must not supply any missing decision.

## Next passive capture

The sole original runner can prepare a separate read-only capture at the same controlled loaded-state boundary. It must preserve configuration identity and before/control/after/repeat raw Saves and known RNG controls, while adding:

1. Actual XP2 engine utility pointer/vptr and relevant slots, current ScriptSystem pointer/vptr, main Lua singleton/state pointer, active controller pointer, lock owner/count and current observing thread identity.
2. Raw main global `GameEvents` and `GameEventFactory` entries, both metatables and their indexer closure/upvalues; raw `CanStartOperation` entry with an explicit absent/non-table/table distinction. Preserve overridden globals/methods as unsupported identities rather than invoking them.
3. Raw `TestAll` method closure kind/code, environment and upvalues. For the proven built-in closure, copy its shared upvalue-1 group table, including exact array/hash storage and iteration order, every group value (including non-table values), local callback arrays with holes and lengths, and each callback slot's type/identity. Capture callback closure/prototype/code, upvalues/environment and script provenance to identify subsequent callback semantics work.

The observer must not call ordinary global/field lookup, indexers, factory creation, Add/Count/TestAll, callbacks, or VM APIs merely to inspect. The strongest read-only design pauses competing mutators/GC at an already authorized quiescent boundary, copies bounded mapped memory, and decodes offline. Pinned `hksi_lua_getupvalue` (`0x3bc16c5`) exposes useful HKS leads: 16-byte TValue entries, main global TValue at lua_State `+0x70`, and C-closure upvalue storage beginning at closure `+0x30`. These are insufficient for a safe complete table reader: close HKS table/string/prototype extents, tags, weak storage and iteration semantics before traversing. Do not use an allocating stack API as a claimed zero-mutation memory snapshot. Missing layout or registration memory is the exact current blocker, not evidence of no callbacks.

Reproduce the source checks without original execution:

```sh
python3 tools/linux/script_can_start_boundary_audit.py --self-test
python3 tools/linux/script_can_start_boundary_audit.py --output target/linux/script-can-start-boundary.json
```

The audit gates both complete ELF identities, 25 host method/body fingerprints, seven concrete slots, event strings, the complete XP2 wrapper and the closed repeated raw capture. Its tests exercise identity/binding mismatch rejection and lock-slot direction. This is source/boundary validation, not native numerical Manager comparison or callback acceptance. Existing Manager adapters, frozen candidates and runtime runner remain unchanged by this task.
