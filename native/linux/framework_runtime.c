#define _GNU_SOURCE
#include "framework_runtime.h"
#include "mod_activation.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int observe_context(void *context) {
    Dl_info module;
    if(!context || !dladdr(*(void**)context,&module) || !module.dli_fname)return 140;
    const char *name=strrchr(module.dli_fname,'/');name=name?name+1:module.dli_fname;
    if(strcmp(name,"libGameCore_Base.so") && strcmp(name,"libGameCore_XP1.so") &&
       strcmp(name,"libGameCore_XP2.so"))return 141;
    if(!module.dli_sname || strcmp(module.dli_sname,"_ZTVN8GameCore7Exports7ContextE"))return 146;
    fprintf(stderr,"C6_LINUX_FRAMEWORK borrowed_context_path %s\n",module.dli_fname);
    fprintf(stderr,"C6_LINUX_FRAMEWORK borrowed_context_vtable %s\n",module.dli_sname);
    /* The module is gated externally. No XP2 RVA/method is applied to Base. */
    fprintf(stderr,"C6_LINUX_FRAMEWORK borrowed_context_module %s\n",name);
    return 0;
}

int c6_linux_framework_run(void) {
    bool (*directory)(const char*)=dlsym(RTLD_DEFAULT,"_ZN3ASL17ASL_SetCurrentDirEPKc");
    if(!directory || !directory("C:\\Emu\\AppAssets\\Base\\Binaries\\Linux"))return 142;
    fprintf(stderr,"C6_LINUX_FRAMEWORK asset_working_directory_return\n");
    void *context=NULL;
    fprintf(stderr,"C6_LINUX_FRAMEWORK initialize_enter\n");
    int result=c6_linux_mod_framework_initialize(&context);
    fprintf(stderr,"C6_LINUX_FRAMEWORK initialize_return %d\n",result);
    if(result)return result;
    result=observe_context(context);if(result)return result;
    void **application=dlsym(RTLD_DEFAULT,"g_pkApplication");
    void *(*configuration_connection)(void*)=dlsym(RTLD_DEFAULT,
        "_ZN7Civ6App34GetConfigurationDatabaseConnectionEv");
    void *(*gameplay_connection)(void*)=dlsym(RTLD_DEFAULT,
        "_ZN7Civ6App29GetGameplayDatabaseConnectionEv");
    if(!application||!*application||!configuration_connection||!gameplay_connection)return 147;
    bool configuration_ready=configuration_connection(*application)!=NULL;
    bool gameplay_ready=gameplay_connection(*application)!=NULL;
    fprintf(stderr,"C6_LINUX_FRAMEWORK app_database_connections configuration %d gameplay %d\n",
            configuration_ready,gameplay_ready);
    if(!configuration_ready||!gameplay_ready)return 148;
    const char *saved=getenv("C6_SAVED_CONFIGURATION");
    if(saved){
        int (*restore)(const char*)=dlsym(RTLD_DEFAULT,"c6_linux_restore_saved_configuration");
        if(!restore)return 143;
        fprintf(stderr,"C6_LINUX saved_configuration_enter\n");
        result=restore(saved);
        fprintf(stderr,"C6_LINUX saved_configuration_return %d\n",result);
        if(result)return result;
    }
    if(getenv("C6_ATTACH_SAVED_SESSION")){
        int (*attach)(void)=dlsym(RTLD_DEFAULT,"c6_linux_save_lifecycle_attach_session");
        int (*guards)(void)=dlsym(RTLD_DEFAULT,"c6_linux_network_check_guards");
        if(!attach||!guards)return 149;
        fprintf(stderr,"C6_LINUX_FRAMEWORK attach_session_enter\n");
        result=attach();
        fprintf(stderr,"C6_LINUX_FRAMEWORK attach_session_return %d\n",result);
        if(result)return result;
        result=guards();if(result)return result;
    }
    if(getenv("C6_FRAMEWORK_CONFIGURE")){
        fprintf(stderr,"C6_LINUX_FRAMEWORK configure_enter\n");
        result=c6_linux_mod_framework_configure();
        fprintf(stderr,"C6_LINUX_FRAMEWORK configure_return %d\n",result);
        if(result)return result;
    }
    if(getenv("C6_RULES_SNAPSHOT_DIR")){
        int (*snapshot)(void*,void*)=dlsym(RTLD_DEFAULT,"c6_linux_rules_snapshot");
        if(!snapshot)return 144;
        fprintf(stderr,"C6_LINUX rules_snapshot_enter\n");
        result=snapshot(NULL,context);
        fprintf(stderr,"C6_LINUX rules_snapshot_return %d\n",result);
        if(result)return result;
    }
    if(getenv("C6_ATTACH_SAVED_SESSION")){
        int (*leave)(void)=dlsym(RTLD_DEFAULT,"c6_linux_network_leave");
        if(!leave)return 150;
        fprintf(stderr,"C6_LINUX_FRAMEWORK leave_session_enter\n");
        result=leave();
        fprintf(stderr,"C6_LINUX_FRAMEWORK leave_session_return %d\n",result);
        if(result)return result;
    }
    fprintf(stderr,"C6_LINUX_FRAMEWORK shutdown_enter\n");
    result=c6_linux_mod_framework_shutdown();
    fprintf(stderr,"C6_LINUX_FRAMEWORK shutdown_return %d\n",result);
    if(result)return result;
    context=NULL;
    result=c6_linux_mod_client_fixture_shutdown();
    fprintf(stderr,"C6_LINUX_FRAMEWORK client_shutdown_return %d\n",result);
    return result;
}
