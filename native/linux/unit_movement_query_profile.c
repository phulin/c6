#define _GNU_SOURCE
#include "unit_movement_query_fixture.h"
#include "unit_movement_query_profile_guard.h"
#include <dlfcn.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/uio.h>
#include <unistd.h>

/* Independent movement-only caller. The runner owns module/host/dependency,
 * input/rules/configuration and actual top-level ownership admission. This
 * wrapper acquires/releases one genuine facade, calls the standalone collector
 * once, and preserves queued effects. No Manager, action, policy, Save or drain. */
static bool movement_running;
static _Noreturn void movement_profile_fatal(const char *why) {
    fprintf(stderr,"C6_UNIT_MOVEMENT_QUERY_PROFILE fatal %s no_further_original_calls\n",why);
    fflush(stderr);_exit(240);
}
static void movement_profile_read(uintptr_t p,void *out,size_t n) {
    if(!p||n>UINTPTR_MAX-p)movement_profile_fatal("invalid_field_extent");
    struct iovec a={out,n},b={(void *)p,n};
    if(process_vm_readv(getpid(),&a,1,&b,1,0)!=(ssize_t)n)movement_profile_fatal("field_unreadable");
}
static uintptr_t movement_profile_ptr(uintptr_t p){uintptr_t v;movement_profile_read(p,&v,8);return v;}
static uint32_t movement_profile_word(uintptr_t p){uint32_t v;movement_profile_read(p,&v,4);return v;}
static void *movement_profile_bind(void *module,const char *name,uintptr_t address,uintptr_t base) {
    void *p=dlsym(module,name);Dl_info info={0};
    if((uintptr_t)p!=address||!dladdr(p,&info)||(uintptr_t)info.dli_fbase!=base)
        movement_profile_fatal("original_binding_changed");
    return p;
}
static void movement_profile_hash_env(const char *key,char out[65]) {
    const char *p=getenv(key);
    if(!p||strlen(p)!=64)movement_profile_fatal("hash_input_missing");
    for(unsigned i=0;i<64;i++)if(!strchr("0123456789abcdef",p[i]))movement_profile_fatal("hash_input_invalid");
    memcpy(out,p,65);
}
static char *movement_profile_path(const char *key) {
    const char *p=getenv(key);
    if(!p||!*p)movement_profile_fatal("path_input_missing");
    char *owned=strdup(p);if(!owned)movement_profile_fatal("path_allocation");return owned;
}
typedef struct {uintptr_t global,object;} MovementUtility;
static MovementUtility movement_profile_utility(uintptr_t base) {
    MovementUtility u={.global=movement_profile_ptr(base+0x3b13848)};
    u.object=movement_profile_ptr(u.global);
    if(movement_profile_ptr(u.object)!=0x53a6c90||
       movement_profile_ptr(0x53a6c90+0x40)!=0x32b3d40||
       movement_profile_ptr(0x53a6c90+0x38)!=0x32b3d60)
        movement_profile_fatal("utility_dispatch_unknown");
    return u;
}
static void movement_profile_facade(void *facade,const void *game,uintptr_t base) {
    uintptr_t p=(uintptr_t)facade;
    if(!p||p>UINTPTR_MAX-0x1c||movement_profile_ptr(p)!=base+0x3aecf60||
       movement_profile_ptr(p+8)!=(uintptr_t)game||movement_profile_word(p+0x18)!=1||
       movement_profile_ptr(base+0x3aecf68)!=base+0x2c86370)
        movement_profile_fatal("facade_identity_unknown");
}
static void movement_profile_phase(const char *phase,const void *game,uintptr_t base) {
    unsigned char rng[64];
    movement_profile_read((uintptr_t)game+0xf8,rng,32);
    movement_profile_read(base+0x3c072f0,rng+32,32);
    uintptr_t controller=movement_profile_ptr(0x6499100);
    if(!controller||controller>UINTPTR_MAX-0x4a8)movement_profile_fatal("controller_extent_unknown");
    fprintf(stderr,"C6_UNIT_MOVEMENT_QUERY_PROFILE {\"kind\":\"phase\",\"phase\":\"%s\",\"native_owner\":%u,\"native_depth\":%u,\"known_rng_raw\":\"",phase,movement_profile_word(controller+0x4a0),movement_profile_word(controller+0x4a4));
    for(unsigned i=0;i<64;i++)fprintf(stderr,"%02x",rng[i]);
    fputs("\",\"complete_rng_inventory\":false}\n",stderr);fflush(stderr);
}
int c6_linux_unit_movement_query_profile_probe(void *module,void *context) {
    if(movement_running)movement_profile_fatal("nested_profile");
    movement_running=true;
    uintptr_t base;const void *game;long main_tid;
    c6_linux_unit_movement_query_profile_identity(module,context,&base,&game,&main_tid);
    char profile[65],candidate[65];
    movement_profile_hash_env("C6_UNIT_MOVEMENT_QUERY_PROFILE_SHA256",profile);
    movement_profile_hash_env("C6_UNIT_MOVEMENT_QUERY_CANDIDATE_CORE_SHA256",candidate);
    /* Identity of the independently frozen Rust reference image, not a native
     * field, desired movement answer or assertion about a produced Save. */
    if(strcmp(candidate,"ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884"))
        movement_profile_fatal("candidate_reference_unsupported");
    char *inputs=movement_profile_path("C6_UNIT_MOVEMENT_QUERY_INPUTS");
    char *answers=movement_profile_path("C6_UNIT_MOVEMENT_QUERY_ANSWERS");
    if(!strcmp(inputs,answers))movement_profile_fatal("duplicate_paths");
    void *(*acquire)(void *)=movement_profile_bind(module,"_ZN8GameCore7Exports7Context7GetGameEv",base+0x2c7cae0,base);
    void (*release)(void *)=movement_profile_bind(module,"_ZN8GameCore10Interfaces10ICvUnknowndlEPv",base+0x290bba0,base);
    (void)movement_profile_bind(module,"_ZN8GameCore7Exports4Game18DecrementReferenceEv",base+0x2c86b80,base);
    if(movement_profile_ptr(base+0x3aecf68)!=base+0x2c86370)movement_profile_fatal("facade_destroy_dispatch_unknown");
    MovementUtility utility=movement_profile_utility(base);
    c6_linux_unit_movement_query_pre_acquisition(module,context,game);
    fprintf(stderr,"C6_UNIT_MOVEMENT_QUERY_PROFILE {\"kind\":\"header\",\"schema\":\"c6-linux-unit-movement-query-profile-v1\",\"profile_sha256\":\"%s\",\"candidate_core_sha256\":\"%s\",\"expected_units\":1262,\"full_manager_called\":false,\"original_action_requested\":false,\"pure_query_claimed\":false,\"Save_equality_is_gate\":false}\n",profile,candidate);fflush(stderr);
    movement_profile_phase("before_acquisition",game,base);
    void *facade=acquire(context);
    movement_profile_facade(facade,game,base);
    c6_linux_unit_movement_query_locked_begin(module,context,game,base,main_tid);
    c6_linux_unit_movement_query_retained_owner(module,context,game);
    movement_profile_phase("before_queries",game,base);
    C6UnitMovementQueryAdmission admission={.module=module,.borrowed_context=context,.retained_game=game,
        .retained_owner=c6_linux_unit_movement_query_retained_owner,.profile_sha256=profile,
        .candidate_core_sha256=candidate,.inputs_path=inputs,.answers_path=answers,.expected_units=1262};
    int status=c6_linux_unit_movement_query_capture(&admission);
    c6_linux_unit_movement_query_retained_owner(module,context,game);
    movement_profile_phase("after_queries",game,base);
    movement_profile_facade(facade,game,base);
    MovementUtility after=movement_profile_utility(base);
    if(after.global!=utility.global||after.object!=utility.object)movement_profile_fatal("release_utility_changed");
    c6_linux_unit_movement_query_locked_end(module,context,game);
    release(facade);facade=NULL;
    c6_linux_unit_movement_query_released_owner(module,context,game);
    movement_profile_phase("after_release",game,base);
    fprintf(stderr,"C6_UNIT_MOVEMENT_QUERY_PROFILE {\"kind\":\"summary\",\"status\":%d,\"facade_released\":true,\"capture_invocations\":1,\"native_query_values_compared\":false,\"complete_affected_state_claimed\":false}\n",status);fflush(stderr);
    free(inputs);free(answers);movement_running=false;return status;
}
