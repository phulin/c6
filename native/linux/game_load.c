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
        if(!result)result=c6_linux_context_save(game,context,repeat);
        fprintf(stderr,"C6_LINUX_GAME_LOAD shutdown_enter\n");
        shutdown(context);
        fprintf(stderr,"C6_LINUX_GAME_LOAD shutdown_return\n");
    }
    fprintf(stderr,"C6_LINUX_GAME_LOAD boundary_return %d host_finalization 0 gameplay_parity 0\n",result);
    dlclose(game);
    return result;
}
