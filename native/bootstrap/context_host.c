/* Real Windows loader boundary; never maps or patches proprietary image bytes. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#include <stdint.h>
#include <stdlib.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include "services/service_fixture.h"
#include "save/save_probe.h"
#include "xml-import/content.h"
#include "localization/localization.h"
#include "database-backup/backup.h"

static const char expected_dll[] = "324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c";
static HMODULE image;
static HMODULE original_executable;
__declspec(dllexport) void *EXP_GetEmptyBasicString(void) {
    typedef void *(*Getter)(void);
    Getter getter = (Getter)(void *)GetProcAddress(original_executable, "EXP_GetEmptyBasicString");
    if (!getter) ExitProcess(87);
    return getter();
}
static const char *phase = "start";
static LONG CALLBACK crashed(EXCEPTION_POINTERS *info) {
    uintptr_t address = (uintptr_t)info->ExceptionRecord->ExceptionAddress;
    HMODULE fault_module = NULL;
    wchar_t module_path[32768] = L"unknown";
    GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                      (const wchar_t *)address, &fault_module);
    if (fault_module) GetModuleFileNameW(fault_module, module_path, 32768);
    const wchar_t *module_name = wcsrchr(module_path, L'\\');
    module_name = module_name ? module_name + 1 : module_path;
    CONTEXT *context = info->ContextRecord;
    fprintf(stderr, "{\"phase\":\"%s\",\"exception\":\"0x%08lx\",\"fault_module\":\"%ls\",\"module_rva\":\"0x%llx\",\"rax\":\"0x%llx\",\"rcx\":\"0x%llx\",\"rdx\":\"0x%llx\",\"r8\":\"0x%llx\",\"r9\":\"0x%llx\"}\n",
            phase, info->ExceptionRecord->ExceptionCode, module_name,
            (unsigned long long)(fault_module ? address - (uintptr_t)fault_module : address),
            (unsigned long long)context->Rax, (unsigned long long)context->Rcx, (unsigned long long)context->Rdx,
            (unsigned long long)context->R8, (unsigned long long)context->R9);
    MEMORY_BASIC_INFORMATION region;
    uintptr_t stack = (uintptr_t)context->Rsp;
    if (VirtualQuery((void *)stack, &region, sizeof region) && region.State == MEM_COMMIT &&
        !(region.Protect & PAGE_GUARD) && (region.Protect & (PAGE_READWRITE | PAGE_READONLY)) &&
        stack <= (uintptr_t)region.BaseAddress + region.RegionSize - 8 * sizeof(uint64_t)) {
        const uint64_t *words = (const uint64_t *)stack;
        fputs("{\"phase\":\"fault_stack_words\",\"words\":[", stderr);
        for (unsigned i = 0; i < 8; ++i) fprintf(stderr, "%s\"0x%llx\"", i ? "," : "", (unsigned long long)words[i]);
        fputs("]}\n", stderr);
    }
    fflush(stderr);
    return EXCEPTION_EXECUTE_HANDLER;
}
static int hash_matches(const wchar_t *path, const char *expected) {
    BCRYPT_ALG_HANDLE alg = NULL;
    BCRYPT_HASH_HANDLE hash = NULL;
    HANDLE file = INVALID_HANDLE_VALUE;
    unsigned char digest[32], buffer[65536];
    char hex[65]; DWORD count; int ok = 0;
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
/* x64 ABI: factory consumes no args; destroy receives pointer in RCX.
 * Query slot 0 compares a pointer to a 16-byte GUID in RDX. */
typedef void *(*CreateContext)(void);
typedef void (*DestroyContext)(void *);
typedef void *(*QueryInterface)(void *, const unsigned char *);
static int sentinel_is_original(void) {
    unsigned char *sentinel = EXP_GetEmptyBasicString();
    return sentinel == (unsigned char *)original_executable + 0x2d227d8 &&
           sentinel == EXP_GetEmptyBasicString() &&
           *(uint32_t *)(sentinel - 8) == 0 && *(uint32_t *)(sentinel - 4) == 0 && sentinel[0] == 0;
}
static int execute_schema_manifest(const wchar_t *path, int strip_bom) {
    FILE *manifest = _wfopen(path, L"rb");
    char line[32768];
    if (!manifest) return 0;
    unsigned index = 0;
    while (fgets(line, sizeof line, manifest)) {
        wchar_t source_path[32768];
        if ((line[0] != '0' && line[0] != '1') || line[1] != '\t') return 0;
        line[strcspn(line, "\r\n")] = 0;
        if (!MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, line + 2, -1, source_path, 32768)) return 0;
        FILE *source = _wfopen(source_path, L"rb");
        if (!source || fseek(source, 0, SEEK_END)) return 0;
        long length = ftell(source);
        if (length < 0 || length > INT_MAX || fseek(source, 0, SEEK_SET)) return 0;
        char *sql = malloc((size_t)length + 1);
        if (!sql || fread(sql, 1, (size_t)length, source) != (size_t)length) return 0;
        sql[length] = 0;
        fclose(source);
        int bom = strip_bom && length >= 3 && (unsigned char)sql[0] == 0xef &&
                  (unsigned char)sql[1] == 0xbb && (unsigned char)sql[2] == 0xbf;
        int ok = bootstrap_service_fixture_execute_sql(line[0] == '1', sql + (bom ? 3 : 0), (int)length - (bom ? 3 : 0));
        free(sql);
        printf("{\"phase\":\"original_database_sql\",\"source_index\":%u,\"gameplay\":%s,\"bytes\":%ld,\"utf8_bom_removed\":%s,\"success\":%s}\n", index++, line[0]=='1'?"true":"false", length, bom?"true":"false", ok?"true":"false");
        if (!ok) return 0;
    }
    fclose(manifest);
    return index == 7;
}
int wmain(int argc, wchar_t **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    SetUnhandledExceptionFilter(crashed);
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    if (argc != 4 && argc != 5 && argc != 6 && argc != 7) { fputs("usage: context_host.exe DLL DEPENDENCY_DIRECTORY ORIGINAL_EXECUTABLE\n", stderr); return 2; }
    if (!hash_matches(argv[1], expected_dll) ||
        !hash_matches(argv[3], "e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63")) { puts("{\"status\":\"unavailable\",\"reason\":\"dll_sha256_gate\"}"); return 3; }
    if (!SetDllDirectoryW(argv[2])) return 4;
    phase = "map_original_host_getter";
    original_executable = LoadLibraryExW(argv[3], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if (!original_executable || !sentinel_is_original()) return 11;
    puts("{\"phase\":\"original_host_sentinel_verified\",\"original_getter_executed\":true,\"original_executable_initialized\":false}");
    phase = "LoadLibraryW";
    image = LoadLibraryW(argv[1]);
    if (!image) { printf("{\"status\":\"unavailable\",\"phase\":\"LoadLibraryW\",\"win32_error\":%lu}\n", GetLastError()); return 5; }
    puts("{\"phase\":\"loaded\",\"normal_windows_loader\":true}");
    CreateContext create = (CreateContext)(void *)GetProcAddress(image, "DllCreateGameContext");
    DestroyContext destroy = (DestroyContext)(void *)GetProcAddress(image, "DllDestroyGameContext");
    if (!create || !destroy) return 6;
    const unsigned char guid1[16] = {0xae,0x14,0x00,0xd7,0x60,0xd9,0x68,0x46,0xbd,0x97,0x88,0xa9,0xc5,0xb3,0x6b,0x9a};
    const unsigned char guid2[16] = {0xe8,0xe4,0xf5,0x18,0x60,0xef,0x8d,0x4c,0xbb,0x1b,0x38,0x34,0xcc,0x57,0x10,0x31};
    const unsigned char unknown[16] = {0};
    for (unsigned cycle = 0; cycle < 3; ++cycle) {
        phase = "DllCreateGameContext";
        void *context = create();
        if (!context) return 7;
        void **vtable = *(void ***)context;
        if ((uintptr_t)vtable - (uintptr_t)image != 0xa101e8 ||
            (uintptr_t)vtable[0] - (uintptr_t)image != 0x5907a0) return 8;
        phase = "QueryInterface";
        QueryInterface query = (QueryInterface)vtable[0];
        int a = query(context, guid1) == context;
        int b = query(context, guid2) == context;
        int c = query(context, unknown) == NULL;
        printf("{\"phase\":\"context_queries\",\"cycle\":%u,\"real_context_created\":true,\"accepted_guid_1\":%s,\"accepted_guid_2\":%s,\"unknown_rejected\":%s}\n", cycle, a?"true":"false", b?"true":"false", c?"true":"false");
        if (argc >= 5 && cycle == 0) {
            int original_services_prepared = 0;
            int content_mode = wcsstr(argv[4], L"content") != NULL;
            int startup_mode = wcscmp(argv[4], L"--startup-game") == 0 || wcscmp(argv[4], L"--load-stream") == 0 || wcsstr(argv[4], L"-startup") || wcsstr(argv[4], L"-load");
            void *engine = bootstrap_service_fixture_create(image);
            if (!engine) return 12;
            phase = "SetEngineInterface";
            ((void (*)(void *, void *))vtable[0x60 / 8])(context, engine);
            if (wcscmp(argv[4], L"--empty-databases") == 0 || argc >= 6) {
                HMODULE database = GetModuleHandleW(L"DatabaseDLL_FinalRelease.dll");
                wchar_t database_path[32768];
                if (!database || !GetModuleFileNameW(database, database_path, 32768) ||
                    !hash_matches(database_path, "e6577e873f946224df65c02f6c4bc3b385607e2a82d957b0fd613e01cbe78b0c")) return 13;
                phase = "prepare_original_database_empty_fixtures";
                if (!bootstrap_service_fixture_prepare_empty_databases(database)) return 14;
            }
            if (argc >= 6) {
                phase = "execute_original_sql_schema_sources";
                if (content_mode) {
                    phase = "prepare_original_host_services";
                    if (!bootstrap_service_fixture_prepare_original_host_services(original_executable)) return 17;
                    original_services_prepared = 1;
                    phase = "execute_original_content_manifest";
                    int content_result = bootstrap_original_content_manifest_with_context(original_executable,
                        bootstrap_service_fixture_database_connection(0),
                        bootstrap_service_fixture_database_connection(1), argv[5], context);
                    if (content_result) return content_result;
                } else if (!execute_schema_manifest(argv[5], wcscmp(argv[4], L"--raw-sql-databases") != 0)) return 15;
            }
            const wchar_t *snapshot_directory = _wgetenv(L"C6_BOOTSTRAP_RULES_SNAPSHOT_DIR");
            if (content_mode && snapshot_directory) {
                wchar_t snapshot_path[32768];
                HMODULE database = GetModuleHandleW(L"DatabaseDLL_FinalRelease.dll");
                phase = "backup_original_rules_databases";
                if (swprintf(snapshot_path, 32768, L"%ls\\configuration.sqlite", snapshot_directory) < 0 ||
                    bootstrap_original_database_backup(database, bootstrap_service_fixture_database_connection(0), snapshot_path, "configuration")) return 19;
                if (swprintf(snapshot_path, 32768, L"%ls\\gameplay.sqlite", snapshot_directory) < 0 ||
                    bootstrap_original_database_backup(database, bootstrap_service_fixture_database_connection(1), snapshot_path, "gameplay")) return 19;
            }
            bootstrap_service_fixture_report(stdout);
            phase = "Initialize";
            int before = ((unsigned char (*)(void *))vtable[0x38 / 8])(context);
            int initialized = ((unsigned char (*)(void *))vtable[0x28 / 8])(context);
            int after = ((unsigned char (*)(void *))vtable[0x38 / 8])(context);
            printf("{\"phase\":\"Initialize_returned\",\"result\":%s,\"is_initialized_before\":%s,\"is_initialized_after\":%s,\"initialized_game\":false}\n", initialized ? "true" : "false", before ? "true" : "false", after ? "true" : "false");
            if (startup_mode) {
                void *instance = ((void **)context)[1];
                void **instance_vtable = *(void ***)instance;
                if (!initialized || !after || (uintptr_t)vtable[0x40 / 8] - (uintptr_t)image != 0x591870 ||
                    (uintptr_t)instance_vtable[0x50 / 8] - (uintptr_t)image != 0x46be0) return 16;
                phase = "prepare_original_host_services";
                if (!original_services_prepared && !bootstrap_service_fixture_prepare_original_host_services(original_executable)) return 17;
                phase = "prepare_original_localization";
                const wchar_t *localization_manifest = _wgetenv(L"C6_BOOTSTRAP_LOCALIZATION_MANIFEST");
                const wchar_t *icu_path = _wgetenv(L"C6_BOOTSTRAP_ICU_PATH");
                if (!localization_manifest || !icu_path) return 18;
                int localization_result = bootstrap_original_localization(original_executable, localization_manifest, icu_path);
                if (localization_result) return localization_result;
                phase = "StartupGame";
                printf("{\"phase\":\"StartupGame_begin\",\"original_wrapper_rva\":\"0x591870\",\"original_implementation_rva\":\"0x46be0\",\"fixture\":\"%s\",\"initialized_game\":false}\n", content_mode ? "original_declared_content_recipe" : "original_sql_only_schema");
                ((void (*)(void *))vtable[0x40 / 8])(context);
                puts("{\"phase\":\"StartupGame_returned\",\"initialized_game\":false}");
                if (argc == 7) {
                    phase = "ContextLoad";
                    int load_result = bootstrap_save_probe(image, context, argv[6]);
                    if (load_result) return load_result;
                }
            }
        }
        phase = "DllDestroyGameContext";
        destroy(context);
        if (!a || !b || !c) return 9;
    }
    phase = "FreeLibrary";
    if (!FreeLibrary(image) || !sentinel_is_original()) return 10;
    puts("{\"status\":\"ok\",\"lifetime_cycles\":3,\"real_context_created\":true,\"initialized_game\":false,\"gameplay_parity\":false}");
    return 0;
}
