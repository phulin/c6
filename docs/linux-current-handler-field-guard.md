# Current-handler caller field guard

This is a source and synthetic checkpoint for a new passive current-handler
metadata scope. It does not admit an original run. The standalone capture helper
remains unchanged at `c27bf26`; no dispatch or full Manager predicate is invoked.

`native/linux/current_handler_metadata_fixture.c` reads its own process with
`process_vm_readv`. The begin/check/end functions call no original game getter,
predicate, script, handler, lock or action. Unknown or changing state terminates
the isolated process with status 240 before further observations or cleanup.

The future caller must first verify exact module/host mappings and sources,
witness its actual main entry, and acquire an original Game facade under a
separately reviewed acquisition scope. It passes its own main-entry OS TID and
retains all owners. Begin records the actual native controller lock owner token
and depth after acquisition. That token is opaque and is never equated to an OS
TID. A nonzero raw token alone does not establish acquisition or ownership; the
caller must provide that missing witness. End checks again before clearing the
scope, while the caller still owns the facade.

The guard requires the same initialized Controller, Context and Game, no worker
pointer, unchanged lock token/depth, no processor or message-execution work, and
both message-list sentinels empty. For GlobalAi it admits only the signed
processing-counter fast-false branch. A positive counter is unsupported rather
than executing the original Player loop. Requiring both queues empty is stricter
than the original playback-dependent pending-message predicate. No queued entry
or stale owner is dereferenced.

The static packet pins complete original field accessor bodies and the existing
native lock source contracts. Twenty-one synthetic failures exercise changed
worker/context/game/lock/work/queue/thread and unreadable fields; GCC strict
warnings, analyzer and UBSan pass. These fixtures contain no original execution.

Reproduce with:

```sh
python3 tools/linux/current_handler_field_guard_audit.py \
  --output target/linux/current-handler-field-guard-source.json
```

Caller acquisition, main-entry witness, two same-context metadata passes, affected
state and known RNG controls, loader/profile integration and independent exact
preparation admission remain pending. Saves are observational evidence, not a
production output or an action acceptance gate. Full gameplay state, actions and
turns remain open.

The new lifecycle callbacks `admission_begin`, `caller_idle` and `admission_end`
are an explicitly separate unlocked caller scope. They use the same raw identity,
work and queue checks and require owner/depth zero. They do not acquire a lock or
stand in for the three-argument helper callback. The three-argument guard still
requires the caller's retained nonzero native lock witness. Source/profile
admission of the enclosing lifecycle remains pending.
