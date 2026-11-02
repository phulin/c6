#define _GNU_SOURCE
#include "action_registration.h"
#include "action_state.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

int c6_linux_action_registration(void *module,void *context) {
    void *(*manager_get)(void),*(*session_get)(void *),*(*player_get)(const void *,int);
    int (*local_get)(const void *),(*host_get)(const void *);
    bool (*is_host)(const void *),(*core_active)(const void *),(*registered)(const void *),(*needs_snapshot)(const void *),(*ignore)(const void *);
    bool (*pending)(void *);void (*update)(void *,float),(*mark)(unsigned);
#define HOST(VAR,SYM) do{VAR=dlsym(RTLD_DEFAULT,SYM);if(!VAR)return 240;}while(0)
    HOST(manager_get,"_ZN7Network7Manager11GetInstanceEv");
    HOST(session_get,"_ZN7Network7Manager10GetSessionEv");
    HOST(player_get,"_ZNK11FNetSession9GetPlayerEi");
    HOST(local_get,"_ZNK11FNetSession16GetLocalPlayerIDEv");
    HOST(host_get,"_ZNK11FNetSession15GetHostPlayerIDEv");
    HOST(is_host,"_ZNK11FNetSession6IsHostEv");
    HOST(core_active,"_ZNK7Network7Manager16IsGameCoreActiveEv");
    HOST(registered,"_ZNK10FNetPlayer16GetAppRegisteredEv");
    HOST(needs_snapshot,"_ZNK10FNetPlayer16GetNeedsSnapshotEv");
    HOST(ignore,"_ZNK11FNetSession24IsIgnoreIncomingMessagesEv");
    HOST(update,"_ZN7Network7Manager6UpdateEf");
    HOST(mark,"c6_linux_action_trace_mark");
#undef HOST
    pending=dlsym(module,"_ZN8GameCore7Exports7Context18HasPendingMessagesEv");
    if(!module||!context||!pending)return 240;
    void *manager=manager_get(),*session=manager?session_get(manager):NULL;
    if(!session||!is_host(session)||core_active(manager)||pending(context))return 241;
    int local=local_get(session),host=host_get(session);
    if(local<0||local>=64||local!=host||!player_get(session,local))return 242;
    int result=c6_linux_action_state_capture(module,NULL,"registration_before");
    if(result)return result;
    bool ready=false;
    for(unsigned pass=0;pass<=8;++pass){
        if(session_get(manager)!=session||core_active(manager)||pending(context))return 243;
        void *player=player_get(session,local),*provider=NULL;
        if(!player)return 244;
        memcpy(&provider,(const unsigned char *)session+0x28,8);
        bool enet=false;uint64_t queued=0;
        if(provider){
            void **table=*(void ***)provider;
            void *native_sync=dlsym(RTLD_DEFAULT,"_ZNK11FNetSession14GetSyncManagerEv");
            if(!native_sync)return 240;
            uintptr_t bias=(uintptr_t)native_sync-0x39d2194;
            uintptr_t send=(uintptr_t)table[0x10/8]-bias,step=(uintptr_t)table[0x38/8]-bias;
            enet=(send==0x39ddfee||send==0x3a06b58)&&step==0x39de160;
            if(enet)memcpy(&queued,(const unsigned char *)provider+0x40,8);
        }
        bool app=registered(player),snapshot=needs_snapshot(player),ignored=ignore(session);
        fprintf(stderr,"C6_LINUX_ACTION_REGISTRATION {\"pass\":%u,\"local\":%d,\"host\":%d,\"core_active\":false,\"app_registered\":%s,\"needs_snapshot\":%s,\"ignore_incoming\":%s,\"provider_enet_layout_proven\":%s,\"provider_queue_u64\":%llu}\n",pass,local,host,app?"true":"false",snapshot?"true":"false",ignored?"true":"false",enet?"true":"false",(unsigned long long)queued);
        if(!enet)return 245;
        ready=app&&!snapshot&&!ignored&&queued==0;
        if(ready||pass==8)break;
        mark(10);update(manager,0.0f);mark(11);
    }
    result=c6_linux_action_state_capture(module,NULL,"registration_after");
    if(result)return result;
    return ready?0:246;
}
