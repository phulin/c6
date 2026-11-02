#define _GNU_SOURCE
#include "temp_heap.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define NATIVE_TEMP_BYTES ((size_t)16777216)
static bool owned;
static void *heap_owner;
static void *arena_owner;

/* Read-only observations of the genuine object, proved by this exact host's
 * GetTempHeap/Startup/Allocate/DeAllocate/Shutdown instructions. No fields are
 * written here. The lock word at +0 tracks outstanding allocation read locks;
 * +0x4c is not an allocation counter. */
struct observation {
    uint32_t locks;
    void *arena;
    uint32_t cursor;
    size_t capacity;
    unsigned char storage_flags;
};
static struct observation observe(const void *heap) {
    struct observation result;
    const unsigned char *p=heap;
    memcpy(&result.locks,p,sizeof result.locks);
    memcpy(&result.arena,p+0x40,sizeof result.arena);
    memcpy(&result.cursor,p+0x48,sizeof result.cursor);
    memcpy(&result.capacity,p+0x50,sizeof result.capacity);
    memcpy(&result.storage_flags,p+0x58,sizeof result.storage_flags);
    return result;
}
static void *symbol(const char *name) {
    void *value=dlsym(RTLD_DEFAULT,name);
    if(!value)fprintf(stderr,"C6_LINUX_TEMPHEAP missing_symbol %s\n",name);
    return value;
}

int c6_linux_temp_heap_initialize(void) {
    void *(*get_heap)(void)=symbol("_ZN8Platform11GetTempHeapEv");
    size_t (*startup)(void *,size_t,unsigned char *)=symbol("_ZN8Platform8TempHeap7StartupEmPh");
    void *(*allocate)(void *,size_t)=symbol("_ZN8Platform8TempHeap8AllocateEm");
    void (*deallocate)(void *,const void *)=symbol("_ZN8Platform8TempHeap10DeAllocateEPKv");
    bool (*owns)(const void *,const void *)=symbol("_ZNK8Platform8TempHeap14OwnsAllocationEPKv");
    bool (*jobs_initialized)(void)=symbol("_ZN8Platform10JobManager13IsInitializedEv");
    if(!get_heap||!startup||!allocate||!deallocate||!owns||!jobs_initialized)return 180;
    void *heap=get_heap();
    if(!heap)return 181;
    struct observation before=observe(heap);
    if(owned)return heap==heap_owner&&before.arena==arena_owner&&
                    before.capacity==NATIVE_TEMP_BYTES?0:182;
    bool jobs=jobs_initialized();
    fprintf(stderr,"C6_LINUX_TEMPHEAP before arena_nonnull %d locks %u cursor %u capacity %zu jobs %d\n",
            before.arena!=NULL,before.locks,before.cursor,before.capacity,jobs);
    /* Do not allocate into or overwrite another native heap/provider owner. */
    if(before.arena||before.locks||before.cursor||before.capacity||jobs)return 183;
    fprintf(stderr,"C6_LINUX_TEMPHEAP startup_enter bytes %zu storage_null 1\n",NATIVE_TEMP_BYTES);
    size_t returned=startup(heap,NATIVE_TEMP_BYTES,NULL);
    struct observation started=observe(heap);
    bool ready=returned==NATIVE_TEMP_BYTES&&started.arena&&
               started.capacity==NATIVE_TEMP_BYTES&&!started.locks&&!started.cursor&&
               (started.storage_flags&1);
    fprintf(stderr,"C6_LINUX_TEMPHEAP startup_return bytes %zu arena_ready %d\n",returned,ready);
    if(!ready)return 184;
    heap_owner=heap;
    arena_owner=started.arena;
    owned=true;
    unsigned char outside=0;
    bool boundaries=!owns(heap,NULL)&&!owns(heap,&outside)&&
                    owns(heap,arena_owner)&&
                    !owns(heap,(const void *)((uintptr_t)arena_owner+NATIVE_TEMP_BYTES));
    void *first=allocate(heap,1),*second=allocate(heap,17);
    struct observation active=observe(heap);
    bool allocated=first&&second&&owns(heap,first)&&owns(heap,second)&&
                   ((uintptr_t)first%16)==0&&((uintptr_t)second%16)==0&&
                   first!=second&&active.locks==2&&active.cursor==48;
    if(first)memset(first,0xa5,1);
    if(second)memset(second,0x5a,17);
    if(first)deallocate(heap,first);
    struct observation one=observe(heap);
    if(second)deallocate(heap,second);
    struct observation released=observe(heap);
    bool reset=one.locks==1&&one.cursor==48&&!released.locks&&!released.cursor;
    void *zero=allocate(heap,0);
    struct observation zero_active=observe(heap);
    bool zero_ok=zero==arena_owner&&zero_active.locks==1&&!zero_active.cursor;
    if(zero)deallocate(heap,zero);
    released=observe(heap);
    zero_ok=zero_ok&&!released.locks&&!released.cursor;
    fprintf(stderr,"C6_LINUX_TEMPHEAP allocation_probe boundaries %d aligned_1_17 %d last_release_reset %d zero_size_release %d\n",
            boundaries,allocated,reset,zero_ok);
    return boundaries&&allocated&&reset&&zero_ok?0:185;
}

int c6_linux_temp_heap_shutdown(void) {
    void *(*get_heap)(void)=symbol("_ZN8Platform11GetTempHeapEv");
    bool (*jobs_initialized)(void)=symbol("_ZN8Platform10JobManager13IsInitializedEv");
    void (*destroy_jobs)(void)=symbol("_ZN8Platform10JobManager7DestroyEv");
    void (*shutdown_perf)(void)=symbol("_ZN8Platform10PerfEvents8ShutdownEv");
    void (*shutdown_heap)(void *)=symbol("_ZN8Platform8TempHeap8ShutdownEv");
    if(!owned||!get_heap||!jobs_initialized||!destroy_jobs||!shutdown_perf||
       !shutdown_heap||get_heap()!=heap_owner)return 186;
    struct observation before=observe(heap_owner);
    if(before.arena!=arena_owner||before.capacity!=NATIVE_TEMP_BYTES)return 187;
    /* Exact native TeardownFXSPlatform order. PerfEvents::Shutdown is a ret in
     * this target. JobManager was absent at startup; destroy only a native
     * owner that actually appeared while this fixture was running. */
    shutdown_perf();
    bool jobs=jobs_initialized();
    if(jobs)destroy_jobs();
    struct observation released=observe(heap_owner);
    fprintf(stderr,"C6_LINUX_TEMPHEAP shutdown_before locks %u cursor %u jobs_destroyed %d jobs_remaining %d\n",
            released.locks,released.cursor,jobs,jobs_initialized());
    if(jobs_initialized()||released.locks||released.cursor)return 188;
    shutdown_heap(heap_owner);
    struct observation after=observe(heap_owner);
    bool clear=!after.arena&&!after.locks&&!after.cursor&&!after.capacity&&
               !(after.storage_flags&1);
    fprintf(stderr,"C6_LINUX_TEMPHEAP shutdown_return native_fields_clear %d\n",clear);
    if(!clear)return 189;
    owned=false;
    heap_owner=NULL;
    arena_owner=NULL;
    return 0;
}
