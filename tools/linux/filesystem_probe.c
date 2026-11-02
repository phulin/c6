#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include "../../native/linux/filesystem.h"

typedef int (*main_fn)(int,char**,char**);
static int filesystem_main(int argc,char **argv,char **envp) {
    (void)argc;(void)argv;(void)envp;
    fprintf(stderr,"C6_LINUX_FILESYSTEM host_main_intercepted\n");
    int result=c6_linux_filesystem_probe_paths("before");
    if(!result) result=c6_linux_filesystem_initialize();
    if(!result) result=c6_linux_filesystem_probe_paths("after");
    if(!result) result=c6_linux_filesystem_shutdown();
    fflush(stderr);
    _exit(result?71:0);
}
int __libc_start_main(main_fn original,int argc,char **argv,void(*init)(void),
                      void(*fini)(void),void(*rtld_fini)(void),void *stack_end) {
    (void)original;
    typedef int (*start_fn)(main_fn,int,char**,void(*)(void),void(*)(void),void(*)(void),void*);
    start_fn real=(start_fn)dlsym(RTLD_NEXT,"__libc_start_main");
    return real(filesystem_main,argc,argv,init,fini,rtld_fini,stack_end);
}
