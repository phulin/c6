/* Compare only retained logging-level metadata with the actual original manager.
 * The fingerprint-gated Python launcher owns module/provider identity checks. */
#include "service_fixture.c"
static int compare_level(void *native, uint32_t id, unsigned case_index) {
    typedef uint32_t (*get_fn)(void *, uint32_t);
    uint32_t expected = ((get_fn)(*(uintptr_t **)native)[7])(native, id);
    uint32_t actual = ((get_fn)logging_slots[7])(&logging_object, id);
    printf("{\"case\":%u,\"id\":%u,\"original_level\":%u,\"fixture_level\":%u,\"equal\":%s}\n", case_index, id, expected, actual, expected == actual ? "true" : "false");
    return expected == actual;
}
static int compare_enabled(void *native, uint32_t id, unsigned case_index) {
    typedef unsigned char (*get_fn)(void *, uint32_t);
    unsigned char expected = ((get_fn)(*(uintptr_t **)native)[2])(native, id);
    unsigned char actual = ((get_fn)logging_slots[2])(&logging_object, id);
    printf("{\"case\":%u,\"id\":%u,\"original_enabled\":%u,\"fixture_enabled\":%u,\"equal\":%s}\n", case_index, id, expected, actual, expected == actual ? "true" : "false");
    return expected == actual;
}
static int compare_file_enabled(void *native_file, void *fixture_file, unsigned case_index) {
    typedef unsigned char (*get_fn)(void *);
    unsigned char expected = ((get_fn)(*(uintptr_t **)native_file)[7])(native_file);
    unsigned char actual = ((get_fn)(*(uintptr_t **)fixture_file)[7])(fixture_file);
    printf("{\"case\":%u,\"original_file_enabled\":%u,\"fixture_file_enabled\":%u,\"equal\":%s}\n", case_index, expected, actual, expected == actual ? "true" : "false");
    return expected == actual;
}
int wmain(int argc, wchar_t **argv) {
    if (argc != 3 || !SetDllDirectoryW(argv[2])) return 2;
    HMODULE original = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if ((uintptr_t)original != 0x140000000ULL || !bootstrap_bind_original_imports(original)) return 3;
    initialize_logging_tables();
    typedef void *(*allocate_fn)(size_t);
    typedef void *(*construct_fn)(void *);
    typedef void (*register_fn)(void *, uint32_t, const wchar_t *, uint32_t, const char *);
    typedef void (*set_fn)(void *, uint32_t, uint32_t);
    void *memory = ((allocate_fn)((uintptr_t)original + 0x982060))(0x2e30);
    if (!memory) return 4;
    void *native = ((construct_fn)((uintptr_t)original + 0x15a740))(memory);
    if (native != memory || *(uintptr_t *)native != (uintptr_t)original + 0xfbf628) return 5;
    register_fn native_register = (register_fn)(*(uintptr_t **)native)[8];
    set_fn native_set = (set_fn)(*(uintptr_t **)native)[6];
    uint32_t id = 0xc95e0bc4;
    unsigned index = 0;
    int ok = compare_level(native, id, index++);
    native_register(native, id, L"C6LoggingProbe.log", 5, NULL);
    log_register(&logging_object, id, L"C6LoggingProbe.log", 5, NULL);
    ok &= compare_level(native, id, index++);
    native_set(native, id, 9); log_set_type_level(&logging_object, id, 9);
    ok &= compare_level(native, id, index++);
    native_register(native, id, NULL, 17, NULL); log_register(&logging_object, id, NULL, 17, NULL);
    ok &= compare_level(native, id, index++);
    native_set(native, id, 0); log_set_type_level(&logging_object, id, 0);
    ok &= compare_level(native, id, index++);
    native_set(native, 0, 0); log_set_type_level(&logging_object, 0, 0);
    ok &= compare_level(native, 0, index++);
    native_set(native, 0, 23); log_set_type_level(&logging_object, 0, 23);
    ok &= compare_level(native, 0, index++);
    native_set(native, UINT32_MAX, UINT32_MAX); log_set_type_level(&logging_object, UINT32_MAX, UINT32_MAX);
    ok &= compare_level(native, UINT32_MAX, index++);
    ok &= compare_level(native, 0x12345678, index++);
    typedef void (*enable_fn)(void *, uint32_t, unsigned char);
    enable_fn native_enable = (enable_fn)(*(uintptr_t **)native)[5];
    ok &= compare_enabled(native, id, index++);
    native_enable(native, id, 0); log_enable(&logging_object, id, 0);
    ok &= compare_enabled(native, id, index++);
    native_register(native, id, L"C6LoggingProbeAgain.log", 5, NULL);
    log_register(&logging_object, id, L"C6LoggingProbeAgain.log", 5, NULL);
    ok &= compare_enabled(native, id, index++);
    native_enable(native, id, 1); log_enable(&logging_object, id, 1);
    ok &= compare_enabled(native, id, index++);
    const uint32_t missing_id = 0x10b39140;
    ok &= compare_enabled(native, missing_id, index++);
    native_enable(native, missing_id, 0); log_enable(&logging_object, missing_id, 0);
    ok &= compare_enabled(native, missing_id, index++);
    native_register(native, missing_id, L"C6LoggingMissing.log", 0, NULL);
    log_register(&logging_object, missing_id, L"C6LoggingMissing.log", 0, NULL);
    ok &= compare_enabled(native, missing_id, index++);
    native_enable(native, 0x12345678, 1); log_enable(&logging_object, 0x12345678, 1);
    ok &= compare_enabled(native, 0x12345678, index++);
    ok &= compare_enabled(native, UINT32_MAX, index++);
    /* Publish the fully native manager at the exact singleton location written
     * by original lazy creator1c97d2. This avoids constructing an app merely to
     * retrieve its logging path. Both queried files are disabled before native
     * GetLogFile, so its original Open path performs no filesystem I/O. */
    void **native_singleton = (void **)((uintptr_t)original + 0x22d8148);
    if (*native_singleton) return 7;
    *native_singleton = native;
    typedef void *(*get_file_fn)(void *, uint32_t);
    get_file_fn native_get_file = (get_file_fn)(*(uintptr_t **)native)[10];
    const uint32_t file_id = 0x7dc77f75;
    native_register(native, file_id, L"C6LoggingDisabledA.log", 0, NULL);
    log_register(&logging_object, file_id, L"C6LoggingDisabledA.log", 0, NULL);
    native_enable(native, file_id, 0); log_enable(&logging_object, file_id, 0);
    native_enable(native, 0x12345678, 0); log_enable(&logging_object, 0x12345678, 0);
    void *native_file_a = native_get_file(native, file_id);
    void *fixture_file_a = get_logfile(&logging_object, file_id);
    void *native_file_b = native_get_file(native, 0x12345678);
    void *fixture_file_b = get_logfile(&logging_object, 0x12345678);
    int identity_ok = native_file_a && native_file_b && fixture_file_a && fixture_file_b &&
        native_file_a != native_file_b && fixture_file_a != fixture_file_b &&
        native_get_file(native, file_id) == native_file_a && get_logfile(&logging_object, file_id) == fixture_file_a &&
        !native_get_file(native, 0x87654321) && !get_logfile(&logging_object, 0x87654321);
    printf("{\"case\":%u,\"file_identity_and_unknown_null\":%s,\"equal\":%s}\n", index++, identity_ok ? "true" : "false", identity_ok ? "true" : "false");
    ok &= identity_ok;
    if (!identity_ok) return 8;
    ok &= compare_file_enabled(native_file_a, fixture_file_a, index++);
    native_enable(native, file_id, 1); log_enable(&logging_object, file_id, 1);
    ok &= compare_file_enabled(native_file_a, fixture_file_a, index++);
    ok &= compare_file_enabled(native_file_b, fixture_file_b, index++);
    native_enable(native, file_id, 0); log_enable(&logging_object, file_id, 0);
    ok &= compare_file_enabled(native_file_a, fixture_file_a, index++);
    *native_singleton = NULL;
    ((void (*)(void *))(*(uintptr_t **)native)[1])(native);
    printf("{\"status\":\"%s\",\"cases\":%u,\"original_logging_cleanup\":true,\"gameplay_parity\":false}\n", ok ? "ok" : "mismatch", index);
    return ok ? 0 : 6;
}
