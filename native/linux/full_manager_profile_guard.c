#define _GNU_SOURCE
#include "full_manager_profile_guard.h"
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

/* Separate from passive current-handler scope. No original getter, lock,
 * predicate, script, policy or turn is called by these admission callbacks. */
typedef struct {
    void *module,*context;
    uintptr_t base,context_vptr,native_context,game,controller;
    long main_tid;
    uint32_t native_owner,native_depth;
    int locked;
} FullManagerScope;
static FullManagerScope full_scope;
static _Noreturn void full_fatal(const char *reason) {
    fprintf(stderr,"C6_FULL_MANAGER_OWNER {\"fatal\":true,\"reason\":\"%s\",\"no_further_original_calls\":true}\n",reason);
    fflush(stderr);_exit(240);
}
static void full_extent(uintptr_t p,size_t n) {
    if(!p||n>UINTPTR_MAX-p)full_fatal("object_extent_unknown");
}
static void full_copy(uintptr_t p,void *out,size_t n) {
    full_extent(p,n);struct iovec a={out,n},b={(void *)p,n};
    if(process_vm_readv(getpid(),&a,1,&b,1,0)!=(ssize_t)n)full_fatal("field_unreadable");
}
static uintptr_t full_ptr(uintptr_t p){uintptr_t v;full_copy(p,&v,8);return v;}
static uint32_t full_word(uintptr_t p){uint32_t v;full_copy(p,&v,4);return v;}
static unsigned full_byte(uintptr_t p){unsigned char v;full_copy(p,&v,1);return v;}
static void full_check(int locked) {
    const FullManagerScope *s=&full_scope;
    if(!s->module||!s->context||!s->base||!s->game||s->main_tid<=0||syscall(SYS_gettid)!=s->main_tid)
        full_fatal("scope_or_main_thread_unknown");
    full_extent(s->controller,0x4a8);full_extent((uintptr_t)s->context,16);
    full_extent(s->native_context,0x30);full_extent(s->game,0x2ad);
    if(full_ptr(0x6499100)!=s->controller||full_ptr(0x6499108)||
       full_byte(s->controller+0xb)!=1||full_ptr(s->controller+0x450)!=(uintptr_t)s->context)
        full_fatal("controller_or_worker_unknown");
    if(full_ptr((uintptr_t)s->context)!=s->context_vptr||full_ptr((uintptr_t)s->context+8)!=s->native_context||
       full_byte(s->native_context+8)!=1||full_ptr(s->native_context+0x28)!=s->game||
       full_ptr(s->base+0x3c051a8)!=s->game||full_byte(s->game+0x2ac)!=1)
        full_fatal("context_or_game_owner_changed");
    uint32_t owner=full_word(s->controller+0x4a0),depth=full_word(s->controller+0x4a4);
    if(locked) {
        if(!s->locked||!s->native_owner||s->native_depth!=1||owner!=s->native_owner||depth!=s->native_depth)
            full_fatal("retained_native_lock_changed");
    } else if(s->locked||owner||depth)full_fatal("unlocked_boundary_unknown");
    /* NeedsUpdate/NeedsTurnAdvance, AI counters, message queues and Unit
     * membership are deliberately not reset or compared here. A returned
     * predicate may legitimately queue work or remove/replace its actor. The
     * actual acquired native lock + no worker + source-closed top-level caller
     * are the competing-mutator barrier. Lua/GC safety needs the separate
     * actual all-stop frame and original-lock observer, not these fields. */
    if(full_ptr(0x6499100)!=s->controller||full_ptr(0x6499108)||
       full_word(s->controller+0x4a0)!=owner||full_word(s->controller+0x4a4)!=depth)
        full_fatal("owner_changed_during_observation");
}
static void full_identity(void *module,void *context,const void *game) {
    if(full_scope.module!=module||full_scope.context!=context||full_scope.game!=(uintptr_t)game)
        full_fatal("admission_identity_changed");
}
int c6_linux_full_manager_profile_admission_begin(void *module,void *context) {
    if(full_scope.module||!module||!context)full_fatal("duplicate_or_missing_admission");
    full_extent((uintptr_t)context,16);uintptr_t vptr=full_ptr((uintptr_t)context);
    Dl_info info={0};if(!dladdr((void *)vptr,&info)||!info.dli_fbase)full_fatal("context_module_unknown");
    uintptr_t base=(uintptr_t)info.dli_fbase;
    if(base>UINTPTR_MAX-0x3c07380)full_fatal("module_extent_unknown");
    long (*main_tid)(void)=dlsym(RTLD_DEFAULT,"c6_linux_mod_main_thread_tid");
    if(!main_tid)full_fatal("main_entry_unwitnessed");
    uintptr_t native=full_ptr((uintptr_t)context+8);full_extent(native,0x30);
    full_scope=(FullManagerScope){.module=module,.context=context,.base=base,.context_vptr=vptr,
        .native_context=native,.game=full_ptr(native+0x28),.controller=full_ptr(0x6499100),.main_tid=main_tid()};
    full_check(0);return 0;
}
int c6_linux_full_manager_profile_caller_idle(void *module,void *context) {
    full_identity(module,context,(void *)full_scope.game);full_check(0);return 0;
}
int c6_linux_full_manager_profile_admission_end(void *module,void *context) {
    c6_linux_full_manager_profile_caller_idle(module,context);
    memset(&full_scope,0,sizeof full_scope);return 0;
}
int c6_linux_full_manager_pre_acquisition(void *module,void *context,const void *game) {
    full_identity(module,context,game);full_check(0);return 0;
}
int c6_linux_full_manager_locked_begin(void *module,void *context,const void *game,uintptr_t base,long tid) {
    full_identity(module,context,game);
    if(full_scope.locked||base!=full_scope.base||tid!=full_scope.main_tid)
        full_fatal("locked_begin_identity_unknown");
    full_extent(full_scope.controller,0x4a8);
    full_scope.native_owner=full_word(full_scope.controller+0x4a0);
    full_scope.native_depth=full_word(full_scope.controller+0x4a4);full_scope.locked=1;
    full_check(1);return 0;
}
int c6_linux_full_manager_retained_owner(void *module,void *context,const void *game) {
    full_identity(module,context,game);full_check(1);return 0;
}
int c6_linux_full_manager_locked_end(void *module,void *context,const void *game) {
    c6_linux_full_manager_retained_owner(module,context,game);
    full_scope.locked=0;full_scope.native_owner=0;full_scope.native_depth=0;return 0;
}
int c6_linux_full_manager_released_owner(void *module,void *context,const void *game) {
    full_identity(module,context,game);full_check(0);return 0;
}

int c6_linux_full_manager_profile_identity(void *module,void *context,uintptr_t *base,const void **game,long *main_tid) {
    if(!base||!game||!main_tid)full_fatal("identity_output_missing");
    c6_linux_full_manager_profile_caller_idle(module,context);
    *base=full_scope.base;*game=(void *)full_scope.game;*main_tid=full_scope.main_tid;return 0;
}
