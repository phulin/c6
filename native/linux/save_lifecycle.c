#define _GNU_SOURCE
#include "save_lifecycle.h"
#include <dlfcn.h>
#include <stdio.h>

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
