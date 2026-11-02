/* Target-version signatures: evidence/linux/runtime-context-abi.json. */
#include <stdbool.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include "host_services.h"
#include "context_initialize.h"

static bool localization_started;
static bool host_prepared;
static void *utility;

/* No independent Context is created here. The original mod framework may own
 * context creation/destruction during its BootstrapContent transitions. */
int c6_linux_prepare_host(void) {
    if(host_prepared)return 0;
    fprintf(stderr, "C6_LINUX host_services_enter\n");
    utility = c6_linux_create_host_engine_utility();
    if (!utility) {
        fprintf(stderr, "C6_LINUX host_services_null\n");
        return 46;
    }
    fprintf(stderr, "C6_LINUX host_services_return\n");
    if(getenv("C6_HOST_NETWORK")){
        int (*guards)(void)=dlsym(RTLD_DEFAULT,"c6_linux_network_check_guards");
        if(!guards)return 149;
        int result=guards();if(result)return result;
    }
    if(getenv("C6_LOCALIZATION")){
        int (*localize)(void)=dlsym(RTLD_DEFAULT,"c6_linux_initialize_localization");
        if(!localize)return 54;
        fprintf(stderr,"C6_LINUX localization_enter\n");
        int result=localize();
        fprintf(stderr,"C6_LINUX localization_return %d\n",result);
        if(result)return 55;
        localization_started=true;
    }
    host_prepared=true;
    return 0;
}

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
    int preparation=c6_linux_prepare_host();
    if(preparation)return preparation;
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
    const char *configuration_path=getenv("C6_SAVED_CONFIGURATION");
    if(configuration_path){
        int (*restore)(const char*)=dlsym(RTLD_DEFAULT,"c6_linux_restore_saved_configuration");
        if(!restore)return 57;
        fprintf(stderr,"C6_LINUX saved_configuration_enter\n");
        int result=restore(configuration_path);
        fprintf(stderr,"C6_LINUX saved_configuration_return %d\n",result);
        if(result)return 58;
    }
    if(getenv("C6_RULES_SNAPSHOT_DIR")){
        int (*snapshot)(void*,void*)=dlsym(RTLD_DEFAULT,"c6_linux_rules_snapshot");
        if(!snapshot)return 59;
        fprintf(stderr,"C6_LINUX rules_snapshot_enter\n");
        int result=snapshot(game,context);
        fprintf(stderr,"C6_LINUX rules_snapshot_return %d\n",result);
        if(result)return result;
    }
    if(getenv("C6_SAVE_LIFECYCLE_CHECK")){
        int (*check)(void)=dlsym(RTLD_DEFAULT,"c6_linux_save_lifecycle_check");
        if(!check)return 77;
        fprintf(stderr,"C6_LINUX save_lifecycle_check_enter\n");
        int result=check();
        fprintf(stderr,"C6_LINUX save_lifecycle_check_return %d\n",result);
        if(result)return result;
    }
    if (getenv("C6_STARTUP")) {
        void (*startup)(void *) = dlsym(game,
            "_ZN8GameCore7Exports7Context11StartupGameEv");
        if (!startup) return 48;
        fprintf(stderr, "C6_LINUX startup_enter\n");
        startup(context);
        fprintf(stderr, "C6_LINUX startup_return\n");
    }
    if(getenv("C6_NETWORK_HOST")){
        int (*host)(void)=dlsym(RTLD_DEFAULT,"c6_linux_network_host_offline");
        int (*guards)(void)=dlsym(RTLD_DEFAULT,"c6_linux_network_check_guards");
        if(!host||!guards)return 151;
        fprintf(stderr,"C6_LINUX network_host_enter\n");
        int result=host();
        fprintf(stderr,"C6_LINUX network_host_return %d\n",result);
        if(result)return result;
        result=guards();if(result)return result;
    }
    const char *load_path=getenv("C6_LOAD_STREAM");
    if(load_path){
        int (*load)(void*,void*,const char*)=dlsym(RTLD_DEFAULT,"c6_linux_context_load");
        if(!load)return 52;
        int result=load(game,context,load_path);if(result)return result;
        const char *save_path=getenv("C6_SAVE_OUTPUT");
        if(save_path){
            int (*save)(void*,void*,const char*)=dlsym(RTLD_DEFAULT,"c6_linux_context_save");
            if(!save)return 53;
            result=save(game,context,save_path);if(result)return result;
        }
    }
    return 0;
}

int c6_linux_context_finish(void) {
    if(localization_started){
        int (*shutdown)(void)=dlsym(RTLD_DEFAULT,"c6_linux_shutdown_localization");
        if(!shutdown)return 56;
        int result=shutdown();if(result)return result;
        localization_started=false;
    }
    int result=c6_linux_destroy_host_services();
    if(!result){host_prepared=false;utility=NULL;}
    return result;
}
