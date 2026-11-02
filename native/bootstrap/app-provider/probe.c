#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "app_provider.h"
#include "../script-system/script_system.h"
#include "../xml-import/imports.h"
static HMODULE original;
static const char *phase = "start";
static LONG CALLBACK crash(EXCEPTION_POINTERS *info) {
    CONTEXT *c = info->ContextRecord;
    fprintf(stderr, "{\"phase\":\"%s\",\"exception\":\"0x%08lx\",\"rip\":\"0x%llx\",\"exe_rva\":\"0x%llx\",\"rcx\":\"0x%llx\"}\n", phase, info->ExceptionRecord->ExceptionCode, (unsigned long long)c->Rip, (unsigned long long)(c->Rip-(uintptr_t)original), (unsigned long long)c->Rcx);
    for (unsigned i = 0; i < 48; ++i) { uint64_t value = ((uint64_t *)(uintptr_t)c->Rsp)[i]; if (value >= (uintptr_t)original && value < (uintptr_t)original + 0x3600000) fprintf(stderr, "stack[%u]=exe+0x%llx\n", i, (unsigned long long)(value-(uintptr_t)original)); }
    return EXCEPTION_EXECUTE_HANDLER;
}
static int hash_matches(const wchar_t *path, const char *expected) {
    BCRYPT_ALG_HANDLE alg = NULL; BCRYPT_HASH_HANDLE hash = NULL;
    HANDLE file = INVALID_HANDLE_VALUE;
    unsigned char digest[32], buffer[65536]; char hex[65]; DWORD count; int ok = 0;
    if (BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA256_ALGORITHM, NULL, 0) < 0 ||
        BCryptCreateHash(alg, &hash, NULL, 0, NULL, 0, 0) < 0) goto done;
    file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) goto done;
    for (;;) {
        if (!ReadFile(file, buffer, sizeof buffer, &count, NULL)) goto done;
        if (!count) break;
        if (BCryptHashData(hash, buffer, count, 0) < 0) goto done;
    }
    if (BCryptFinishHash(hash, digest, sizeof digest, 0) < 0) goto done;
    for (unsigned i = 0; i < 32; ++i) sprintf(hex + 2*i, "%02x", digest[i]);
    ok = strcmp(hex, expected) == 0;
done:
    if (file != INVALID_HANDLE_VALUE) CloseHandle(file);
    if (hash) BCryptDestroyHash(hash);
    if (alg) BCryptCloseAlgorithmProvider(alg, 0);
    return ok;
}
int wmain(int argc, wchar_t **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    SetUnhandledExceptionFilter(crash);
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    if (argc != 4) return 2;
    if (!hash_matches(argv[1], "e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63")) return 3;
    if (!SetDllDirectoryW(argv[2])) return 4;
    original = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if ((uintptr_t)original != 0x140000000ULL) return 5;
    if (!bootstrap_bind_original_imports(original)) return 6;
    HMODULE database = LoadLibraryW(L"DatabaseDLL_FinalRelease.dll");
    unsigned char (*initialize)(void) = (void *)GetProcAddress(database, "?Initialize@Database@@YA_NXZ");
    if (!initialize || !initialize()) return 9;
    phase = "restore_original_native_rules_snapshot";
    char snapshot[32768];
    if (!WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, argv[3], -1, snapshot, sizeof snapshot, NULL, NULL)) return 10;
    void *(*create)(const char *, int, void *) = (void *)GetProcAddress(database, "?CreateConnection@Database@@YAPEAVIDatabaseConnection1@1@PEBDHPEAVIDatabaseLogger@1@@Z");
    if (!create) return 11;
    void *disk = create(snapshot, 1, NULL);
    void *expected = create("file:gameplay-0?mode=memory&cache=shared", 0x46, NULL);
    if (!disk || !expected || !((unsigned char (*)(void *, void *))(*(uintptr_t **)expected)[24])(expected, disk)) return 12;
    puts("{\"phase\":\"native_rules_snapshot_restored\",\"native_copy_from\":true}");
    phase = "construct_original_app";
    void *app = bootstrap_original_app_construct(original);
    if (!app) return 7;
    phase = "open_original_gameplay_wrapper";
    if (!bootstrap_original_app_gameplay_database(original, app, expected)) return 8;
    phase = "construct_original_script_system";
    void *system = bootstrap_original_script_system(original);
    if (!system) return 13;
    phase = "create_original_script_state_with_app";
    void *state = ((void *(*)(void *, const char *, unsigned char))(*(uintptr_t **)system)[0])(system, "GameCore_Tuner", 1);
    if (!state) return 14;
    puts("{\"phase\":\"original_script_state_with_app_created\",\"nonnull\":true}");
    phase = "remove_original_script_state_with_app";
    ((void (*)(void *, void *))(*(uintptr_t **)system)[1])(system, state);
    puts("{\"phase\":\"original_app_probe_complete\",\"process_exit_cleanup_only\":true}");
    return 0;
}
