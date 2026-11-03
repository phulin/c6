# Passive current-handler caller preparation

This source-only caller wraps the standalone passive helper in a genuine Game
facade lifetime. It has not been connected to a loader mode or admitted for an
original run. The helper and its field-only guard remain a separate scope with
zero original calls. The caller's original facade acquisition, four observational
Saves and release are explicitly outside that scope.

The caller requires an OS main-thread ID recorded at our intercepted original
main entry under `C6_CURRENT_HANDLER_METADATA`. It checks that ID before acquiring
anything. It requires no worker, an unlocked Controller bound to this Context,
and initialized native Context/Game fields. It reads the actual EngineUtility GOT
binding and object, concrete vptr and lock virtual targets before invoking the
pinned original Context.GetGame factory. The created facade's native Game,
vptr and refcount must match. The field guard then records and retains the
original native lock token and depth while the facade remains alive.

Two independent helper calls receive distinct exclusive output paths and the
same retained Game. Before/control Saves precede them; after/repeat Saves follow.
Every boundary checks the passive field guard. The caller records both known
32-byte RNG objects before and after. Any unknown state exits before additional
observations or native teardown. A normal helper/Save error can return only after
quiescence is re-proven. The caller checks native ownership again before releasing
the created facade and requires zero lock depth/owner afterwards. No borrowed
native Game, Unit, Player or Manager is released.

Environment names use `C6_CURRENT_HANDLER_METADATA_`: `OUTPUT`, `REPEAT_OUTPUT`,
`CONTROL_OUTPUT`, `SNAPSHOT_BEFORE`, `SNAPSHOT_CONTROL`, `SNAPSHOT_AFTER`,
`SNAPSHOT_REPEAT`, and `PROFILE_SHA256`. The future profile must admit distinct
fresh private paths, exact module/host/source/dependency/rules identities, the
original lock acquisition chain, loader initialization lifetime, two outer state
observations, and actual terminal cleanup. No current-handler/full Manager
predicate, Lua callback, action or turn is called by this source checkpoint.

Reproduce static/compiler and six pre-original-call refusal tests with:

```sh
python3 tools/linux/current_handler_caller_audit.py \
  --output target/linux/current-handler-caller-source.json
```

The static packet pins complete factory, facade constructor/destructor, Release
and RNG accessor bodies. Independent source/lifetime and exact integrated-profile
review are still required. Save output is an observational oracle, not a required
simulator format or a condition for action support. Known RNG checks cover these
two objects only. Complete affected gameplay state and RNG remain in scope.
