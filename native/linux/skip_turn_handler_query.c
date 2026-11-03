#define _GNU_SOURCE
#include "skip_turn_handler_query.h"
#include <dlfcn.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/* Original Operation constructor owns this complete local object. The handler
 * never accesses it; its original destructor sees the constructor's null
 * shared_ptr control block. No copied native object or synthetic receiver. */
typedef struct { _Alignas(8) unsigned char bytes[32]; } OperationStorage;
_Static_assert(sizeof(OperationStorage)==32,"original Operation size");
_Static_assert(_Alignof(OperationStorage)==8,"original Operation alignment");
static uintptr_t pointer_at(const void *object,size_t offset) {
    uintptr_t p;memcpy(&p,(const unsigned char *)object+offset,sizeof p);return p;
}
static void quiescent(int (*idle)(void *,void *),void *module,void *context) {
    int rc=idle(module,context);
    if(rc) {
        fprintf(stderr,"C6_LINUX_SKIP_TURN_HANDLER quiescence_unproven %d no_post_state_or_cleanup\n",rc);
        fflush(stderr);_exit(240);
    }
}
int c6_linux_skip_turn_handler_query(void *module,void *context,
    const void *unit,const void *plot,bool visible_only,
    C6LinuxSkipTurnHandlerResult *result) {
    if(!module||!context||!result)return 241;
    void *anchor=dlsym(module,"_ZN8GameCore4Unit10Operations7Manager3GetEv");
    Dl_info info={0};
    if(!anchor||!dladdr(anchor,&info)||!info.dli_fbase)return 241;
    uintptr_t base=(uintptr_t)info.dli_fbase;
    if((uintptr_t)anchor!=base+0x2da1260)return 241;
    int (*idle)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_placement_metadata_quiescent");
    if(!idle)return 241;
    quiescent(idle,module,context);
    const void *(*manager_get)(void)=(void *)anchor;
    const void *(*handler_get)(const void *,uint32_t)=(void *)(base+0x2da16f0);
    int32_t (*id_for_hash)(const void *,uint32_t)=(void *)(base+0x2da1720);
    uint64_t (*handler_count)(const void *)=(void *)(base+0x2da2f30);
    int32_t (*id_get)(const void *)=(void *)(base+0x2d95610);
    uint32_t (*hash_get)(const void *)=(void *)(base+0x2d95650);
    const void *manager=manager_get();if(!manager)return 242;
    uintptr_t manager_table=pointer_at(manager,0);
    if(manager_table!=base+0x3aff880||
       pointer_at((const void *)manager_table,0xa0)!=base+0x2da1720||
       pointer_at((const void *)manager_table,0x88)!=base+0x2da1650)return 242;
    int32_t resolved_id=id_for_hash(manager,0x2c681908);
    uint64_t slots=handler_count((const unsigned char *)manager+24);
    if(resolved_id<0||(uint64_t)resolved_id>=slots)return 242;
    const void *handler=handler_get(manager,0x2c681908);if(!handler)return 242;
    uintptr_t table=pointer_at(handler,0);
    if(table!=base+0x3b00db0||pointer_at((const void *)table,0x18)!=base+0x2dac950)return 242;
    int32_t id=id_get(handler);uint32_t hash=hash_get(handler);
    if(id!=resolved_id||hash!=0x2c681908)return 242;
    if(unit&&pointer_at(unit,0)!=base+0x3ae5f80)return 243;
    if(plot&&pointer_at(plot,0)!=base+0x3ae23a0)return 243;
    bool allowed=false;
    if(unit) {
        void (*construct)(void *,int32_t)=(void *)(base+0x213d4a0);
        void (*destroy)(void *)=(void *)(base+0x213cac0);
        bool (*can_start)(const void *,const void *,const void *,const void *,bool,void *)=
            (void *)pointer_at((const void *)table,0x18);
        OperationStorage operation;
        construct(&operation,resolved_id);
        quiescent(idle,module,context);
        allowed=can_start(handler,unit,&operation,plot,visible_only,NULL);
        quiescent(idle,module,context);
        destroy(&operation);
    }
    quiescent(idle,module,context);
    *result=(C6LinuxSkipTurnHandlerResult){unit!=NULL,allowed,id,hash};
    return 0;
}
