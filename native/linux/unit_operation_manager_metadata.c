#define _GNU_SOURCE
#include "unit_operation_manager_metadata.h"
#include <dlfcn.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef const void *(*P0)(void);
typedef const void *(*P1)(const void *);
typedef const void *(*PI)(const void *,uint32_t);
typedef uint32_t (*U1)(const void *);
typedef int32_t (*I1)(const void *);
typedef int32_t (*II)(const void *,uint32_t);
typedef struct { const void *entry; } Iterator;
_Static_assert(sizeof(uintptr_t)==8,"Linux x86-64 ABI");
_Static_assert(sizeof(Iterator)==8,"original iterator ABI");
static uintptr_t ptr(const void *p,size_t off) {
    uintptr_t v;memcpy(&v,(const unsigned char *)p+off,sizeof v);return v;
}
static int32_t i32(const void *p,size_t off) {
    int32_t v;memcpy(&v,(const unsigned char *)p+off,sizeof v);return v;
}
static unsigned byte(const void *p,size_t off) {return ((const unsigned char *)p)[off];}
static void raw(FILE *out,const void *p,size_t n) {
    fputc('"',out);for(size_t i=0;i<n;i++)fprintf(out,"%02x",byte(p,i));fputc('"',out);
}
static bool span(uintptr_t begin,uintptr_t end,size_t width,size_t limit,size_t *count) {
    if(end<begin||!width||(end-begin)%width||(!begin&&end))return false;
    uintptr_t n=(end-begin)/width;if(n>limit)return false;
    *count=(size_t)n;return true;
}
static void idle(int (*guard)(void *,void *),void *module,void *context) {
    int rc=guard(module,context);
    if(rc){fprintf(stderr,"C6_LINUX_MANAGER_METADATA quiescence_unproven %d no_post_state_or_cleanup\n",rc);fflush(stderr);_exit(240);}
}
static bool map_binding(const void *map,uintptr_t host) {
    return map&&ptr(map,0)==host+0x53b3690&&
        ptr((const void *)(host+0x53b3690),0x70)==host+0x395f480;
}
static int configuration(FILE *out,uintptr_t base,uintptr_t host) {
    P0 get=(void *)(base+0x2003e60);
    const void *config=get();if(!config)return 243;
    const void *root=(const void *)ptr(config,0x80),*map=(const void *)ptr(config,0x88);
    /* InitializeGameVariants borrows the Key_Game child of root. The runner
     * retains root's actual owner and its variant manager throughout this call.
     * Unknown provider overrides are unsupported before any virtual lookup. */
    if(!map_binding(root,host)||!map_binding(map,host))return 243;
    PI lookup=(void *)(host+0x395f480);
    const void *key_game=(const void *)ptr((const void *)(base+0x3b21748),0);
    const void *key_mode=(const void *)ptr((const void *)(base+0x3b1eb38),0);
    const void *key_phase=(const void *)ptr((const void *)(base+0x3b15228),0);
    if(!key_game||!key_mode||!key_phase)return 243;
    uint32_t game_key=(uint32_t)i32(key_game,0),mode_key=(uint32_t)i32(key_mode,0),phase_key=(uint32_t)i32(key_phase,0);
    const void *child=lookup(root,game_key);
    const void *(*as_map)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK4Data12TypedVariantcvPKNS_15TypedVariantMapEEv");
    if((uintptr_t)as_map!=host+0x321c700||!child||as_map(child)!=map)return 243;
    const void *mode=lookup(map,mode_key),*phase=lookup(map,phase_key);
    if(!mode||!phase)return 243;
    U1 configured=(void *)(base+0x28e4870);
    fprintf(out,"{\"kind\":\"configuration\",\"receiver\":%" PRIuPTR ",\"root\":%" PRIuPTR ",\"map\":%" PRIuPTR ",\"map_vptr\":%" PRIuPTR ",\"lookup\":%" PRIuPTR ",\"game_key\":%u,\"mode_key\":%u,\"phase_key\":%u,\"mode_variant_raw\":",(uintptr_t)config,(uintptr_t)root,(uintptr_t)map,ptr(map,0),ptr((const void *)ptr(map,0),0x70),game_key,mode_key,phase_key);
    raw(out,mode,24);fputs(",\"phase_variant_raw\":",out);raw(out,phase,24);
    fprintf(out,",\"configured_phase_hash\":%u}\n",configured(config));return 0;
}
static int phases(FILE *out,uintptr_t base) {
    const void *vector=(const void *)(base+0x3c05f08);
    uintptr_t begin=ptr(vector,0),end=ptr(vector,8),cap=ptr(vector,16);size_t n,capacity;
    if(!span(begin,end,8,65536,&n)||!span(begin,cap,8,65536,&capacity)||n>capacity)return 244;
    fprintf(out,"{\"kind\":\"phases\",\"current_phase\":%d,\"vector_raw\":",i32((const void *)(base+0x3c004f8),0));raw(out,vector,32);
    fprintf(out,",\"begin\":%" PRIuPTR ",\"end\":%" PRIuPTR ",\"capacity\":%" PRIuPTR ",\"allocator_raw\":",begin,end,cap);raw(out,(const unsigned char *)vector+24,8);fputs(",\"slots\":[",out);
    for(size_t i=0;i<n;i++) {
        const void *phase=(const void *)ptr((const void *)begin,i*8);
        fprintf(out,"%s{\"slot\":%zu,\"pointer\":%" PRIuPTR,i?",":"",i,(uintptr_t)phase);
        if(phase) {
            const void *segment=(const void *)ptr(phase,0x60);
            fprintf(out,",\"rowid\":%d,\"type_hash\":%u,\"phase_order\":%d,\"active_segment\":%" PRIuPTR,i32(phase,0x10),(uint32_t)i32(phase,0x20),i32(phase,0x28),(uintptr_t)segment);
            if(segment)fprintf(out,",\"segment_rowid\":%d,\"segment_flags_raw\":%u",i32(segment,0x10),byte(segment,0x50));
        }
        fputc('}',out);
    }
    fputs("]}\n",out);return 0;
}
static int registry(FILE *out,uintptr_t base) {
    P0 defs_get=(void *)(base+0x2214890),manager_get=(void *)(base+0x2da1260);
    U1 count_get=(void *)(base+0x20e9bc0),index=(void *)(base+0x20b42c0),hash_get=(void *)(base+0x20b4900);
    PI handler=(void *)(base+0x2da1650);
    II find_index=(void *)(base+0x2c2c220),handler_id_for_hash=(void *)(base+0x2da1720);
    int32_t (*hash_index)(uint32_t,const void *)=(void *)(base+0x20e9f70);
    const void *defs=defs_get(),*manager=manager_get();if(!defs||!manager)return 245;
    size_t n,slots;
    if(!span(ptr(defs,0),ptr(defs,8),8,65536,&n)||!span(ptr(manager,24),ptr(manager,32),8,65536,&slots)||n!=count_get(defs))return 245;
    if(ptr(manager,0)!=base+0x3aff880)return 245;
    fprintf(out,"{\"kind\":\"operation_definitions\",\"count\":%u,\"handler_slots\":%zu,\"entries\":[",count_get(defs),slots);
    for(uint32_t i=0;i<n;i++) {
        const void *definition=(const void *)ptr((const void *)ptr(defs,0),(size_t)i*8);if(!definition)return 245;
        uint32_t h=hash_get(definition);int32_t runtime=handler_id_for_hash(manager,h);
        const void *registered=runtime>=0&&(size_t)runtime<slots?handler(manager,(uint32_t)runtime):NULL;
        fprintf(out,"%s{\"ordinal\":%u,\"collection_index\":%u,\"hash\":%u,\"numeric_find_index\":%d,\"hash_find_index\":%d,\"numeric_hash_to_index\":%d,\"hash_hash_to_index\":%d,\"runtime_id\":%d,\"handler_pointer\":%" PRIuPTR,i?",":"",i,index(definition),h,find_index(defs,i),find_index(defs,h),hash_index(i,defs),hash_index(h,defs),runtime,(uintptr_t)registered);
        if(registered)fprintf(out,",\"handler_id\":%d,\"handler_hash\":%u,\"handler_vptr\":%" PRIuPTR,i32(registered,8),(uint32_t)i32(registered,12),ptr(registered,0));
        fputc('}',out);
    }
    fputs("]}\n",out);return 0;
}
static int players(FILE *out,uintptr_t base,const void *game,int (*guard)(void *,void *),void *module,void *context) {
    P1 manager_get=(void *)(base+0x20e0670),config_get=(void *)(base+0x2b315e0),units_get=(void *)(base+0x1fccc70);
    const void *(*player_get)(const void *,int32_t)=(void *)(base+0x2003940);
    I1 owner=(void *)(base+0x1fcc140),slot=(void *)(base+0x28ebe70);
    U1 units_count=(void *)(base+0x1fe4ee0),unit_id=(void *)(base+0x2021490);
    Iterator (*begin)(const void *)=(void *)(base+0x2021450),(*end)(const void *)=(void *)(base+0x2021470);
    const void *(*deref)(const Iterator *)=(void *)(base+0x2021480);
    Iterator *(*next)(Iterator *)=(void *)(base+0x20214a0);
    const void *manager=manager_get(game);if(!manager)return 246;
    size_t players_n;if(!span(ptr(manager,0x20),ptr(manager,0x28),8,65,&players_n)||players_n!=65)return 246;
    for(int32_t p=0;p<65;p++) {
        idle(guard,module,context);const void *player=player_get(manager,p);
        if(!player)continue;
        const void *config=config_get(player),*units=units_get(player);
        if(!config||!units||owner(player)!=p)return 246;
        fprintf(out,"{\"kind\":\"player\",\"query\":%d,\"actual_id\":%d,\"turn_active_raw\":%u,\"slot_status\":%d}\n",p,owner(player),byte(player,0x290),slot(config));
        uint32_t count=units_count(units);if(count>65536)return 246;
        Iterator it=begin(units),last=end(units);uint32_t n=0;
        for(;it.entry!=last.entry;n++,next(&it)) {
            if(n>=count)return 246;
            idle(guard,module,context);const void *unit=deref(&it);
            if(!unit||ptr(unit,0)!=base+0x3ae5f80||i32(unit,0x130)!=p)return 246;
            const void *list=(const unsigned char *)unit+0xbe0;
            uint32_t size=(uint32_t)i32(list,16),capacity=(uint32_t)i32(list,8);uintptr_t data=ptr(list,0);
            if(size>capacity||capacity>65536||(!data&&size)||data>UINTPTR_MAX-size)return 246;
            fprintf(out,"{\"kind\":\"unit_disabled\",\"owner\":%d,\"ordinal\":%u,\"id\":%u,\"size\":%u,\"capacity\":%u,\"default_raw\":%u,\"list_raw\":",p,n,unit_id(unit),size,capacity,byte(list,24));raw(out,list,32);fputs(",\"values_raw\":",out);raw(out,(const void *)data,size);fputs("}\n",out);
        }
        if(n!=count)return 246;
    }
    return 0;
}
int c6_linux_unit_operation_manager_metadata_probe(void *module,void *context) {
    if(!module||!context)return 241;
    void *anchor=dlsym(module,"_ZN8GameCore4Unit10Operations7Manager3GetEv");Dl_info game_info={0},host_info={0};
    void *lookup=dlsym(RTLD_DEFAULT,"_ZNK4Data15TypedVariantMap10GetVariantEj");
    int (*guard)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_unit_operation_manager_metadata_quiescent");
    if(!anchor||!lookup||!guard||!dladdr(anchor,&game_info)||!dladdr(lookup,&host_info))return 241;
    uintptr_t base=(uintptr_t)game_info.dli_fbase,host=(uintptr_t)host_info.dli_fbase;
    if(!base||!host||(uintptr_t)anchor!=base+0x2da1260||(uintptr_t)lookup!=host+0x395f480)return 241;
    idle(guard,module,context);
    P0 game_get=(void *)(base+0x1fcc320),script_get=(void *)(base+0x2171660);
    const void *game=game_get();if(!game)return 242;
    const char *path=getenv("C6_UNIT_OPERATION_MANAGER_METADATA_OUTPUT");if(!path||!*path)return 242;
    FILE *out=fopen(path,"wx");if(!out)return 242;
    int rc=configuration(out,base,host);
    if(!rc)rc=phases(out,base);
    if(!rc)rc=registry(out,base);
    if(!rc)rc=players(out,base,game,guard,module,context);
    if(!rc) {
        idle(guard,module,context);const void *script=script_get();
        fprintf(out,"{\"kind\":\"globals\",\"game_pause_raw\":%u,\"automation_player_raw\":%d,\"script_system\":%" PRIuPTR,byte(game,0x228),i32((const void *)(base+0x3c06598),0),(uintptr_t)script);
        if(script)fprintf(out,",\"script_vptr\":%" PRIuPTR,ptr(script,0));
        fputs(",\"script_callbacks_inspected\":false}\n",out);idle(guard,module,context);
    }
    if(ferror(out)&&!rc)rc=247;
    if(fclose(out)&&!rc)rc=247;
    return rc;
}
