#define _GNU_SOURCE
#include "save_lifecycle.h"
#include <dlfcn.h>
#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

int c6_linux_save_lifecycle_check(void) {
    void *(*last_stream)(void)=dlsym(RTLD_DEFAULT,
        "_ZN8GameCore10Serializer7Manager13GetLastStreamEv");
    int (*unserialize)(unsigned,void*)=dlsym(RTLD_DEFAULT,
        "_ZN8GameCore10Serializer7Manager11UnserializeENS0_8SaveTypeEPN13Serialization7IStreamE");
    int (*local_player)(void)=dlsym(RTLD_DEFAULT,
        "_ZN8GameCore3App5Cache14GetLocalPlayerEv");
    int *request_result=dlsym(RTLD_DEFAULT,
        "_ZN8GameCore10Serializer17ms_eRequestResultE");
    if(!last_stream||!unserialize||!local_player||!request_result)return 100;
    if(last_stream()){
        fprintf(stderr,"C6_LINUX_SAVE_LIFECYCLE stream_already_present\n");
        return 101;
    }
    int before=*request_result;
    /* Original null-stream branch only reads stream/type globals then returns
     * integer 5. No Load, pending settings or player-selection setters run. */
    int result=unserialize(0,NULL);
    int unchanged=last_stream()==NULL && *request_result==before;
    fprintf(stderr,"C6_LINUX_SAVE_LIFECYCLE no_stream_result %d request_unchanged %d local_player %d\n",
            result,unchanged,local_player());
    return result==5&&unchanged?0:102;
}

int c6_linux_save_lifecycle_attach_session(void) {
    void *(*network)(void)=dlsym(RTLD_DEFAULT,"_ZN7Network7Manager11GetInstanceEv");
    void *(*firewire)(void)=dlsym(RTLD_DEFAULT,"_ZN11FireWireMgr11GetInstanceEv");
    void *(*archive)(void)=dlsym(RTLD_DEFAULT,"_ZN19FAutoArchiveFactory11GetInstanceEv");
    void *(*configuration)(void)=dlsym(RTLD_DEFAULT,"_ZN8GameCore3App13Configuration4Game16GetConfigurationEv");
    uint32_t (*mode)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance11GetGameModeEv");
    bool (*worldbuilder)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK8GameCore3App13Configuration4Game8Instance20IsWorldBuilderEditorEv");
    bool (*available)(void)=dlsym(RTLD_DEFAULT,"_ZN8GameCore18IsContextAvailableEv");
    bool (*in_session)(const void*)=dlsym(RTLD_DEFAULT,"_ZNK7Network7Manager11IsInSessionEv");
    int (*attach)(void)=dlsym(RTLD_DEFAULT,"_ZN8GameCore10Serializer7Manager15AttachToSessionEv");
    int (*observe)(const char*)=dlsym(RTLD_DEFAULT,"c6_linux_network_observe_session");
    const unsigned char *location=dlsym(RTLD_DEFAULT,"_ZN8GameCore10Serializer23ms_kRequestSaveLocationE");
    if(!network||!firewire||!archive||!configuration||!mode||!worldbuilder||
       !available||!in_session||!attach||!observe||!location)return 103;
    uint32_t source_type;
    /* Read-only original SaveLocation field used by AttachToSession's own gate
     * at 0x324cae9. Never construct or change serializer request state here. */
    memcpy(&source_type,location+8,sizeof source_type);
    void *owner=network();
    if(!owner||!firewire()||!archive()||!available()||in_session(owner)||
       mode(configuration())!=UINT32_C(3915931367)||worldbuilder(configuration())||source_type==4){
        fprintf(stderr,"C6_LINUX_SAVE_LIFECYCLE attach_preconditions_refused\n");
        return 104;
    }
    fprintf(stderr,"C6_LINUX_SAVE_LIFECYCLE attach_enter\n");
    int result=attach();
    fprintf(stderr,"C6_LINUX_SAVE_LIFECYCLE attach_return %d in_session %d\n",result,in_session(owner));
    /* AttachToSession itself ignores HostGame's return, so inspect the genuine
     * session and peer through the separately owned original-network fixture. */
    if(result!=0||!in_session(owner))return 105;
    return observe("saved_configuration_attached");
}
