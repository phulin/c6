#define _GNU_SOURCE
#include "action_execution.h"
#include <stdbool.h>
#include <dlfcn.h>
#include <stdio.h>

int c6_linux_action_execution(void *module,void *context) {
    if(!module||!context)return 190;
    int (*local)(const void *)=dlsym(module,"_ZNK8GameCore7Exports7Context14GetLocalPlayerEv");
    void *(*live_player)(void *,int)=dlsym(module,"_ZN8GameCore7Exports7Context9GetPlayerENS_11PlayerTypesE");
    void *(*cache_player)(const void *,int)=dlsym(module,"_ZNK8GameCore7Exports7Context14GetCachePlayerENS_11PlayerTypesE");
    int (*player_id)(const void *)=dlsym(module,"_ZNK8GameCore7Exports6Player5GetIDEv");
    bool (*active)(const void *)=dlsym(module,"_ZNK8GameCore7Exports6Player12IsTurnActiveEv");
    bool (*human)(const void *)=dlsym(module,"_ZNK8GameCore7Exports6Player7IsHumanEv");
    void (*release)(void *)=dlsym(module,"_ZN8GameCore10Interfaces10ICvUnknowndlEPv");
    void **active_controller=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController21ms_pkActiveControllerE");
    bool (*controller_initialized)(void *)=dlsym(RTLD_DEFAULT,"_ZN18GameCoreController13IsInitializedEv");
    void *(*controller_context)(void *,const void *)=dlsym(RTLD_DEFAULT,"_ZNK18GameCoreController10GetContextEv");
    void *(*host_context)(void *)=dlsym(RTLD_DEFAULT,"_ZN8GameCore10GetContextEv");
    void *(*network)(void)=dlsym(RTLD_DEFAULT,"_ZN7Network7Manager11GetInstanceEv");
    bool (*network_active)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK7Network7Manager16IsGameCoreActiveEv");
    if(!local||!live_player||!cache_player||!player_id||!active||!human||!release||!active_controller||!controller_initialized||!controller_context||!host_context||!network||!network_active)return 191;
    void *controller=*active_controller,*bound=NULL,*host_bound=NULL;
    if(controller)controller_context(&bound,controller);
    host_context(&host_bound);
    bool initialized=controller&&controller_initialized(controller);
    bool net_active=controller&&network()&&network_active(network());
    fprintf(stderr,"C6_LINUX_ACTION_EXECUTION {\"stage\":\"host_delivery_observation\",\"controller_present\":%d,\"controller_initialized\":%d,\"controller_context_matches\":%d,\"host_context_matches\":%d,\"network_gamecore_active\":%d,\"action_executed\":false}\n",controller!=NULL,initialized,bound==context,host_bound==context,net_active);
    /* Native getters copy existing Context identity; these are borrowed,
     * unlike GetPlayer-created facades. Do not dispatch Destroy on them. */
    if(!initialized){
        fprintf(stderr,"C6_LINUX_ACTION_EXECUTION rejected uninitialized_controller cache_query_not_called 1 action_executed 0\n");
        return 201;
    }
    int who=local(context);
    if(who<0||who>=64)return 192;
    void *live=live_player(context,who),*cache=cache_player(context,who);
    if(!live){if(cache)release(cache);return 193;}
    int live_id=player_id(live),cache_id=cache?player_id(cache):-1;
    bool live_active=active(live),live_human=human(live);
    bool cache_active=cache?active(cache):false,cache_human=cache?human(cache):false;
    fprintf(stderr,"C6_LINUX_ACTION_EXECUTION {\"stage\":\"cache_dispatch_observation\",\"local_player\":%d,\"live_player\":%d,\"live_active\":%d,\"live_human\":%d,\"cache_present\":%d,\"cached_player\":%d,\"cached_active\":%d,\"cached_human\":%d,\"request_cache_predicate_passes\":%d,\"action_executed\":false}\n",who,live_id,live_active,live_human,cache!=NULL,cache_id,cache_active,cache_human,cache&&cache_active&&cache_id==who);
    if(cache)release(cache);
    release(live);
    return 0;
}
