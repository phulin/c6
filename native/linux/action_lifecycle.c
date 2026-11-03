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

typedef enum {
    QUERY_NONE,
    QUERY_MOVEMENT_COST,
    QUERY_DOMAIN_METADATA,
    QUERY_PLACEMENT_METADATA,
    QUERY_PLACEMENT_HANDLER_METADATA,
    QUERY_UNIT_OPERATION_MANAGER_METADATA,
    QUERY_CURRENT_HANDLER_METADATA
} QueryProfile;

static _Noreturn void query_fatal(QueryProfile profile,const char *stage) {
    const char *marker=profile==QUERY_CURRENT_HANDLER_METADATA?
        "C6_LINUX_CURRENT_HANDLER_METADATA_LIFECYCLE":profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?
        "C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_LIFECYCLE":profile==QUERY_PLACEMENT_HANDLER_METADATA?
        "C6_LINUX_PLACEMENT_HANDLER_METADATA_LIFECYCLE":profile==QUERY_PLACEMENT_METADATA?
        "C6_LINUX_PLACEMENT_METADATA_LIFECYCLE":profile==QUERY_DOMAIN_METADATA?
        "C6_LINUX_DOMAIN_METADATA_LIFECYCLE":"C6_LINUX_MOVEMENT_LIFECYCLE";
    fprintf(stderr,"%s fatal %s process_discarded 1\n",marker,stage);
    fflush(stderr);
    _exit(184);
}

static int preparation_failure(QueryProfile profile,const char *stage,int legacy_code) {
    if(profile!=QUERY_NONE)query_fatal(profile,stage);
    return legacy_code;
}

static int prepare(void *module,void *context,bool dispatch,bool execute,bool register_session,bool core_component,QueryProfile profile) {
    bool movement_cost=profile!=QUERY_NONE;
    if(profile==QUERY_CURRENT_HANDLER_METADATA)
        fprintf(stderr,"C6_LINUX_CURRENT_HANDLER_METADATA_LIFECYCLE mode current_handler_metadata read_only 1\n");
    if(profile==QUERY_UNIT_OPERATION_MANAGER_METADATA)
        fprintf(stderr,"C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_LIFECYCLE mode unit_operation_manager_metadata read_only 1\n");
    if(profile==QUERY_PLACEMENT_HANDLER_METADATA)
        fprintf(stderr,"C6_LINUX_PLACEMENT_HANDLER_METADATA_LIFECYCLE mode placement_handler_metadata read_only 1\n");
    if(profile==QUERY_PLACEMENT_METADATA)
        fprintf(stderr,"C6_LINUX_PLACEMENT_METADATA_LIFECYCLE mode placement_metadata read_only 1\n");
    if(profile==QUERY_DOMAIN_METADATA)
        fprintf(stderr,"C6_LINUX_DOMAIN_METADATA_LIFECYCLE mode domain_metadata read_only 1\n");
    if(!module||!context)return preparation_failure(profile,"missing_input",194);
    void **active=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController21ms_pkActiveControllerE");
    bool (*initialized)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController13IsInitializedEv");
    void *(*get_context)(void *,const void *)=dlsym(RTLD_DEFAULT,"_ZNK18GameCoreController10GetContextEv");
    void (*initialize)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController10InitializeEv");
    void (*initialize_cache)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController15InitializeCacheEv");
    void (*uninitialize)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController12UninitializeEv");
    bool (*executing)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController11IsExecutingEv");
    if(!active||!initialized||!get_context||!initialize||!initialize_cache||!uninitialize||!executing)return preparation_failure(profile,"missing_lifecycle_symbol",195);
    int (*trace_begin)(void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_trace_begin");
    int (*trace_end)(unsigned)=dlsym(RTLD_DEFAULT,"c6_linux_action_trace_end");
    if(!trace_begin||!trace_end)return preparation_failure(profile,"missing_trace_symbol",202);
    void *controller=*active,*bound=NULL;
    if(!controller)return preparation_failure(profile,"missing_controller",196);
    get_context(&bound,controller);
    /* Existing Context identity is borrowed. The genuine framework owns it.
     * Initialize/Uninitialize owns the controller/cache lifecycle only. */
    if(bound!=context||initialized(controller)||executing(controller))return preparation_failure(profile,"initial_identity_or_state",197);
    int trace_result=trace_begin(module);
    if(trace_result)return preparation_failure(profile,"trace_begin_failed",trace_result);
    if(register_session){
        int registration=c6_linux_action_registration(module,context);
        if(registration){(void)trace_end(0);return registration;}
    }
    fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_enter bound_matches 1\n");
    initialize(controller);
    if(movement_cost&&*active!=controller)query_fatal(profile,"controller_changed_during_initialize");
    int result=initialized(controller)?0:199;
    if(movement_cost&&result)query_fatal(profile,"initialize_failed");
    if(!result){
        fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_cache_enter\n");
        initialize_cache(controller);
        fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE initialize_cache_return\n");
        if(movement_cost){
            if(*active!=controller)query_fatal(profile,"controller_changed_during_cache");
            bound=NULL;get_context(&bound,controller);
            if(!initialized(controller)||bound!=context||executing(controller))query_fatal(profile,"post_cache_identity_or_state");
            /* Current-handler lifecycle boundaries are unlocked and field-only.
             * Its three-argument locked helper guard is a separate ABI and is
             * owned by the probe, alongside genuine facade acquire/release. */
            int (*caller_idle)(void *,void *)=dlsym(RTLD_DEFAULT,profile==QUERY_CURRENT_HANDLER_METADATA?
                "c6_linux_current_handler_metadata_caller_idle":profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?
                "c6_linux_unit_operation_manager_metadata_quiescent":profile==QUERY_PLACEMENT_HANDLER_METADATA?
                "c6_linux_placement_handler_metadata_quiescent":profile==QUERY_PLACEMENT_METADATA?
                "c6_linux_placement_metadata_quiescent":profile==QUERY_DOMAIN_METADATA?
                "c6_linux_domain_metadata_quiescent":"c6_linux_movement_cost_quiescent");
            int (*probe)(void *,void *)=dlsym(RTLD_DEFAULT,profile==QUERY_CURRENT_HANDLER_METADATA?
                "c6_linux_current_handler_metadata_probe":profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?
                "c6_linux_unit_operation_manager_metadata_fixture_probe":profile==QUERY_PLACEMENT_HANDLER_METADATA?
                "c6_linux_placement_handler_metadata_probe":profile==QUERY_PLACEMENT_METADATA?
                "c6_linux_placement_metadata_probe":profile==QUERY_DOMAIN_METADATA?
                "c6_linux_domain_metadata_probe":"c6_linux_movement_cost_probe");
            int (*admission_begin)(void *,void *)=dlsym(RTLD_DEFAULT,profile==QUERY_CURRENT_HANDLER_METADATA?
                "c6_linux_current_handler_metadata_admission_begin":profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?
                "c6_linux_unit_operation_manager_metadata_admission_begin":profile==QUERY_PLACEMENT_HANDLER_METADATA?
                "c6_linux_placement_handler_metadata_admission_begin":profile==QUERY_PLACEMENT_METADATA?
                "c6_linux_placement_metadata_admission_begin":profile==QUERY_DOMAIN_METADATA?
                "c6_linux_domain_metadata_admission_begin":"c6_linux_movement_cost_admission_begin");
            int (*admission_end)(void *,void *)=dlsym(RTLD_DEFAULT,profile==QUERY_CURRENT_HANDLER_METADATA?
                "c6_linux_current_handler_metadata_admission_end":profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?
                "c6_linux_unit_operation_manager_metadata_admission_end":profile==QUERY_PLACEMENT_HANDLER_METADATA?
                "c6_linux_placement_handler_metadata_admission_end":profile==QUERY_PLACEMENT_METADATA?
                "c6_linux_placement_metadata_admission_end":profile==QUERY_DOMAIN_METADATA?
                "c6_linux_domain_metadata_admission_end":"c6_linux_movement_cost_admission_end");
            if(!caller_idle||!probe||!admission_begin||!admission_end)query_fatal(profile,
                profile==QUERY_CURRENT_HANDLER_METADATA?"missing_current_handler_metadata_callback":profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?"missing_unit_operation_manager_metadata_callback":profile==QUERY_PLACEMENT_HANDLER_METADATA?"missing_placement_handler_metadata_callback":profile==QUERY_PLACEMENT_METADATA?"missing_placement_metadata_callback":profile==QUERY_DOMAIN_METADATA?"missing_domain_metadata_callback":"missing_movement_callback");
            if(admission_begin(module,context))query_fatal(profile,"admission_begin_failed");
            if(caller_idle(module,context))query_fatal(profile,"post_cache_not_quiescent");
            result=probe(module,context);
            if(caller_idle(module,context))query_fatal(profile,"post_probe_not_quiescent");
            if(*active!=controller)query_fatal(profile,"controller_changed_during_probe");
            bound=NULL;get_context(&bound,controller);
            if(!initialized(controller)||bound!=context||executing(controller))query_fatal(profile,"post_probe_identity_or_state");
            /* Drop addresses before controller/cache teardown. An unknown/live
             * path above still exits without poststate or Uninitialize. */
            if(admission_end(module,context))query_fatal(profile,"admission_end_failed");
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
    if(movement_cost&&*active!=controller)query_fatal(profile,"controller_changed_during_uninitialize");
    bound=NULL;get_context(&bound,controller);
    bool clean=!initialized(controller)&&bound==NULL&&!executing(controller);
    if(movement_cost&&!clean)query_fatal(profile,"uninitialize_not_clean");
    fprintf(stderr,"C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized %d context_bound %d executing %d cleanup %d dispatch_mode %d requested_execution %d core_component_mode %d\n",initialized(controller),bound!=NULL,executing(controller),clean,dispatch,execute,core_component);
    trace_result=trace_end((dispatch||core_component)&&execute?1:0);
    return result?result:!clean?200:trace_result;
}

int c6_linux_action_prepare_observe(void *module,void *context) {
    return prepare(module,context,false,false,false,false,QUERY_NONE);
}
int c6_linux_action_prepare_dispatch(void *module,void *context,bool execute) {
    return prepare(module,context,true,execute,false,false,QUERY_NONE);
}
int c6_linux_action_prepare_registered_dispatch(void *module,void *context,bool execute) {
    return prepare(module,context,true,execute,true,false,QUERY_NONE);
}
/* A separate component experiment admits trusted original command bytes at
 * the Core ingress. It never claims successful host transport/readiness. */
int c6_linux_action_prepare_core_component(void *module,void *context,bool execute) {
    return prepare(module,context,false,execute,false,true,QUERY_NONE);
}
/* Read-only query profile. The callback owns its quiescent observations around
 * each internal Save and query block; this wrapper never schedules a worker. */
int c6_linux_action_prepare_movement_cost(void *module,void *context) {
    return prepare(module,context,false,false,false,false,QUERY_MOVEMENT_COST);
}

/* Separate read-only metadata profile; independent callback/admission scope. */
int c6_linux_action_prepare_domain_metadata(void *module,void *context) {
    return prepare(module,context,false,false,false,false,QUERY_DOMAIN_METADATA);
}

/* Separate read-only placement-count profile with independently admitted owners. */
int c6_linux_action_prepare_placement_metadata(void *module,void *context) {
    return prepare(module,context,false,false,false,false,QUERY_PLACEMENT_METADATA);
}

/* Separate original handler predicate queries, not Manager/action admission. */
int c6_linux_action_prepare_placement_handler_metadata(void *module,void *context) {
    return prepare(module,context,false,false,false,false,QUERY_PLACEMENT_HANDLER_METADATA);
}

int c6_linux_action_prepare_unit_operation_manager_metadata(void *module,void *context) {
    return prepare(module,context,false,false,false,false,QUERY_UNIT_OPERATION_MANAGER_METADATA);
}

/* Passive current-handler capture has its own unlocked caller admission scope.
 * The probe alone owns original facade lifetime and diagnostic Save calls. */
int c6_linux_action_prepare_current_handler_metadata(void *module,void *context) {
    return prepare(module,context,false,false,false,false,QUERY_CURRENT_HANDLER_METADATA);
}
