#define _GNU_SOURCE
#include "unit_operation_manager_metadata_fixture.h"
#include <dlfcn.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <stdlib.h>
#include "context_stream.h"
#include "unit_operation_manager_metadata.h"

/* A potentially active/unknown owner must never return into fixture teardown.
 * This is a caller-side observation gate, not an original-service replacement. */
static _Noreturn void unit_operation_manager_not_quiescent(const char *reason) {
    fprintf(stderr,"C6_LINUX_UNIT_OPERATION_MANAGER_QUIESCENCE {\"fatal\":true,\"reason\":\"%s\",\"poststate_allowed\":false,\"cleanup_proven\":false}\n",reason);
    fflush(stderr);
    _exit(184);
}

static void *unit_operation_manager_symbol(void *owner,const char *name,uintptr_t expected,void *base) {
    void *symbol=dlsym(owner,name);Dl_info info={0};
    if(!symbol||(uintptr_t)symbol!=expected||!dladdr(symbol,&info)||info.dli_fbase!=base)
        unit_operation_manager_not_quiescent("original_binding_unknown");
    return symbol;
}

/* Single synchronous unit operation manager metadata probe only. These are bindings, never snapshots
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
} UnitOperationManagerBindings;
static UnitOperationManagerBindings unit_operation_manager_bindings;

static Dl_info unit_operation_manager_context_owner(void *module,void *context) {
    Dl_info info={0};
    if(!module||!context||!dladdr(*(void **)context,&info)||!info.dli_fbase||!info.dli_fname)
        unit_operation_manager_not_quiescent("context_owner_unknown");
    const char *leaf=strrchr(info.dli_fname,'/');leaf=leaf?leaf+1:info.dli_fname;
    if(strcmp(leaf,"libGameCore_XP2.so"))unit_operation_manager_not_quiescent("context_module_unknown");
    return info;
}

int c6_linux_unit_operation_manager_metadata_admission_begin(void *module,void *context) {
    if(unit_operation_manager_bindings.module)unit_operation_manager_not_quiescent("admission_already_active");
    Dl_info info=unit_operation_manager_context_owner(module,context);
    uintptr_t base=(uintptr_t)info.dli_fbase;
    UnitOperationManagerBindings b={.module=module,.context=context,.vptr=*(void **)context,.base=info.dli_fbase};
#define UNIT_OPERATION_MANAGER_CORE(name,rva) unit_operation_manager_symbol(module,name,base+(rva),info.dli_fbase)
#define UNIT_OPERATION_MANAGER_HOST(name,address) unit_operation_manager_symbol(RTLD_DEFAULT,name,address,(void *)0x400000)
    b.active=UNIT_OPERATION_MANAGER_HOST("_ZN18GameCoreController21ms_pkActiveControllerE",0x6499100);
    b.initialized=UNIT_OPERATION_MANAGER_HOST("_ZN18GameCoreController13IsInitializedEv",0x31d1720);
    b.executing=UNIT_OPERATION_MANAGER_HOST("_ZN18GameCoreController11IsExecutingEv",0x31d18a0);
    b.get_context=UNIT_OPERATION_MANAGER_HOST("_ZNK18GameCoreController10GetContextEv",0x31d5090);
    b.context_initialized=UNIT_OPERATION_MANAGER_CORE("_ZN8GameCore7Exports7Context12IsInitialiedEv",0x2c7c9b0);
    b.pending=UNIT_OPERATION_MANAGER_CORE("_ZN8GameCore7Exports7Context18HasPendingMessagesEv",0x2c7d340);
    b.get_game=UNIT_OPERATION_MANAGER_CORE("_ZN8GameCore7Exports7Context7GetGameEv",0x2c7cae0);
    b.release=UNIT_OPERATION_MANAGER_CORE("_ZN8GameCore10Interfaces10ICvUnknowndlEPv",0x290bba0);
    b.needs_update=UNIT_OPERATION_MANAGER_CORE("_ZNK8GameCore7Exports4Game11NeedsUpdateEv",0x2c86a60);
    b.needs_advance=UNIT_OPERATION_MANAGER_CORE("_ZNK8GameCore7Exports4Game16NeedsTurnAdvanceEv",0x2c869f0);
    b.ai=UNIT_OPERATION_MANAGER_CORE("_ZNK8GameCore7Exports4Game14IsAiProcessingEv",0x2c867e0);
    b.processing=UNIT_OPERATION_MANAGER_CORE("_ZNK8GameCore7Exports4Game16IsTurnProcessingEv",0x2c86800);
#undef UNIT_OPERATION_MANAGER_CORE
#undef UNIT_OPERATION_MANAGER_HOST
    unit_operation_manager_bindings=b;
    return 0;
}

int c6_linux_unit_operation_manager_metadata_admission_end(void *module,void *context) {
    if(!unit_operation_manager_bindings.module||unit_operation_manager_bindings.module!=module||unit_operation_manager_bindings.context!=context)
        unit_operation_manager_not_quiescent("admission_scope_unknown");
    unit_operation_manager_bindings=(UnitOperationManagerBindings){0};
    return 0;
}

int c6_linux_unit_operation_manager_metadata_quiescent(void *module,void *context) {
    Dl_info info=unit_operation_manager_context_owner(module,context);
    const UnitOperationManagerBindings *b=&unit_operation_manager_bindings;
    if(!b->module||b->module!=module||b->context!=context||b->vptr!=*(void **)context||b->base!=info.dli_fbase)
        unit_operation_manager_not_quiescent("admission_scope_unknown");
    void *controller=*b->active,*bound=NULL;
    if(!controller)unit_operation_manager_not_quiescent("controller_unknown");
    b->get_context(&bound,controller);
    if(*b->active!=controller||bound!=context||!b->initialized(controller)||!b->context_initialized(context)||b->executing(controller)||b->pending(context))
        unit_operation_manager_not_quiescent("controller_or_context_not_idle");
    void *game=b->get_game(context);
    if(!game)unit_operation_manager_not_quiescent("game_facade_unavailable");
    bool work=b->needs_update(game)||b->needs_advance(game)||b->ai(game)||b->processing(game);
    if(work)unit_operation_manager_not_quiescent("game_work_detected");
    /* Only the acquired interface wrapper is owned. Both Context and its native
     * Game remain borrowed; no facade survives this check or any metadata query. */
    b->release(game);
    bound=NULL;b->get_context(&bound,controller);
    if(*b->active!=controller||bound!=context||!b->initialized(controller)||!b->context_initialized(context)||b->executing(controller)||b->pending(context))
        unit_operation_manager_not_quiescent("game_or_controller_not_idle");
    return 0;
}


/* Caller-side controls are separate from raw reconstruction inputs/targets. */
static void manager_hex(FILE *out,const unsigned char *data,size_t size) {
    fputc('"',out);for(size_t i=0;i<size;i++)fprintf(out,"%02x",data[i]);fputc('"',out);
}
int c6_linux_unit_operation_manager_metadata_fixture_probe(void *module,void *context) {
    const char *names[]={"OUTPUT","REPEAT_OUTPUT","CONTROL_OUTPUT","SNAPSHOT_BEFORE","SNAPSHOT_CONTROL","SNAPSHOT_AFTER","SNAPSHOT_REPEAT"};
    const char *paths[7];char key[128];
    for(unsigned i=0;i<7;i++) {
        snprintf(key,sizeof key,"C6_UNIT_OPERATION_MANAGER_METADATA_%s",names[i]);paths[i]=getenv(key);
        if(!paths[i]||!*paths[i])return 242;
        for(unsigned j=0;j<i;j++)if(!strcmp(paths[i],paths[j]))return 242;
    }
    const char *profile=getenv("C6_UNIT_OPERATION_MANAGER_METADATA_PROFILE_SHA256");
    if(!profile||strlen(profile)!=64)return 242;
    for(unsigned i=0;i<64;i++)if(!((profile[i]>='0'&&profile[i]<='9')||(profile[i]>='a'&&profile[i]<='f')))return 242;
    c6_linux_unit_operation_manager_metadata_quiescent(module,context);
    Dl_info info=unit_operation_manager_context_owner(module,context);uintptr_t base=(uintptr_t)info.dli_fbase;
    const void *(*game_get)(void)=unit_operation_manager_symbol(module,"_ZN8GameCore7Context7Globals7GetGameEv",base+0x1fcc320,info.dli_fbase);
    const void *(*sync_get)(const void *)=unit_operation_manager_symbol(module,"_ZNK8GameCore4Game8Instance13GetSyncRandomEv",base+0x2d7b4d0,info.dli_fbase);
    const void *map_rng=unit_operation_manager_symbol(module,"_ZN8GameCore3Map9Generator10ms_kRandomE",base+0x3c072f0,info.dli_fbase);
    const void *game=game_get(),*sync=game?sync_get(game):NULL;
    if(!game||!sync)return 248;
    unsigned char before[64];memcpy(before,sync,32);memcpy(before+32,map_rng,32);
    char *owned_paths[7]={0};
    for(unsigned i=0;i<7;i++) {
        owned_paths[i]=strdup(paths[i]);
        if(!owned_paths[i]){for(unsigned j=0;j<i;j++)free(owned_paths[j]);return 242;}
        paths[i]=owned_paths[i];
    }
    FILE *out=fopen(paths[2],"wx");
    if(!out){for(unsigned i=0;i<7;i++)free(owned_paths[i]);return 242;}
    fprintf(out,"{\"kind\":\"header\",\"schema\":\"c6-linux-unit-operation-manager-controls-v1\",\"profile_sha256\":\"%s\",\"full_manager_called\":false,\"script_callbacks_called\":false}\n",profile);
    int rc=0;
    for(unsigned i=3;i<5&&!rc;i++) {
        c6_linux_unit_operation_manager_metadata_quiescent(module,context);
        rc=c6_linux_context_save(module,context,paths[i]);
        c6_linux_unit_operation_manager_metadata_quiescent(module,context);
    }
    if(!rc)rc=c6_linux_unit_operation_manager_metadata_probe(module,context);
    c6_linux_unit_operation_manager_metadata_quiescent(module,context);
    if(!rc) {
        /* Environment selection changes only this capture's output destination. */
        char *first=strdup(paths[0]);if(!first)rc=242;
        else {
            if(setenv("C6_UNIT_OPERATION_MANAGER_METADATA_OUTPUT",paths[1],1))rc=242;
            else rc=c6_linux_unit_operation_manager_metadata_probe(module,context);
            if(setenv("C6_UNIT_OPERATION_MANAGER_METADATA_OUTPUT",first,1)&&!rc)rc=242;
            free(first);
        }
    }
    c6_linux_unit_operation_manager_metadata_quiescent(module,context);
    for(unsigned i=5;i<7&&!rc;i++) {
        rc=c6_linux_context_save(module,context,paths[i]);
        c6_linux_unit_operation_manager_metadata_quiescent(module,context);
    }
    bool same=!memcmp(before,sync,32)&&!memcmp(before+32,map_rng,32);if(!same&&!rc)rc=250;
    fprintf(out,"{\"kind\":\"summary\",\"status\":%d,\"known_rng_equal\":%s,\"complete_rng_inventory\":false,\"rng_before\":",rc,same?"true":"false");
    manager_hex(out,before,64);fputs(",\"sync_rng_after\":",out);manager_hex(out,sync,32);
    fputs(",\"map_rng_after\":",out);manager_hex(out,map_rng,32);fputs("}\n",out);
    if(ferror(out)&&!rc)rc=247;
    if(fclose(out)&&!rc)rc=247;
    for(unsigned i=0;i<7;i++)free(owned_paths[i]);
    return rc;
}
