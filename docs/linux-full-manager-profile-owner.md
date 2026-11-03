# Full Manager caller ownership preparation

This new field-only scope supports the separately reviewed full Manager predicate
caller. It has not been admitted for original execution. It does not share the
passive current-handler guard's scope or require pre/post gameplay equality.

The lifecycle boundaries require the same initialized Controller, exported/native
Context and Game, no worker, a genuine own-main-entry OS TID and zero native lock
owner/depth. The caller then acquires its original Game facade. Locked begin
records the actual opaque native owner token and depth one. Endpoint guards
require that witness to match until the caller releases the facade; released
owner requires zero owner/depth again. The token is not converted to an OS TID.
Acquisition origin still requires the reviewed original caller and exact profile.

These callbacks call no original getters, locks, Lua, policies or predicates.
They do not retain/dereference an actor and permit its legitimate removal or
replacement. Pending messages, processor work flags and AI counters are neither
cleared nor forced back to pre-query values. The original lock, no worker and
source-closed top-level caller supply the competing-mutator boundary; actual
caller ancestry and Lua/GC safety require the separate hardware observer. An
idle field is not a decision boundary or an empty/safe script backend.

A separate observational trace end retains actual queue/receive/execute/report
counters without zero/equality expectations or draining. Legacy trace end ABI
and source stay unchanged. Structural and IO failures remain failures. Existing
known policy/update hard-stop hooks remain unavailable-run causes, not accepted
replacement behavior or evidence that all policies were observed.

Reproduce source/synthetic checks without original execution:

```sh
python3 tools/linux/full_manager_profile_guard_audit.py \
  --output target/linux/full-manager-profile-owner-source.json
python3 tools/linux/full_manager_trace_policy_audit.py \
  --output target/linux/full-manager-trace-policy.json
```

The owner tests cover an opaque-token lock cycle, protected actor memory,
unchanged queued effects, ten targeted fatal refusals and strict C/analyzer/UBSan.
The trace tests preserve nonzero/asymmetric counters and legacy behavior while
retaining structural/IO failures. These are mocks and source checks, not native
lock ownership or gameplay acceptance. Loader, callback observation schema,
compiled caller/marker and actual frame/VM/profile admission remain pending.
