# Original native job-set cleanup boundary

The bootstrap admits three bounded original executable operations: destroying
and waiting on an empty job set, and freeing an actual zero job-list handle,
while the original job-manager singleton is absent. It executes the genuine
`EXP_JobManager_DestroyJobSet`, `EXP_JobManager_Wait`, and
`EXP_JobManager_FreeJobList` exports; it does not replace those operations with
host no-ops. This does not initialize the job manager, schedule
jobs, or establish gameplay or turn equivalence.

The exact executable SHA-256 is
`e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63`.
`bootstrap_job_manager_prepare` verifies the loaded file hash, its preferred
base `0x140000000`, the native export address, and the absent manager at RVA
`0x2d09b00`. No original machine instructions are patched. The helper is in
`native/bootstrap/job-manager/job_manager.[ch]`; host integration calls prepare
before DLL teardown and links the exported adapter.

The original export at RVA `0x986040` tail-jumps to `0x985660`. That function
first tests the original manager singleton. With no manager, it reads the first
job-set pointer, conditionally disposes its native object, and clears both
pointer fields. A set whose two pointer fields are already zero therefore
requires no allocator, imports, scheduler, or manager construction. The adapter
admits exactly that empty branch, records both fields before and after, and
calls the original export. A nonempty set, null/unreadable object, changed manager
state, or unprepared adapter terminates explicitly with status 86 before calling
the original function. Those cases are unsupported, not passing native parity
comparisons. In particular, the native EXE export itself dereferences a null
set; the GameCore wrapper at RVA `0x992190` owns its separate null short circuit.

The original `Wait` export is RVA `0x985e20`. Its Windows x64 ABI receives
set in RCX and the DLL wrapper's copied 16-byte descriptor pointer in RDX.
The DLL wrapper at RVA `0x9922c0` copies the descriptor before forwarding.
The native EXE function reads set field `+8`; when zero, it returns without
reading the descriptor, globals, or any scheduler object. On a nonzero active
pointer it can invoke original routines `0xe3e700` and `0xe3ea70` and clear that
field, so that branch remains unsupported. The adapter admits the same verified
empty native sets as destroy, forwards the original descriptor pointer without
interpreting its bytes, and calls the actual native export. The positive probe
checks both canary and null descriptors, unchanged descriptor bytes, and all
16 bytes of the job set. This establishes only that descriptor non-access on
the empty branch; it does not recover scheduling descriptor semantics.

The original `FreeJobList` export at RVA `0x986050` jumps to `0x9857b0`.
It receives a handle in RCX. For the actual zero handle, it returns before
accessing any pool/global memory. The adapter forwards exactly zero to the
original export, and rejects every nonzero handle before native execution;
it never substitutes zero for the caller's handle. The standalone positive
checks this native zero-handle branch and the negative checks rejection of
handle 1. Actual Load reached this operation with RCX zero after its genuine
empty-set wait returned (`native-xp2-original-load-wait.json`). Valid nonzero
handles still require native pool construction and provenance.

The standalone probe fills 80 bytes of aligned storage with poison, calls the
complete original constructor at RVA `0x985170`, and verifies its original
vtable and empty embedded set at offset `0x38`. It then calls the adapter and
checks all 16 bytes affected by the native operation. Thus the admitted positive
fixture comes from original construction, not an invented empty object. Five
negative cases deliberately pass a nonempty synthetic pointer field and null
to both set adapters, plus a nonzero handle to the list adapter; all must stop
before original execution. The synthetic field is only a rejection
fixture and is never advanced or freed as native state.

Reproduce with an idle shared Wine prefix:

```sh
python3 tools/analysis/job_manager_evidence.py
python3 tools/analysis/job_manager_probe.py
```

The public result is `evidence/bootstrap/job-manager.json`, including fresh
source and executable identities. Raw original instruction ranges remain in
`../c6-ghidra/evidence/jobs`. The positive test does not invoke EXE CRT startup
or bind its imports because this audited branch and constructor do not call
imports. The probe deliberately leaves manager construction and nonempty-set
allocator behavior unclaimed.

The integrated `native-xp2-startup-complete-lifecycle.json` run now records
successful original StartupGame, two native empty-set destruction pairs, later
context factory/query cycles, and final exit 0 after DLL unload. This covers
the actual startup/context teardown boundary with these empty sets. It does
not establish original App/provider teardown or job execution. Later actual
Load required the native empty-set Wait branch; see the runtime owner's
separately fingerprinted Load evidence for its progress and remaining gaps.

Operations on actual work and nonzero handles remain unavailable. Their bounded dependency audit is:

| Export | Native target | Required state beyond admitted empty sets |
| --- | --- | --- |
| `AddJob` (`0x986010`) | `0x985490` | Constructs/activates native group through `0x985d70`, allocates native task, and submits through scheduler routines. |
| `CreateJobList` (`0x986030`) | `0x985550` | Takes synchronized original pool at `0x2d09280`, moves a free record to active list; returns zero if none. |
| `FreeJobList` nonzero handles (`0x986050`) | `0x9857b0` | Checks handle 1–64 and record availability, takes pool lock and moves/clears an active native record. |
| `PushJobOntoList` (`0x986060`) | `0x985b20` | Activates native group, allocates native task, mutates linked list using handle. |
| `SpawnList` (`0x986080`) | `0x985bf0` | Converts handle to pool record, drains task list and can submit or run tasks through original scheduler. |

The uninitialized pool's native zero return is not published as working
scheduling. `SpawnList` and `PushJobOntoList` require native handle provenance;
passing zero or guessed pool entries is unsafe. No `CreateJobSet`/`Run`/`Join`
host export appears in the fingerprinted executable's 26-export inventory;
the seven `EXP_JobManager_*` exports (five above plus Wait/Destroy) are the closure
listed in `evidence/bootstrap/exports.json`. A populated native manager and
allocator/scheduler state must be recovered before admitting actual work.
