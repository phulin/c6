#ifndef C6_BOOTSTRAP_JOB_MANAGER_H
#define C6_BOOTSTRAP_JOB_MANAGER_H
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
/* SHA-gated original EXE, preferred base, absent manager. Admits empty native
 * sets only; unsupported objects terminate before original execution/mutation.
 * Does not initialize the manager or admit job scheduling. */
int bootstrap_job_manager_prepare(HMODULE executable);
__declspec(dllexport) void EXP_JobManager_DestroyJobSet(void *set);
#endif
