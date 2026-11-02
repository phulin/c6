/* Genuine original sessionless NetworkManager identity and cleanup probe.
 * Whole module/provider hashes are gated by the Python launcher. */
#include "service_fixture.c"
static const char *network_phase = "start";
static LONG CALLBACK network_fault(EXCEPTION_POINTERS *info) {
    fprintf(stderr, "{\"phase\":\"%s\",\"exception\":\"0x%08lx\",\"exe_rva\":\"0x%llx\",\"rcx\":\"0x%llx\"}\n", network_phase, info->ExceptionRecord->ExceptionCode, (unsigned long long)(info->ContextRecord->Rip - (uintptr_t)original_host_image), (unsigned long long)info->ContextRecord->Rcx);
    fflush(stderr);
    return EXCEPTION_EXECUTE_HANDLER;
}
int wmain(int argc, wchar_t **argv) {
    SetUnhandledExceptionFilter(network_fault);
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    if (argc != 3 || !SetDllDirectoryW(argv[2])) return 2;
    HMODULE original = LoadLibraryExW(argv[1], NULL, DONT_RESOLVE_DLL_REFERENCES);
    if (!bootstrap_service_fixture_prepare_original_host_services(original)) return 3;
    network_phase = "original_network_creator";
    if (!bootstrap_service_fixture_prepare_original_sessionless_network(original)) return 4;
    void **singleton = (void **)((uintptr_t)original + 0x22ee8c8);
    void *manager = *singleton;
    if (!manager || ((void **)manager)[1]) return 5;
    network_phase = "original_sessionless_identity";
    for (unsigned i = 0; i < 3; ++i) {
        uint32_t direct = ((uint32_t (*)(void *))((uintptr_t)original + 0x27ceb0))(manager);
        uint32_t utility = network_local_player_id(&engine);
        printf("{\"case\":%u,\"native_manager_identity\":%u,\"native_utility_identity\":%u,\"session_null\":true}\n", i, direct, utility);
        if (direct != UINT32_MAX || direct != utility) return 6;
    }
    if (bootstrap_service_fixture_prepare_original_sessionless_network(original) || *singleton != manager) return 7;
    puts("{\"phase\":\"existing_network_provider_rejected_unchanged\"}");
    network_phase = "original_network_destructor";
    ((void (*)(void *))((uintptr_t)original + 0x276440))(manager);
    network_phase = "original_network_deallocator";
    ((void (*)(void *))((uintptr_t)original + 0x982080))(manager);
    /* Explicit fixture lifetime cleanup restores the originally null singleton
     * only after full original destruction and the paired original deallocator. */
    *singleton = NULL;
    puts("{\"status\":\"ok\",\"native_creator\":true,\"native_destructor\":true,\"session_created\":false,\"local_player_selected\":false,\"gameplay_parity\":false}");
    return 0;
}
