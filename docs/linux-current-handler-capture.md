# Actual current-handler metadata

The corrected field-only native fixture completed with status 0 under profile
`321c39581fd268b9e75d2efaa2a672e8662a3468573bc1cad8bfbec8e15e7045`.
Two complete raw passes are byte-identical: 65 Player query slots, 43 Players,
1,262 stored Units and 63 registered handler slots. All Units are retained,
including unplaced and delayed-death objects. There are 308 nonempty queues and
954 empty queues.

The capture retains actual Manager virtual slots and four complete method bodies,
63 actual handler virtual slot/code records, raw headers and complete Unit queue
capacity blocks. Code bytes independently match the pinned original Linux file.
The numeric front-to-slot lookup is derived from source and actual raw inputs.
No native `GetHandler` return, `CanInterrupt` result, full Manager predicate,
script callback or action result was observed by this helper.

The original-call-free helper and guard execute within a caller-held genuine
facade lifetime. Four internal and two outer Save observations are each exactly
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`
(80,657,462 bytes), with zero masks. The two known 32-byte RNG objects match at
the caller's before/after endpoints. These observations do not establish complete
state/RNG equivalence. Saves are an observational oracle, not a production
format or an action gate; uninterrupted lock ownership is not claimed inside
transitive original Save calls.

The frozen collector executed the 26-byte anchor/mapped-file and native
lock/controller/thread/work checks. Their complete raw maps and lock/token dumps
were not retained; they are source-bound execution witnesses, distinct from the
67 retained Manager/handler slot/code records. Native Uninitialize reports
initialized=0, Context unbound, executing=0 and cleanup=1. Framework and client
cleanup completed. The subprocess returned normally and a subsequent executable
scan found no Civ6/GDB process. The exact host starttime was not retained, so no
retrospective PID-generation proof is claimed.

Reproduce the closed artifact audit with:

```sh
python3 tools/linux/current_handler_capture_audit.py \
  --output target/linux/current-handler-capture.json
```

It verifies both raw passes, complete roster/queue framing, selected slot
identities, retained method bytes, archived source/preload/configuration,
observational Saves, known RNG and terminal controls. Thirteen targeted malformed
observation cases are refused. The audit executes no original code.

The earlier `04ff` preparation remains unexecuted. The `9450` fixture ended with
242 before facts; its three Save observations and successful teardown remain
preserved. `O_NOFOLLOW` rejecting the actual launch symlink was reproduced as a
necessary blocker, not recorded as the failed run's exact errno or last
instruction. The corrected helper validates canonical-file identity against the
actual mappings and retains named pre-output failures.

This result closes bounded actual dispatch/front inputs. Full Manager and
SkipTurn action execution, script effects, exact complete gameplay state/RNG and
turns remain open. A separate full Manager caller and passive Lua observer need
new source/lifetime/profile admission before execution.
