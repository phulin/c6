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
