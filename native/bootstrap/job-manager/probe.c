#include "job_manager.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "../xml-import/imports.h"
int wmain(int argc, wchar_t **argv) {
    if (argc < 2 || argc > 3) return 2;
    setvbuf(stdout, NULL, _IONBF, 0);
    HMODULE executable = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if (!bootstrap_job_manager_prepare(executable)) return 3;
    if (argc == 3 && (!wcscmp(argv[2], L"pool") || !wcscmp(argv[2], L"spawn-nonempty"))) {
        wchar_t directory[32768];
        if (wcslen(argv[1]) >= 32768) return 14;
        wcscpy(directory, argv[1]);
        wchar_t *slash = wcsrchr(directory, L'\\');
        if (!slash) return 15;
        *slash = 0;
        if (!SetDllDirectoryW(directory) || !bootstrap_bind_original_imports(executable)) return 16;
        uintptr_t handles[64];
        for (unsigned i = 0; i < 64; ++i) {
            handles[i] = EXP_JobManager_CreateJobList();
            if (handles[i] != i + 1) return 17;
        }
        if (EXP_JobManager_CreateJobList()) return 18;
        if (!bootstrap_original_onexit_prepare(executable)) return 22;
        if (!wcscmp(argv[2], L"spawn-nonempty")) {
            *(uintptr_t *)((unsigned char *)executable + 0x2d092d0) = 1;
            EXP_JobManager_SpawnList(handles[0]);
            return 19;
        }
        for (unsigned i = 64; i > 0; --i) {
            EXP_JobManager_SpawnList(handles[i-1]);
            EXP_JobManager_FreeJobList(handles[i-1]);
        }
        if (*(void **)((unsigned char *)executable + 0x2d092c0) ||
            *(void **)((unsigned char *)executable + 0x2d092c8) != (unsigned char *)executable + 0x2d092d0) return 20;
        for (unsigned i = 0; i < 64; ++i) {
            unsigned char *record = (unsigned char *)executable + 0x2d092d0 + 32*i;
            if (*(void **)record || *(void **)(record+8) != record || record[24] ||
                *(void **)(record+16) != (i < 63 ? record+32 : NULL)) return 21;
        }
        puts("{\"phase\":\"original_pool_capacity_and_cleanup_passed\",\"native_handles\":64,\"capacity_exhaustion_returned_zero\":true,\"all_records_restored\":true,\"job_execution_tested\":false}");
        return 0;
    }
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
