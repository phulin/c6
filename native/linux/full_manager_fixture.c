#define _GNU_SOURCE
#include "full_manager_fixture.h"
#include "context_stream.h"
#include <dlfcn.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

/* Deliberately not wired to any existing loader/profile. Original calls in this
 * file are enclosing fixture work, never part of the field-only guard/observer. */
typedef struct { _Alignas(8) unsigned char bytes[32]; } OperationStorage;
_Static_assert(sizeof(OperationStorage)==32,"original Operation complete storage");
_Static_assert(_Alignof(OperationStorage)==8,"original Operation alignment");
_Static_assert(sizeof(uintptr_t)==8,"Linux x86-64 ABI");
typedef struct {
    void *(*acquire)(void *);
    void (*release)(void *);
    const void *(*manager_get)(void);
    int32_t (*id_for_hash)(const void *,uint32_t);
    const void *(*handler_for_hash)(const void *,uint32_t);
    void (*construct)(void *,int32_t),(*destroy)(void *);
    bool (*can_start)(const void *,const void *,const void *,const void *,bool,void *);
    const void *(*players)(const void *),*(*player)(const void *,int32_t);
    const void *(*units)(const void *),*(*find_unit)(const void *,uint32_t);
    uint32_t (*unit_id)(const void *);
    const void *(*formation)(const void *);
} Bindings;
static _Noreturn void fatal(const char *why) {
    fprintf(stderr,"C6_FULL_MANAGER_FIXTURE fatal %s no_poststate_or_cleanup\n",why);
    fflush(stderr);_exit(240);
}
static void extent(uintptr_t p,size_t bytes) {
    if(!p||bytes>UINTPTR_MAX-p)fatal("invalid_extent");
}
static void copy(uintptr_t p,void *out,size_t n) {
    extent(p,n);
    struct iovec local={out,n},remote={(void *)p,n};
    if(process_vm_readv(getpid(),&local,1,&remote,1,0)!=(ssize_t)n)fatal("unreadable");
}
static uintptr_t ptr(uintptr_t p) {uintptr_t v;copy(p,&v,sizeof v);return v;}
static uint32_t word(uintptr_t p) {uint32_t v;copy(p,&v,sizeof v);return v;}
static void guard(C6FullManagerOwnerGuard check,const C6FullManagerFixtureAdmission *a,const void *game) {
    if(check(a->module,a->context,game))fatal("ownership_unproven");
}
static void *binding(void *module,const char *name,uintptr_t expected,void *base) {
    void *p=dlsym(module,name);Dl_info owner={0};
    if((uintptr_t)p!=expected||!dladdr(p,&owner)||owner.dli_fbase!=base)fatal("original_binding_unknown");
    return p;
}
__attribute__((naked,noinline,used,visibility("default")))
void c6_linux_full_manager_fixture_marker(
    const C6FullManagerObservation *observation __attribute__((unused))) {
    __asm__ volatile("ret");
}
static void notify(const C6FullManagerFixtureAdmission *a,C6FullManagerFixtureResult *r,
                   C6FullManagerObservation *o,C6FullManagerPhase phase,FILE *out) {
    o->phase=phase;o->predicate_returned=r->predicate_returned;o->allowed=r->allowed;
    uintptr_t controller=ptr(0x6499100);extent(controller,0x4a8);
    unsigned char native_lock[8];copy(controller+0x4a0,native_lock,sizeof native_lock);
    fprintf(out,"{\"kind\":\"core_lock_endpoint\",\"phase\":%u,\"controller\":%" PRIuPTR ",\"owner_u32_depth_u32_raw\":\"",
            (unsigned)phase,controller);
    for(size_t i=0;i<sizeof native_lock;i++)fprintf(out,"%02x",native_lock[i]);
    fputs("\",\"owner_is_os_tid\":false,\"owner_is_lua_ThreadHANDLE\":false}\n",out);
    fprintf(out,"{\"kind\":\"phase\",\"phase\":%u,\"predicate_returned\":%s,\"allowed\":%s}\n",
            (unsigned)phase,r->predicate_returned?"true":"false",r->allowed?"true":"false");
    fflush(out); /* Preserve the before-call boundary if originals abort/unwind. */
    c6_linux_full_manager_fixture_marker(o);
    int status=a->observe(o);
    if(status&&!r->observation_status)r->observation_status=status;
}
static void rng(FILE *out,const char *phase,uintptr_t game,uintptr_t base) {
    unsigned char bytes[64];extent(game,0x118);
    copy(game+0xf8,bytes,32);copy(base+0x3c072f0,bytes+32,32);
    fprintf(out,"{\"kind\":\"known_rng\",\"phase\":\"%s\",\"complete_rng_inventory\":false,\"raw\":\"",phase);
    for(size_t i=0;i<sizeof bytes;i++)fprintf(out,"%02x",bytes[i]);
    fputs("\"}\n",out);
}
static const void *actor(const Bindings *b,const C6FullManagerFixtureAdmission *a,const void *game,
                         uintptr_t base) {
    const void *players=b->players(game);
    if(!players)fatal("players_absent");
    const void *player=b->player(players,a->actor_owner);
    if(!player)fatal("player_absent");
    const void *units=b->units(player);
    if(!units)fatal("units_absent");
    const void *unit=b->find_unit(units,a->actor_full_id);
    if(!unit||unit!=a->expected_unit)fatal("actor_not_loaded_member");
    extent((uintptr_t)unit,0x134);
    if(ptr((uintptr_t)unit)!=base+0x3ae5f80||
       (int32_t)word((uintptr_t)unit+0x130)!=a->actor_owner||b->unit_id(unit)!=a->actor_full_id)
        fatal("actor_identity_unknown");
    /* Same conservative admission as reviewed SkipTurn helper; this is not a
     * native false result. The real handler can dereference Formation. */
    if(!b->formation(unit))fatal("actor_formation_absent");
    return unit;
}
static void utility(uintptr_t base,uintptr_t *global,uintptr_t *receiver) {
    *global=ptr(base+0x3b13848);*receiver=ptr(*global);
    if(!*receiver||ptr(*receiver)!=0x53a6c90||
       ptr(0x53a6c90+0x30)!=0x32b3da0||ptr(0x53a6c90+0x38)!=0x32b3d60||
       ptr(0x53a6c90+0x40)!=0x32b3d40)fatal("utility_dispatch_unknown");
}
static void invoke(const Bindings *b,const C6FullManagerFixtureAdmission *a,
                   C6FullManagerFixtureResult *r,C6FullManagerObservation *o,FILE *out) {
    uintptr_t base=o->core_base;
    guard(a->retained_owner,a,o->game);
    const void *manager=b->manager_get();
    if(!manager)fatal("manager_absent");
    extent((uintptr_t)manager,104);
    uintptr_t table=ptr((uintptr_t)manager);
    if(table!=base+0x3aff880||ptr(table+0xa0)!=base+0x2da1720||
       ptr(table+0x88)!=base+0x2da1650)fatal("manager_dispatch_unknown");
    int32_t id=b->id_for_hash(manager,0x2c681908);
    uintptr_t first=ptr((uintptr_t)manager+24),last=ptr((uintptr_t)manager+32);
    if(!first||last<first||(last-first)%8||(last-first)/8>65536||id<0||(uintptr_t)id>=(last-first)/8)
        fatal("runtime_id_unbound");
    const void *handler=b->handler_for_hash(manager,0x2c681908);
    if(!handler)fatal("skipturn_handler_absent");
    extent((uintptr_t)handler,16);
    uintptr_t ht=ptr((uintptr_t)handler);
    if(ht!=base+0x3b00db0||ptr(ht+0x18)!=base+0x2dac950||
       (int32_t)word((uintptr_t)handler+8)!=id||word((uintptr_t)handler+12)!=0x2c681908)
        fatal("skipturn_handler_identity_unknown");
    o->unit=actor(b,a,o->game,base);o->manager=manager;o->operation_id=id;
    r->operation_id=id;r->operation_hash=0x2c681908;
    OperationStorage operation;
    b->construct(&operation,id);
    o->operation=&operation;
    guard(a->retained_owner,a,o->game);
    notify(a,r,o,C6_FULL_MANAGER_PREDICATE_ENTER,out);
    guard(a->retained_owner,a,o->game);
    if(r->observation_status){
        b->destroy(&operation);o->operation=NULL;o->unit=NULL;
        guard(a->retained_owner,a,o->game);return;
    }
    r->predicate_invoked=true;
    /* Genuine complete Manager, not handler+0x18. NULL Plot intentionally
     * selects the Manager's original post-script current-Plot fallback. */
    r->allowed=b->can_start(manager,o->unit,&operation,NULL,a->visible_only,NULL);
    r->predicate_returned=true;
    guard(a->retained_owner,a,o->game);
    notify(a,r,o,C6_FULL_MANAGER_PREDICATE_RETURN,out);
    guard(a->retained_owner,a,o->game);
    /* Operation is local and still alive; Unit may have been removed by a
     * genuine callback, so no stale Unit access is performed after the call. */
    b->destroy(&operation);o->operation=NULL;o->unit=NULL;
    guard(a->retained_owner,a,o->game);
}
static int run_bound(const Bindings *bound,const C6FullManagerFixtureAdmission *a,
                     C6FullManagerFixtureResult *r,uintptr_t base) {
    const Bindings b=*bound;
    bool saves=a->diagnostic_save_paths[0]!=NULL;
    uintptr_t ug,ur;utility(base,&ug,&ur);
    if(ptr(base+0x3aecf68)!=base+0x2c86370)fatal("facade_destroy_unknown");
    FILE *out=fopen(a->output_path,"wx");if(!out)fatal("output_not_fresh");
    fprintf(out,"{\"kind\":\"header\",\"schema\":\"c6-linux-full-manager-fixture-v1\",\"profile_sha256\":\"%s\",\"actor_owner\":%d,\"actor_full_id\":%u,\"visible_only\":%s,\"requested_plot\":null,\"results\":null,\"save_diagnostics_enabled\":%s,\"parity_accepted\":false}\n",
            a->profile_sha256,a->actor_owner,a->actor_full_id,a->visible_only?"true":"false",saves?"true":"false");
    fflush(out);
    guard(a->pre_acquisition,a,a->expected_game);
    void *facade=b.acquire(a->context);
    extent((uintptr_t)facade,0x1c);
    if(ptr((uintptr_t)facade)!=base+0x3aecf60||word((uintptr_t)facade+0x18)!=1||
       ptr((uintptr_t)facade+8)!=(uintptr_t)a->expected_game)fatal("created_facade_unknown");
    if(a->locked_begin(a->module,a->context,a->expected_game,base,a->main_entry_tid))fatal("locked_begin_unproven");
    C6FullManagerObservation o={.module=a->module,.context=a->context,.game=a->expected_game,
        .facade=facade,.core_base=base,.actor_owner=a->actor_owner,.actor_full_id=a->actor_full_id,
        .visible_only=a->visible_only,.operation_id=-1};
    guard(a->retained_owner,a,o.game);
    notify(a,r,&o,C6_FULL_MANAGER_FACADE_ACQUIRED,out);
    for(unsigned i=0;i<4;i++){
        if(i==2){
            rng(out,"before_predicate",(uintptr_t)o.game,base);
            if(!r->observation_status)invoke(&b,a,r,&o,out);
            rng(out,"after_predicate",(uintptr_t)o.game,base);
        }
        guard(a->retained_owner,a,o.game);
        C6FullManagerPhase phase=i==0?C6_FULL_MANAGER_BEFORE:i==1?C6_FULL_MANAGER_CONTROL:i==2?C6_FULL_MANAGER_AFTER:C6_FULL_MANAGER_REPEAT;
        notify(a,r,&o,phase,out);
        guard(a->retained_owner,a,o.game);
        if(saves){
            r->save_status[i]=c6_linux_context_save(a->module,a->context,a->diagnostic_save_paths[i]);
            guard(a->retained_owner,a,o.game);
            fprintf(out,"{\"kind\":\"save_diagnostic\",\"ordinal\":%u,\"status\":%d}\n",i,r->save_status[i]);
        }
    }
    guard(a->retained_owner,a,o.game);
    notify(a,r,&o,C6_FULL_MANAGER_BEFORE_RELEASE,out);
    guard(a->retained_owner,a,o.game);
    if(ptr((uintptr_t)facade)!=base+0x3aecf60||word((uintptr_t)facade+0x18)!=1||
       ptr(base+0x3aecf68)!=base+0x2c86370)fatal("facade_release_changed");
    uintptr_t ug_after,ur_after;utility(base,&ug_after,&ur_after);
    /* A changed utility is preserved as an unsupported release context, never
     * restored. Manager's own original refetch semantics already ran. */
    if(ug_after!=ug||ur_after!=ur)fatal("release_utility_changed");
    guard(a->locked_end,a,o.game);
    b.release(facade);r->facade_released=true;o.facade=NULL;
    guard(a->released_owner,a,o.game);
    notify(a,r,&o,C6_FULL_MANAGER_RELEASED,out);
    fprintf(out,"{\"kind\":\"summary\",\"predicate_invoked\":%s,\"predicate_returned\":%s,\"allowed\":%s,\"facade_released\":true,\"operation_id\":%d,\"operation_hash\":%u,\"observation_status\":%d,\"full_manager_parity\":false,\"complete_state_actions_rng_turns\":false}\n",
            r->predicate_invoked?"true":"false",r->predicate_returned?"true":"false",r->allowed?"true":"false",r->operation_id,r->operation_hash,r->observation_status);
    int io=ferror(out);if(fclose(out))io=1;
    return io?247:r->observation_status;
}
int c6_linux_full_manager_fixture_run(const C6FullManagerFixtureAdmission *a,C6FullManagerFixtureResult *r) {
    if(!a||!r||!a->module||!a->context||!a->expected_game||!a->expected_unit||
       a->actor_owner<0||a->actor_owner>=65||!a->pre_acquisition||!a->locked_begin||
       !a->retained_owner||!a->locked_end||!a->released_owner||!a->observe)
        fatal("admission_incomplete");
    if(a->main_entry_tid<=0||a->main_entry_tid!=syscall(SYS_gettid))fatal("main_entry_unwitnessed");
    if(!a->profile_sha256||strlen(a->profile_sha256)!=64)fatal("profile_missing");
    for(unsigned i=0;i<64;i++)if(!strchr("0123456789abcdef",a->profile_sha256[i]))fatal("profile_invalid");
    if(!a->output_path||!*a->output_path)fatal("output_missing");
    bool saves=a->diagnostic_save_paths[0]!=NULL;
    for(unsigned i=0;i<4;i++){
        const char *p=a->diagnostic_save_paths[i];
        if((p!=NULL)!=saves||(p&&(!*p||!strcmp(p,a->output_path))))fatal("diagnostic_paths_unknown");
        for(unsigned j=0;j<i&&p;j++)if(!strcmp(p,a->diagnostic_save_paths[j]))fatal("diagnostic_paths_duplicate");
    }
    *r=(C6FullManagerFixtureResult){.operation_id=-1};
    guard(a->pre_acquisition,a,a->expected_game);
    Dl_info info={0};extent((uintptr_t)a->context,16);
    if(!dladdr((void *)ptr((uintptr_t)a->context),&info)||!info.dli_fbase)fatal("context_module_unknown");
    uintptr_t base=(uintptr_t)info.dli_fbase;extent(base,0x3c07380);
    Bindings b={0};
#define BIND(field,name,rva) b.field=binding(a->module,name,base+(rva),info.dli_fbase)
    BIND(acquire,"_ZN8GameCore7Exports7Context7GetGameEv",0x2c7cae0);
    BIND(release,"_ZN8GameCore10Interfaces10ICvUnknowndlEPv",0x290bba0);
    BIND(manager_get,"_ZN8GameCore4Unit10Operations7Manager3GetEv",0x2da1260);
    BIND(id_for_hash,"_ZNK8GameCore4Unit10Operations7Manager20GetHandlerIDFromHashENS1_5Types8TypeHashE",0x2da1720);
    BIND(handler_for_hash,"_ZNK8GameCore4Unit10Operations7Manager18GetHandlerFromHashENS1_5Types8TypeHashE",0x2da16f0);
    BIND(construct,"_ZN8GameCore4Unit10Operations9OperationC2ENS_18UnitOperationTypesE",0x213d4a0);
    BIND(destroy,"_ZN8GameCore4Unit10Operations9OperationD2Ev",0x213cac0);
    BIND(can_start,"_ZNK8GameCore4Unit10Operations7Manager8CanStartEPKNS0_8InstanceERKNS1_9OperationEPKNS_3Map4Plot8InstanceEbPN4Data15TypedVariantMapE",0x2da20f0);
    BIND(players,"_ZN8GameCore4Game8Instance16GetPlayerManagerEv",0x20e0670);
    BIND(player,"_ZNK8GameCore6Player7Manager14GetValidPlayerENS_11PlayerTypesE",0x2003940);
    BIND(units,"_ZNK8GameCore6Player8Instance8GetUnitsEv",0x1fccc70);
    BIND(find_unit,"_ZNK8GameCore6Player5Units4FindEj",0x2ba5580);
    BIND(unit_id,"_ZNK8GameCore4Unit9IInstance5GetIDEv",0x2021490);
    BIND(formation,"_ZNK8GameCore4Unit8Instance12GetFormationEv",0x20214e0);
#undef BIND
    return run_bound(&b,a,r,base);
}
