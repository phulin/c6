#define _GNU_SOURCE
#include "domain_metadata_fixture.h"
#include <dlfcn.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/* A potentially active/unknown owner must never return into fixture teardown.
 * This is a caller-side observation gate, not an original-service replacement. */
static _Noreturn void domain_not_quiescent(const char *reason) {
    fprintf(stderr,"C6_LINUX_DOMAIN_QUIESCENCE {\"fatal\":true,\"reason\":\"%s\",\"poststate_allowed\":false,\"cleanup_proven\":false}\n",reason);
    fflush(stderr);
    _exit(184);
}

static void *domain_symbol(void *owner,const char *name,uintptr_t expected,void *base) {
    void *symbol=dlsym(owner,name);Dl_info info={0};
    if(!symbol||(uintptr_t)symbol!=expected||!dladdr(symbol,&info)||info.dli_fbase!=base)
        domain_not_quiescent("original_binding_unknown");
    return symbol;
}

/* Single synchronous domain metadata probe only. These are bindings, never snapshots
 * of controller/context state. The caller holds its RTLD_NOLOAD reference. */
typedef struct {
    void *module,*context,*vptr,*base;
    void **active;
    bool (*initialized)(void *);
    bool (*executing)(void *);
    void *(*get_context)(void *,const void *);
    bool (*context_initialized)(void *);
    bool (*pending)(void *);
    void *(*get_game)(void *);
    void (*release)(void *);
    bool (*needs_update)(const void *);
    bool (*needs_advance)(const void *);
    bool (*ai)(const void *);
    bool (*processing)(const void *);
} DomainBindings;
static DomainBindings domain_bindings;

static Dl_info domain_context_owner(void *module,void *context) {
    Dl_info info={0};
    if(!module||!context||!dladdr(*(void **)context,&info)||!info.dli_fbase||!info.dli_fname)
        domain_not_quiescent("context_owner_unknown");
    const char *leaf=strrchr(info.dli_fname,'/');leaf=leaf?leaf+1:info.dli_fname;
    if(strcmp(leaf,"libGameCore_XP2.so"))domain_not_quiescent("context_module_unknown");
    return info;
}

int c6_linux_domain_metadata_admission_begin(void *module,void *context) {
    if(domain_bindings.module)domain_not_quiescent("admission_already_active");
    Dl_info info=domain_context_owner(module,context);
    uintptr_t base=(uintptr_t)info.dli_fbase;
    DomainBindings b={.module=module,.context=context,.vptr=*(void **)context,.base=info.dli_fbase};
#define DOMAIN_CORE(name,rva) domain_symbol(module,name,base+(rva),info.dli_fbase)
#define DOMAIN_HOST(name,address) domain_symbol(RTLD_DEFAULT,name,address,(void *)0x400000)
    b.active=DOMAIN_HOST("_ZN18GameCoreController21ms_pkActiveControllerE",0x6499100);
    b.initialized=DOMAIN_HOST("_ZN18GameCoreController13IsInitializedEv",0x31d1720);
    b.executing=DOMAIN_HOST("_ZN18GameCoreController11IsExecutingEv",0x31d18a0);
    b.get_context=DOMAIN_HOST("_ZNK18GameCoreController10GetContextEv",0x31d5090);
    b.context_initialized=DOMAIN_CORE("_ZN8GameCore7Exports7Context12IsInitialiedEv",0x2c7c9b0);
    b.pending=DOMAIN_CORE("_ZN8GameCore7Exports7Context18HasPendingMessagesEv",0x2c7d340);
    b.get_game=DOMAIN_CORE("_ZN8GameCore7Exports7Context7GetGameEv",0x2c7cae0);
    b.release=DOMAIN_CORE("_ZN8GameCore10Interfaces10ICvUnknowndlEPv",0x290bba0);
    b.needs_update=DOMAIN_CORE("_ZNK8GameCore7Exports4Game11NeedsUpdateEv",0x2c86a60);
    b.needs_advance=DOMAIN_CORE("_ZNK8GameCore7Exports4Game16NeedsTurnAdvanceEv",0x2c869f0);
    b.ai=DOMAIN_CORE("_ZNK8GameCore7Exports4Game14IsAiProcessingEv",0x2c867e0);
    b.processing=DOMAIN_CORE("_ZNK8GameCore7Exports4Game16IsTurnProcessingEv",0x2c86800);
#undef DOMAIN_CORE
#undef DOMAIN_HOST
    domain_bindings=b;
    return 0;
}

int c6_linux_domain_metadata_admission_end(void *module,void *context) {
    if(!domain_bindings.module||domain_bindings.module!=module||domain_bindings.context!=context)
        domain_not_quiescent("admission_scope_unknown");
    domain_bindings=(DomainBindings){0};
    return 0;
}

int c6_linux_domain_metadata_quiescent(void *module,void *context) {
    Dl_info info=domain_context_owner(module,context);
    const DomainBindings *b=&domain_bindings;
    if(!b->module||b->module!=module||b->context!=context||b->vptr!=*(void **)context||b->base!=info.dli_fbase)
        domain_not_quiescent("admission_scope_unknown");
    void *controller=*b->active,*bound=NULL;
    if(!controller)domain_not_quiescent("controller_unknown");
    b->get_context(&bound,controller);
    if(*b->active!=controller||bound!=context||!b->initialized(controller)||!b->context_initialized(context)||b->executing(controller)||b->pending(context))
        domain_not_quiescent("controller_or_context_not_idle");
    void *game=b->get_game(context);
    if(!game)domain_not_quiescent("game_facade_unavailable");
    bool work=b->needs_update(game)||b->needs_advance(game)||b->ai(game)||b->processing(game);
    if(work)domain_not_quiescent("game_work_detected");
    /* Only the acquired interface wrapper is owned. Both Context and its native
     * Game remain borrowed; no facade survives this check or any metadata query. */
    b->release(game);
    bound=NULL;b->get_context(&bound,controller);
    if(*b->active!=controller||bound!=context||!b->initialized(controller)||!b->context_initialized(context)||b->executing(controller)||b->pending(context))
        domain_not_quiescent("game_or_controller_not_idle");
    return 0;
}

