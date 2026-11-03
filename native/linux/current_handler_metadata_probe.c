#define _GNU_SOURCE
#include "current_handler_metadata_fixture.h"
#include "current_handler_interruption.h"
#include "context_stream.h"
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/uio.h>
#include <sys/syscall.h>
#include <unistd.h>

/* Enclosing caller scope: original facade acquisition/release and Save are
 * deliberately outside the passive helper and field-only guard. Preparation
 * source only; loader/profile admission and original execution remain pending. */
static _Noreturn void caller_fatal(const char *why) {
    fprintf(stderr,"C6_CURRENT_HANDLER_CALLER fatal %s no_poststate_or_cleanup\n",why);
    fflush(stderr);_exit(240);
}
static void read_field(uintptr_t p,void *out,size_t n) {
    if(!p||n>UINTPTR_MAX-p)caller_fatal("invalid_read");
    struct iovec a={out,n},b={(void *)p,n};
    if(process_vm_readv(getpid(),&a,1,&b,1,0)!=(ssize_t)n)caller_fatal("unreadable");
}
static uintptr_t getptr(uintptr_t p){uintptr_t v;read_field(p,&v,8);return v;}
static uint32_t getword(uintptr_t p){uint32_t v;read_field(p,&v,4);return v;}
static void *binding(void *module,const char *name,uintptr_t address,void *base) {
    void *p=dlsym(module,name);Dl_info info={0};
    if((uintptr_t)p!=address||!dladdr(p,&info)||info.dli_fbase!=base)caller_fatal("binding_mismatch");
    return p;
}
static void hex(FILE *out,const unsigned char *bytes,size_t n) {
    fputc('"',out);for(size_t i=0;i<n;i++)fprintf(out,"%02x",bytes[i]);fputc('"',out);
}
int c6_linux_current_handler_metadata_probe(void *module,void *context) {
    static const char *names[]={"OUTPUT","REPEAT_OUTPUT","CONTROL_OUTPUT","SNAPSHOT_BEFORE","SNAPSHOT_CONTROL","SNAPSHOT_AFTER","SNAPSHOT_REPEAT"};
    char *paths[7]={0};char key[96];
    for(unsigned i=0;i<7;i++){
        snprintf(key,sizeof key,"C6_CURRENT_HANDLER_METADATA_%s",names[i]);
        const char *p=getenv(key);
        if(!p||!*p)caller_fatal("missing_path");
        paths[i]=strdup(p);if(!paths[i])caller_fatal("path_allocation");
        for(unsigned j=0;j<i;j++)if(!strcmp(paths[i],paths[j]))caller_fatal("duplicate_path");
    }
    const char *profile=getenv("C6_CURRENT_HANDLER_METADATA_PROFILE_SHA256");
    if(!profile||strlen(profile)!=64)caller_fatal("profile_missing");
    for(unsigned i=0;i<64;i++)if(!strchr("0123456789abcdef",profile[i]))caller_fatal("profile_invalid");
    Dl_info info={0};
    if(!module||!context||(uintptr_t)context>UINTPTR_MAX-16||!dladdr((void *)getptr((uintptr_t)context),&info)||!info.dli_fbase)
        caller_fatal("context_owner_unknown");
    uintptr_t base=(uintptr_t)info.dli_fbase;
    if(base>UINTPTR_MAX-0x3c07380)caller_fatal("core_extent_unknown");
    void *(*acquire)(void *)=binding(module,"_ZN8GameCore7Exports7Context7GetGameEv",base+0x2c7cae0,info.dli_fbase);
    void (*release)(void *)=binding(module,"_ZN8GameCore10Interfaces10ICvUnknowndlEPv",base+0x290bba0,info.dli_fbase);
    long (*main_tid)(void)=dlsym(RTLD_DEFAULT,"c6_linux_mod_main_thread_tid");
    if(!main_tid||main_tid()<=0||main_tid()!=syscall(SYS_gettid))caller_fatal("main_entry_unwitnessed");
    uintptr_t controller=getptr(0x6499100);
    if(!controller||controller>UINTPTR_MAX-0x4a8||getptr(0x6499108)||getword(controller+0x4a0)||getword(controller+0x4a4)||
       getptr(controller+0x450)!=(uintptr_t)context)caller_fatal("pre_acquisition_unknown");
    uintptr_t native_context=getptr((uintptr_t)context+8);
    if(!native_context||native_context>UINTPTR_MAX-0x30)caller_fatal("native_context_extent");
    unsigned char initialized;
    read_field(native_context+8,&initialized,1);
    uintptr_t native_game=getptr(native_context+0x28);
    if(initialized!=1||!native_game||native_game>UINTPTR_MAX-0x2ad)caller_fatal("native_game_extent");
    read_field(native_game+0x2ac,&initialized,1);
    if(initialized!=1||getptr(base+0x3c051a8)!=native_game)caller_fatal("native_game_not_initialized");
    /* Capture the actual utility GOT binding, object and concrete virtual lock
     * targets before the factory can call them. Never infer these from ELF relocation alone. */
    uintptr_t utility_global=getptr(base+0x3b13848),utility=getptr(utility_global);
    if(!utility||getptr(utility)!=0x53a6c90||getptr(0x53a6c90+0x40)!=0x32b3d40||
       getptr(0x53a6c90+0x38)!=0x32b3d60)caller_fatal("utility_lock_dispatch_unknown");
    void *facade=acquire(context);
    if(!facade||(uintptr_t)facade>UINTPTR_MAX-0x1c||getptr((uintptr_t)facade)!=base+0x3aecf60||getword((uintptr_t)facade+0x18)!=1)
        caller_fatal("facade_unknown");
    uintptr_t game=getptr((uintptr_t)facade+8);
    if(game!=native_game)caller_fatal("facade_game_identity");
    c6_linux_current_handler_metadata_fields_begin(module,context,(void *)game,base,main_tid());
    unsigned char before[64],after[64];
    read_field(game+0xf8,before,32);read_field(base+0x3c072f0,before+32,32);
    FILE *out=fopen(paths[2],"wx");if(!out)caller_fatal("control_output_unavailable");
    fprintf(out,"{\"kind\":\"header\",\"schema\":\"c6-linux-current-handler-controls-v1\",\"profile_sha256\":\"%s\",\"original_calls_in_helper_or_guard\":false,\"full_manager_called\":false}\n",profile);
    int rc=0;
    for(unsigned i=3;i<5&&!rc;i++){
        c6_linux_current_handler_metadata_quiescent(module,context,(void *)game);
        rc=c6_linux_context_save(module,context,paths[i]);
        c6_linux_current_handler_metadata_quiescent(module,context,(void *)game);
    }
    for(unsigned i=0;i<2&&!rc;i++){
        C6CurrentHandlerInterruptionAdmission admission={.module=module,.borrowed_context=context,.retained_game=(void *)game,
            .profile_sha256=profile,.output_path=paths[i],.quiescent=c6_linux_current_handler_metadata_quiescent};
        rc=c6_linux_current_handler_interruption_capture(&admission);
        c6_linux_current_handler_metadata_quiescent(module,context,(void *)game);
    }
    for(unsigned i=5;i<7&&!rc;i++){
        rc=c6_linux_context_save(module,context,paths[i]);
        c6_linux_current_handler_metadata_quiescent(module,context,(void *)game);
    }
    c6_linux_current_handler_metadata_quiescent(module,context,(void *)game);
    read_field(game+0xf8,after,32);read_field(base+0x3c072f0,after+32,32);
    int equal=!memcmp(before,after,64);
    if(!equal&&!rc)rc=250;
    fprintf(out,"{\"kind\":\"summary\",\"status\":%d,\"known_rng_equal\":%s,\"complete_rng_inventory\":false,\"before\":",rc,equal?"true":"false");
    hex(out,before,64);fputs(",\"after\":",out);hex(out,after,64);fputs("}\n",out);
    if(ferror(out)&&!rc)rc=247;
    if(fclose(out)&&!rc)rc=247;
    c6_linux_current_handler_metadata_fields_end(module,context,(void *)game);
    release(facade);
    if(getptr(0x6499100)!=controller||getptr(0x6499108)||getword(controller+0x4a0)||getword(controller+0x4a4))
        caller_fatal("post_release_unknown");
    for(unsigned i=0;i<7;i++)free(paths[i]);
    return rc;
}
