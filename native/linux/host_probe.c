#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

typedef int (*main_fn)(int,char**,char**);
static int actual_main_argc;
static char **actual_main_argv;
/* Original main arguments remain owned by the CRT for the process lifetime. */
int c6_linux_main_arguments(int *argc,char ***argv){
    if(!argc||!argv||!actual_main_argv)return 1;
    *argc=actual_main_argc;*argv=actual_main_argv;
    return 0;
}
static int finish_host(void){
    int (*finish)(void)=dlsym(RTLD_DEFAULT,"c6_linux_context_finish");
    if(!finish)return 49;
    fprintf(stderr,"C6_LINUX host_services_destroy_enter\n");
    int result=finish();
    fprintf(stderr,"C6_LINUX host_services_destroy_return %d\n",result);
    return result;
}
static int finish_filesystem(void){
    if(!getenv("C6_FILESYSTEM"))return 0;
    int (*shutdown)(void)=dlsym(RTLD_DEFAULT,"c6_linux_filesystem_shutdown");
    if(!shutdown)return 76;
    int result=shutdown();
    fprintf(stderr,"C6_LINUX filesystem_shutdown_return %d\n",result);
    return result;
}
static int run_probe(int argc,char **argv,char **envp) {
    actual_main_argc=argc;actual_main_argv=argv;(void)envp;
    fprintf(stderr,"C6_LINUX host_main_intercepted\n");
    if(getenv("C6_FILESYSTEM")){
        int (*initialize)(void)=dlsym(RTLD_DEFAULT,"c6_linux_filesystem_initialize");
        if(!initialize)return 75;
        fprintf(stderr,"C6_LINUX filesystem_enter\n");
        int result=initialize();
        fprintf(stderr,"C6_LINUX filesystem_return %d\n",result);
        if(result)return result;
    }
    if(getenv("C6_PREPARE_HOST_FIRST")){
        int (*prepare)(void)=dlsym(RTLD_DEFAULT,"c6_linux_prepare_host");
        if(!prepare)return 78;
        int result=prepare();if(result)return result;
    }
    if(getenv("C6_FRAMEWORK")){
        int (*framework)(void)=dlsym(RTLD_DEFAULT,"c6_linux_framework_run");
        if(!framework)return 145;
        int result=framework();if(result)return result;
        result=finish_host();if(result)return result;
        return finish_filesystem();
    }
    void *game=dlopen(getenv("C6_GAMECORE"),RTLD_NOW|RTLD_LOCAL);
    if(!game){fprintf(stderr,"C6_LINUX dlopen_error %s\n",dlerror());return 41;}
    fprintf(stderr,"C6_LINUX whole_gamecore_loaded\n");
    if(getenv("C6_STREAM_CHECK")){
        int (*check)(void)=dlsym(RTLD_DEFAULT,"c6_linux_stream_check");
        if(!check)return 51;
        int result=check();if(result)return result;
    }
    void *(*create)(void)=dlsym(game,"DllCreateGameContext");
    void (*destroy)(void*)=dlsym(game,"DllDestroyGameContext");
    if(!create||!destroy){fprintf(stderr,"C6_LINUX factory_missing\n");return 42;}
    for(int i=0;i<3;i++){
        fprintf(stderr,"C6_LINUX create_enter %d\n",i);
        void *context=create();
        fprintf(stderr,"C6_LINUX create_return %d %s\n",i,context?"nonnull":"null");
        if(!context)return 43;
        if(i==0 && getenv("C6_INITIALIZE")){
            int (*initialize)(void*,void*)=dlsym(RTLD_DEFAULT,"c6_linux_context_initialize");
            if(!initialize){fprintf(stderr,"C6_LINUX initialize_adapter_missing\n");return 45;}
            int result=initialize(game,context);
            if(result)return result;
        }
        if(i==0 && getenv("C6_NETWORK_HOST")){
            int (*leave)(void)=dlsym(RTLD_DEFAULT,"c6_linux_network_leave");
            if(!leave)return 152;
            fprintf(stderr,"C6_LINUX network_leave_enter\n");
            int result=leave();
            fprintf(stderr,"C6_LINUX network_leave_return %d\n",result);
            if(result)return result;
        }
        destroy(context);
        fprintf(stderr,"C6_LINUX destroy_return %d\n",i);
    }
    if(getenv("C6_INITIALIZE")){
        int result=finish_host();if(result)return result;
    }
    int closed=dlclose(game);
    fprintf(stderr,"C6_LINUX dlclose_return %d\n",closed);
    int filesystem_result=finish_filesystem();if(filesystem_result)return filesystem_result;
    fflush(stderr);
    /* Host application shutdown was never initialized. Avoid its atexit path. */
    return closed?44:0;
}
static int probe_main(int argc,char **argv,char **envp) {
    int result=run_probe(argc,argv,envp);
    fflush(stderr);
    _exit(result);
}
int __libc_start_main(main_fn main,int argc,char **argv,void (*init)(void),void (*fini)(void),void (*rtld_fini)(void),void *stack_end){
    (void)main;
    typedef int (*start_fn)(main_fn,int,char**,void(*)(void),void(*)(void),void(*)(void),void*);
    start_fn real=(start_fn)dlsym(RTLD_NEXT,"__libc_start_main");
    return real(probe_main,argc,argv,init,fini,rtld_fini,stack_end);
}
