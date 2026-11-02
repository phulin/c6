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
int bootstrap_job_manager_prepare(HMODULE executable) {
    if (!exe_gate(executable) ||
        (void *)GetProcAddress(executable, "EXP_JobManager_DestroyJobSet") != (unsigned char *)executable + 0x986040 ||
        (void *)GetProcAddress(executable, "EXP_JobManager_Wait") != (unsigned char *)executable + 0x985e20 ||
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
