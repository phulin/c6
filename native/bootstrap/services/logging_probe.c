/* Compare only retained logging-level metadata with the actual original manager.
 * The fingerprint-gated Python launcher owns module/provider identity checks. */
#include "service_fixture.c"
static int compare_level(void *native, uint32_t id, unsigned case_index) {
    typedef uint32_t (*get_fn)(void *, uint32_t);
    uint32_t expected = ((get_fn)(*(uintptr_t **)native)[7])(native, id);
    uint32_t actual = log_get_type_level(&logging_object, id);
    printf("{\"case\":%u,\"id\":%u,\"original_level\":%u,\"fixture_level\":%u,\"equal\":%s}\n", case_index, id, expected, actual, expected == actual ? "true" : "false");
    return expected == actual;
}
int wmain(int argc, wchar_t **argv) {
    if (argc != 3 || !SetDllDirectoryW(argv[2])) return 2;
    HMODULE original = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if ((uintptr_t)original != 0x140000000ULL || !bootstrap_bind_original_imports(original)) return 3;
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
    ((void (*)(void *))(*(uintptr_t **)native)[1])(native);
    printf("{\"status\":\"%s\",\"cases\":%u,\"original_logging_cleanup\":true,\"gameplay_parity\":false}\n", ok ? "ok" : "mismatch", index);
    return ok ? 0 : 6;
}
