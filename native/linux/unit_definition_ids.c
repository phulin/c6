#define _GNU_SOURCE
#include "unit_definition_ids.h"
#include <dlfcn.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* Exact supplied Linux XP2 offsets, independently fingerprinted by
 * unit_definition_ids_audit.py. The caller must enforce the module/configuration gate.
 * Native getters do not acquire wrapper objects or mutate the registry. */
int c6_linux_unit_definition_ids(void *module, void *context) {
    (void)context;
    void *symbol=dlsym(module,"_ZN8GameCore11Definitions18GetUnitDefinitionsEv");
    Dl_info info;
    if(!symbol || !dladdr(symbol,&info) || !info.dli_fbase)return 180;
    uintptr_t base=(uintptr_t)info.dli_fbase;
    if((uintptr_t)symbol!=base+0x22147d0)return 181;
    void *(*get)(void)=(void *(*)(void))symbol;
    uint64_t (*count)(const void *)=(void *)(base+0x1fe82a0);
    void *(*at)(const void *,uint32_t)=(void *)(base+0x1fcd120);
    uint32_t (*hash)(const void *)=(void *)(base+0x1fe5a70);
    uint32_t (*index)(const void *)=(void *)(base+0x1fcccb0);
    int32_t (*row)(const void *)=(void *)(base+0x2114a70);
    const char *(*name)(const void *)=(void *)(base+0x1fe4b00);
    int32_t (*find)(const void *,uint32_t)=(void *)(base+0x290b650);
    uint32_t (*make_hash)(const char *)=(void *)(base+0x2d577c0);
    void *collection=get();
    if(!collection)return 182;
    uint64_t size=count(collection);
    if(!size || size>1000000)return 183;
    uint32_t n=(uint32_t)size;
    if(!n || n>1000000)return 183;
    fprintf(stderr,"C6_LINUX_UNIT_DEFINITION_IDS {\"registry\":\"Units\",\"count\":%" PRIu32 ",\"entries\":[",n);
    for(uint32_t i=0;i<n;i++) {
        void *definition=at(collection,i);
        if(!definition)return 184;
        const char *type=name(definition);
        if(!type || strnlen(type,4096)==4096)return 185;
        uint32_t h=hash(definition);
        fprintf(stderr,"%s{\"index\":%" PRIu32 ",\"database_rowid\":%" PRId32 ",\"hash\":%" PRIu32 ",\"find_index\":%" PRId32 ",\"native_string_hash\":%" PRIu32 ",\"type_hex\":\"",i?",":"",index(definition),row(definition),h,find(collection,h),make_hash(type));
        for(const unsigned char *p=(const unsigned char *)type;*p;p++)fprintf(stderr,"%02x",*p);
        fprintf(stderr,"\"}");
    }
    /* Include both numeric-index and an absent-hash boundary. A value >=count
     * is sought until FindIndex returns -1, rather than assuming zero is absent. */
    uint32_t missing=UINT32_MAX;
    while(missing>=n && find(collection,missing)!=-1)missing--;
    if(missing<n)return 186;
    fprintf(stderr,"],\"index_zero\":%" PRId32 ",\"at_count_is_null\":%s,\"missing_hash\":%" PRIu32 ",\"missing_result\":%" PRId32 ",\"empty_string_hash\":%" PRIu32 ",\"lowercase_warrior_hash\":%" PRIu32 "}\n",find(collection,0),at(collection,n)?"false":"true",missing,find(collection,missing),make_hash(""),make_hash("unit_warrior"));
    return 0;
}
