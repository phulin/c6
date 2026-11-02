/* Exact-image, child-process original XML importer probe. No host startup. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#include "../xml-import/imports.h"
#include "localization.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static HMODULE original;
static const char *phase = "start";
static LONG CALLBACK crash(EXCEPTION_POINTERS *info) {
    CONTEXT *c = info->ContextRecord;
    fprintf(stderr, "{\"phase\":\"%s\",\"exception\":\"0x%08lx\",\"rip\":\"0x%llx\",\"exe_rva\":\"0x%llx\",\"rax\":\"0x%llx\",\"rcx\":\"0x%llx\",\"rdx\":\"0x%llx\",\"stack0\":\"0x%llx\"}\n", phase,
            info->ExceptionRecord->ExceptionCode, (unsigned long long)c->Rip,
            (unsigned long long)(c->Rip - (uintptr_t)original),
            (unsigned long long)c->Rax, (unsigned long long)c->Rcx,
            (unsigned long long)c->Rdx, *(unsigned long long *)(uintptr_t)c->Rsp);
    fflush(stderr);
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
    setvbuf(stdout, NULL, _IONBF, 0); SetUnhandledExceptionFilter(crash);
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    if (argc != 5) return 2;
    wchar_t database_path[32768];
    if (swprintf(database_path, 32768, L"%ls\\DatabaseDLL_FinalRelease.dll", argv[2]) < 0) return 2;
    if (!hash_matches(argv[1], "e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63") ||
        !hash_matches(database_path, "e6577e873f946224df65c02f6c4bc3b385607e2a82d957b0fd613e01cbe78b0c")) return 3;
    SetDllDirectoryW(argv[2]);
    original = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if ((uintptr_t)original != 0x140000000ULL || !bootstrap_bind_original_imports(original)) return 4;
    HMODULE database = GetModuleHandleW(L"DatabaseDLL_FinalRelease.dll");
    typedef unsigned char (*Initialize)(void);
    Initialize initialize = (Initialize)(void *)GetProcAddress(database, "?Initialize@Database@@YA_NXZ");
    if (!initialize || !initialize()) return 5;
    if (!LoadLibraryW(L"Localization_FinalRelease.dll")) return 6;
    phase = "localization_invalid_arguments";
    typedef unsigned char (*LocInit)(const wchar_t *, void *, const char *, unsigned char, void *);
    typedef unsigned char (*DictInit)(void *, const char *, unsigned char);
    HMODULE localization = GetModuleHandleW(L"Localization_FinalRelease.dll");
    LocInit loc_init = (LocInit)(void *)GetProcAddress(localization, "?Initialize@Localization@@YA_NPEB_SPEAVIDatabaseConnection1@Database@@PEBD_NPEAVILocalizationLogger@1@@Z");
    DictInit dict_init = (DictInit)(void *)GetProcAddress(localization, "?InitializeDictionary@Localization@@YA_NPEAVIDatabaseConnection1@Database@@PEBD_N@Z");
    if (!loc_init || !dict_init || loc_init(NULL, NULL, "en_US", 0, NULL) || dict_init(NULL, "en_US", 0) || *(void **)((char *)localization + 0x18c6c0)) return 7;
    puts("{\"phase\":\"original_localization_invalid_arguments\",\"null_icu_rejected\":true,\"null_connection_rejected\":true,\"dictionary_remains_null\":true}");
    phase = "localization";
    return bootstrap_original_localization(original, argv[3], argv[4]);
}
