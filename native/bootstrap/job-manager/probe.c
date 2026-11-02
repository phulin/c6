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
    if (argc == 3 && !wcscmp(argv[2], L"wait-nonempty")) {
        set[1] = 1;
        EXP_JobManager_Wait(set, NULL);
        return 9;
    }
    if (argc == 3 && !wcscmp(argv[2], L"wait-null")) {
        EXP_JobManager_Wait(NULL, NULL);
        return 10;
    }
    if (argc == 3 && !wcscmp(argv[2], L"free-nonzero")) {
        EXP_JobManager_FreeJobList(1);
        return 13;
    }
    EXP_JobManager_FreeJobList(0);
    /* Arbitrary descriptor canaries are only checking the branch's proven
     * non-access; they do not model a scheduling descriptor's semantics. */
    uint64_t descriptor[2] = {UINT64_C(0xfeedfacecafebeef), UINT64_C(0x1234567890abcdef)};
    EXP_JobManager_Wait(set, descriptor);
    if (set[0] || set[1] || descriptor[0] != UINT64_C(0xfeedfacecafebeef) || descriptor[1] != UINT64_C(0x1234567890abcdef)) return 11;
    EXP_JobManager_Wait(set, NULL);
    if (set[0] || set[1]) return 12;
    EXP_JobManager_DestroyJobSet(set);
    if (set[0] || set[1]) return 8;
    puts("{\"phase\":\"native_job_set_probe_complete\",\"manager_initialized\":false,\"job_execution_tested\":false}");
    return 0;
}
