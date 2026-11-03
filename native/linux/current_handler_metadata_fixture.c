#define _GNU_SOURCE
#include "current_handler_metadata_fixture.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

/* This scope contains no calls into original code. Host ET_EXEC addresses and
 * Core RVAs below are source-layout contracts, not an execution identity gate.
 * The future enclosing caller must independently verify pinned files/mappings,
 * main-entry provenance, facade acquisition and owners before invoking begin. */
typedef struct {
    void *module,*context;
    const void *game;
    uintptr_t base,controller,native_context,context_vptr;
    uint32_t lock_owner,lock_depth;
    long main_tid;
} FieldScope;
static FieldScope scope;

static _Noreturn void fields_fatal(const char *reason) {
    fprintf(stderr,"C6_CURRENT_HANDLER_FIELDS {\"fatal\":true,\"reason\":\"%s\",\"no_poststate_or_cleanup\":true}\n",reason);
    fflush(stderr);
    _exit(240);
}
static void copy_field(uintptr_t address,void *out,size_t count) {
    if(!address||count>UINTPTR_MAX-address)fields_fatal("field_address_unknown");
    struct iovec local={out,count},remote={(void *)address,count};
    if(process_vm_readv(getpid(),&local,1,&remote,1,0)!=(ssize_t)count)
        fields_fatal("field_unreadable");
}
static void object_extent(uintptr_t p,size_t bytes) {
    if(!p||bytes>UINTPTR_MAX-p)fields_fatal("object_extent_unknown");
}
static uintptr_t pointer(uintptr_t p) {uintptr_t v;copy_field(p,&v,sizeof v);return v;}
static uint32_t word(uintptr_t p) {uint32_t v;copy_field(p,&v,sizeof v);return v;}
static unsigned byte(uintptr_t p) {unsigned char v;copy_field(p,&v,sizeof v);return v;}
static void empty_list(uintptr_t list) {
    /* eastl list end is its embedded ListNodeBase. Require both links equal
     * end, rather than execute size()/distance or inspect queued entries. */
    uintptr_t links[2];copy_field(list,links,sizeof links);
    if(links[0]!=list||links[1]!=list)fields_fatal("message_list_not_empty");
}
static void check_fields(const FieldScope *s) {
    if(!s->module||!s->context||!s->game||!s->base||!s->controller||
       !s->native_context||s->main_tid<=0||syscall(SYS_gettid)!=s->main_tid)
        fields_fatal("scope_or_main_thread_unknown");
    object_extent(s->controller,0x4a8);
    object_extent((uintptr_t)s->context,16);
    object_extent(s->native_context,0x30);
    object_extent((uintptr_t)s->game,0x2ad);
    if(pointer(0x6499100)!=s->controller||pointer(0x6499108)!=0||
       byte(s->controller+0xb)!=1||pointer(s->controller+0x450)!=(uintptr_t)s->context)
        fields_fatal("controller_or_worker_unknown");
    if(pointer((uintptr_t)s->context)!=s->context_vptr||
       pointer((uintptr_t)s->context+8)!=s->native_context||
       byte(s->native_context+8)!=1||pointer(s->native_context+0x28)!=(uintptr_t)s->game||
       pointer(s->base+0x3c051a8)!=(uintptr_t)s->game||byte((uintptr_t)s->game+0x2ac)!=1)
        fields_fatal("context_or_game_unknown");
    /* Native owner is an opaque original token, not an OS TID. Its witness is
     * recorded only after the caller's genuine facade acquisition. */
    if(!s->lock_owner||!s->lock_depth||word(s->controller+0x4a0)!=s->lock_owner||
       word(s->controller+0x4a4)!=s->lock_depth)
        fields_fatal("native_lock_witness_changed");
    if(byte(s->base+0x3c05cbb)||byte(s->base+0x3c05cb9)||
       byte(s->base+0x3c05cb8)||byte(s->base+0x3c07378))
        fields_fatal("game_or_message_work");
    uintptr_t global_ai=pointer((uintptr_t)s->game+0x258);
    /* Conservative admission of the exact fast-false path only. A positive
     * processing counter requires the native Player loop, which is forbidden
     * in this passive scope; it is unsupported even if that loop might return false. */
    object_extent(global_ai,0x114);
    if((int32_t)word(global_ai+0x110)>0)
        fields_fatal("ai_fast_false_unproven");
    empty_list(s->base+0x3c07348);
    empty_list(s->base+0x3c07360);
    /* Repeat ownership endpoints after all work/queue observations. */
    if(pointer(0x6499100)!=s->controller||pointer(0x6499108)||
       word(s->controller+0x4a0)!=s->lock_owner||word(s->controller+0x4a4)!=s->lock_depth)
        fields_fatal("owner_changed_during_observation");
}
int c6_linux_current_handler_metadata_fields_begin(void *module,void *context,
 const void *game,uintptr_t base,long main_tid) {
    if(scope.module||!module||!context||!game||!base||base>UINTPTR_MAX-0x3c07380)
        fields_fatal("invalid_begin");
    FieldScope s={.module=module,.context=context,.game=game,.base=base,.main_tid=main_tid};
    s.controller=pointer(0x6499100);
    if(!s.controller)fields_fatal("controller_absent");
    object_extent(s.controller,0x4a8);
    object_extent((uintptr_t)context,16);
    s.context_vptr=pointer((uintptr_t)context);
    s.native_context=pointer((uintptr_t)context+8);
    s.lock_owner=word(s.controller+0x4a0);
    s.lock_depth=word(s.controller+0x4a4);
    check_fields(&s);
    scope=s;
    return 0;
}
int c6_linux_current_handler_metadata_quiescent(void *module,void *context,const void *game) {
    if(scope.module!=module||scope.context!=context||scope.game!=game)
        fields_fatal("scope_identity_mismatch");
    check_fields(&scope);
    return 0;
}
int c6_linux_current_handler_metadata_fields_end(void *module,void *context,const void *game) {
    c6_linux_current_handler_metadata_quiescent(module,context,game);
    memset(&scope,0,sizeof scope);
    return 0;
}
