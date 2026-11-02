#define _GNU_SOURCE
#include "action_registration.h"
#include "action_state.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int event_state(void *system,void *(*queue_get)(void *),void *(*receiving)(void *),unsigned pass,const char *label,unsigned *used) {
    void *queue=queue_get(system),*stream=queue?receiving(queue):NULL;
    if(!stream)return 247;
    memcpy(used,stream,4);if(*used>4194304)return 247;
    const unsigned char *data=(const unsigned char *)stream+0x80;
    const char *directory=getenv("C6_ACTION_TRACE_DIRECTORY");if(!directory)return 247;
    char path[4096],name[96];snprintf(name,sizeof name,"registration-events-%02u-%s.bin",pass,label);
    int length=snprintf(path,sizeof path,"%s/%s",directory,name);
    if(length<0||(size_t)length>=sizeof path)return 247;
    FILE *out=fopen(path,"wx");if(!out)return 247;
    bool failed=fwrite(data,1,*used,out)!=*used;if(fclose(out))failed=true;if(failed)return 247;
    fprintf(stderr,"C6_LINUX_ACTION_REGISTRATION_EVENTS {\"pass\":%u,\"stage\":\"%s\",\"bytes\":%u,\"file\":\"%s\",\"ids\":[",pass,label,*used,name);
    unsigned offset=0;
    while(offset<*used){
        uint32_t size,id;if(*used-offset<16){fputs("]}\n",stderr);return 247;}
        memcpy(&size,data+offset,4);memcpy(&id,data+offset+4,4);
        if(size<16||size>*used-offset){fputs("]}\n",stderr);return 247;}
        fprintf(stderr,"%s%u",offset?",":"",id);offset+=size;
    }
    fputs("]}\n",stderr);return 0;
}
int c6_linux_action_registration(void *module,void *context) {
    void *(*manager_get)(void),*(*session_get)(void *),*(*player_get)(const void *,int);
    int (*local_get)(const void *),(*host_get)(const void *);
    bool (*is_host)(const void *),(*core_active)(const void *),(*registered)(const void *),(*needs_snapshot)(const void *),(*ignore)(const void *);
    bool (*pending)(void *);void (*update)(void *,float),(*mark)(unsigned),(*publish)(void *);
    void *event_system,*(*queue_get)(void *),*(*receiving)(void *);
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
    HOST(publish,"_ZN6Events11EventSystemINS_10EventQueueINS_19EventStream_FixedMTILj4194304EEEEENS_15EventDispatcherIS4_NS_12DispatchDataEEELj256EE13PublishEventsEv");
    HOST(event_system,"g_EventSystemLocalMachine");
    HOST(queue_get,"_ZN6Events11EventSystemINS_10EventQueueINS_19EventStream_FixedMTILj4194304EEEEENS_15EventDispatcherIS4_NS_12DispatchDataEEELj256EE10EventQueueEv");
    HOST(receiving,"_ZN6Events10EventQueueINS_19EventStream_FixedMTILj4194304EEEE18GetReceivingStreamEv");
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
        unsigned event_bytes=0;
        result=event_state(event_system,queue_get,receiving,pass,"before",&event_bytes);
        if(result)return result;
        bool app=registered(player),snapshot=needs_snapshot(player),ignored=ignore(session);
        fprintf(stderr,"C6_LINUX_ACTION_REGISTRATION {\"pass\":%u,\"local\":%d,\"host\":%d,\"core_active\":false,\"app_registered\":%s,\"needs_snapshot\":%s,\"ignore_incoming\":%s,\"provider_enet_layout_proven\":%s,\"provider_queue_u64\":%llu,\"local_machine_pending_bytes\":%u}\n",pass,local,host,app?"true":"false",snapshot?"true":"false",ignored?"true":"false",enet?"true":"false",(unsigned long long)queued,event_bytes);
        if(!enet)return 245;
        ready=app&&!snapshot&&!ignored&&queued==0&&event_bytes==0;
        if(ready||pass==8)break;
        mark(10);update(manager,0.0f);mark(11);
        result=event_state(event_system,queue_get,receiving,pass,"before-publish",&event_bytes);
        if(result)return result;
        mark(12);publish(event_system);mark(13);
        result=event_state(event_system,queue_get,receiving,pass,"after-publish",&event_bytes);
        if(result)return result;
    }
    result=c6_linux_action_state_capture(module,NULL,"registration_after");
    if(result)return result;
    return ready?0:246;
}
