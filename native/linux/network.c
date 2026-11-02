#define _GNU_SOURCE
#include "network.h"
#include "host_services.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static bool owned;
static void *owned_manager;
static void *owned_application;
static void *symbol(const char *name) {
    void *value=dlsym(RTLD_DEFAULT,name);
    if(!value)fprintf(stderr,"C6_LINUX_NETWORK missing_symbol %s\n",name);
    return value;
}

int c6_linux_network_initialize(void *application) {
    bool (*setup)(void *)=symbol("_ZN7Civ6App12SetupNetworkEv");
    void *(*get_manager)(void)=symbol("_ZN7Network7Manager11GetInstanceEv");
    void *(*get_archive)(void)=symbol("_ZN19FAutoArchiveFactory11GetInstanceEv");
    void *(*get_firewire)(void)=symbol("_ZN11FireWireMgr11GetInstanceEv");
    void *(*get_sync)(void)=symbol("_ZN16FGameSyncManager11GetInstanceEv");
    void *(*get_messages)(void)=symbol("_ZN18FNetMessageFactory11GetInstanceEv");
    void *(*get_session)(void *)=symbol("_ZN7Network7Manager10GetSessionEv");
    void **published_app=symbol("g_pkApplication");
    if(!published_app||*published_app!=application)return 150;
    if(!setup||!get_manager||!get_archive||!get_firewire||!get_sync||
       !get_messages||!get_session||!application||owned)return 150;
    /* Original Network::Initialize overwrites Manager/Archive/FireWire owners.
     * FireWire C1 0x3a017c5..d7 preserves an existing CRT MessageFactory.
     * Require that same native object after Setup; do not replace it. */
    void *manager_before=get_manager(), *archive_before=get_archive();
    void *firewire_before=get_firewire(), *sync_before=get_sync();
    void *messages_before=get_messages();
    fprintf(stderr,"C6_LINUX_NETWORK providers_before manager %d archive %d firewire %d sync %d messages %d\n",
            manager_before!=NULL,archive_before!=NULL,firewire_before!=NULL,
            sync_before!=NULL,messages_before!=NULL);
    if(manager_before||archive_before||firewire_before||sync_before) {
        fprintf(stderr,"C6_LINUX_NETWORK existing_owner_refused\n");
        return 151;
    }
    if(c6_linux_bind_host_arguments(application))return 152;
    if(c6_linux_register_host_events())return 153;
    fprintf(stderr,"C6_LINUX_NETWORK setup_enter argc %d\n",
            *(int *)((char *)application+0x30));
    bool result=setup(application);
    owned_manager=get_manager();
    owned=owned_manager!=NULL;
    if(owned)owned_application=application;
    bool messages_preserved=!messages_before||get_messages()==messages_before;
    bool ready=result&&owned_manager&&get_archive()&&get_firewire()&&
               get_sync()&&get_messages()&&messages_preserved&&!get_session(owned_manager);
    fprintf(stderr,"C6_LINUX_NETWORK native_message_factory_preserved %d\n",messages_preserved);
    fprintf(stderr,"C6_LINUX_NETWORK setup_return %d providers_ready %d no_session %d\n",
            result,ready,owned_manager?!get_session(owned_manager):0);
    return ready?0:154;
}

int c6_linux_network_observe_session(const char *phase) {
    void *(*get_manager)(void)=symbol("_ZN7Network7Manager11GetInstanceEv");
    void *(*get_session)(void *)=symbol("_ZN7Network7Manager10GetSessionEv");
    bool (*in_session)(const void *)=symbol("_ZNK7Network7Manager11IsInSessionEv");
    int (*manager_local)(const void *)=symbol("_ZNK7Network7Manager16GetLocalPlayerIDEv");
    int (*session_local)(const void *)=symbol("_ZNK11FNetSession16GetLocalPlayerIDEv");
    int (*session_host)(const void *)=symbol("_ZNK11FNetSession15GetHostPlayerIDEv");
    bool (*is_host)(const void *)=symbol("_ZNK11FNetSession6IsHostEv");
    void *(*get_player)(const void *,int)=symbol("_ZNK11FNetSession9GetPlayerEi");
    if(!owned||!phase||!get_manager||!get_session||!in_session||!manager_local||
       !session_local||!session_host||!is_host||!get_player)return 155;
    if(get_manager()!=owned_manager)return 156;
    void *session=get_session(owned_manager);
    if(!session) {
        fprintf(stderr,"C6_LINUX_NETWORK session_observed %s nonnull 0\n",phase);
        return 157;
    }
    Dl_info identity;
    bool concrete=dladdr(*(void **)session,&identity)&&identity.dli_sname&&
                  !strcmp(identity.dli_sname,"_ZTV11FNetSession");
    if(!concrete) {
        fprintf(stderr,"C6_LINUX_NETWORK session_concrete_type_refused\n");
        return 158;
    }
    int local=manager_local(owned_manager);
    int peer=session_local(session);
    int host=session_host(session);
    bool hosting=is_host(session);
    bool connected=in_session(owned_manager);
    void *player=local>=0?get_player(session,local):NULL;
    bool ready=connected&&hosting&&local>=0&&local==peer&&host==local&&player;
    fprintf(stderr,"C6_LINUX_NETWORK session_observed %s concrete_fnet 1 connected %d hosting %d manager_local %d session_local %d host %d player_nonnull %d ready %d\n",
            phase,connected,hosting,local,peer,host,player!=NULL,ready);
    return ready?0:159;
}

int c6_linux_network_host_offline(void) {
    int (*host)(void *,int,bool)=symbol("_ZN7Network7Manager8HostGameE10ServerTypeb");
    void *(*get_manager)(void)=symbol("_ZN7Network7Manager11GetInstanceEv");
    if(!owned||!get_manager||get_manager()!=owned_manager)return 160;
    void *(*get_session)(void *)=symbol("_ZN7Network7Manager10GetSessionEv");
    if(!owned||!host||!get_session||get_session(owned_manager))return 160;
    fprintf(stderr,"C6_LINUX_NETWORK host_offline_enter server_type 0 lobby 0\n");
    int result=host(owned_manager,0,false);
    fprintf(stderr,"C6_LINUX_NETWORK host_offline_return %d\n",result);
    int observed=c6_linux_network_observe_session("host_offline");
    return result==1?observed:161;
}

int c6_linux_network_check_guards(void) {
    const char *names[]={"_ZN7Network7Manager11GetInstanceEv",
                         "_ZN19FAutoArchiveFactory11GetInstanceEv",
                         "_ZN11FireWireMgr11GetInstanceEv",
                         "_ZN16FGameSyncManager11GetInstanceEv",
                         "_ZN18FNetMessageFactory11GetInstanceEv"};
    void *(*getters[5])(void);
    void *before[5];
    if(!owned||!owned_application)return 167;
    for(unsigned i=0;i<5;i++) {
        getters[i]=symbol(names[i]);
        if(!getters[i])return 167;
        before[i]=getters[i]();
    }
    if(before[0]!=owned_manager)return 167;
    int argc_before=*(int *)((char *)owned_application+0x30);
    char **argv_before=*(char ***)((char *)owned_application+0x38);
    int refused=c6_linux_network_initialize(owned_application);
    bool unchanged=refused==150&&
        *(int *)((char *)owned_application+0x30)==argc_before&&
        *(char ***)((char *)owned_application+0x38)==argv_before;
    for(unsigned i=0;i<5;i++)unchanged=unchanged&&getters[i]()==before[i];
    fprintf(stderr,"C6_LINUX_NETWORK duplicate_setup_refused %d state_unchanged %d\n",
            refused,unchanged);
    if(!unchanged)return 168;
    void *(*get_session)(void *)=symbol("_ZN7Network7Manager10GetSessionEv");
    int (*local)(const void *)=symbol("_ZNK7Network7Manager16GetLocalPlayerIDEv");
    if(!get_session||!local)return 169;
    void *session=get_session(owned_manager);
    if(session) {
        int peer=local(owned_manager);
        int duplicate=c6_linux_network_host_offline();
        unchanged=duplicate==160&&get_session(owned_manager)==session&&
                  local(owned_manager)==peer;
        fprintf(stderr,"C6_LINUX_NETWORK duplicate_host_refused %d state_unchanged %d\n",
                duplicate,unchanged);
        if(!unchanged)return 170;
    }
    return 0;
}

int c6_linux_network_leave(void) {
    void *(*get_manager)(void)=symbol("_ZN7Network7Manager11GetInstanceEv");
    if(!owned||!get_manager||get_manager()!=owned_manager)return 162;
    void *(*get_session)(void *)=symbol("_ZN7Network7Manager10GetSessionEv");
    void (*leave)(void *)=symbol("_ZN7Network7Manager9LeaveGameEv");
    if(!owned||!get_session||!leave)return 162;
    if(!get_session(owned_manager))return 0;
    fprintf(stderr,"C6_LINUX_NETWORK leave_enter\n");
    leave(owned_manager);
    bool absent=get_session(owned_manager)==NULL;
    fprintf(stderr,"C6_LINUX_NETWORK leave_return session_null %d\n",absent);
    return absent?0:163;
}

int c6_linux_network_shutdown(void) {
    void (*free_manager)(void)=symbol("_ZN7Network7Manager12FreeInstanceEv");
    void *(*get_manager)(void)=symbol("_ZN7Network7Manager11GetInstanceEv");
    void *(*get_session)(void *)=symbol("_ZN7Network7Manager10GetSessionEv");
    void *(*get_archive)(void)=symbol("_ZN19FAutoArchiveFactory11GetInstanceEv");
    void *(*get_firewire)(void)=symbol("_ZN11FireWireMgr11GetInstanceEv");
    void *(*get_sync)(void)=symbol("_ZN16FGameSyncManager11GetInstanceEv");
    void *(*get_messages)(void)=symbol("_ZN18FNetMessageFactory11GetInstanceEv");
    if(!owned||!free_manager||!get_manager||!get_session||!get_archive||
       !get_firewire||!get_sync||!get_messages||get_manager()!=owned_manager)return 164;
    /* Context users must already be destroyed; leave is an earlier stage. */
    if(get_session(owned_manager))return 165;
    fprintf(stderr,"C6_LINUX_NETWORK shutdown_enter\n");
    free_manager();
    bool released=!get_manager()&&!get_archive()&&!get_firewire()&&!get_sync()&&!get_messages();
    owned=false;
    owned_manager=NULL;
    owned_application=NULL;
    fprintf(stderr,"C6_LINUX_NETWORK shutdown_return providers_null %d\n",released);
    return released?0:166;
}
