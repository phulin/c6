/* Only the native empty job-set boundary is admitted. See docs/bootstrap-job-manager.md. */
#include "job_manager.h"
#include <stdint.h>
#include <bcrypt.h>
#include <stdio.h>
#include <string.h>
static int module_gate(HMODULE module, const char *expected) {
    wchar_t path[32768]; unsigned char block[65536], sum[32];
    BCRYPT_ALG_HANDLE alg = NULL; BCRYPT_HASH_HANDLE hash = NULL;
    HANDLE file = INVALID_HANDLE_VALUE; DWORD count; char hex[65]; int ok = 0;
    if (!module || !GetModuleFileNameW(module, path, 32768) ||
        BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA256_ALGORITHM, NULL, 0) < 0 ||
        BCryptCreateHash(alg, &hash, NULL, 0, NULL, 0, 0) < 0) goto done;
    file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL);
    if (file == INVALID_HANDLE_VALUE) goto done;
    for (;;) {
        if (!ReadFile(file, block, sizeof block, &count, NULL)) goto done;
        if (!count) break;
        if (BCryptHashData(hash, block, count, 0) < 0) goto done;
    }
    if (BCryptFinishHash(hash, sum, sizeof sum, 0) < 0) goto done;
    for (unsigned i = 0; i < 32; ++i) sprintf(hex + i * 2, "%02x", sum[i]);
    ok = !strcmp(hex, expected);
done:
    if (file != INVALID_HANDLE_VALUE) CloseHandle(file);
    if (hash) BCryptDestroyHash(hash);
    if (alg) BCryptCloseAlgorithmProvider(alg, 0);
    return ok;
}
static int exe_gate(HMODULE executable) {
    return (uintptr_t)executable == UINT64_C(0x140000000) && module_gate(executable,
        "e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63");
}

static HMODULE original;
static int pool_ready;
static HMODULE onexit_original;
static int empty_active_handle(uintptr_t handle);
int bootstrap_job_manager_prepare(HMODULE executable) {
    if (!exe_gate(executable) ||
        (void *)GetProcAddress(executable, "EXP_JobManager_DestroyJobSet") != (unsigned char *)executable + 0x986040 ||
        (void *)GetProcAddress(executable, "EXP_JobManager_Wait") != (unsigned char *)executable + 0x985e20 ||
        (void *)GetProcAddress(executable, "EXP_JobManager_FreeJobList") != (unsigned char *)executable + 0x986050 ||
        (void *)GetProcAddress(executable, "EXP_JobManager_CreateJobList") != (unsigned char *)executable + 0x986030 ||
        (void *)GetProcAddress(executable, "EXP_JobManager_SpawnList") != (unsigned char *)executable + 0x986080 ||
        *(void **)((unsigned char *)executable + 0x2d09b00)) return 0;
    original = executable;
    puts("{\"phase\":\"original_job_manager_empty_boundary_prepared\",\"manager_initialized\":false,\"original_destroy_export_rva\":\"0x986040\"}");
    return 1;
}
static int readable_set(void *set) {
    MEMORY_BASIC_INFORMATION region;
    uintptr_t pointer = (uintptr_t)set;
    return set && VirtualQuery(set, &region, sizeof region) && region.State == MEM_COMMIT &&
        !(region.Protect & (PAGE_GUARD | PAGE_NOACCESS)) &&
        region.RegionSize >= 16 && pointer >= (uintptr_t)region.BaseAddress &&
        pointer - (uintptr_t)region.BaseAddress <= region.RegionSize - 16;
}
__declspec(dllexport) void EXP_JobManager_DestroyJobSet(void *set) {
    /* Native EXE DestroyJobSet does not accept NULL. The DLL wrapper owns its
     * own NULL short circuit; do not silently change this export's ABI. */
    if (!original || !readable_set(set) ||
        *(void **)((unsigned char *)original + 0x2d09b00) ||
        ((uintptr_t *)set)[0] || ((uintptr_t *)set)[1]) {
        fputs("{\"phase\":\"unsupported_job_set_boundary\",\"original_destroy_executed\":false}\n", stderr);
        ExitProcess(86);
    }
    printf("{\"phase\":\"original_destroy_empty_job_set_before\",\"set\":\"%p\",\"first\":0,\"second\":0}\n", set);
    ((void (*)(void *))((unsigned char *)original + 0x986040))(set);
    printf("{\"phase\":\"original_destroy_empty_job_set_after\",\"first\":%llu,\"second\":%llu,\"original_code_executed\":true}\n", (unsigned long long)((uintptr_t *)set)[0], (unsigned long long)((uintptr_t *)set)[1]);
}

__declspec(dllexport) void EXP_JobManager_Wait(void *set, const void *descriptor) {
    /* Native EXE 985e20 uses only set[1] on this admitted branch. RDX is
     * forwarded for ABI fidelity; neither adapter nor native branch reads it. */
    if (!original || !readable_set(set) ||
        *(void **)((unsigned char *)original + 0x2d09b00) ||
        ((uintptr_t *)set)[0] || ((uintptr_t *)set)[1]) {
        fputs("{\"phase\":\"unsupported_job_set_wait_boundary\",\"original_wait_executed\":false}\n", stderr);
        ExitProcess(86);
    }
    printf("{\"phase\":\"original_wait_empty_job_set_before\",\"set\":\"%p\",\"first\":0,\"second\":0,\"descriptor_read\":false}\n", set);
    ((void (*)(void *, const void *))((unsigned char *)original + 0x985e20))(set, descriptor);
    printf("{\"phase\":\"original_wait_empty_job_set_after\",\"first\":%llu,\"second\":%llu,\"original_code_executed\":true}\n", (unsigned long long)((uintptr_t *)set)[0], (unsigned long long)((uintptr_t *)set)[1]);
}

__declspec(dllexport) void EXP_JobManager_FreeJobList(uintptr_t handle) {
    /* Native EXE 9857b0 returns for zero before pool access. Nonzero handles
     * require an actually constructed native pool and empty active record. */
    if (!original || (handle && !empty_active_handle(handle)) || *(void **)((unsigned char *)original + 0x2d09b00)) {
        fputs("{\"phase\":\"unsupported_job_list_free_boundary\",\"original_free_executed\":false}\n", stderr);
        ExitProcess(86);
    }
    printf("{\"phase\":\"original_free_job_list_before\",\"handle\":%llu}\n", (unsigned long long)handle);
    ((void (*)(uintptr_t))((unsigned char *)original + 0x986050))(handle);
    puts("{\"phase\":\"original_free_job_list_after\",\"original_code_executed\":true}");
}

static int all_zero(const unsigned char *bytes, size_t count) {
    for (size_t i = 0; i < count; ++i) if (bytes[i]) return 0;
    return 1;
}
int bootstrap_original_onexit_prepare(HMODULE executable) {
    if (!exe_gate(executable)) return 0;
    if (onexit_original) return executable == onexit_original;
    unsigned char *base = (void *)executable;
    HMODULE crt = GetModuleHandleW(L"ucrtbase.dll");
    if (!module_gate(crt, "2ab2a74bcb5bfd8248d232eb3bc56698fb5173b9ff7fc0daf87d8120d0f448d7") ||
        *(uint32_t *)(base + 0x12b4840) != 1 || !all_zero(base + 0x357f4a0, 48)) return 0;
    /* Entire original scoped CRT onexit initializer. Verify genuine imported
     * providers before it mutates native globals. Shared by original static
     * providers; never reset these tables after their first registration. */
    static const struct { size_t rva; const char *name; } imports[] = {
        {0xf97798, "_initialize_onexit_table"},
        {0xf977b8, "_register_onexit_function"},
        {0xf977c8, "_crt_atexit"}
    };
    for (size_t i = 0; i < sizeof imports / sizeof imports[0]; ++i)
        if (*(void **)(base + imports[i].rva) != (void *)GetProcAddress(crt, imports[i].name)) return 0;
    if (!((unsigned char (*)(unsigned))(base + 0xe9469c))(0)) return 0;
    onexit_original = executable;
    puts("{\"phase\":\"original_onexit_tables_initialized\",\"native_initializer_rva\":\"0xe9469c\",\"module_type\":0}");
    return 1;
}
static int pool_prepare(void) {
    unsigned char *base = (void *)original;
    if (!original || *(void **)(base + 0x2d09b00) ||
        !all_zero(base + 0x2d09280, 0x880) || !bootstrap_original_onexit_prepare(original)) return 0;
    if (((int (*)(void))(base + 0x241a0))() != 0) return 0;
    if (*(uint32_t *)(base + 0x2d09280) || *(void **)(base + 0x2d092c0) ||
        *(void **)(base + 0x2d092c8) != base + 0x2d092d0) return 0;
    for (size_t i = 0; i < 64; ++i) {
        unsigned char *record = base + 0x2d092d0 + 32 * i;
        if (*(void **)record || *(void **)(record + 8) != record ||
            *(void **)(record + 16) != (i < 63 ? record + 32 : NULL) || record[24]) return 0;
    }
    pool_ready = 1;
    puts("{\"phase\":\"original_job_list_pool_constructed\",\"native_initializer_rva\":\"0x241a0\",\"records\":64,\"all_records_verified\":true,\"native_atexit_registered\":true,\"manager_initialized\":false}");
    return 1;
}
static int empty_active_handle(uintptr_t handle) {
    unsigned char *base = (void *)original;
    if (!pool_ready || handle < 1 || handle > 64) return 0;
    unsigned char *record = base + 0x2d092d0 + 32 * (handle - 1);
    if (*(void **)record || *(void **)(record + 8) != record || !record[24]) return 0;
    unsigned char *node = *(void **)(base + 0x2d092c0);
    for (unsigned i = 0; node && i < 64; ++i) {
        uintptr_t offset = (uintptr_t)node - (uintptr_t)(base + 0x2d092d0);
        if (offset >= 64 * 32 || offset % 32) return 0;
        if (node == record) return 1;
        node = *(void **)(node + 16);
    }
    return 0;
}
__declspec(dllexport) uintptr_t EXP_JobManager_CreateJobList(void) {
    if (!original || *(void **)((unsigned char *)original + 0x2d09b00) ||
        (!pool_ready && !pool_prepare())) {
        fputs("{\"phase\":\"unsupported_job_list_pool_boundary\",\"original_create_executed\":false}\n", stderr);
        ExitProcess(86);
    }
    uintptr_t handle = ((uintptr_t (*)(void))((unsigned char *)original + 0x986030))();
    if (handle && !empty_active_handle(handle)) ExitProcess(86);
    printf("{\"phase\":\"original_create_job_list_returned\",\"handle\":%llu,\"original_code_executed\":true}\n", (unsigned long long)handle);
    return handle;
}
__declspec(dllexport) void EXP_JobManager_SpawnList(uintptr_t handle) {
    if (!original || *(void **)((unsigned char *)original + 0x2d09b00) || !empty_active_handle(handle)) {
        fputs("{\"phase\":\"unsupported_job_list_spawn_boundary\",\"original_spawn_executed\":false}\n", stderr);
        ExitProcess(86);
    }
    ((void (*)(uintptr_t))((unsigned char *)original + 0x986080))(handle);
    printf("{\"phase\":\"original_spawn_empty_job_list_returned\",\"handle\":%llu,\"original_code_executed\":true}\n", (unsigned long long)handle);
}
