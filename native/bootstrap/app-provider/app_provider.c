#include "app_provider.h"
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
void *bootstrap_original_app_construct(HMODULE executable) {
    unsigned char *base = (void *)executable;
    if (!exe_gate(executable) || *(void **)(base + 0x12c2c88)) return NULL;
    void *storage = ((void *(*)(size_t))(base + 0x982060))(0x5890);
    if (!storage) return NULL;
    puts("{\"phase\":\"original_app_allocation\",\"bytes\":22672}");
    void *app = ((void *(*)(void *))(base + 0x5ccb0))(storage);
    if (app != storage || *(void **)app != base + 0xfa5be0 || *(void **)((unsigned char *)app + 0x54d8)) return NULL;
    /* Verified original publication is EXE 79c95, after this exact constructor.
     * Installation is explicit fixture wiring; the original full AppInit is
     * deliberately not invoked and no unconstructed shell is substituted. */
    *(void **)(base + 0x12c2c88) = app;
    puts("{\"phase\":\"original_app_constructed\",\"constructor_rva\":\"0x5ccb0\",\"vtable_rva\":\"0xfa5be0\",\"singleton_fixture_publication\":true,\"full_app_initialized\":false}");
    return app;
}

static int scalar(void *connection, const char *sql, int64_t *value) {
    void *query = ((void *(*)(void *, const char *, int))(*(uintptr_t **)connection)[3])(connection, sql, -1);
    if (!query) return 0;
    int ok = ((unsigned char (*)(void *))(*(uintptr_t **)query)[4])(query) != 0;
    if (ok) *value = ((int64_t (*)(void *, int))(*(uintptr_t **)query)[36])(query, 0);
    ((void (*)(void *))(*(uintptr_t **)query)[1])(query);
    return ok;
}
static int compare_gameplay(void *expected, void *actual) {
    int64_t tables, actual_tables;
    if (!expected || !scalar(expected, "SELECT count(*) FROM sqlite_master WHERE type='table'", &tables) ||
        !scalar(actual, "SELECT count(*) FROM sqlite_master WHERE type='table'", &actual_tables) ||
        tables <= 0 || tables != actual_tables) return 0;
    void *query = ((void *(*)(void *, const char *, int))(*(uintptr_t **)expected)[3])(expected,
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name", -1);
    if (!query) return 0;
    uint64_t digest = UINT64_C(14695981039346656037);
    int64_t total = 0, seen = 0; int ok = 1;
    while (((unsigned char (*)(void *))(*(uintptr_t **)query)[4])(query)) {
        const char *name = ((const char *(*)(void *, int))(*(uintptr_t **)query)[30])(query, 0);
        if (!name || strlen(name) > 1024) { ok = 0; break; }
        char sql[2100] = "SELECT count(*) FROM \""; size_t at = strlen(sql);
        for (size_t i = 0; name[i]; ++i) {
            sql[at++] = name[i];
            if (name[i] == '\"') sql[at++] = '\"';
            digest = (digest ^ (unsigned char)name[i]) * UINT64_C(1099511628211);
        }
        sql[at++] = '\"'; sql[at] = 0;
        int64_t a, b;
        if (!scalar(expected, sql, &a) || !scalar(actual, sql, &b) || a != b || a < 0 || a > INT64_MAX - total) { ok = 0; break; }
        for (unsigned i = 0; i < 8; ++i) digest = (digest ^ ((uint64_t)a >> (8*i) & 255)) * UINT64_C(1099511628211);
        total += a; ++seen;
    }
    ((void (*)(void *))(*(uintptr_t **)query)[1])(query);
    if (!ok || seen != tables || total <= 0) return 0;
    printf("{\"phase\":\"original_gameplay_attachment_validated\",\"tables\":%lld,\"rows\":%lld,\"all_table_row_counts_match\":true,\"table_row_count_fnv1a64\":\"%016llx\",\"full_content_hash\":false}\n", (long long)tables, (long long)total, (unsigned long long)digest);
    return 1;
}
int bootstrap_original_app_gameplay_database(HMODULE executable, void *app, void *expected) {
    unsigned char *base = (void *)executable;
    if (!exe_gate(executable) || !app || !expected ||
        !module_gate(GetModuleHandleW(L"DatabaseDLL_FinalRelease.dll"), "e6577e873f946224df65c02f6c4bc3b385607e2a82d957b0fd613e01cbe78b0c") ||
        *(void **)(base + 0x12c2c88) != app || *(void **)app != base + 0xfa5be0 ||
        *(void **)((unsigned char *)app + 0x54d8)) return 0;
    HMODULE database = GetModuleHandleW(L"DatabaseDLL_FinalRelease.dll");
    if ((uintptr_t)*(void **)expected - (uintptr_t)database != 0xb8578) return 0;
    void *storage = ((void *(*)(size_t))(base + 0x982060))(0x40);
    if (!storage) return 0;
    void *wrapper = ((void *(*)(void *))(base + 0xbf070))(storage);
    if (wrapper != storage || *(void **)wrapper != base + 0xfb3410) return 0;
    puts("{\"phase\":\"original_gameplay_wrapper_constructed\",\"constructor_rva\":\"0xbf070\",\"bytes\":64}");
    const char *uri = (char *)wrapper + 0x20;
    if (((size_t *)wrapper)[7] >= 0x10) uri = *(const char **)uri;
    printf("{\"phase\":\"original_gameplay_wrapper_uri\",\"uri\":\"%s\"}\n", uri);
    if (strcmp(uri, "file:gameplay-0?mode=memory&cache=shared")) return 0;
    void **connection_slot = (void **)((unsigned char *)wrapper + 8);
    void *opened = ((void *(*)(void *, void **, const char *))(base + 0xbf6a0))(wrapper, connection_slot, uri);
    printf("{\"phase\":\"original_gameplay_factory_returned\",\"return_matches_output\":%s,\"connection_nonnull\":%s}\n", opened == connection_slot ? "true" : "false", *connection_slot ? "true" : "false");
    if (opened != connection_slot || !*connection_slot) return 0;
    void *connection = *connection_slot;
    if (*(void **)connection != *(void **)expected) return 0;
    if (!compare_gameplay(expected, connection)) return 0;
    void *statement = ((void *(*)(void *, const char *, int))(*(uintptr_t **)connection)[3])(connection, (const char *)base + 0xfb3570, -1);
    if (!statement) { puts("{\"phase\":\"original_global_parameters_statement_unavailable\"}"); return 0; }
    ((void **)wrapper)[2] = statement;
    ((unsigned *)wrapper)[7] = ((unsigned (*)(void *))(*(uintptr_t **)connection)[6])(connection);
    /* Explicit fixture ownership wiring mirrors original AppInit 718f4.
     * Every pointed-to class was initialized by its complete original ctor. */
    *(void **)((unsigned char *)app + 0x54d8) = wrapper;
    puts("{\"phase\":\"original_gameplay_wrapper_connection_opened\",\"native_open_rva\":\"0xbf6a0\",\"uri\":\"file:gameplay-0?mode=memory&cache=shared\",\"native_global_parameters_statement\":true,\"app_field_fixture_install\":true,\"schema_bootstrap_executed\":false}");
    return ((void *(*)(void *))(base + 0x64660))(app) == connection;
}
