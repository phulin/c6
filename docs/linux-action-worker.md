# Original Linux worker boundary

This is a static scheduling and ownership contract for the pinned Linux host and XP2, not an executed-action or gameplay-parity result. Reproduce the byte checks with:

```sh
python3 tools/linux/action_worker_audit.py --output /tmp/action-worker-abi.json
cmp /tmp/action-worker-abi.json evidence/linux/action-worker-abi.json
```

`Controller.ExecuteGameCoreUpdate` (`0x31d4950`) lazily constructs the original worker and calls `SetExecuting` before `Resume`. `SetExecuting` uses the original interlocked compare-exchange to change the worker counter from zero to one. `GameCoreThread.Run` (`0x31d11d0`) acquires its own critical section, obtains the live Game facade, calls original `Context.ProcessMessages`, and then calls `SetComplete` while still holding that critical section. `SetComplete` uses the original interlocked decrement. The original `IsExecuting` getter compares that aligned native counter; the adapter must call the getter rather than implement a replacement counter.

`Controller.Synchronize` (`0x31d1730`) returns immediately if `IsExecuting` is false. Conversely, calling it immediately after scheduling can acquire and release the worker lock before the worker acquires it. Neither pattern alone establishes completed processing. The bounded component contract is one original Execute, a monotonic-deadline wait for original `IsExecuting` to become false, then original `GameCoreThread.JoinOnPassComplete` (`0x31d4ee0`) on the actual borrowed worker. Join acquires and releases the same critical section after checking it is not the current thread. Observe the original suspended predicate before teardown. A deadline failure is a failed isolated run; do not take concurrent snapshots or report an unsupported nonmutating operation.

The byte at worker `+0x45` is a completion notification consumed by `Controller.SendProcessingComplete`, not a fresh per-pass condition: the constructor clears it, but `SetExecuting` does not. It is unsuitable as an independent replacement for the executing-counter contract.

Live Game and Player facade constructors acquire the controller's game lock through original EngineUtility vtable slot `+0x40`; their destructors release it through slot `+0x38`. The verified host bindings are `GetGameCoreLock` (`0x32b3d40`) and `ReleaseGameCoreLock` (`0x32b3d60`). Worker `Run` obtains another live Game facade before processing messages. Keeping an acquired Game or Player facade alive across worker scheduling can therefore block processing. Release acquired facades through their original ownership interface before Execute, preserve only valid borrowed core references, and reacquire facades after completion. Never release borrowed Context or worker pointers.

The failed initial conditional component execution queued a genuine original command but observed `executing=1`, pending command state, and no receive/operation trace. Its Saves are diagnostic captures, not proven coherent post-action snapshots. An outer cleanup marker alone does not prove that worker processing had completed. The revised lifetime and completion contract still needs a fresh native control and execution. Static construction/start call paths do not prove successful OS-thread creation in that fixture.

The compact method identities and checked call sites are in [action-worker-abi.json](../evidence/linux/action-worker-abi.json). Raw original disassembly is retained privately under `evidence/linux-action-execution/` in the proprietary evidence repository. Full-host command admission remains separately blocked by its genuine App loading predicate; this component scheduling contract does not resolve that acceptance gap.

## Passive diagnosis after the revised timeout

The combined lifetime/completion correction also timed out before command receive. It produced no post-worker snapshot or teardown acceptance. Increasing the deadline or adding another Execute would not identify the cause. A single passive replay must inspect all thread PCs/backtraces and actual native thread storage.

The borrowed worker global is host `0x6499108`; its `+0x48` points to the original platform thread. The original creation result is visible in RAX at `0x31d11af`. Within that platform thread, `+0xb0` is the OS handle, `+0xb8` the signal event, `+0xc0` the start event, `+0xc8` the native thread ID, `+0xcd` the logical started byte and `+0xce` the initialized byte. The OS-creation call returns at `0x3c44df4`. These are observation leads verified against the original methods, not fields the adapter may fabricate or write.

Original Resume calls `IsRunning` and `IsStarted`, then Start only when alive and not started. `IsRunning` (`0x3c44f2a`) tests whether the original zero-timeout wait on the OS handle returns `0x102`; nonzero handle alone does not establish this. Start (`0x3c44f4c`) marks started and signals the real start event. The OS entry (`0x3c44e22`) first calls `JobManager.RegisterMasterThread`, then waits on that event. The delegate call is `0x3c44eac`. Thus a started byte does not prove the worker reached Run.

Within Run, distinguish entry `0x31d11d0`, worker-lock acquisition return `0x31d11dd`, EditGame return `0x31d11e7`, and ProcessMessages call `0x31d1213`. The worker lock is at worker base; the separate controller game lock is at controller `+0x460`. Controller `+0x4a0` and `+0x4a4` record the successful owner's platform thread ID and acquisition depth. That ID must not be assumed to equal a Linux debugger LWP. All-thread backtraces are needed to identify a creator failure, start-event wait, registration callback block, or actual lock wait; static addresses alone cannot select among them.

## Original creation failure path

The single passive replay of the revised execution observed original Platform.CreateThread returning NULL at `0x31d11af`, with only the main OS thread present. The worker's platform-thread pointer remained NULL; Run and EditGame were never reached, and the controller lock owner/depth were zero. This timeout is therefore a creation failure, not an observed lock wait. The earlier facade ownership and synchronization corrections remain valid contracts.

Platform.CreateThread constructs WinThread and invokes its original Initialize. An unsuccessful Initialize destroys the object and returns NULL. Initialize first creates two events; either NULL event reaches its failure path before ASL thread creation. Its ASL call returns at `0x3c44df4`.

`ASL.WinCreateThreadWithName` (`0x2cdfa0b`) has an earlier admission gate: at `0x2cdfa25` it loads host byte `0x544cc70`, then tests bit zero. The branch at `0x2cdfa2d` returns NULL immediately when the bit is clear, without allocating a ThreadHANDLE or calling pthread. The original `ASL.PrepareMainThread` (`0x2cdf8e0`) exchanges this byte to one and invokes original OnCreateThread for its TLS/name/DLL notification work. This identifies a real lifecycle lead; it does not prove that the current fixture failed this gate or authorize writing the byte directly.

If admission passes, ThreadHANDLE calls pthread_attr_init (return at `0x2cdfbd5`) and pthread_attr_setdetachstate with zero (return at `0x2cdfbe5`). Either nonzero return leaves the handle invalid. It requests `max(requested_stack, 1048576)` bytes with pthread_attr_setstacksize; the original ignores that call's return at `0x2cdfc0b`. The actual pthread_create error is available in EAX at `0x2cdfc25`. No affinity operation appears in this audited constructor path. The worker's original option one yields ASL stack request zero and flags `0x10000`; its stack request therefore normalizes to one MiB.

Capture the first failing branch or library return, rather than infer an error from the final NULL. Admission rejection sets no error. A pthread_create failure code is overwritten before returning; it is not propagated into SetLastError. The later GetLastError reads a separate TLS error slot and can be stale. No replacement thread, fabricated return, admission-byte write, process-affinity change, or deadline extension has been made.
