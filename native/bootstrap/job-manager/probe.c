#include "job_manager.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>
int wmain(int argc, wchar_t **argv) {
    if (argc < 2 || argc > 3) return 2;
    setvbuf(stdout, NULL, _IONBF, 0);
    HMODULE executable = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if (!bootstrap_job_manager_prepare(executable)) return 3;
    /* Entire original ctor writes these bytes, including its embedded set.
     * Poison first so the observed empty set cannot come from host zero fill. */
    _Alignas(16) unsigned char storage[0x50];
    memset(storage, 0xa5, sizeof storage);
    void *constructed = ((void *(*)(void *))((unsigned char *)executable + 0x985170))(storage);
    if (constructed != storage || *(void **)storage != (unsigned char *)executable + 0x106b350) return 4;
    uintptr_t *set = (void *)(storage + 0x38);
    if (set[0] || set[1]) return 5;
    puts("{\"phase\":\"native_job_set_constructed\",\"constructor_rva\":\"0x985170\",\"embedded_set_offset\":56,\"empty\":true,\"poison_overwritten\":true}");
    if (argc == 3 && !wcscmp(argv[2], L"nonempty")) {
        /* Negative only: deliberately inadmissible pointer, never native data. */
        set[0] = 1;
        EXP_JobManager_DestroyJobSet(set);
        return 6;
    }
    if (argc == 3 && !wcscmp(argv[2], L"null")) {
        EXP_JobManager_DestroyJobSet(NULL);
        return 7;
    }
    EXP_JobManager_DestroyJobSet(set);
    if (set[0] || set[1]) return 8;
    puts("{\"phase\":\"native_job_set_probe_complete\",\"manager_initialized\":false,\"job_execution_tested\":false}");
    return 0;
}
