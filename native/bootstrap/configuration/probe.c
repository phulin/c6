/* Original saved TypedVariantMap parsing and serialization, not a game loader.
 * Launcher fingerprints host, GameCore, providers, save and executable. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../xml-import/imports.h"
#include "configuration.h"
#include "../job-manager/job_manager.h"
typedef void *(*ctor)(void *);
static uintptr_t base;
static HMODULE original_host;
__declspec(dllexport) void *EXP_GetEmptyBasicString(void) {
    void *(*get)(void)=(void *(*)(void))(void *)GetProcAddress(original_host,"EXP_GetEmptyBasicString");
    if(!get) ExitProcess(87);
    return get();
}
static LONG WINAPI fault(EXCEPTION_POINTERS *p) {
    fprintf(stderr,"native configuration fault %08lx at %p rcx=%llx rdx=%llx r8=%llx ret=%llx\n",p->ExceptionRecord->ExceptionCode,p->ExceptionRecord->ExceptionAddress,(unsigned long long)p->ContextRecord->Rcx,(unsigned long long)p->ContextRecord->Rdx,(unsigned long long)p->ContextRecord->R8,(unsigned long long)*(uint64_t *)p->ContextRecord->Rsp);
    fflush(NULL); return EXCEPTION_EXECUTE_HANDLER;
}
int wmain(int argc,wchar_t **argv) {
    if((argc!=6 && argc!=7) || !SetDllDirectoryW(argv[2])) return 2;
    SetUnhandledExceptionFilter(fault);
    HMODULE host=LoadLibraryExW(argv[1],NULL,DONT_RESOLVE_DLL_REFERENCES);
    original_host=host;
    HMODULE core=LoadLibraryW(argv[3]);
    if((uintptr_t)host!=0x140000000ULL || !core) { fprintf(stderr,"loader host=%p core=%p error=%lu\n",host,core,GetLastError()); return 3; }
    if(!bootstrap_bind_original_imports(host)) return 3;
    base=(uintptr_t)host;
    if(!bootstrap_original_onexit_prepare(host) || !bootstrap_saved_configuration_prepare_allocator(host)) return 10;
    FILE *input=_wfopen(argv[4],L"rb"); if(!input) return 4;
    if(fseek(input,0,SEEK_END)) return 4;
    long length=ftell(input); rewind(input);
    if(length!=7348717) return 4;
    unsigned char *bytes=malloc((size_t)length);
    if(!bytes || fread(bytes,1,(size_t)length,input)!=(size_t)length) return 4;
    fclose(input);
    const size_t map_starts[]={8850,8918,65941,65953,66257};
    const size_t map_ends[]={8914,65937,65945,66257,103901};
    for(unsigned i=0;i<5;++i) {
        union { uint64_t align; unsigned char data[0x58]; } map;
        union { uint64_t align; unsigned char data[0x28]; } stream;
        union { uint64_t align; unsigned char data[0x28]; } output;
        if(((ctor)(base+0x7a44a0))(&map)!=&map || *(uintptr_t *)&map!=base+0x103c918) return 5;
        typedef void *(*input_ctor)(void *,void *,uint64_t);
        if(((input_ctor)((uintptr_t)core+0x9a9ec0))(&stream,bytes+map_starts[i],map_ends[i]-map_starts[i])!=&stream) return 6;
        typedef void (*read_fn)(void *,void *,uint64_t);
        ((read_fn)(*(uintptr_t **)&map)[0x1e8/8])(&map,&stream,0);
        uint64_t consumed=*(uint64_t *)(stream.data+0x10);
        if(i==1 || i==4) {
            typedef const unsigned char *(*find_fn)(void *,uint32_t);
            find_fn find=(find_fn)(*(uintptr_t **)&map)[0x60/8];
            const unsigned char *root_variant=find(&map,0x871d5b63U);
            if(!root_variant || *(uint16_t *)root_variant!=10) return 11;
            void *game_root=*(void **)(root_variant+8);
            uint32_t enabled=0x1b8cd1c8U,required=0xfed47f44U,modid=0x04c45f54U;
            typedef uint64_t (*count_fn)(void *,const uint32_t *);
            typedef const char *(*field_fn)(void *,const uint32_t *,unsigned,const uint32_t *);
            uint64_t enabled_count=((count_fn)(base+0x25abc0))(game_root,&enabled);
            uint64_t required_count=((count_fn)(base+0x25abc0))(game_root,&required);
            const char *enabled_id=((field_fn)(base+0x25ac50))(game_root,&enabled,0,&modid);
            const char *required_id=((field_fn)(base+0x25ac50))(game_root,&required,0,&modid);
            const unsigned char *rule=((find_fn)(*(uintptr_t **)game_root)[0x60/8])(game_root,0x34a1e258U);
            const char *rule_text=rule && *(uint16_t *)rule==5 ? *(const char **)(rule+8) : NULL;
            int match=enabled_count==1 && required_count==1 && enabled_id && required_id && rule_text &&
                !strcmp(enabled_id,"4873eb62-8ccc-4574-b784-dda455e74e68") && !strcmp(enabled_id,required_id) &&
                !strcmp(rule_text,"RULESET_EXPANSION_2");
            printf("{\"phase\":\"original_saved_mod_lookups\",\"map\":%u,\"enabled_count\":%llu,\"required_count\":%llu,\"saved_xp2_identity_equal\":%s}\n",i,(unsigned long long)enabled_count,(unsigned long long)required_count,match?"true":"false");fflush(stdout);
            if(!match) return 11;
        }
        typedef void *(*output_ctor)(void *,void *,uint64_t);
        ((output_ctor)((uintptr_t)core+0x9a9f20))(&output,(void *)((uintptr_t)core+0xb60c10),1024);
        ((read_fn)(*(uintptr_t **)&map)[0x1f0/8])(&map,&output,0);
        uint64_t written=*(uint64_t *)(output.data+0x10);
        /* OStream embeds Buffer; its written size is Buffer.size at +0x10. */
        unsigned char *serialized=*(unsigned char **)(output.data+8);
        int equal=written==map_ends[i]-map_starts[i] && !memcmp(serialized,bytes+map_starts[i],(size_t)written);
        wchar_t destination[32768];
        if(swprintf(destination,32768,L"%ls/map-%u.bin",argv[5],i)<0) return 9;
        FILE *dump=_wfopen(destination,L"wb");
        if(!dump || fwrite(serialized,1,(size_t)written,dump)!=(size_t)written || fclose(dump)) return 9;
        printf("{\"map\":%u,\"start\":%zu,\"end\":%zu,\"consumed\":%llu,\"serialized_bytes\":%llu,\"byte_equal\":%s}\n",i,map_starts[i],map_ends[i],(unsigned long long)consumed,(unsigned long long)written,equal?"true":"false"); fflush(stdout);
        ((void (*)(void *))((uintptr_t)core+0x9a9f80))(&stream);
        ((void (*)(void *))(base+0x7a45c0))(&map);
        /* Original scalar-deleting destructor with delete flag zero. */
        ((void (*)(void *,unsigned))(*(uintptr_t **)&output)[0])(&output,0);
        if(consumed!=map_ends[i]-map_starts[i] || written!=consumed) return 7;
    }
    free(bytes);
    if(argc==7 && !bootstrap_saved_configuration_restore(host,core,argv[4])) return 8;
    puts("{\"status\":\"ok\",\"native_maps\":5,\"live_configuration_restored\":false,\"gameplay_parity\":false}"); return 0;
}
