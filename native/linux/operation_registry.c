#define _GNU_SOURCE
#include "operation_registry.h"
#include <dlfcn.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* Audited in operation_registry_audit.py. Never calls Initialize, Register,
 * Edit, MakeParameters or a handler factory. Handler identities are read only. */
static uintptr_t pointer_at(const void *p, size_t offset) {
    uintptr_t value;
    memcpy(&value,(const unsigned char *)p+offset,sizeof value);
    return value;
}
int c6_linux_operation_registry(void *module, void *context) {
    (void)context;
    void *symbol=dlsym(module,"_ZN8GameCore4Unit10Operations7Manager3GetEv");
    Dl_info info;
    if(!symbol || !dladdr(symbol,&info) || !info.dli_fbase)return 240;
    uintptr_t base=(uintptr_t)info.dli_fbase;
    if((uintptr_t)symbol!=base+0x2da1260)return 241;
    const void *(*get_manager)(void)=(void *)symbol;
    const void *(*get_definitions)(void)=(void *)(base+0x2214890);
    uint64_t (*definition_count)(const void *)=(void *)(base+0x20e9bd0);
    const void *(*definition_at)(const void *,uint32_t)=(void *)(base+0x20b3d20);
    uint32_t (*definition_index)(const void *)=(void *)(base+0x20b42c0);
    uint32_t (*definition_hash)(const void *)=(void *)(base+0x20b4900);
    uint64_t (*handler_count)(const void *)=(void *)(base+0x2da2f30);
    const void *(*handler_at)(const void *,int32_t)=(void *)(base+0x2da1650);
    const void *(*handler_for_hash)(const void *,uint32_t)=(void *)(base+0x2da16f0);
    int32_t (*id_for_hash)(const void *,uint32_t)=(void *)(base+0x2da1720);
    int32_t (*handler_id)(const void *)=(void *)(base+0x2d95610);
    uint32_t (*handler_hash)(const void *)=(void *)(base+0x2d95650);
    const void *manager=get_manager(), *definitions=get_definitions();
    if(!manager || !definitions)return 242;
    uint64_t n=definition_count(definitions);
    uint64_t slots=handler_count((const unsigned char *)manager+24);
    /* Bound actual original storage before const GetHandler, which has no bounds check. */
    if(!n || n>1000000 || slots!=n)return 243;
    fprintf(stderr,"C6_LINUX_OPERATION_REGISTRY {\"count\":%" PRIu64 ",\"handler_slots\":%" PRIu64 ",\"entries\":[",n,slots);
    for(uint32_t i=0;i<n;i++) {
        const void *definition=definition_at(definitions,i);
        if(!definition || definition_index(definition)!=i)return 244;
        uint32_t hash=definition_hash(definition);
        const void *handler=handler_at(manager,(int32_t)i);
        int32_t resolved=id_for_hash(manager,hash);
        const void *by_hash=handler_for_hash(manager,hash);
        fprintf(stderr,"%s{\"id\":%" PRIu32 ",\"hash\":%" PRIu32 ",\"resolved_id\":%" PRId32 ",\"handler_present\":%s,\"lookup_matches\":%s",i?",":"",i,hash,resolved,handler?"true":"false",by_hash==handler?"true":"false");
        if(handler) {
            uintptr_t table=pointer_at(handler,0);
            Dl_info table_info;
            if(!dladdr((void *)table,&table_info) || table_info.dli_fbase!=info.dli_fbase)return 245;
            fprintf(stderr,",\"handler_id\":%" PRId32 ",\"handler_hash\":%" PRIu32 ",\"vtable_rva\":%" PRIuPTR ",\"make_parameters_rva\":%" PRIuPTR ",\"write_rva\":%" PRIuPTR ",\"read_rva\":%" PRIuPTR,handler_id(handler),handler_hash(handler),table-base,pointer_at((void *)table,0x80)-base,pointer_at((void *)table,0xb0)-base,pointer_at((void *)table,0xb8)-base);
        }
        fprintf(stderr,"}");
    }
    uint32_t missing=UINT32_MAX;
    while(missing>=n && id_for_hash(manager,missing)!=-1)missing--;
    if(missing<n)return 246;
    fprintf(stderr,"],\"missing_hash\":%" PRIu32 ",\"missing_id\":%" PRId32 ",\"missing_handler_is_null\":%s}\n",missing,id_for_hash(manager,missing),handler_for_hash(manager,missing)?"false":"true");
    return 0;
}
