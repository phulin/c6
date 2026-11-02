#define _GNU_SOURCE
#include "request_load.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>

static bool prepared;

int c6_linux_request_load_clear(void) {
    void (*reset)(void)=dlsym(RTLD_DEFAULT,"_ZN8GameCore10Serializer7Manager5ResetEv");
    void (*clear)(void)=dlsym(RTLD_DEFAULT,"_ZN8GameCore10Serializer7Manager15ClearLastStreamEv");
    void *(*last)(void)=dlsym(RTLD_DEFAULT,"_ZN8GameCore10Serializer7Manager13GetLastStreamEv");
    const int *kind=dlsym(RTLD_DEFAULT,"_ZN8GameCore10Serializer15ms_eRequestTypeE");
    if(!reset||!clear||!last||!kind)return 181;
    if(!prepared)return 0;
    reset();clear();
    fprintf(stderr,"C6_LINUX_REQUEST clear_return kind %d stream_null %d\n",*kind,last()==NULL);
    if(*kind||last())return 182;
    prepared=false;
    return 0;
}

int c6_linux_request_load_prepare(const char *path) {
    void *(*allocate)(size_t); void (*release)(void*);
    void (*stream_ctor)(void*,const void*,size_t); void (*stream_dtor)(void*);
    void (*location_ctor)(void*,int,int,int); void (*location_dtor)(void*);
    bool (*request)(const wchar_t*,const void*,void*);
    void *(*last)(void); int (*last_type)(void); int (*progress)(void);
    const void *(*bytes_of)(const void*); size_t (*size_of)(const void*);
    size_t (*offset_of)(const void*);
    void *(*current)(const void*); void *(*game_view)(const void*);
    const int *kind; const bool *in_progress; void **app;
#define RESOLVE(p,n) do {p=dlsym(RTLD_DEFAULT,n);if(!p)return 183;}while(0)
    RESOLVE(allocate,"_Znwm"); RESOLVE(release,"_ZdlPv");
    RESOLVE(stream_ctor,"_ZN13Serialization13MemoryIStreamC1EPKvm");
    RESOLVE(stream_dtor,"_ZN13Serialization13MemoryIStreamD1Ev");
    RESOLVE(location_ctor,"_ZN8GameCore10Serializer12SaveLocation8InstanceC2ENS0_8SaveTypeENS1_6DeviceENS1_7OptionsE");
    RESOLVE(location_dtor,"_ZN8GameCore10Serializer12SaveLocation8InstanceD2Ev");
    RESOLVE(request,"_ZN8GameCore10Serializer7Manager11RequestLoadEPKwRKNS0_12SaveLocation8InstanceEPN13Serialization7IStreamE");
    RESOLVE(last,"_ZN8GameCore10Serializer7Manager13GetLastStreamEv");
    RESOLVE(last_type,"_ZN8GameCore10Serializer7Manager17GetLastStreamTypeEv");
    RESOLVE(progress,"_ZN8GameCore10Serializer7Manager20GetRequestInProgressEv");
    RESOLVE(bytes_of,"_ZNK13Serialization13MemoryIStream8GetBytesEv");
    RESOLVE(size_of,"_ZNK13Serialization13MemoryIStream7GetSizeEv");
    RESOLVE(offset_of,"_ZNK13Serialization13MemoryIStream9GetOffsetEv");
    RESOLVE(current,"_ZN7Civ6App19GetCurrentGameStateEv");
    RESOLVE(game_view,"_ZN7Civ6App16GetGameViewStateEv");
    RESOLVE(kind,"_ZN8GameCore10Serializer15ms_eRequestTypeE");
    RESOLVE(in_progress,"_ZN8GameCore10Serializer21ms_bRequestInProgressE");
    RESOLVE(app,"g_pkApplication");
#undef RESOLVE
    if(prepared||!*app||current(*app)||game_view(*app)||*kind||*in_progress||last()){
        fprintf(stderr,"C6_LINUX_REQUEST preconditions_refused\n");return 184;
    }
    const size_t count=7348717;
    FILE *source=fopen(path,"rb");if(!source)return 185;
    unsigned char *bytes=malloc(count);
    if(!bytes){fclose(source);return 186;}
    size_t got=fread(bytes,1,count,source);
    int extra=fgetc(source),failed=ferror(source);fclose(source);
    if(got!=count||extra!=EOF||failed){free(bytes);return 187;}
    void *stream=allocate(0x28),*location=allocate(0x28);
    if(!stream||!location){if(stream)release(stream);if(location)release(location);free(bytes);return 188;}
    stream_ctor(stream,bytes,count);
    /* Native Lua enum registration proves SaveTypes.SINGLE_PLAYER=1.
     * Device4 is the original supplied-stream branch, not LOCAL_STORAGE(1).
     * Original ctor initializes every location field; no field writes here. */
    location_ctor(location,1,4,0);
    fprintf(stderr,"C6_LINUX_REQUEST prepare_enter complete_bytes %zu current_state_null 1 game_view_null 1 device 4\n",count);
    bool accepted=request(NULL,location,stream);
    prepared=accepted;
    void *retained=last();
    bool copied=retained&&retained!=stream&&bytes_of(retained)!=bytes&&
        size_of(retained)==count&&offset_of(retained)==0&&
        !memcmp(bytes_of(retained),bytes,count);
    fprintf(stderr,"C6_LINUX_REQUEST prepare_return accepted %d kind %d in_progress %d progress_getter %d retained_type %d complete_copy %d\n",
        accepted,*kind,*in_progress,progress(),last_type(),copied);
    location_dtor(location);release(location);stream_dtor(stream);release(stream);free(bytes);
    bool retained_valid=copied&&last()==retained&&size_of(retained)==count&&offset_of(retained)==0;
    fprintf(stderr,"C6_LINUX_REQUEST supplied_stream_released retained_valid %d\n",retained_valid);
    if(!accepted||*kind!=2||*in_progress||progress()!=0||last_type()!=1||!retained_valid){
        if(prepared)c6_linux_request_load_clear();
        return 189;
    }
    return 0;
}
