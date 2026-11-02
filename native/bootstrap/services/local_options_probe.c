/* Exercise real original LocalOptions global construction and missing-key reads.
 * Full module/provider hashes are verified by the Python launcher. */
#include "service_fixture.c"
int wmain(int argc, wchar_t **argv) {
    if ((argc != 3 && argc != 4) || !SetDllDirectoryW(argv[2])) return 2;
    HMODULE original = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if (!bootstrap_service_fixture_prepare_original_host_services(original)) return 3;
    if (argc == 4) {
        void *main_map = (void *)((uintptr_t)original + 0x22d64f0);
        void *fallback = (void *)((uintptr_t)original + 0x22d6550);
        unsigned char before_main[0x60], before_fallback[0x60];
        ((void *(*)(void *))((uintptr_t)original + 0x7a44a0))(main_map);
        memcpy(before_main, main_map, sizeof before_main);
        memcpy(before_fallback, fallback, sizeof before_fallback);
        if (bootstrap_service_fixture_prepare_empty_local_options(original) ||
            memcmp(before_main, main_map, sizeof before_main) ||
            memcmp(before_fallback, fallback, sizeof before_fallback)) return 7;
        ((void (*)(void))((uintptr_t)original + 0xf7d5e0))();
        puts("{\"status\":\"ok\",\"existing_map_rejected\":true,\"both_native_objects_unchanged\":true,\"original_cleanup\":true}");
        return 0;
    }
    if (!bootstrap_service_fixture_prepare_empty_local_options(original)) return 3;
    uintptr_t expected_vptr = (uintptr_t)original + 0x103c918;
    void *all = local_options_all(&engine);
    if (all != (void *)((uintptr_t)original + 0x22d64f0) || *(uintptr_t *)all != expected_vptr) return 4;
    const uint32_t groups[] = {0, 0xfbb4e6a9, UINT32_MAX};
    typedef const unsigned char *(*get_value_fn)(void *, uint32_t);
    for (size_t i = 0; i < sizeof groups / sizeof groups[0]; ++i) {
        void *map = local_options_group(&engine, groups[i]);
        if (map != (void *)((uintptr_t)original + 0x22d6550) || *(uintptr_t *)map != expected_vptr) return 5;
        get_value_fn get = (get_value_fn)(*(uintptr_t **)map)[13];
        const unsigned char *first = get(map, 0xc181eb6e);
        const unsigned char *second = get(map, 0xc181eb6e);
        if (!first || first != second) return 6;
        uint16_t type; uint32_t flags;
        memcpy(&type, first, sizeof type);
        memcpy(&flags, first + 0x14, sizeof flags);
        printf("{\"group\":%u,\"native_fallback_identity\":true,\"native_property_lookup\":true,\"repeated_reference_identity\":true,\"type\":%u,\"flags\":%u}\n", groups[i], type, flags);
    }
    ((void (*)(void))((uintptr_t)original + 0xf7d5e0))();
    ((void (*)(void))((uintptr_t)original + 0xf7d5d0))();
    puts("{\"status\":\"ok\",\"groups\":3,\"original_map_cleanup\":true,\"app_options_imported\":false,\"gameplay_parity\":false}");
    return 0;
}
