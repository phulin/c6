# Original native job-set cleanup boundary

The bootstrap admits one bounded original executable operation: destroying an
empty job set while the original job-manager singleton is absent. It executes
the genuine `EXP_JobManager_DestroyJobSet` export; it does not replace that
operation with a host no-op. This does not initialize the job manager, schedule
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

The standalone probe fills 80 bytes of aligned storage with poison, calls the
complete original constructor at RVA `0x985170`, and verifies its original
vtable and empty embedded set at offset `0x38`. It then calls the adapter and
checks all 16 bytes affected by the native operation. Thus the admitted positive
fixture comes from original construction, not an invented empty object. Two
negative cases deliberately pass a nonempty synthetic pointer field and null;
both must stop before original execution. The synthetic field is only a rejection
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

Other requested job hooks remain unavailable. `CreateJobList` (`0x986030`)
uses a native synchronized pool and may legitimately return zero when the
uninitialized pool has no available records; publishing that as working
scheduling would conceal missing initialization. `FreeJobList` (`0x986050`)
checks handle bounds and record availability, and `Wait` (`0x985e20`) uses
additional native job-state services for a nonempty active pointer. Their
addresses are evidence leads only; no host success implementations are supplied.
