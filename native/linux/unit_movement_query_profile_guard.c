#define _GNU_SOURCE
#include "unit_movement_query_profile_guard.h"
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

/* Separate from passive current-handler scope. No original getter, lock,
 * query, script, policy or turn is called by these admission callbacks. */
typedef struct {
    void *module,*context;
    uintptr_t base,context_vptr,native_context,game,controller;
    long main_tid;
    uint32_t native_owner,native_depth;
    int locked;
} MovementQueryScope;
static MovementQueryScope movement_scope;
static _Noreturn void movement_fatal(const char *reason) {
    fprintf(stderr,"C6_UNIT_MOVEMENT_QUERY_OWNER {\"fatal\":true,\"reason\":\"%s\",\"no_further_original_calls\":true}\n",reason);
    fflush(stderr);_exit(240);
}
static void movement_extent(uintptr_t p,size_t n) {
    if(!p||n>UINTPTR_MAX-p)movement_fatal("object_extent_unknown");
}
static void movement_copy(uintptr_t p,void *out,size_t n) {
    movement_extent(p,n);struct iovec a={out,n},b={(void *)p,n};
    if(process_vm_readv(getpid(),&a,1,&b,1,0)!=(ssize_t)n)movement_fatal("field_unreadable");
}
static uintptr_t movement_ptr(uintptr_t p){uintptr_t v;movement_copy(p,&v,8);return v;}
static uint32_t movement_word(uintptr_t p){uint32_t v;movement_copy(p,&v,4);return v;}
static unsigned movement_byte(uintptr_t p){unsigned char v;movement_copy(p,&v,1);return v;}
static void movement_check(int locked) {
    const MovementQueryScope *s=&movement_scope;
    if(!s->module||!s->context||!s->base||!s->game||s->main_tid<=0||syscall(SYS_gettid)!=s->main_tid)
        movement_fatal("scope_or_main_thread_unknown");
    movement_extent(s->controller,0x4a8);movement_extent((uintptr_t)s->context,16);
    movement_extent(s->native_context,0x30);movement_extent(s->game,0x2ad);
    if(movement_ptr(0x6499100)!=s->controller||movement_ptr(0x6499108)||
       movement_byte(s->controller+0xb)!=1||movement_ptr(s->controller+0x450)!=(uintptr_t)s->context)
        movement_fatal("controller_or_worker_unknown");
    if(movement_ptr((uintptr_t)s->context)!=s->context_vptr||movement_ptr((uintptr_t)s->context+8)!=s->native_context||
       movement_byte(s->native_context+8)!=1||movement_ptr(s->native_context+0x28)!=s->game||
       movement_ptr(s->base+0x3c051a8)!=s->game||movement_byte(s->game+0x2ac)!=1)
        movement_fatal("context_or_game_owner_changed");
    uint32_t owner=movement_word(s->controller+0x4a0),depth=movement_word(s->controller+0x4a4);
    if(locked) {
        if(!s->locked||!s->native_owner||s->native_depth!=1||owner!=s->native_owner||depth!=s->native_depth)
            movement_fatal("retained_native_lock_changed");
    } else if(s->locked||owner||depth)movement_fatal("unlocked_boundary_unknown");
    /* NeedsUpdate/NeedsTurnAdvance, AI counters, message queues and Unit
     * membership are deliberately not reset or compared here. A returned
     * query interval must preserve queued gameplay effects. The
     * actual acquired native lock + no worker + source-closed top-level caller
     * are the competing-mutator barrier. The movement-only source closure is
     * reviewed separately; these guards introduce no debugger or VM gate. */
    if(movement_ptr(0x6499100)!=s->controller||movement_ptr(0x6499108)||
       movement_word(s->controller+0x4a0)!=owner||movement_word(s->controller+0x4a4)!=depth)
        movement_fatal("owner_changed_during_observation");
}
static void movement_identity(void *module,void *context,const void *game) {
    if(movement_scope.module!=module||movement_scope.context!=context||movement_scope.game!=(uintptr_t)game)
        movement_fatal("admission_identity_changed");
}
int c6_linux_unit_movement_query_profile_admission_begin(void *module,void *context) {
    if(movement_scope.module||!module||!context)movement_fatal("duplicate_or_missing_admission");
    movement_extent((uintptr_t)context,16);uintptr_t vptr=movement_ptr((uintptr_t)context);
    Dl_info info={0};if(!dladdr((void *)vptr,&info)||!info.dli_fbase)movement_fatal("context_module_unknown");
    uintptr_t base=(uintptr_t)info.dli_fbase;
    if(base>UINTPTR_MAX-0x3c07380)movement_fatal("module_extent_unknown");
    long (*main_tid)(void)=dlsym(RTLD_DEFAULT,"c6_linux_mod_main_thread_tid");
    if(!main_tid)movement_fatal("main_entry_unwitnessed");
    uintptr_t native=movement_ptr((uintptr_t)context+8);movement_extent(native,0x30);
    movement_scope=(MovementQueryScope){.module=module,.context=context,.base=base,.context_vptr=vptr,
        .native_context=native,.game=movement_ptr(native+0x28),.controller=movement_ptr(0x6499100),.main_tid=main_tid()};
    movement_check(0);return 0;
}
int c6_linux_unit_movement_query_profile_caller_idle(void *module,void *context) {
    movement_identity(module,context,(void *)movement_scope.game);movement_check(0);return 0;
}
int c6_linux_unit_movement_query_profile_admission_end(void *module,void *context) {
    c6_linux_unit_movement_query_profile_caller_idle(module,context);
    memset(&movement_scope,0,sizeof movement_scope);return 0;
}
int c6_linux_unit_movement_query_pre_acquisition(void *module,void *context,const void *game) {
    movement_identity(module,context,game);movement_check(0);return 0;
}
int c6_linux_unit_movement_query_locked_begin(void *module,void *context,const void *game,uintptr_t base,long tid) {
    movement_identity(module,context,game);
    if(movement_scope.locked||base!=movement_scope.base||tid!=movement_scope.main_tid)
        movement_fatal("locked_begin_identity_unknown");
    movement_extent(movement_scope.controller,0x4a8);
    movement_scope.native_owner=movement_word(movement_scope.controller+0x4a0);
    movement_scope.native_depth=movement_word(movement_scope.controller+0x4a4);movement_scope.locked=1;
    movement_check(1);return 0;
}
int c6_linux_unit_movement_query_retained_owner(void *module,void *context,const void *game) {
    movement_identity(module,context,game);movement_check(1);return 0;
}
int c6_linux_unit_movement_query_locked_end(void *module,void *context,const void *game) {
    c6_linux_unit_movement_query_retained_owner(module,context,game);
    movement_scope.locked=0;movement_scope.native_owner=0;movement_scope.native_depth=0;return 0;
}
int c6_linux_unit_movement_query_released_owner(void *module,void *context,const void *game) {
    movement_identity(module,context,game);movement_check(0);return 0;
}

int c6_linux_unit_movement_query_profile_identity(void *module,void *context,uintptr_t *base,const void **game,long *main_tid) {
    if(!base||!game||!main_tid)movement_fatal("identity_output_missing");
    c6_linux_unit_movement_query_profile_caller_idle(module,context);
    *base=movement_scope.base;*game=(void *)movement_scope.game;*main_tid=movement_scope.main_tid;return 0;
}
