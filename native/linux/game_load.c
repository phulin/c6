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
        if(!result && getenv("C6_ACTION_OBSERVE")) {
            int (*action_observe)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_observe");
            result=action_observe?action_observe(game,context):168;
        }
        if(!result && getenv("C6_ACTION_LIFECYCLE")) {
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
