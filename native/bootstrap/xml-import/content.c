#include "content.h"
#include <bcrypt.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef unsigned char (*Execute)(void *, const char *, int);
typedef void (*Destructor)(void *);
typedef void *(*Query)(void *, const char *, int);
typedef unsigned char (*Step)(void *);
typedef int64_t (*GetInt64)(void *, int);
typedef const char *(*GetText)(void *, int);
static void constraint_diagnostics(void *connection) {
    const char *sql = "PRAGMA foreign_key_check";
    void *query = ((Query)(*(uintptr_t **)connection)[3])(connection, sql, (int)strlen(sql));
    if (!query) return;
    unsigned count = 0;
    while (((Step)(*(uintptr_t **)query)[4])(query)) {
        if (count < 20) {
            const char *table = ((GetText)(*(uintptr_t **)query)[30])(query, 0);
            const char *parent = ((GetText)(*(uintptr_t **)query)[30])(query, 2);
            int64_t row = ((GetInt64)(*(uintptr_t **)query)[36])(query, 1);
            printf("{\"phase\":\"original_foreign_key_violation\",\"table\":\"%s\",\"rowid\":%lld,\"parent\":\"%s\"}\n", table, (long long)row, parent);
        }
        ++count;
    }
    ((Destructor)(*(uintptr_t **)query)[1])(query);
    printf("{\"phase\":\"original_foreign_key_check\",\"violation_count\":%u,\"examples_limited_to\":20}\n", count);
}
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
static int command(void *connection, const char *sql) {
    return ((Execute)(*(uintptr_t **)connection)[5])(connection, sql, (int)strlen(sql)) != 0;
}
static int transaction(void *connection, unsigned slot) {
    return ((Step)(*(uintptr_t **)connection)[slot])(connection) != 0;
}
static int savepoint(void *connection, unsigned slot, const char *name) {
    typedef unsigned char (*Savepoint)(void *, const char *);
    return ((Savepoint)(*(uintptr_t **)connection)[slot])(connection, name) != 0;
}
static int validate_manifest(const wchar_t *manifest) {
    FILE *input = _wfopen(manifest, L"rb");
    if (!input) return 0;
    char line[65536]; unsigned count = 0; int ok = 1;
    while (fgets(line, sizeof line, input)) {
        char *db = strtok(line, "\t"), *format = strtok(NULL, "\t"), *stage = strtok(NULL, "\t");
        char *hash = strtok(NULL, "\t"), *path = strtok(NULL, "\t\r\n");
        wchar_t wide_path[32768];
        if (!db || !format || !stage || !hash || !path ||
            (strcmp(db, "gameplay") && strcmp(db, "configuration")) ||
            (strcmp(format, "sql") && strcmp(format, "xml")) ||
            (strcmp(stage, "schema") && strcmp(stage, "content") && strcmp(stage, "mod_content")) ||
            !MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, path, -1, wide_path, 32768) ||
            !hash_matches(wide_path, hash)) { ok = 0; break; }
        ++count;
    }
    if (ferror(input)) ok = 0;
    fclose(input);
    return ok && count;
}
static char *read_source(const wchar_t *path, long *size) {
    FILE *source = _wfopen(path, L"rb");
    if (!source) return NULL;
    char *bytes = NULL;
    if (!fseek(source, 0, SEEK_END) && (*size = ftell(source)) >= 0 &&
        !fseek(source, 0, SEEK_SET)) {
        bytes = malloc((size_t)*size + 1);
        if (bytes && fread(bytes, 1, (size_t)*size, source) != (size_t)*size) {
            free(bytes); bytes = NULL;
        }
        if (bytes) bytes[*size] = 0;
    }
    fclose(source);
    return bytes;
}
int bootstrap_original_content_manifest_with_context(HMODULE original, void *configuration, void *gameplay, const wchar_t *manifest, void *gamecore_context) {
    /* Reject the complete source identity/configuration before registering any
     * function, setting any pragma or starting any transaction. Recheck each
     * source immediately before reading as well. */
    if (!validate_manifest(manifest)) return 27;
    void *connections[2] = {configuration, gameplay};
    struct bootstrap_original_xml contexts[2];
    FILE *input = NULL;
    unsigned constructed = 0, index = 0;
    int current = -1, content = -1, result = 0, active_savepoint = 0, embedded_data = 0;
#define FAIL(N) do { result = (N); goto done; } while (0)
    for (unsigned i = 0; i < 2; ++i) {
        if (!connections[i] || bootstrap_original_make_hash(original, connections[i]) != 0 ||
            !command(connections[i], "PRAGMA foreign_keys=ON;PRAGMA recursive_triggers=ON;")) FAIL(20);
        bootstrap_original_xml_construct(&contexts[i], original, connections[i]);
        ++constructed;
        printf("{\"phase\":\"original_make_hash_verified\",\"database\":%u,\"kind_city\":%lld}\n", i,
               (long long)scalar(connections[i], "SELECT Make_Hash('KIND_CITY')"));
    }
    input = _wfopen(manifest, L"rb");
    if (!input) FAIL(21);
    char line[65536], current_action[256] = {0};
    while (fgets(line, sizeof line, input)) {
        char *db = strtok(line, "\t"), *format = strtok(NULL, "\t"), *stage = strtok(NULL, "\t");
        char *hash = strtok(NULL, "\t"), *path = strtok(NULL, "\t\r\n");
        char *action = strtok(NULL, "\t\r\n");
        if (!action) action = "";
        if (!db || !format || !stage || !hash || !path) FAIL(22);
        if ((strcmp(db, "gameplay") && strcmp(db, "configuration")) ||
            (strcmp(format, "sql") && strcmp(format, "xml")) ||
            (strcmp(stage, "schema") && strcmp(stage, "content") && strcmp(stage, "mod_content"))) FAIL(22);
        int selected = !strcmp(db, "gameplay");
        int next_content = !strcmp(stage, "mod_content") ? 2 : !strcmp(stage, "content");
        void *connection = connections[selected];
        if (strlen(action) >= sizeof current_action) FAIL(22);
        if (selected != current || next_content != content) {
            if (active_savepoint) {
                if (!savepoint(connections[current], 19, current_action)) FAIL(32);
                active_savepoint = 0;
            }
            if (current >= 0 && !transaction(connections[current], 16)) {
                constraint_diagnostics(connections[current]); FAIL(23);
            }
            current = selected; content = next_content;
            if (selected && content == 2 && !embedded_data) {
                HMODULE gamecore = GetModuleHandleW(L"GameCore_XP2_FinalRelease.dll");
                if (!gamecore || !gamecore_context) FAIL(33);
                uintptr_t method = (*(uintptr_t **)gamecore_context)[10];
                if (method - (uintptr_t)gamecore != 0x590600) FAIL(34);
                if (!command(connection, "PRAGMA foreign_keys=ON;PRAGMA defer_foreign_keys=ON;") ||
                    !transaction(connection, 13)) FAIL(34);
                int64_t before = scalar(connection, "SELECT count(*) FROM Types");
                int64_t changes_before = scalar(connection, "SELECT total_changes()");
                typedef unsigned char (*InitializeDatabase)(void *, void *);
                unsigned char status = ((InitializeDatabase)method)(gamecore_context, connection);
                int64_t after = scalar(connection, "SELECT count(*) FROM Types");
                int64_t changes_after = scalar(connection, "SELECT total_changes()");
                printf("{\"phase\":\"original_gamecore_embedded_database\",\"native_status\":%u,\"types_before\":%lld,\"types_after\":%lld,\"total_changes_delta\":%lld}\n",
                       status, (long long)before, (long long)after, (long long)(changes_after - changes_before));
                if (!status || after <= before || changes_after <= changes_before || !transaction(connection, 16)) {
                    constraint_diagnostics(connection); FAIL(34);
                }
                embedded_data = 1;
            }
            if (content == 2) {
                if (!command(connection, "PRAGMA foreign_keys=ON;PRAGMA defer_foreign_keys=ON;")) FAIL(24);
            } else {
                if (content && selected && !command(connection, "PRAGMA foreign_keys=OFF;PRAGMA defer_foreign_keys=OFF;")) FAIL(24);
                if ((!content || !selected) && !command(connection, "PRAGMA defer_foreign_keys=ON;")) FAIL(25);
            }
            if (!transaction(connection, content == 2 ? 13 : 15)) FAIL(26);
        }
        if (content == 2 && (!active_savepoint || strcmp(action, current_action))) {
            if (!*action) FAIL(22);
            if (active_savepoint && !savepoint(connection, 19, current_action)) FAIL(32);
            strcpy(current_action, action);
            if (!savepoint(connection, 18, current_action)) FAIL(32);
            active_savepoint = 1;
        }
        wchar_t wide_path[32768];
        if (!MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, path, -1, wide_path, 32768) ||
            !hash_matches(wide_path, hash)) FAIL(27);
        long size = 0;
        char *bytes = read_source(wide_path, &size);
        if (!bytes) FAIL(28);
        int skip = size >= 3 && !memcmp(bytes, "\xef\xbb\xbf", 3) ? 3 : 0;
        int status = !strcmp(format, "sql") ?
            (((Execute)(*(uintptr_t **)connection)[5])(connection, bytes + skip, (int)size - skip) ? 0 : -1) :
            bootstrap_original_xml_parse(&contexts[selected], path, bytes + skip);
        free(bytes);
        printf("{\"phase\":\"original_source_import\",\"source_index\":%u,\"database\":\"%s\",\"format\":\"%s\",\"native_status\":%d}\n", index++, db, format, status);
        if (status) {
            typedef const char *(*Error)(void *);
            const char *error = ((Error)(*(uintptr_t **)connection)[35])(connection);
            fprintf(stderr, "source %u failed: %s: %s\n", index - 1, path, error ? error : "unknown");
            FAIL(30);
        }
    }
    if (ferror(input)) FAIL(28);
    fclose(input); input = NULL;
    if (active_savepoint) {
        if (!savepoint(connections[current], 19, current_action)) FAIL(32);
        active_savepoint = 0;
    }
    if (current >= 0 && !transaction(connections[current], 16)) {
        constraint_diagnostics(connections[current]); FAIL(31);
    }
    for (unsigned i = 0; i < 2; ++i) {
        printf("{\"phase\":\"original_recipe_database\",\"database\":\"%s\",\"tables\":%lld}\n",
               i ? "gameplay" : "configuration", (long long)scalar(connections[i], "SELECT count(*) FROM sqlite_master WHERE type='table'"));
    }
    printf("{\"phase\":\"original_recipe_complete\",\"source_count\":%u,\"mod_selection_engine_executed\":false,\"initialized_game\":false}\n", index);
done:
    if (result && active_savepoint) {
        savepoint(connections[current], 20, current_action);
        savepoint(connections[current], 19, current_action);
    }
    if (result && current >= 0) transaction(connections[current], 17);
    if (input) fclose(input);
    for (unsigned i = 0; i < constructed; ++i) bootstrap_original_xml_destroy(&contexts[i]);
#undef FAIL
    return result;
}
int bootstrap_original_content_manifest(HMODULE original, void *configuration, void *gameplay, const wchar_t *manifest) {
    return bootstrap_original_content_manifest_with_context(original, configuration, gameplay, manifest, NULL);
}
