/* Exact-image, child-process original XML importer probe. No host startup. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#include "imports.h"
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
typedef unsigned char (*Initialize)(void);
typedef void *(*CreateConnection)(const char *, int, void *);
typedef unsigned char (*Execute)(void *, const char *, int);
typedef void *(*Constructor)(void *, void *);
typedef void *(*XmlConstructor)(void *, void *, unsigned char);
typedef int (*Parse)(void *, const char *, char *, int, void *);
typedef void (*Destructor)(void *);
typedef void *(*Query)(void *, const char *, int);
typedef unsigned char (*Step)(void *);
typedef int64_t (*GetInt64)(void *, int);
static int64_t scalar(void *connection, const char *sql) {
    void *query = ((Query)(*(uintptr_t **)connection)[3])(connection, sql, (int)strlen(sql));
    if (!query || !((Step)(*(uintptr_t **)query)[4])(query)) ExitProcess(81);
    int64_t value = ((GetInt64)(*(uintptr_t **)query)[36])(query, 0);
    printf("{\"phase\":\"original_query_scalar\",\"value\":%lld,\"getter_rva\":\"0x%llx\"}\n", (long long)value,
           (unsigned long long)((*(uintptr_t **)query)[36] - (uintptr_t)GetModuleHandleW(L"DatabaseDLL_FinalRelease.dll")));
    if (((Step)(*(uintptr_t **)query)[4])(query)) ExitProcess(82);
    ((Destructor)(*(uintptr_t **)query)[1])(query);
    return value;
}
int wmain(int argc, wchar_t **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    SetUnhandledExceptionFilter(crash);
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    if (argc != 3) { fputs("usage: xml_import_probe.exe ORIGINAL_EXE DEPENDENCY_DIRECTORY\n", stderr); return 2; }
    wchar_t database_path[32768];
    if (swprintf(database_path, 32768, L"%ls\\DatabaseDLL_FinalRelease.dll", argv[2]) < 0) return 2;
    if (!hash_matches(argv[1], "e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63") ||
        !hash_matches(database_path, "e6577e873f946224df65c02f6c4bc3b385607e2a82d957b0fd613e01cbe78b0c")) return 3;
    if (!SetDllDirectoryW(argv[2])) return 4;
    original = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if ((uintptr_t)original != 0x140000000ULL) return 5;
    phase = "bind_imports";
    int bound = bootstrap_bind_original_imports(original);
    if (!bound) return 6;
    printf("{\"phase\":\"genuine_imports_bound\",\"count\":%d,\"original_exe_startup\":false}\n", bound);
    HMODULE database = GetModuleHandleW(L"DatabaseDLL_FinalRelease.dll");
    Initialize initialize = (Initialize)(void *)GetProcAddress(database, "?Initialize@Database@@YA_NXZ");
    CreateConnection create = (CreateConnection)(void *)GetProcAddress(database, "?CreateConnection@Database@@YAPEAVIDatabaseConnection1@1@PEBDHPEAVIDatabaseLogger@1@@Z");
    phase = "database_initialize";
    if (!initialize || !create || !initialize()) return 7;
    void *connection = create(":memory:", 6, NULL);
    if (!connection) return 8;
    Execute execute = (Execute)(*(uintptr_t **)connection)[5];
    const char *schema = "CREATE TABLE C6Probe(Value INTEGER CHECK(Value=7));";
    if (!execute(connection, schema, (int)strlen(schema))) return 9;
    unsigned char object[0x320] = {0};
    phase = "original_connection_context_constructor";
    ((Constructor)((unsigned char *)original + 0x799910))(object + 8, connection);
    phase = "original_xml_importer_constructor";
    ((XmlConstructor)((unsigned char *)original + 0x799d80))(object + 0x50, connection, 1);
    puts("{\"phase\":\"original_xml_importer_constructed\"}");
    char xml[] = "<GameData><C6Probe><Row Value=\"7\"/></C6Probe></GameData>";
    phase = "original_xml_parse";
    int status = ((Parse)((unsigned char *)original + 0x79d3a0))(object + 0x50, "c6-probe.xml", xml, -1, object + 8);
    printf("{\"phase\":\"original_xml_parse\",\"native_status\":%d}\n", status);
    if (status) return 10;
    phase = "original_database_verify_row";
    const char *verify = "CREATE TABLE C6Verify(Ok INTEGER CHECK(Ok=1)); INSERT INTO C6Verify SELECT CASE WHEN count(*)=1 AND min(Value)=7 AND max(Value)=7 THEN 1 ELSE 0 END FROM C6Probe;";
    if (!execute(connection, verify, (int)strlen(verify))) return 11;
    if (scalar(connection, "SELECT count(*) FROM C6Probe") != 1 ||
        scalar(connection, "SELECT Value FROM C6Probe") != 7) return 12;
    puts("{\"phase\":\"original_xml_rows_verified\",\"row_count\":1,\"value\":7,\"gameplay_parity\":false}");
    phase = "original_xml_importer_destructor";
    ((Destructor)((unsigned char *)original + 0x799e30))(object + 0x50);
    phase = "original_connection_context_destructor";
    ((Destructor)((unsigned char *)original + 0x799930))(object + 8);
    phase = "original_database_connection_destroy";
    ((Destructor)(*(uintptr_t **)connection)[1])(connection);
    puts("{\"status\":\"ok\",\"original_xml_import\":true,\"initialized_game\":false}");
    return 0;
}
