#define _GNU_SOURCE
#include "unit_movement_query_fixture.h"
#include <dlfcn.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/uio.h>
#include <unistd.h>

typedef struct { const void *entry; } Iterator;
_Static_assert(sizeof(Iterator)==8,"Linux original one-word iterator");
typedef const void *(*P0)(void);
typedef const void *(*P1)(const void *);
typedef int32_t (*I1)(const void *);
typedef uint32_t (*U1)(const void *);
typedef uint64_t (*N1)(const void *);
typedef bool (*B1)(const void *);
typedef struct {
    uintptr_t base;
    P1 manager,units,configuration,diplomacy,influence,definition,plot;
    P0 parameters,relations;
    B1 initialized,alive,turn_active,has_moved,is_embarked;
    bool (*valid)(const void *,int32_t);
    const void *(*player)(const void *,int32_t),*(*find)(const void *,uint32_t);
    const int32_t *(*mapping)(const void *);
    N1 mapping_size;
    I1 owner,unit_owner,status,team,embarked_bonus,suzerain,turn,domain,base_moves;
    I1 max_moves,plot_owner,plot_cost,embarked_base,city_cost;
    U1 id,count,definition_index,definition_hash;
    Iterator (*begin)(const void *),(*end)(const void *);
    const void *(*deref)(const Iterator *);
    Iterator *(*next)(Iterator *);
} Api;
static _Noreturn void fatal(const char *why) {
    fprintf(stderr,"C6_UNIT_MOVEMENT_QUERY fatal %s no_further_original_calls\n",why);
    fflush(stderr);_exit(240);
}
static void read_at(const void *p,size_t off,void *out,size_t n) {
    uintptr_t start=(uintptr_t)p;
    if(!start||off>UINTPTR_MAX-start||n>UINTPTR_MAX-start-off)fatal("field_extent");
    struct iovec local={out,n},remote={(void *)(start+off),n};
    if(process_vm_readv(getpid(),&local,1,&remote,1,0)!=(ssize_t)n)fatal("field_unreadable");
}
static uintptr_t ptr(const void *p,size_t off){uintptr_t v;read_at(p,off,&v,8);return v;}
static int32_t word(const void *p,size_t off){int32_t v;read_at(p,off,&v,4);return v;}
static unsigned byte(const void *p,size_t off){unsigned char v;read_at(p,off,&v,1);return v;}
static const char *boolean(bool b){return b?"true":"false";}
static void hex(FILE *f,const void *p,size_t n){const unsigned char *b=p;fputc('"',f);for(size_t i=0;i<n;i++)fprintf(f,"%02x",b[i]);fputc('"',f);}
static bool hash(const char *s){if(!s||strlen(s)!=64)return false;for(unsigned i=0;i<64;i++)if(!strchr("0123456789abcdef",s[i]))return false;return true;}
static void guard(const C6UnitMovementQueryAdmission *a) {
    if(a->retained_owner(a->module,a->borrowed_context,a->retained_game))fatal("retained_owner_unknown");
}
static void bind_api(const C6UnitMovementQueryAdmission *a,Api *api) {
    void *anchor=dlsym(a->module,"_ZNK8GameCore4Unit8Instance11GetMaxMovesEv");Dl_info info={0};
    if(!anchor||!dladdr(anchor,&info)||!info.dli_fbase)fatal("module_binding");
    api->base=(uintptr_t)info.dli_fbase;
    if((uintptr_t)anchor!=api->base+0x2c27180)fatal("target_binding");
#define BIND(F,R) do {void *p=(void *)(api->base+(R));Dl_info d={0};if(!dladdr(p,&d)||d.dli_fbase!=info.dli_fbase)fatal("callee_module");api->F=p;}while(0)
    BIND(manager,0x20e0670);BIND(initialized,0x2b51830);BIND(mapping,0x2004680);BIND(mapping_size,0x27e4520);
    BIND(valid,0x2004fc0);BIND(player,0x2003940);BIND(owner,0x1fcc140);BIND(units,0x1fccc70);
    BIND(configuration,0x2b315e0);BIND(status,0x28ebe70);BIND(team,0x28ebce0);BIND(alive,0x2003cb0);BIND(turn_active,0x2021030);
    BIND(diplomacy,0x1fe5670);BIND(influence,0x208ec20);BIND(suzerain,0x2b20950);BIND(embarked_bonus,0x2c2c380);
    BIND(begin,0x2021450);BIND(end,0x2021470);BIND(deref,0x2021480);BIND(next,0x20214a0);BIND(count,0x1fe4ee0);
    BIND(id,0x2021490);BIND(unit_owner,0x2004670);BIND(find,0x2ba5580);BIND(definition,0x20037c0);
    BIND(domain,0x1fe4f10);BIND(base_moves,0x20df1d0);BIND(definition_index,0x1fcccb0);BIND(definition_hash,0x1fe52c0);
    BIND(plot,0x2c1c820);BIND(plot_owner,0x1fe23d0);BIND(plot_cost,0x2bc9510);
    BIND(turn,0x1fe4810);BIND(parameters,0x1fe4cb0);BIND(embarked_base,0x2c2c370);BIND(city_cost,0x2bcad50);BIND(relations,0x2980f10);
    BIND(is_embarked,0x2c12ca0);BIND(max_moves,0x2c27180);BIND(has_moved,0x2c27b50);
#undef BIND
}
static void global_inputs(FILE *out,const C6UnitMovementQueryAdmission *a,const Api *api,unsigned pass) {
    guard(a);const void *parameters=api->parameters(),*relations=api->relations();
    if(!parameters||!relations)fatal("global_owner");
    unsigned char rng[64];read_at(a->retained_game,0xf8,rng,32);read_at((void *)api->base,0x3c072f0,rng+32,32);
    fprintf(out,"{\"kind\":\"global\",\"pass\":%u,\"turn\":%d,\"embarked_base\":%d,\"city_cost\":%d,\"known_rng\":",pass,api->turn(a->retained_game),api->embarked_base(parameters),api->city_cost(parameters));hex(out,rng,64);
    int32_t n=word(relations,0xc0);uintptr_t begin=ptr(relations,0x218),end=ptr(relations,0x220);
    if(n<0||n>65||end<begin||(end-begin)%8||end-begin!=(uintptr_t)(n*(n-1)/2)*8)fatal("alliance_extent");
    fprintf(out,",\"alliance_player_count\":%d,\"alliances\":[",n);
    for(size_t i=0;i<(end-begin)/8;i++)fprintf(out,"%s[%d,%d]",i?",":"",word((void *)begin,i*8),word((void *)begin,i*8+4));
    fputs("]}\n",out);guard(a);
}
static void player_inputs(FILE *out,const C6UnitMovementQueryAdmission *a,const Api *api,unsigned pass,int32_t query,int32_t mapped,bool valid,const void *stored,const void *p) {
    fprintf(out,"{\"kind\":\"player\",\"pass\":%u,\"query\":%d,\"mapped_index\":%d,\"valid\":%s,\"stored_nonnull\":%s,\"resolved\":%s",pass,query,mapped,boolean(valid),boolean(stored!=NULL),boolean(p!=NULL));
    if(p){
        const void *config=api->configuration(p),*units=api->units(p),*dip=api->diplomacy(p),*influence=api->influence(p);
        if(!config||!units||!dip||!influence||api->owner(p)!=query)fatal("player_inputs_owner");
        fprintf(out,",\"owner\":%d,\"slot_status\":%d,\"team\":%d,\"alive\":%s,\"turn_active\":%s,\"embarked_modifier\":%d,\"suzerain\":%d,\"timing\":[",query,api->status(config),api->team(config),boolean(api->alive(p)),boolean(api->turn_active(p)),api->embarked_bonus(units),api->suzerain(influence));
        for(unsigned i=0;i<10;i++)fprintf(out,"%s%d",i?",":"",word(dip,0x2000+i*24));
        uintptr_t begin=ptr(dip,0x1f40),end=ptr(dip,0x1f48);
        if(end<begin||(end-begin)%12||(end-begin)/12>65536)fatal("modifier_extent");
        fputs("],\"movement_modifiers\":[",out);
        for(size_t i=0;i<(end-begin)/12;i++)fprintf(out,"%s[%d,%d,%d]",i?",":"",word((void *)begin,i*12),word((void *)begin,i*12+4),word((void *)begin,i*12+8));
        fputc(']',out);
    }
    fputs("}\n",out);guard(a);
}
static void unit_inputs(FILE *out,const Api *api,unsigned pass,int32_t owner,uint32_t ordinal,uint32_t id,const void *u) {
    fprintf(out,"{\"kind\":\"unit\",\"pass\":%u,\"owner\":%d,\"ordinal\":%u,\"id\":%u,\"remaining_q8\":%d,\"x\":%d,\"y\":%d,\"embarked\":%s,\"activity\":%d,\"effective_embarked\":%s,\"delayed_death\":%s,\"domain\":%d,\"runtime_type\":%d,\"fight_embarked\":%s,\"modifiers\":[%d,%d,%d,%d]",pass,owner,ordinal,id,word(u,0x4f8),word(u,0x100),word(u,0x104),boolean(byte(u,0x2c0)!=0),word(u,0x290),boolean(api->is_embarked(u)),boolean(byte(u,0x2f0)!=0),api->domain(u),word(u,0xd0),boolean(byte(u,0x82a)!=0),word(u,0x840),word(u,0x844),word(u,0x848),word(u,0x84c));
    const void *d=api->definition(u),*plot=api->plot(u);
    fputs(",\"definition\":",out);if(d)fprintf(out,"{\"index\":%u,\"hash\":%u,\"base_moves\":%d}",api->definition_index(d),api->definition_hash(d),api->base_moves(d));else fputs("null",out);
    fputs(",\"plot\":",out);if(plot){
        if(ptr((void *)ptr(plot,0),0x20)!=api->base+0x2bc9510)fatal("plot_cost_dispatch");
        fprintf(out,"{\"owner\":%d,\"movement_cost\":%d}",api->plot_owner(plot),api->plot_cost(plot));
    }else fputs("null",out);
    fputs("}\n",out);
}
int c6_linux_unit_movement_query_capture(const C6UnitMovementQueryAdmission *a) {
    if(!a||!a->module||!a->borrowed_context||!a->retained_game||!a->retained_owner||!hash(a->profile_sha256)||!hash(a->candidate_core_sha256)||!a->inputs_path||!a->answers_path||!strcmp(a->inputs_path,a->answers_path)||!a->expected_units||a->expected_units>8192)return 241;
    /* Future runner supplies this NEW distinct guard after actual acquisition.
     * An old two-argument callback cannot be passed through this typed ABI. */
    guard(a);Api api={0};bind_api(a,&api);
    FILE *out=fopen(a->inputs_path,"wx");if(!out)return 242;
    FILE *answers=fopen(a->answers_path,"wx");if(!answers){fclose(out);return 242;}
    fprintf(out,"{\"kind\":\"header\",\"schema\":\"c6-linux-unit-movement-inputs-v1\",\"profile_sha256\":\"%s\",\"candidate_core_sha256\":\"%s\",\"expected_units\":%u,\"desired_answers_in_inputs\":false}\n",a->profile_sha256,a->candidate_core_sha256,a->expected_units);
    fprintf(answers,"{\"kind\":\"header\",\"schema\":\"c6-linux-unit-movement-answers-v1\",\"profile_sha256\":\"%s\"}\n",a->profile_sha256);
    const void *manager=api.manager(a->retained_game);
    if(!manager||!api.initialized(manager)||api.mapping_size((const unsigned char *)manager+0x1a08)!=65)fatal("manager_extent");
    const int32_t *mapping=api.mapping(manager);uintptr_t begin=ptr(manager,0x20),end=ptr(manager,0x28);
    if(!mapping||!begin||end-begin!=65*8)fatal("manager_slots");
    uint32_t totals[3]={0},targets=0,unplaced=0,delayed=0;
    for(unsigned pass=0;pass<3;pass++){
        global_inputs(out,a,&api,pass);
        for(int32_t owner=0;owner<65;owner++){
            guard(a);bool valid=api.valid(manager,owner);const void *p=api.player(manager,owner),*stored=(void *)ptr((void *)begin,(size_t)owner*8);
            if(p!=(valid?stored:NULL))fatal("player_lookup");
            if(pass!=1)player_inputs(out,a,&api,pass,owner,mapping[owner],valid,stored,p);
            if(!p)continue;
            const void *units=api.units(p);if(!units)fatal("unit_container");uint32_t n=api.count(units),seen=0;
            if(n>65536)fatal("unit_count");
            Iterator it=api.begin(units),last=api.end(units);
            for(;it.entry!=last.entry;api.next(&it)){
                guard(a);if(seen>=n||totals[pass]>=8192)fatal("unit_extent");
                const void *u=api.deref(&it);if(!u)fatal("unit_absent");uint32_t id=api.id(u);
                if(id==UINT32_MAX||api.unit_owner(u)!=owner||api.find(units,id)!=u)fatal("unit_identity");
                if(pass!=1)unit_inputs(out,&api,pass,owner,seen,id,u);
                else {
                    guard(a);int32_t max=api.max_moves(u);guard(a);bool moved=api.has_moved(u);guard(a);
                    fprintf(answers,"{\"kind\":\"answer\",\"owner\":%d,\"ordinal\":%u,\"id\":%u,\"max_moves\":%d,\"has_moved\":%s}\n",owner,seen,id,max,boolean(moved));targets++;
                    if(word(u,0x100)==-9999||word(u,0x104)==-9999)unplaced++;
                    if(byte(u,0x2f0))delayed++;
                }
                seen++;totals[pass]++;guard(a);
            }
            if(seen!=n)fatal("unit_membership");
        }
    }
    unsigned status=(totals[0]!=a->expected_units||totals[1]!=a->expected_units||totals[2]!=a->expected_units)?243:0;
    fprintf(out,"{\"kind\":\"summary\",\"status\":%u,\"units\":%u,\"complete\":%s,\"unplaced_queried\":%u,\"delayed_death_queried\":%u,\"known_rng_only\":true}\n",status,totals[0],boolean(!status),unplaced,delayed);
    fprintf(answers,"{\"kind\":\"summary\",\"status\":%u,\"answers\":%u,\"complete\":%s}\n",status,targets,boolean(!status));
    if(ferror(out)||ferror(answers))status=244;
    if(fclose(out))status=244;
    if(fclose(answers))status=244;
    guard(a);return (int)status;
}
