#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

/* Original two-reference operator<< returns its original OStream reference.
 * No stream/game fields or vtables are changed. The owner manifest gates the
 * exact XP2 symbols/signatures; output-byte equivalence is a separate check. */
typedef void *(*serializer_fn)(void *,const void *);
static void *trace_module,*trace_stream;
static FILE *trace_file;
static size_t (*get_offset)(const void *);
static unsigned trace_depth,trace_save;
static unsigned long long trace_call,trace_parent;
static bool trace_failed;

int c6_linux_save_normalization_begin(void *module,void *stream) {
    if(trace_stream || !module || !stream)return 180;
    const char *path=getenv("C6_SAVE_NORMALIZATION_TRACE");
    if(!path)return 181;
    if(!trace_file){
        trace_file=fopen(path,"wx");
        if(!trace_file)return 182;
    }
    get_offset=dlsym(RTLD_DEFAULT,"_ZNK13Serialization13MemoryOStream9GetOffsetEv");
    if(!get_offset)return 183;
    trace_module=module;trace_stream=stream;trace_depth=0;trace_call=0;trace_parent=0;trace_failed=false;++trace_save;
    fprintf(trace_file,"{\"event\":\"save_begin\",\"save\":%u,\"offset\":%zu}\n",trace_save,get_offset(stream));
    return 0;
}
int c6_linux_save_normalization_end(void *stream) {
    if(stream!=trace_stream || !trace_file)return 184;
    fprintf(trace_file,"{\"event\":\"save_end\",\"save\":%u,\"offset\":%zu,\"depth\":%u}\n",trace_save,get_offset(stream),trace_depth);
    trace_stream=NULL;trace_module=NULL;
    if(fflush(trace_file) || ferror(trace_file) || trace_failed || trace_depth)return 185;
    return 0;
}

#define OWNER(ID,SYMBOL,LABEL) \
void *c6_normalization_##ID(void *,const void *) __asm__(SYMBOL); \
void *c6_normalization_##ID(void *stream,const void *object) { \
    serializer_fn original=dlsym(trace_stream?trace_module:RTLD_NEXT,SYMBOL); \
    if(!original || original==c6_normalization_##ID){ \
        fprintf(stderr,"C6_SAVE_NORMALIZATION resolve_failed %s\n",SYMBOL);_exit(186); \
    } \
    if(stream!=trace_stream)return original(stream,object); \
    unsigned depth=trace_depth++;size_t start=get_offset(stream); \
    unsigned long long call=++trace_call,parent=trace_parent;trace_parent=call; \
    void *result=original(stream,object);size_t end=get_offset(stream);--trace_depth;trace_parent=parent; \
    if(end<start || result!=stream)trace_failed=true; \
    fprintf(trace_file,"{\"event\":\"owner\",\"save\":%u,\"depth\":%u,\"call\":%llu,\"parent\":%llu,\"owner\":\"%s\",\"start\":%zu,\"end\":%zu}\n",trace_save,depth,call,parent,LABEL,start,end); \
    return result; \
}
#include "save_normalization_owners.inc"
#undef OWNER
