#include "localization.h"
#include <bcrypt.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
typedef unsigned char (*Execute)(void *, const char *, int);
typedef void *(*Create)(const char *, int, void *);
typedef unsigned char (*Init)(const wchar_t *, void *, const char *, unsigned char, void *);
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

int bootstrap_original_localization(HMODULE executable, const wchar_t *manifest, const wchar_t *icu_path) {
    HMODULE dll = GetModuleHandleW(L"Localization_FinalRelease.dll");
    HMODULE db = GetModuleHandleW(L"DatabaseDLL_FinalRelease.dll");
    wchar_t dll_path[32768];
    if (!dll || !db || !GetModuleFileNameW(dll, dll_path, 32768) ||
        !hash_matches(dll_path, "0becd5c425a3d6edc88f5315c01af3dd30dd7cb158271d56f4bba7ff2bb4e056") ||
        !hash_matches(icu_path, "02cfa347cf8e80e3d9ca925a8c3371db9a125c53234f3e1c03fc0eae4705153f")) return 90;
    Create create = (Create)(void *)GetProcAddress(db, "?CreateConnection@Database@@YAPEAVIDatabaseConnection1@1@PEBDHPEAVIDatabaseLogger@1@@Z");
    Init initialize = (Init)(void *)GetProcAddress(dll, "?Initialize@Localization@@YA_NPEB_SPEAVIDatabaseConnection1@Database@@PEBD_NPEAVILocalizationLogger@1@@Z");
    if (!create || !initialize) return 91;
    void *connection = create("file:Localization?mode=memory&cache=shared", 0x46, NULL);
    if (!connection) return 92;
    struct bootstrap_original_xml context;
    bootstrap_original_xml_construct(&context, executable, connection);
    FILE *input = _wfopen(manifest, L"rb");
    if (!input) return 93;
    char line[65536]; unsigned count = 0;
    while (fgets(line, sizeof line, input)) {
        char *format = strtok(line, "\t"), *hash = strtok(NULL, "\t"), *path = strtok(NULL, "\r\n");
        wchar_t wide[32768];
        if (!format || !hash || !path || (strcmp(format, "sql") && strcmp(format, "xml")) ||
            !MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, path, -1, wide, 32768) || !hash_matches(wide, hash)) return 94;
        FILE *source = _wfopen(wide, L"rb");
        if (!source || fseek(source, 0, SEEK_END)) return 95;
        long size = ftell(source);
        if (size < 0 || fseek(source, 0, SEEK_SET)) return 95;
        char *bytes = malloc((size_t)size + 1);
        if (!bytes || fread(bytes, 1, (size_t)size, source) != (size_t)size) return 95;
        fclose(source); bytes[size] = 0;
        int skip = size >= 3 && !memcmp(bytes, "\xef\xbb\xbf", 3) ? 3 : 0;
        int status = !strcmp(format, "sql") ? (((Execute)(*(uintptr_t **)connection)[5])(connection, bytes + skip, (int)size - skip) ? 0 : -1) : bootstrap_original_xml_parse(&context, path, bytes + skip);
        free(bytes);
        if (status) { fprintf(stderr, "localization source %u failed: %s (%d)\n", count, path, status); return 96; }
        ++count;
    }
    fclose(input);
    bootstrap_original_xml_destroy(&context);
    unsigned char ok = initialize(icu_path, connection, "en_US", 0, NULL);
    printf("{\"phase\":\"original_localization_initialize\",\"success\":%s,\"language\":\"en_US\",\"source_count\":%u,\"dictionary_nonnull\":%s}\n", ok ? "true" : "false", count, *(void **)((char *)dll + 0x18c6c0) ? "true" : "false");
    if (!ok) return 97;
    typedef void *(*Lookup)(void *, const char *);
    typedef const char *(*Base)(void *, unsigned *, int);
    typedef void (*Destroy)(void *);
    Lookup lookup = (Lookup)(void *)GetProcAddress(dll, "?Lookup@Localization@@YA?AVString@1@PEBD@Z");
    Base base = (Base)(void *)GetProcAddress(dll, "?toUTF8@String@Localization@@QEAAPEBDAEAIH@Z");
    Destroy destroy = (Destroy)(void *)GetProcAddress(dll, "??1String@Localization@@UEAA@XZ");
    if (!lookup || !base || !destroy) return 98;
    unsigned char object[512] = {0}; unsigned length = 0;
    lookup(object, "LOC_TECH_POTTERY_NAME");
    const char *text = base(object, &length, 1);
    int match = text && length == 7 && !memcmp(text, "Pottery", 7);
    printf("{\"phase\":\"original_localization_lookup\",\"key\":\"LOC_TECH_POTTERY_NAME\",\"expected_match\":%s,\"length\":%u}\n", match ? "true" : "false", length);
    destroy(object);
    typedef unsigned char (*SetLanguage)(const char *);
    typedef void *(*GetLanguage)(void);
    typedef const char *(*LanguageType)(void *);
    SetLanguage set_language = (SetLanguage)(void *)GetProcAddress(dll, "?SetCurrentLanguage@Localization@@YA_NPEBD@Z");
    GetLanguage get_language = (GetLanguage)(void *)GetProcAddress(dll, "?GetCurrentLanguage@Localization@@YAAEBVLanguage@1@XZ");
    LanguageType language_type = (LanguageType)(void *)GetProcAddress(dll, "?GetType@Language@Localization@@QEBAPEBDXZ");
    if (!set_language || !get_language || !language_type) return 98;
    int rejected = !set_language("c6_invalid_locale");
    int retained = !strcmp(language_type(get_language()), "en_US");
    printf("{\"phase\":\"original_localization_invalid_language\",\"rejected\":%s,\"en_US_retained\":%s}\n", rejected ? "true" : "false", retained ? "true" : "false");
    return match && rejected && retained ? 0 : 99;
}
