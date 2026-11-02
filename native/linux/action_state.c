#define _GNU_SOURCE
#include "action_state.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define QUEUE_GET "_ZNK8GameCore4Unit8Instance17GetOperationQueueEv"
#define QUEUE_SIZE "_ZNK8GameCore9Utilities5QueueINS_4Unit10Operations4NodeELi4EE7GetSizeEv"

static void quoted(const char *s) {
    fputc('"',stderr);
    for(const unsigned char *p=(const unsigned char *)s;*p;++p) {
        if(*p=='"'||*p=='\\')fprintf(stderr,"\\%c",*p);
        else if(*p<32)fprintf(stderr,"\\u%04x",*p);
        else fputc(*p,stderr);
    }
    fputc('"',stderr);
}
static void hex(const void *data,size_t size) {
    const unsigned char *p=data;
    fputc('"',stderr);
    for(size_t i=0;i<size;++i)fprintf(stderr,"%02x",p[i]);
    fputc('"',stderr);
}
static uint32_t u32(const void *data,size_t offset) {
    uint32_t value;
    memcpy(&value,(const unsigned char *)data+offset,sizeof value);
    return value;
}
static void random_record(const unsigned char raw[32]) {
    fprintf(stderr,"{\"raw32\":");hex(raw,32);
    fprintf(stderr,",\"seed\":%u,\"calls\":%u,\"resets\":%u,\"lock\":%u,\"raw_sync\":%u}",
            u32(raw,8),u32(raw,12),u32(raw,16),u32(raw,20),raw[24]);
}
int c6_linux_action_unit_queue_size(void *module,const void *unit,unsigned *out) {
    if(!module||!unit||!out)return 210;
    const void *(*get)(const void *)=dlsym(module,QUEUE_GET);
    unsigned (*size)(const void *)=dlsym(module,QUEUE_SIZE);
    if(!get||!size)return 211;
    const void *queue=get(unit);
    if(!queue)return 212;
    *out=size(queue);
    return 0;
}
int c6_linux_action_state_capture(void *module,const void *unit,const char *stage) {
    if(!module||!stage)return 210;
    const void *(*game_get)(void)=dlsym(module,"_ZN8GameCore4Game11GetInstanceEv");
    const void *(*sync_get)(const void *)=dlsym(module,"_ZNK8GameCore4Game8Instance13GetSyncRandomEv");
    const void *map_random=dlsym(module,"_ZN8GameCore3Map9Generator10ms_kRandomE");
    if(!game_get||!sync_get||!map_random)return 211;
    const void *game=game_get();
    if(!game)return 212;
    const void *sync_random=sync_get(game);
    if(!sync_random)return 212;
    unsigned char sync_before[32],map_before[32];
    memcpy(sync_before,sync_random,32);memcpy(map_before,map_random,32);
    int (*activity)(const void *)=dlsym(module,"_ZNK8GameCore4Unit9CInstance15GetActivityTypeEv");
    int (*automation)(const void *)=dlsym(module,"_ZNK8GameCore4Unit9CInstance15GetAutomateTypeEv");
    int (*timer)(const void *)=dlsym(module,"_ZNK8GameCore4Unit8Instance17GetOperationTimerEv");
    /* FixedPoint has a nontrivial C++ copy constructor despite its 4-byte size:
     * explicit hidden return storage RDI, genuine unit receiver RSI. */
    void *(*moves)(void *,const void *)=dlsym(module,"_ZNK8GameCore4Unit8Instance17GetMovesRemainingEv");
    const void *(*queue_get)(const void *)=dlsym(module,QUEUE_GET);
    unsigned (*queue_size)(const void *)=dlsym(module,QUEUE_SIZE);
    const void *(*queue_at)(const void *,unsigned)=dlsym(module,"_ZNK8GameCore9Utilities5QueueINS_4Unit10Operations4NodeELi4EE5GetAtEj");
    const void *(*archive_get)(const void *)=dlsym(module,"_ZN8GameCore4Unit8Instance14GetAutoArchiveEv");
    bool (*has_deltas)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK12FAutoArchive9hasDeltasEv");
    void *(*delta_indexes)(void *,const void *)=dlsym(RTLD_DEFAULT,"_ZNK12FAutoArchive15getDeltaIndexesEv");
    unsigned (*vector_size)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK5Types6VectorIjNS_11LinearAlloc7DefaultILj1ELj0EEEE4sizeEv");
    const unsigned *(*vector_at)(const void *,size_t)=dlsym(RTLD_DEFAULT,"_ZNK5Types6VectorIjNS_11LinearAlloc7DefaultILj1ELj0EEEEixEm");
    void (*vector_destroy)(void *)=dlsym(RTLD_DEFAULT,"_ZN5Types6VectorIjNS_11LinearAlloc7DefaultILj1ELj0EEEED2Ev");
    if(unit&&(!activity||!automation||!timer||!moves||!queue_get||!queue_size||!queue_at||!archive_get||!has_deltas||!delta_indexes||!vector_size||!vector_at||!vector_destroy))return 211;
    fprintf(stderr,"C6_LINUX_ACTION_STATE {\"stage\":");quoted(stage);
    fprintf(stderr,",\"unit_present\":%s",unit?"true":"false");
    int result=0;
    if(unit) {
        uint32_t movement;
        moves(&movement,unit);
        const void *queue=queue_get(unit),*archive=archive_get(unit);
        if(!queue||!archive){fprintf(stderr,",\"error\":\"missing_original_container\"}\n");return 212;}
        unsigned count=queue_size(queue);
        fprintf(stderr,",\"activity\":%d,\"automation\":%d,\"operation_timer\":%d,\"moves_raw_u32\":%u,\"queue_size\":%u,\"queue\":[",activity(unit),automation(unit),timer(unit),movement,count);
        for(unsigned i=0;i<count;++i) {
            const void *node=queue_at(queue,i);
            if(i)fputc(',',stderr);
            if(!node){fputs("null",stderr);result=212;continue;}
            fprintf(stderr,"{\"index\":%u,\"raw40\":",i);hex(node,40);
            fprintf(stderr,",\"operation\":%u,\"directive\":%u,\"flags\":%u,\"push_turn\":%u,\"pre_activity\":%u,\"pre_automation\":%u}",u32(node,0),u32(node,4),u32(node,24),u32(node,28),u32(node,32),u32(node,36));
        }
        /* Uninitialized output storage is constructed completely by the native
         * getDeltaIndexes; never fabricate vector internals or reorder entries. */
        _Alignas(8) unsigned char indexes[24];
        bool dirty=has_deltas(archive);
        delta_indexes(indexes,archive);
        unsigned size=vector_size(indexes);
        fprintf(stderr,"],\"has_deltas\":%s,\"dirty_indexes\":[",dirty?"true":"false");
        for(unsigned i=0;i<size;++i) {
            if(i)fputc(',',stderr);
            fprintf(stderr,"%u",*vector_at(indexes,i));
        }
        fputc(']',stderr);
        vector_destroy(indexes);
    }
    fprintf(stderr,",\"game_sync_random\":");random_record(sync_before);
    fprintf(stderr,",\"map_generator_random\":");random_record(map_before);
    bool unchanged=!memcmp(sync_before,sync_random,32)&&!memcmp(map_before,map_random,32);
    fprintf(stderr,",\"rng_unchanged_during_observation\":%s,\"complete_affected_state\":false}\n",unchanged?"true":"false");
    if(!unchanged)return 213;
    return result;
}
