# Original native job cleanup and list-pool boundary

The bootstrap executes genuine original job exports for empty job sets and
empty native job lists. It also constructs the original 64-record list pool
and returns genuine native list handles. It does not construct the scheduler,
execute tasks, or establish gameplay or turn equivalence. Unsupported work
terminates explicitly before original execution; no export fabricates success.

The exact executable SHA-256 is
`e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63`.
`bootstrap_job_manager_prepare` verifies the loaded file hash, preferred base
`0x140000000`, original export addresses, and absent manager at RVA `0x2d09b00`.
No original machine instructions are patched. Host integration links
`native/bootstrap/job-manager/job_manager.[ch]` and calls prepare before use.

`EXP_JobManager_DestroyJobSet`, RVA `0x986040`, jumps to `0x985660`.
With no manager and both native pointer fields zero, its complete original
implementation clears both fields and returns without an allocator or scheduler.
The adapter admits exactly that empty branch and records all 16 affected bytes
before and after. Nonempty, null/unreadable objects or changed manager state
terminate with status 86 before native execution. The EXE export itself does
not accept null; GameCore wrapper `0x992190` owns its null short circuit.

`EXP_JobManager_Wait`, RVA `0x985e20`, receives the set in RCX and a pointer to
the DLL wrapper's copied 16-byte descriptor in RDX. The wrapper is RVA `0x9922c0`.
When set field `+8` is zero, native Wait returns without reading the descriptor,
globals, or scheduler. A nonzero pointer can invoke routines `0xe3e700` and
`0xe3ea70` and clear that field; that branch remains unsupported. The adapter
admits the same two-zero-field sets as destroy and forwards both actual arguments
without interpreting descriptor bytes. Canary and null descriptor cases check
only native non-access on the empty branch, not descriptor semantics.

`EXP_JobManager_FreeJobList`, RVA `0x986050`, jumps to `0x9857b0`.
Its actual zero-handle branch returns before pool access. Zero is forwarded
unchanged; the adapter never replaces a caller's nonzero handle with zero.
After original pool construction, it also admits empty active handles with
verified native pool provenance. Native code removes and clears the actual record.

`EXP_JobManager_CreateJobList`, RVA `0x986030`, jumps to `0x985550` and consumes
an original free record under its native lock. It is admitted only after the
complete original pool initializer at RVA `0x241a0` constructs all 64 records,
links the free list, and registers original cleanup through native atexit.
Before construction, the helper verifies the entire pool's pristine loader state;
a partially initialized or previously modified pool is rejected. Every constructed
record and both native list heads are checked before publication. The genuine
export then returns native handles 1–64 and zero only at actual capacity exhaustion.
No uninitialized-pool zero return is published as working list creation.

The pool initializer requires original CRT onexit tables. Shared helper
`bootstrap_original_onexit_prepare(HMODULE)` executes the complete original
scoped initializer at RVA `0xe9469c` with module type 0. It verifies original
mode value 1, both pristine native tables, exact UCRT SHA-256
`2ab2a74bcb5bfd8248d232eb3bc56698fb5173b9ff7fc0daf87d8120d0f448d7`,
and actual IAT targets for `_initialize_onexit_table`, `_register_onexit_function`,
and `_crt_atexit` before any initialization. Successful reuse is once-only and
does not reset tables after earlier native providers register cleanup.
The helper does not run registered exit callbacks or full EXE CRT startup.
Pool creation invokes this helper lazily after genuine imports have been bound.

`EXP_JobManager_SpawnList`, RVA `0x986080`, jumps to `0x985bf0`.
The adapter admits only an empty active record from the original pool. It checks
handle bounds, availability, native active-list membership, null queue head and
self-referential queue tail before delegating to the complete native export.
A nonempty queue is rejected before task execution. Actual scheduling remains
unsupported, including `AddJob` (`0x986010` → `0x985490`) and
`PushJobOntoList` (`0x986060` → `0x985b20`), which activate native groups,
allocate tasks and invoke scheduler services. The fingerprinted EXE's seven
job exports are inventoried in `evidence/bootstrap/exports.json`; no separate
`CreateJobSet`/`Run`/`Join` host export appears there.

The standalone probe poisons 80 aligned bytes, runs complete original ctor
`0x985170`, verifies its native vtable and embedded empty set at offset `0x38`,
and invokes genuine Wait and Destroy. Pool tests create all 64 handles, check
native capacity exhaustion, spawn the genuinely empty lists and free every
handle in reverse order, then compare all record fields and native list heads
with their original constructed state. Negative fixtures reject nonempty/null
sets, unproven nonzero handles and a deliberately populated synthetic queue
before native calls. Those rejection checks are not work-execution parity tests.

Reproduce with an exclusively reserved shared Wine prefix:

```sh
python3 tools/analysis/job_manager_evidence.py
python3 tools/analysis/job_manager_probe.py
```

Public evidence is `evidence/bootstrap/job-manager.json`, including fresh source,
host executable and configuration identities. Original instructions remain private
in `../c6-ghidra/evidence/jobs`. Genuine CRT providers must already be installed
beside the probe as in the main bootstrap; the probe does not recopy shared DLLs.

Integrated `native-xp2-startup-complete-lifecycle.json` records successful native
StartupGame, two empty-set destruction pairs, subsequent context query cycles
and final exit 0 after DLL unload. Actual Load evidence
`native-xp2-original-load-wait.json` and `native-xp2-original-load-free-list.json`
records genuine empty Wait and zero-handle Free returning. Later runtime evidence
owns further initialized-game progress. App/provider teardown, registered EXE
exit cleanup, nonempty task behavior and complete save/turn equivalence remain
unverified.
