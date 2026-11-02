#include "backup.h"
#include <bcrypt.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void *(*Create)(const char *, int, void *);
typedef unsigned char (*CopyFrom)(void *, void *);
typedef void *(*Query)(void *, const char *, int);
typedef unsigned char (*Step)(void *);
typedef int64_t (*Int64)(void *, int);
typedef const char *(*Text)(void *, int);
typedef void (*Destroy)(void *);
static int module_gate(HMODULE module) {
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
    ok = !strcmp(hex, "e6577e873f946224df65c02f6c4bc3b385607e2a82d957b0fd613e01cbe78b0c");
done:
    if (file != INVALID_HANDLE_VALUE) CloseHandle(file);
    if (hash) BCryptDestroyHash(hash);
    if (alg) BCryptCloseAlgorithmProvider(alg, 0);
    return ok;
}
static int scalar(void *connection, const char *sql, int64_t *value) {
    void *query = ((Query)(*(uintptr_t **)connection)[3])(connection, sql, -1);
    if (!query) return 0;
    int ok = ((Step)(*(uintptr_t **)query)[4])(query) != 0;
    if (ok) *value = ((Int64)(*(uintptr_t **)query)[36])(query, 0);
    ((Destroy)(*(uintptr_t **)query)[1])(query);
    return ok;
}
static int compare_counts(void *source, void *destination, unsigned *table_count, int64_t *rows) {
    int64_t source_tables, destination_tables;
    const char *sql = "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name";
    if (!scalar(source, "SELECT count(*) FROM sqlite_master WHERE type='table'", &source_tables) ||
        !scalar(destination, "SELECT count(*) FROM sqlite_master WHERE type='table'", &destination_tables) ||
        source_tables != destination_tables) return 0;
    void *query = ((Query)(*(uintptr_t **)source)[3])(source, sql, -1);
    if (!query) return 0;
    int ok = 1; *table_count = 0; *rows = 0;
    while (((Step)(*(uintptr_t **)query)[4])(query)) {
        const char *name = ((Text)(*(uintptr_t **)query)[30])(query, 0);
        if (!name || strlen(name) > 1024) { ok = 0; break; }
        char count_sql[2100] = "SELECT count(*) FROM \"";
        size_t at = strlen(count_sql);
        for (size_t i = 0; name[i]; ++i) {
            count_sql[at++] = name[i];
            if (name[i] == '"') count_sql[at++] = '"';
        }
        count_sql[at++] = '"'; count_sql[at] = 0;
        int64_t a, b;
        if (!scalar(source, count_sql, &a) || !scalar(destination, count_sql, &b) || a != b || a < 0 ||
            a > INT64_MAX - *rows) { ok = 0; break; }
        *rows += a; ++*table_count;
    }
    ((Destroy)(*(uintptr_t **)query)[1])(query);
    return ok && *table_count == (uint64_t)source_tables;
}
int bootstrap_original_database_backup(HMODULE database, void *source,
    const wchar_t *new_path, const char *label) {
    if (!module_gate(database) || !source || !new_path || !label ||
        (strcmp(label, "configuration") && strcmp(label, "gameplay"))) return 90;
    uintptr_t *vtable = *(uintptr_t **)source;
    if ((uintptr_t)vtable - (uintptr_t)database != 0xb8578 ||
        vtable[24] - (uintptr_t)database != 0x2670) return 91;
    int bytes = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, new_path, -1, NULL, 0, NULL, NULL);
    if (bytes <= 1) return 92;
    char *utf8_path = malloc((size_t)bytes);
    if (!utf8_path) return 92;
    if (!WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, new_path, -1, utf8_path, bytes, NULL, NULL)) {
        free(utf8_path); return 92;
    }
    HANDLE created = CreateFileW(new_path, GENERIC_WRITE, 0, NULL, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, NULL);
    if (created == INVALID_HANDLE_VALUE) { free(utf8_path); return 93; }
    CloseHandle(created);
    Create create = (Create)(void *)GetProcAddress(database,
        "?CreateConnection@Database@@YAPEAVIDatabaseConnection1@1@PEBDHPEAVIDatabaseLogger@1@@Z");
    void *destination = create ? create(utf8_path, 6, NULL) : NULL;
    free(utf8_path);
    int result = 94; unsigned tables = 0; int64_t rows = 0; int copied = 0;
    if (destination) {
        copied = ((CopyFrom)(*(uintptr_t **)destination)[24])(destination, source) != 0;
        if (copied && compare_counts(source, destination, &tables, &rows)) result = 0;
        ((Destroy)(*(uintptr_t **)destination)[1])(destination);
    }
    printf("{\"phase\":\"original_database_backup\",\"database\":\"%s\",\"copy_rva\":\"0x2670\",\"native_success\":%s,\"row_counts_match\":%s,\"tables\":%u,\"rows\":%lld,\"result\":%d}\n",
        label, copied ? "true" : "false", result ? "false" : "true", tables, (long long)rows, result);
    if (result) DeleteFileW(new_path);
    return result;
}
