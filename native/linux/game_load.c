#define _GNU_SOURCE
#include "game_load.h"
#include "context_stream.h"
#include <dlfcn.h>
#include <errno.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* A potentially active/unknown owner must never return into fixture teardown.
 * This is a caller-side observation gate, not an original-service replacement. */
static _Noreturn void movement_not_quiescent(const char *reason) {
    fprintf(stderr,"C6_LINUX_MOVEMENT_QUIESCENCE {\"fatal\":true,\"reason\":\"%s\",\"poststate_allowed\":false,\"cleanup_proven\":false}\n",reason);
    fflush(stderr);
    _exit(184);
}

static void *movement_symbol(void *owner,const char *name,uintptr_t expected,void *base) {
    void *symbol=dlsym(owner,name);Dl_info info={0};
    if(!symbol||(uintptr_t)symbol!=expected||!dladdr(symbol,&info)||info.dli_fbase!=base)
        movement_not_quiescent("original_binding_unknown");
    return symbol;
}

/* Single synchronous movement probe only. These are bindings, never snapshots
 * of controller/context state. The caller holds its RTLD_NOLOAD reference. */
typedef struct {
    void *module,*context,*vptr,*base;
    void **active;
    bool (*initialized)(void *);
    bool (*executing)(void *);
    void *(*get_context)(void *,const void *);
    bool (*context_initialized)(void *);
    bool (*pending)(void *);
    void *(*get_game)(void *);
    void (*release)(void *);
    bool (*needs_update)(const void *);
    bool (*needs_advance)(const void *);
    bool (*ai)(const void *);
    bool (*processing)(const void *);
} MovementBindings;
static MovementBindings movement_bindings;

static Dl_info movement_context_owner(void *module,void *context) {
    Dl_info info={0};
    if(!module||!context||!dladdr(*(void **)context,&info)||!info.dli_fbase||!info.dli_fname)
        movement_not_quiescent("context_owner_unknown");
    const char *leaf=strrchr(info.dli_fname,'/');leaf=leaf?leaf+1:info.dli_fname;
    if(strcmp(leaf,"libGameCore_XP2.so"))movement_not_quiescent("context_module_unknown");
    return info;
}

int c6_linux_movement_cost_admission_begin(void *module,void *context) {
    if(movement_bindings.module)movement_not_quiescent("admission_already_active");
    Dl_info info=movement_context_owner(module,context);
    uintptr_t base=(uintptr_t)info.dli_fbase;
    MovementBindings b={.module=module,.context=context,.vptr=*(void **)context,.base=info.dli_fbase};
#define MOVEMENT_CORE(name,rva) movement_symbol(module,name,base+(rva),info.dli_fbase)
#define MOVEMENT_HOST(name,address) movement_symbol(RTLD_DEFAULT,name,address,(void *)0x400000)
    b.active=MOVEMENT_HOST("_ZN18GameCoreController21ms_pkActiveControllerE",0x6499100);
    b.initialized=MOVEMENT_HOST("_ZN18GameCoreController13IsInitializedEv",0x31d1720);
    b.executing=MOVEMENT_HOST("_ZN18GameCoreController11IsExecutingEv",0x31d18a0);
    b.get_context=MOVEMENT_HOST("_ZNK18GameCoreController10GetContextEv",0x31d5090);
    b.context_initialized=MOVEMENT_CORE("_ZN8GameCore7Exports7Context12IsInitialiedEv",0x2c7c9b0);
    b.pending=MOVEMENT_CORE("_ZN8GameCore7Exports7Context18HasPendingMessagesEv",0x2c7d340);
    b.get_game=MOVEMENT_CORE("_ZN8GameCore7Exports7Context7GetGameEv",0x2c7cae0);
    b.release=MOVEMENT_CORE("_ZN8GameCore10Interfaces10ICvUnknowndlEPv",0x290bba0);
    b.needs_update=MOVEMENT_CORE("_ZNK8GameCore7Exports4Game11NeedsUpdateEv",0x2c86a60);
    b.needs_advance=MOVEMENT_CORE("_ZNK8GameCore7Exports4Game16NeedsTurnAdvanceEv",0x2c869f0);
    b.ai=MOVEMENT_CORE("_ZNK8GameCore7Exports4Game14IsAiProcessingEv",0x2c867e0);
    b.processing=MOVEMENT_CORE("_ZNK8GameCore7Exports4Game16IsTurnProcessingEv",0x2c86800);
#undef MOVEMENT_CORE
#undef MOVEMENT_HOST
    movement_bindings=b;
    return 0;
}

int c6_linux_movement_cost_admission_end(void *module,void *context) {
    if(!movement_bindings.module||movement_bindings.module!=module||movement_bindings.context!=context)
        movement_not_quiescent("admission_scope_unknown");
    movement_bindings=(MovementBindings){0};
    return 0;
}

int c6_linux_movement_cost_quiescent(void *module,void *context) {
    Dl_info info=movement_context_owner(module,context);
    const MovementBindings *b=&movement_bindings;
    if(!b->module||b->module!=module||b->context!=context||b->vptr!=*(void **)context||b->base!=info.dli_fbase)
        movement_not_quiescent("admission_scope_unknown");
    void *controller=*b->active,*bound=NULL;
    if(!controller)movement_not_quiescent("controller_unknown");
    b->get_context(&bound,controller);
    if(*b->active!=controller||bound!=context||!b->initialized(controller)||!b->context_initialized(context)||b->executing(controller)||b->pending(context))
        movement_not_quiescent("controller_or_context_not_idle");
    void *game=b->get_game(context);
    if(!game)movement_not_quiescent("game_facade_unavailable");
    bool work=b->needs_update(game)||b->needs_advance(game)||b->ai(game)||b->processing(game);
    /* Only the acquired interface wrapper is owned. Both Context and its native
     * Game remain borrowed; no facade survives this check or any cost call. */
    b->release(game);
    bound=NULL;b->get_context(&bound,controller);
    if(*b->active!=controller||bound!=context||!b->initialized(controller)||!b->context_initialized(context)||b->executing(controller)||b->pending(context)||work)
        movement_not_quiescent("game_or_controller_not_idle");
    return 0;
}

/* Matching target DWARF and exact exported method byte identities are checked
 * by tools/linux/game_load_state_audit.py. This observes the actual facade,
 * never a fabricated Game/Context receiver or a cache stand-in. */
static int observe_loaded_state(void *module,void *context) {
    void *(*get_game)(void *)=dlsym(module,"_ZN8GameCore7Exports7Context7GetGameEv");
    void (*release)(void *)=dlsym(module,"_ZN8GameCore10Interfaces10ICvUnknowndlEPv");
    int (*context_local)(const void *)=dlsym(module,"_ZNK8GameCore7Exports7Context14GetLocalPlayerEv");
    bool (*pending)(void *)=dlsym(module,"_ZN8GameCore7Exports7Context18HasPendingMessagesEv");
    int (*turn)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game14GetCurrentTurnEv");
    int (*phase)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game15GetCurrentPhaseEv");
    int (*game_local)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game14GetLocalPlayerEv");
    bool (*needs_update)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game11NeedsUpdateEv");
    bool (*needs_advance)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game16NeedsTurnAdvanceEv");
    bool (*ai)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game14IsAiProcessingEv");
    bool (*processing)(const void *)=dlsym(module,"_ZNK8GameCore7Exports4Game16IsTurnProcessingEv");
    if(!get_game||!release||!context_local||!pending||!turn||!phase||!game_local||
       !needs_update||!needs_advance||!ai||!processing)return 166;
    void *game=get_game(context);
    if(!game)return 167;
    int observed_turn=turn(game),observed_phase=phase(game);
    int observed_context_local=context_local(context),observed_game_local=game_local(game);
    bool observed_pending=pending(context),observed_update=needs_update(game);
    bool observed_advance=needs_advance(game),observed_ai=ai(game),observed_processing=processing(game);
    /* Native ICvUnknown delete dispatches Destroy; it owns only the acquired
     * interface wrapper, never the underlying game or framework Context. */
    release(game);
    fprintf(stderr,"C6_LINUX_GAME_STATE {\"phase\":\"between_saves\",\"current_turn\":%d,\"current_phase\":%d,\"context_local_player\":%d,\"game_local_player\":%d,\"pending_messages\":%d,\"needs_update\":%d,\"needs_turn_advance\":%d,\"ai_processing\":%d,\"turn_processing\":%d,\"native_wrapper_released\":true}\n",
        observed_turn,observed_phase,observed_context_local,observed_game_local,
        observed_pending,observed_update,observed_advance,observed_ai,observed_processing);
    return 0;
}

int c6_linux_game_load(void *context) {
    const char *input=getenv("C6_GAME_LOAD_STREAM");
    const char *output=getenv("C6_GAME_SAVE_OUTPUT");
    const char *repeat=getenv("C6_GAME_SAVE_REPEAT_OUTPUT");
    const char *sized=getenv("C6_GAME_LOAD_EXPECTED_SIZE");
    size_t expected_size=0;
    if(sized){
        char *end=NULL;errno=0;
        unsigned long long parsed=strtoull(sized,&end,10);
        if(errno || *sized=='-' || end==sized || *end || !parsed || parsed>SIZE_MAX)return 165;
        expected_size=(size_t)parsed;
    }
    Dl_info module;
    if(!context || !input || !output || !repeat ||
       !dladdr(*(void **)context,&module) || !module.dli_fname)return 160;
    const char *leaf=strrchr(module.dli_fname,'/');
    leaf=leaf?leaf+1:module.dli_fname;
    if(strcmp(leaf,"libGameCore_XP2.so")!=0)return 161;
    void *game=dlopen(module.dli_fname,RTLD_NOW|RTLD_NOLOAD);
    if(!game)return 162;
    bool (*initialized)(void *)=dlsym(game,"_ZN8GameCore7Exports7Context12IsInitialiedEv");
    void (*startup)(void *)=dlsym(game,"_ZN8GameCore7Exports7Context11StartupGameEv");
    void (*shutdown)(void *)=dlsym(game,"_ZN8GameCore7Exports7Context12ShutdownGameEv");
    int (*observe)(const char *)=dlsym(RTLD_DEFAULT,"c6_linux_network_observe_session");
    if(!initialized || !startup || !shutdown || !observe){dlclose(game);return 163;}
    bool ready=initialized(context);
    fprintf(stderr,"C6_LINUX_GAME_LOAD module %s context_initialized %d host_finalization 0\n",module.dli_fname,ready);
    int result=ready?observe("before_core_load"):164;
    if(!result)result=c6_linux_stream_check();
    if(!result){
        fprintf(stderr,"C6_LINUX_GAME_LOAD startup_enter\n");
        startup(context);
        fprintf(stderr,"C6_LINUX_GAME_LOAD startup_return\n");
        result=sized?c6_linux_context_load_sized(game,context,input,expected_size):
                     c6_linux_context_load(game,context,input);
        if(!result)result=observe("after_core_load");
        if(!result)result=c6_linux_context_save(game,context,output);
        if(!result && getenv("C6_GAME_LOAD_OBSERVE"))result=observe_loaded_state(game,context);
        if(!result && getenv("C6_DEFINITION_IDS")) {
            int (*definition_ids)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_definition_ids");
            result=definition_ids?definition_ids(game,context):171;
        }
        if(!result && getenv("C6_UNIT_DEFINITION_IDS")) {
            int (*unit_definition_ids)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_unit_definition_ids");
            result=unit_definition_ids?unit_definition_ids(game,context):173;
        }
        if(!result && getenv("C6_OPERATION_REGISTRY")) {
            int (*operation_registry)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_operation_registry");
            result=operation_registry?operation_registry(game,context):174;
        }
        if(!result && getenv("C6_COMBAT_RANDOM_INPUT")) {
            int (*combat_random)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_combat_random_probe");
            result=combat_random?combat_random(game,context):182;
        }
        if(!result && getenv("C6_COMBAT_KERNEL_INPUT")) {
            int (*combat_kernel)(void *)=dlsym(RTLD_DEFAULT,"c6_linux_combat_kernel_probe");
            result=combat_kernel?combat_kernel(game):180;
        }
        if(!result && getenv("C6_ACTION_OBSERVE")) {
            int (*action_observe)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_observe");
            result=action_observe?action_observe(game,context):168;
        }
        if(!result && getenv("C6_DOMAIN_METADATA_OUTPUT")) {
            int (*metadata)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_prepare_domain_metadata");
            if(!metadata){
                fprintf(stderr,"C6_LINUX_DOMAIN_METADATA_LIFECYCLE fatal missing_lifecycle process_discarded 1\n");
                fflush(stderr);_exit(184);
            }
            result=metadata(game,context);
        } else if(!result && getenv("C6_MOVEMENT_COST_MODE")) {
            int (*movement)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_prepare_movement_cost");
            if(!movement)movement_not_quiescent("movement_lifecycle_unavailable");
            result=movement(game,context);
        } else if(!result && getenv("C6_CORE_COMPONENT_MODE")) {
            int (*component)(void *,void *,bool)=dlsym(RTLD_DEFAULT,"c6_linux_action_prepare_core_component");
            result=component?component(game,context,!strcmp(getenv("C6_CORE_COMPONENT_MODE"),"execute")):181;
        } else if(!result && getenv("C6_ACTION_SKIP_TURN")) {
            const char *dispatch_name=getenv("C6_ACTION_REGISTER_PRECONTROLLER")?"c6_linux_action_prepare_registered_dispatch":"c6_linux_action_prepare_dispatch";
            int (*dispatch)(void *,void *,bool)=dlsym(RTLD_DEFAULT,dispatch_name);
            bool execute=!strcmp(getenv("C6_ACTION_SKIP_TURN"),"execute");
            result=dispatch?dispatch(game,context,execute):172;
        } else if(!result && getenv("C6_ACTION_LIFECYCLE")) {
            int (*lifecycle_observe)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_prepare_observe");
            result=lifecycle_observe?lifecycle_observe(game,context):170;
        } else if(!result && getenv("C6_ACTION_EXECUTION")) {
            int (*execution_observe)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_execution");
            result=execution_observe?execution_observe(game,context):169;
        }
        if(!result)result=c6_linux_context_save(game,context,repeat);
        fprintf(stderr,"C6_LINUX_GAME_LOAD shutdown_enter\n");
        shutdown(context);
        fprintf(stderr,"C6_LINUX_GAME_LOAD shutdown_return\n");
    }
    fprintf(stderr,"C6_LINUX_GAME_LOAD boundary_return %d host_finalization 0 gameplay_parity 0\n",result);
    dlclose(game);
    return result;
}
