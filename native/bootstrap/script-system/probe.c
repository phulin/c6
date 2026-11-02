#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "script_system.h"
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
    if (argc != 3 && argc != 4) return 2;
    wchar_t path[32768];
    if (!hash_matches(argv[1], "e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63")) return 3;
    if (swprintf(path, 32768, L"%ls\\HavokScript_FinalRelease.dll", argv[2]) < 0 || !hash_matches(path, "9fb4e34afa6666aafb519d6edb9c1e4e9add9cce2cecaa3b1e5bb8f08b168532")) return 3;
    if (!SetDllDirectoryW(argv[2])) return 4;
    original = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if ((uintptr_t)original != 0x140000000ULL) return 5;
    if (!bootstrap_bind_original_imports(original)) return 6;
    phase = "construct_original_script_system";
    void *system = bootstrap_original_script_system(original);
    if (!system) return 7;
    if (argc == 4 && !wcscmp(argv[3], L"--lifecycle")) {
        unsigned char *base = (void *)original;
        void *manager = *(void **)(base + 0x2d08f88);
        phase = "create_original_lua_thread";
        void *thread = ((void *(*)(void *))(*(uintptr_t **)manager)[1])(manager);
        if (!thread || thread == ((void **)manager)[2]) return 10;
        HMODULE havok = GetModuleHandleW(L"HavokScript_FinalRelease.dll");
        int (*gettop)(void *) = (void *)GetProcAddress(havok, "?lua_gettop@@YAHPEAUlua_State@@@Z");
        void (*push)(void *, int) = (void *)GetProcAddress(havok, "?lua_pushinteger@@YAXPEAUlua_State@@H@Z");
        int (*integer)(void *, int) = (void *)GetProcAddress(havok, "?lua_tointeger@@YAHPEAUlua_State@@H@Z");
        void (*settop)(void *, int) = (void *)GetProcAddress(havok, "?lua_settop@@YAXPEAUlua_State@@H@Z");
        if (!gettop || !push || !integer || !settop || gettop(thread) != 0) return 11;
        push(thread, 17431);
        if (gettop(thread) != 1 || integer(thread, -1) != 17431) return 12;
        settop(thread, 0);
        if (gettop(thread) != 0) return 13;
        puts("{\"phase\":\"original_lua_thread_validated\",\"distinct_from_main_state\":true,\"native_integer_roundtrip\":17431}");
        phase = "release_original_lua_thread";
        ((void (*)(void *, void *))(*(uintptr_t **)manager)[3])(manager, thread);
        puts("{\"phase\":\"original_lua_thread_release_returned\"}");
        phase = "destroy_original_script_system";
        ((void (*)(void *, unsigned))(base + 0x161810))(system, 1);
        phase = "destroy_original_lua_manager";
        ((void (*)(void *, unsigned))(*(uintptr_t **)manager)[0])(manager, 1);
        phase = "destroy_original_lua_lock";
        ((void (*)(void *))(base + 0xf607e0))(base + 0x3552fa0);
        puts("{\"phase\":\"original_native_cleanup_returned\",\"process_lifetime_only\":true}");
        return 0;
    }
    phase = "create_original_script_state";
    void *state = ((void *(*)(void *, const char *, unsigned char))(*(uintptr_t **)system)[0])(system, "GameCore_Tuner", 1);
    if (!state) return 8;
    puts("{\"phase\":\"original_script_state_created\",\"lua_state_nonnull\":true}");
    phase = "remove_original_script_state";
    ((void (*)(void *, void *))(*(uintptr_t **)system)[1])(system, state);
    if (((uint64_t *)system)[3] != 0) return 9;
    puts("{\"phase\":\"original_script_state_removed\",\"state_count\":0}");
    return 0;
}
