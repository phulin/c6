#define _GNU_SOURCE
#include "action_lifecycle.h"
#include "action_execution.h"
#include "action_observer.h"
#include "action_dispatch.h"
#include "action_registration.h"
#include <stdbool.h>
#include <dlfcn.h>
#include <stdio.h>
#include <unistd.h>

static _Noreturn void movement_fatal(const char *stage) {
    fprintf(stderr,"C6_LINUX_MOVEMENT_LIFECYCLE fatal %s process_discarded 1\n",stage);
    fflush(stderr);
    _exit(184);
}

static int preparation_failure(bool movement_cost,const char *stage,int legacy_code) {
    if(movement_cost)movement_fatal(stage);
    return legacy_code;
}

static int prepare(void *module,void *context,bool dispatch,bool execute,bool register_session,bool core_component,bool movement_cost) {
    if(!module||!context)return preparation_failure(movement_cost,"missing_input",194);
    void **active=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController21ms_pkActiveControllerE");
    bool (*initialized)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController13IsInitializedEv");
    void *(*get_context)(void *,const void *)=dlsym(RTLD_DEFAULT,"_ZNK18GameCoreController10GetContextEv");
    void (*initialize)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController10InitializeEv");
    void (*initialize_cache)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController15InitializeCacheEv");
    void (*uninitialize)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController12UninitializeEv");
    bool (*executing)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController11IsExecutingEv");
    if(!active||!initialized||!get_context||!initialize||!initialize_cache||!uninitialize||!executing)return preparation_failure(movement_cost,"missing_lifecycle_symbol",195);
    int (*trace_begin)(void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_trace_begin");
    int (*trace_end)(unsigned)=dlsym(RTLD_DEFAULT,"c6_linux_action_trace_end");
    if(!trace_begin||!trace_end)return preparation_failure(movement_cost,"missing_trace_symbol",202);
    void *controller=*active,*bound=NULL;
    if(!controller)return preparation_failure(movement_cost,"missing_controller",196);
    get_context(&bound,controller);
    /* Existing Context identity is borrowed. The genuine framework owns it.
     * Initialize/Uninitialize owns the controller/cache lifecycle only. */
    if(bound!=context||initialized(controller)||executing(controller))return preparation_failure(movement_cost,"initial_identity_or_state",197);
    int trace_result=trace_begin(module);
    if(trace_result)return preparation_failure(movement_cost,"trace_begin_failed",trace_result);
    if(register_session){
        int registration=c6_linux_action_registration(module,context);
        if(registration){(void)trace_end(0);return registration;}
    }
    fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_enter bound_matches 1\n");
    initialize(controller);
    if(movement_cost&&*active!=controller)movement_fatal("controller_changed_during_initialize");
    int result=initialized(controller)?0:199;
    if(movement_cost&&result)movement_fatal("initialize_failed");
    if(!result){
        fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_cache_enter\n");
        initialize_cache(controller);
        fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_cache_return\n");
        if(movement_cost){
            if(*active!=controller)movement_fatal("controller_changed_during_cache");
            bound=NULL;get_context(&bound,controller);
            if(!initialized(controller)||bound!=context||executing(controller))movement_fatal("post_cache_identity_or_state");
            int (*quiescent)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_movement_cost_quiescent");
            int (*probe)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_movement_cost_probe");
            if(!quiescent||!probe)movement_fatal("missing_movement_callback");
            if(quiescent(module,context))movement_fatal("post_cache_not_quiescent");
            result=probe(module,context);
            if(quiescent(module,context))movement_fatal("post_probe_not_quiescent");
            if(*active!=controller)movement_fatal("controller_changed_during_probe");
            bound=NULL;get_context(&bound,controller);
            if(!initialized(controller)||bound!=context||executing(controller))movement_fatal("post_probe_identity_or_state");
        } else {
            result=c6_linux_action_execution(module,context);
            if(!result)result=c6_linux_action_observe(module,context);
            if(!result&&dispatch)result=c6_linux_action_dispatch(module,context,execute);
            if(!result&&core_component){
                int (*component)(void *,void *,bool)=dlsym(RTLD_DEFAULT,"c6_linux_core_component");
                result=component?component(module,context,execute):251;
            }
        }
    }
    fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE uninitialize_enter result %d\n",result);
    uninitialize(controller);
    if(movement_cost&&*active!=controller)movement_fatal("controller_changed_during_uninitialize");
    bound=NULL;get_context(&bound,controller);
    bool clean=!initialized(controller)&&bound==NULL&&!executing(controller);
    if(movement_cost&&!clean)movement_fatal("uninitialize_not_clean");
    fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized %d context_bound %d executing %d cleanup %d dispatch_mode %d requested_execution %d core_component_mode %d\n",initialized(controller),bound!=NULL,executing(controller),clean,dispatch,execute,core_component);
    trace_result=trace_end((dispatch||core_component)&&execute?1:0);
    return result?result:!clean?200:trace_result;
}

int c6_linux_action_prepare_observe(void *module,void *context) {
    return prepare(module,context,false,false,false,false,false);
}
int c6_linux_action_prepare_dispatch(void *module,void *context,bool execute) {
    return prepare(module,context,true,execute,false,false,false);
}
int c6_linux_action_prepare_registered_dispatch(void *module,void *context,bool execute) {
    return prepare(module,context,true,execute,true,false,false);
}
/* A separate component experiment admits trusted original command bytes at
 * the Core ingress. It never claims successful host transport/readiness. */
int c6_linux_action_prepare_core_component(void *module,void *context,bool execute) {
    return prepare(module,context,false,execute,false,true,false);
}
/* Read-only query profile. The callback owns its quiescent observations around
 * each internal Save and query block; this wrapper never schedules a worker. */
int c6_linux_action_prepare_movement_cost(void *module,void *context) {
    return prepare(module,context,false,false,false,false,true);
}
