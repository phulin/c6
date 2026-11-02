#define _GNU_SOURCE
#include "context_stream.h"
#include <dlfcn.h>
#include <signal.h>
#include <link.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ucontext.h>
#include <unistd.h>

/* Actual host constructors initialize vptr + 0x20-byte MemoryBuffer.
 * Evidence: private linux-runtime/memory-{i,o}stream-constructors.txt and
 * memory-buffer-constructors.txt. No object fields are synthesized here. */
#define NATIVE_STREAM_SIZE 0x28u
#define FIXTURE_STREAM_SIZE 76556938u
#define SAVE_CAPACITY (128u * 1024u * 1024u)
static void *(*native_allocate)(size_t);
static void (*native_free)(void *);
static void (*input_construct)(void *, const void *, size_t);
static void (*input_destruct)(void *);
static size_t (*input_offset)(const void *);
static size_t (*input_size)(const void *);
static const void *(*input_bytes)(const void *);
static void (*input_set_offset)(void *, size_t);
static void (*output_construct)(void *, void *, size_t);
static void (*output_destruct)(void *);
static size_t (*output_offset)(const void *);
static int (*output_write)(void *, const void *, size_t);
static void *active_input;
static uintptr_t core_base,core_end;
static int core_bounds(struct dl_phdr_info *info,size_t size,void *data){
    (void)size;(void)data;
    if((uintptr_t)info->dlpi_addr!=core_base)return 0;
    for(ElfW(Half) i=0;i<info->dlpi_phnum;i++){
        const ElfW(Phdr) *p=&info->dlpi_phdr[i];
        if(p->p_type==PT_LOAD && core_base+p->p_vaddr+p->p_memsz>core_end)
            core_end=core_base+p->p_vaddr+p->p_memsz;
    }
    return 1;
}

static int resolve_streams(void) {
#define RESOLVE(variable, name) do { \
    variable=dlsym(RTLD_DEFAULT,name); \
    if(!variable){fprintf(stderr,"C6_LINUX_STREAM missing_symbol %s\n",name);return 60;} \
} while(0)
    RESOLVE(native_allocate,"_Znwm");
    RESOLVE(native_free,"_ZdlPv");
    RESOLVE(input_construct,"_ZN13Serialization13MemoryIStreamC1EPKvm");
    RESOLVE(input_destruct,"_ZN13Serialization13MemoryIStreamD1Ev");
    RESOLVE(input_offset,"_ZNK13Serialization13MemoryIStream9GetOffsetEv");
    RESOLVE(input_size,"_ZNK13Serialization13MemoryIStream7GetSizeEv");
    RESOLVE(input_bytes,"_ZNK13Serialization13MemoryIStream8GetBytesEv");
    RESOLVE(input_set_offset,"_ZN13Serialization13MemoryIStream9SetOffsetEm");
    RESOLVE(output_construct,"_ZN13Serialization13MemoryOStreamC1EPvm");
    RESOLVE(output_destruct,"_ZN13Serialization13MemoryOStreamD0Ev");
    RESOLVE(output_offset,"_ZNK13Serialization13MemoryOStream9GetOffsetEv");
    RESOLVE(output_write,"_ZN13Serialization13MemoryOStream5WriteEPKvm");
#undef RESOLVE
    return 0;
}

int c6_linux_stream_check(void) {
    int code=resolve_streams();
    if(code)return code;
    unsigned char source[8]={0,1,2,3,4,5,6,7}, destination[8]={0};
    void *input=native_allocate(NATIVE_STREAM_SIZE);
    void *output=native_allocate(NATIVE_STREAM_SIZE);
    if(!input||!output)return 61;
    input_construct(input,source,sizeof source);
    int valid=input_offset(input)==0 && input_size(input)==sizeof source &&
              input_bytes(input)==source;
    input_set_offset(input,sizeof source);
    valid=valid && input_offset(input)==sizeof source;
    input_set_offset(input,0);
    valid=valid && input_offset(input)==0;
    output_construct(output,destination,sizeof destination);
    int empty=output_write(output,source,0);
    int full=output_write(output,source,sizeof source);
    size_t full_offset=output_offset(output);
    unsigned char snapshot[8], object_snapshot[NATIVE_STREAM_SIZE];
    memcpy(snapshot,destination,sizeof snapshot);
    memcpy(object_snapshot,output,sizeof object_snapshot);
    int overflow=output_write(output,source,1);
    valid=valid && empty==0 && full==0 && full_offset==8 && overflow==1 &&
          output_offset(output)==8 && !memcmp(source,destination,sizeof source) &&
          !memcmp(snapshot,destination,sizeof snapshot) &&
          !memcmp(object_snapshot,output,sizeof object_snapshot);
    input_destruct(input);native_free(input);
    output_destruct(output);
    fprintf(stderr,"C6_LINUX_STREAM boundary_check %s\n",valid?"pass":"fail");
    return valid?0:62;
}

/* Crash observer only: reads the original accessor and emits decimal numbers
 * with async-signal-safe write/_exit. It never changes stream/game state. */
static char *append_text(char *p,const char *text){while(*text)*p++=*text++;return p;}
static char *append_number(char *p,uint64_t value){
    char digits[24];size_t count=0;
    do{digits[count++]=(char)('0'+value%10);value/=10;}while(value);
    while(count)*p++=digits[--count];
    return p;
}
static void stream_fault(int signal_number,siginfo_t *information,void *machine){
    (void)information;
    char buffer[256],*p=buffer;
    p=append_text(p,"C6_LINUX_STREAM fault signal ");p=append_number(p,(unsigned)signal_number);
    p=append_text(p," offset ");p=append_number(p,active_input?input_offset(active_input):0);
    p=append_text(p," instruction ");
    uintptr_t instruction=(uintptr_t)((ucontext_t*)machine)->uc_mcontext.gregs[REG_RIP];
    p=append_number(p,instruction);
    p=append_text(p," gamecore_rva ");
    p=append_number(p,instruction>=core_base && instruction<core_end ? instruction-core_base : 0);
    *p++='\n';
    (void)write(STDERR_FILENO,buffer,(size_t)(p-buffer));
    _exit(128+signal_number);
}

int c6_linux_context_load(void *game,void *context,const char *path){
    return c6_linux_context_load_sized(game,context,path,FIXTURE_STREAM_SIZE);
}

int c6_linux_context_load_sized(void *game,void *context,const char *path,size_t expected_size){
    if(!expected_size || expected_size>SAVE_CAPACITY)return 66;
    int code=resolve_streams();if(code)return code;
    int (*load)(void *,void *)=dlsym(game,
        "_ZN8GameCore7Exports7Context4LoadERN13Serialization7IStreamE");
    if(!load)return 63;
    Dl_info info;
    if(!dladdr((void*)load,&info))return 63;
    core_base=(uintptr_t)info.dli_fbase;core_end=core_base;
    dl_iterate_phdr(core_bounds,NULL);
    FILE *file=fopen(path,"rb");if(!file)return 64;
    unsigned char *payload=malloc(expected_size);
    if(!payload){fclose(file);return 65;}
    size_t count=fread(payload,1,expected_size,file);
    int trailing=fgetc(file);int failed=ferror(file);fclose(file);
    if(count!=expected_size||trailing!=EOF||failed){free(payload);return 66;}
    void *stream=native_allocate(NATIVE_STREAM_SIZE);if(!stream){free(payload);return 67;}
    input_construct(stream,payload,count);
    if(input_size(stream)!=count||input_offset(stream)!=0||input_bytes(stream)!=payload)return 68;
    active_input=stream;
    struct sigaction observer={0},old_segv,old_abrt,old_bus;
    observer.sa_sigaction=stream_fault;observer.sa_flags=SA_SIGINFO;
    sigemptyset(&observer.sa_mask);
    if(sigaction(SIGSEGV,&observer,&old_segv)||sigaction(SIGABRT,&observer,&old_abrt)||
       sigaction(SIGBUS,&observer,&old_bus))return 69;
    fprintf(stderr,"C6_LINUX_STREAM load_enter size %zu\n",count);
    int result=load(context,stream);
    fprintf(stderr,"C6_LINUX_STREAM load_return result %d offset %zu size %zu\n",
            result,input_offset(stream),count);
    sigaction(SIGSEGV,&old_segv,NULL);sigaction(SIGABRT,&old_abrt,NULL);sigaction(SIGBUS,&old_bus,NULL);
    active_input=NULL;
    input_destruct(stream);native_free(stream);free(payload);
    return result==0?0:70;
}

int c6_linux_context_save(void *game,void *context,const char *path){
    int code=resolve_streams();if(code)return code;
    int (*save)(void *,void *)=dlsym(game,
        "_ZN8GameCore7Exports7Context4SaveERN13Serialization7OStreamE");
    if(!save)return 71;
    void *buffer=malloc(SAVE_CAPACITY),*stream=native_allocate(NATIVE_STREAM_SIZE);
    if(!buffer||!stream)return 72;
    output_construct(stream,buffer,SAVE_CAPACITY);
    int (*trace_end)(void *)=NULL;
    if(getenv("C6_SAVE_NORMALIZATION_TRACE")) {
        int (*trace_begin)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_save_normalization_begin");
        trace_end=dlsym(RTLD_DEFAULT,"c6_linux_save_normalization_end");
        int traced=trace_begin&&trace_end?trace_begin(game,stream):75;
        fprintf(stderr,"C6_LINUX_STREAM normalization_begin_return %d\n",traced);
        if(traced){output_destruct(stream);free(buffer);return traced;}
    }
    fprintf(stderr,"C6_LINUX_STREAM save_enter capacity %u\n",SAVE_CAPACITY);
    int result=save(context,stream);size_t size=output_offset(stream);
    fprintf(stderr,"C6_LINUX_STREAM save_return result %d size %zu\n",result,size);
    int trace_result=trace_end?trace_end(stream):0;
    if(trace_end)fprintf(stderr,"C6_LINUX_STREAM normalization_end_return %d\n",trace_result);
    int status=73;
    if(result==0 && !trace_result && size<=SAVE_CAPACITY){
        FILE *file=fopen(path,"wbx");
        if(file){int written=fwrite(buffer,1,size,file)==size;int closed=fclose(file)==0;
            status=written&&closed?0:74;}
    }
    output_destruct(stream);free(buffer);
    return trace_result?trace_result:status;
}
