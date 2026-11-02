#define _GNU_SOURCE
#include "action_lifecycle.h"
#include "action_execution.h"
#include "action_observer.h"
#include "action_dispatch.h"
#include "action_registration.h"
#include <stdbool.h>
#include <dlfcn.h>
#include <stdio.h>

static int prepare(void *module,void *context,bool dispatch,bool execute,bool register_session,bool core_component) {
    if(!module||!context)return 194;
    void **active=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController21ms_pkActiveControllerE");
    bool (*initialized)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController13IsInitializedEv");
    void *(*get_context)(void *,const void *)=dlsym(RTLD_DEFAULT,"_ZNK18GameCoreController10GetContextEv");
    void (*initialize)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController10InitializeEv");
    void (*initialize_cache)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController15InitializeCacheEv");
    void (*uninitialize)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController12UninitializeEv");
    bool (*executing)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController11IsExecutingEv");
    if(!active||!initialized||!get_context||!initialize||!initialize_cache||!uninitialize||!executing)return 195;
    int (*trace_begin)(void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_trace_begin");
    int (*trace_end)(unsigned)=dlsym(RTLD_DEFAULT,"c6_linux_action_trace_end");
    if(!trace_begin||!trace_end)return 202;
    void *controller=*active,*bound=NULL;
    if(!controller)return 196;
    get_context(&bound,controller);
    /* Existing Context identity is borrowed. The genuine framework owns it.
     * Initialize/Uninitialize owns the controller/cache lifecycle only. */
    if(bound!=context||initialized(controller)||executing(controller))return 197;
    int trace_result=trace_begin(module);
    if(trace_result)return trace_result;
    if(register_session){
        int registration=c6_linux_action_registration(module,context);
        if(registration){(void)trace_end(0);return registration;}
    }
    fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_enter bound_matches 1\n");
    initialize(controller);
    int result=initialized(controller)?0:199;
    if(!result){
        fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_cache_enter\n");
        initialize_cache(controller);
        fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_cache_return\n");
        result=c6_linux_action_execution(module,context);
        if(!result)result=c6_linux_action_observe(module,context);
        if(!result&&dispatch)result=c6_linux_action_dispatch(module,context,execute);
        if(!result&&core_component){
            int (*component)(void *,void *,bool)=dlsym(RTLD_DEFAULT,"c6_linux_core_component");
            result=component?component(module,context,execute):251;
        }
    }
    fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE uninitialize_enter result %d\n",result);
    uninitialize(controller);
    bound=NULL;get_context(&bound,controller);
    bool clean=!initialized(controller)&&bound==NULL&&!executing(controller);
    fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized %d context_bound %d executing %d cleanup %d dispatch_mode %d requested_execution %d core_component_mode %d\n",initialized(controller),bound!=NULL,executing(controller),clean,dispatch,execute,core_component);
    trace_result=trace_end((dispatch||core_component)&&execute?1:0);
    return result?result:!clean?200:trace_result;
}

int c6_linux_action_prepare_observe(void *module,void *context) {
    return prepare(module,context,false,false,false,false);
}
int c6_linux_action_prepare_dispatch(void *module,void *context,bool execute) {
    return prepare(module,context,true,execute,false,false);
}
int c6_linux_action_prepare_registered_dispatch(void *module,void *context,bool execute) {
    return prepare(module,context,true,execute,true,false);
}
/* A separate component experiment admits trusted original command bytes at
 * the Core ingress. It never claims successful host transport/readiness. */
int c6_linux_action_prepare_core_component(void *module,void *context,bool execute) {
    return prepare(module,context,false,execute,false,true);
}
