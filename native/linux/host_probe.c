#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

typedef int (*main_fn)(int,char**,char**);
static int run_probe(int argc,char **argv,char **envp) {
    (void)argc; (void)argv; (void)envp;
    fprintf(stderr,"C6_LINUX host_main_intercepted\n");
    void *game=dlopen(getenv("C6_GAMECORE"),RTLD_NOW|RTLD_LOCAL);
    if(!game){fprintf(stderr,"C6_LINUX dlopen_error %s\n",dlerror());return 41;}
    fprintf(stderr,"C6_LINUX whole_gamecore_loaded\n");
    void *(*create)(void)=dlsym(game,"DllCreateGameContext");
    void (*destroy)(void*)=dlsym(game,"DllDestroyGameContext");
    if(!create||!destroy){fprintf(stderr,"C6_LINUX factory_missing\n");return 42;}
    for(int i=0;i<3;i++){
        fprintf(stderr,"C6_LINUX create_enter %d\n",i);
        void *context=create();
        fprintf(stderr,"C6_LINUX create_return %d %s\n",i,context?"nonnull":"null");
        if(!context)return 43;
        destroy(context);
        fprintf(stderr,"C6_LINUX destroy_return %d\n",i);
    }
    int closed=dlclose(game);
    fprintf(stderr,"C6_LINUX dlclose_return %d\n",closed);
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
