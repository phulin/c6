#define _GNU_SOURCE
#include "filesystem.h"
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>

static int initialized;
static void *lookup(const char *name) {
    void *value=dlsym(RTLD_DEFAULT,name);
    if(!value) fprintf(stderr,"C6_LINUX_FILESYSTEM missing_symbol %s\n",name);
    return value;
}

int c6_linux_filesystem_probe_paths(const char *phase) {
    /* Original C wrapper disassembly 0x2cca527 forwards output, capacity and
     * input to genuine ASL_ToNativePath; no libc++ object layout required. */
    char *(*normalize)(char *,size_t,const char *)=lookup("ASL_EmuToNativePath");
    if(!normalize) return 1;
    const char *paths[]={
        "C:\\Emu\\AppAssets\\Base\\Assets\\Gameplay\\Data\\Schema\\01_GameplaySchema.sql",
        "C:\\Emu\\AppAssets\\Base\\Assets\\Configuration\\Data\\Schema\\SetupParameters.sql",
        "C:\\Emu\\AppAssets\\DLC\\Expansion2\\Expansion2.modinfo",
        "C:\\Emu\\AppAssets\\__c6_absent_fixture__",
        "C:\\Emu\\AppData",
        "C:\\Emu\\AppTemp"
    };
    for(size_t i=0;i<sizeof paths/sizeof paths[0];i++) {
        char output[4096];
        if(normalize(output,sizeof output,paths[i])!=output) return 1;
        struct stat metadata;
        int exists=stat(output,&metadata)==0;
        fprintf(stderr,"C6_LINUX_FILESYSTEM normalized %s %zu exists %d path %s\n",
                phase,i,exists,output);
    }
    return 0;
}

int c6_linux_filesystem_initialize(void) {
    void (*initialize)(void)=lookup("_ZN3ASL12ASL_FileInitEv");
    if(!initialize||initialized) return 1;
    fprintf(stderr,"C6_LINUX_FILESYSTEM initialize_enter\n");
    initialize();
    initialized=1;
    fprintf(stderr,"C6_LINUX_FILESYSTEM initialize_return\n");
    /* Original function is a guarded one-time initializer. Re-entry must not
     * replace roots or alter normalized paths; diagnostic caller compares. */
    initialize();
    fprintf(stderr,"C6_LINUX_FILESYSTEM initialize_repeat_return\n");
    return 0;
}

int c6_linux_filesystem_shutdown(void) {
    void (*shutdown)(void)=lookup("_ZN3ASL12ASL_FileQuitEv");
    if(!shutdown||initialized!=1) return 1;
    fprintf(stderr,"C6_LINUX_FILESYSTEM shutdown_enter\n");
    shutdown();
    initialized=2; /* Original cached temp-root strings do not support restart. */
    fprintf(stderr,"C6_LINUX_FILESYSTEM shutdown_return\n");
    return 0;
}
