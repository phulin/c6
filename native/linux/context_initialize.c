/* Target-version signatures: evidence/linux/runtime-context-abi.json. */
#include <stdbool.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include "host_services.h"

int c6_linux_context_initialize(void *game, void *context) {
    void (*set_engine)(void *, void *) = dlsym(game,
        "_ZN8GameCore7Exports7Context18SetEngineInterfaceEPNS_10Interfaces15IEngineUtility1E");
    void (*initialize)(void *) = dlsym(game,
        "_ZN8GameCore7Exports7Context10InitializeEv");
    bool (*is_initialized)(void *) = dlsym(game,
        "_ZN8GameCore7Exports7Context12IsInitialiedEv");
    if (!set_engine || !initialize || !is_initialized) {
        fprintf(stderr, "C6_LINUX initialize_symbols_missing\n");
        return 45;
    }
    fprintf(stderr, "C6_LINUX host_services_enter\n");
    void *utility = c6_linux_create_host_engine_utility();
    if (!utility) {
        fprintf(stderr, "C6_LINUX host_services_null\n");
        return 46;
    }
    fprintf(stderr, "C6_LINUX host_services_return\n");
    fprintf(stderr, "C6_LINUX set_engine_enter\n");
    set_engine(context, utility);
    fprintf(stderr, "C6_LINUX set_engine_return\n");
    fprintf(stderr, "C6_LINUX is_initialized_before %d\n", is_initialized(context));
    fprintf(stderr, "C6_LINUX initialize_enter\n");
    /* Facade returns void. Internal Instance::Initialize bool is not its ABI. */
    initialize(context);
    fprintf(stderr, "C6_LINUX initialize_return\n");
    bool initialized = is_initialized(context);
    fprintf(stderr, "C6_LINUX is_initialized_after %d\n", initialized);
    if (!initialized) return 47;
    if (getenv("C6_STARTUP")) {
        void (*startup)(void *) = dlsym(game,
            "_ZN8GameCore7Exports7Context11StartupGameEv");
        if (!startup) return 48;
        fprintf(stderr, "C6_LINUX startup_enter\n");
        startup(context);
        fprintf(stderr, "C6_LINUX startup_return\n");
    }
    return 0;
}

int c6_linux_context_finish(void) {
    return c6_linux_destroy_host_services();
}
