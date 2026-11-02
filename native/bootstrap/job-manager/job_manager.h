#ifndef C6_BOOTSTRAP_JOB_MANAGER_H
#define C6_BOOTSTRAP_JOB_MANAGER_H
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdint.h>
/* SHA-gated original EXE, preferred base, absent manager. Admits empty native
 * sets only; unsupported objects terminate before original execution/mutation.
 * Does not initialize the manager or admit job scheduling. */
int bootstrap_job_manager_prepare(HMODULE executable);
/* Shared once-only complete original CRT onexit-table initialization. Requires
 * fingerprinted original EXE and bound genuine UCRT imports; never resets tables
 * with prior registrations. Does not execute registered exit callbacks. */
int bootstrap_original_onexit_prepare(HMODULE executable);
__declspec(dllexport) void EXP_JobManager_DestroyJobSet(void *set);
/* RDX points to the DLL wrapper's copied 16-byte descriptor. Its semantics
 * are outside the admitted native empty-set branch, which never reads it. */
__declspec(dllexport) void EXP_JobManager_Wait(void *set, const void *descriptor);
/* Zero handle, or verified empty active original pool handle only. */
__declspec(dllexport) void EXP_JobManager_FreeJobList(uintptr_t handle);
/* Lazily constructs original64-record pool with genuine scoped CRT setup.
 * Requires bound SHA-gated genuine UCRT IAT. Does not construct scheduler. */
__declspec(dllexport) uintptr_t EXP_JobManager_CreateJobList(void);
__declspec(dllexport) void EXP_JobManager_SpawnList(uintptr_t handle);
#endif
