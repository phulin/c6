#define _GNU_SOURCE
#include "movement_cost.h"
#include "context_stream.h"
#include <dlfcn.h>
#include <fenv.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <xmmintrin.h>

/* Supplied Linux XP2 only. Every BIND RVA is checked by movement_probe_audit.py.
 * Borrowed game owners are never fabricated, retained, relocated or modified.
 * Values, FixedPoint and ID-vector outputs are constructed by original code.
 * The caller must gate whole module/dependency/configuration identities first. */
typedef struct { int32_t x,y; } Coord;
typedef struct __attribute__((aligned(8))) { int16_t owner,type;uint32_t id; } ComponentId;
_Static_assert(sizeof(ComponentId)==8,"original ComponentID size");
_Static_assert(_Alignof(ComponentId)>=8,"original ComponentID alignment");
typedef struct {
    const void *formation,*from,*to;
    int32_t base,max,remaining,exit,zoc,hard,soft,regular,to_exit;
} Values;
_Static_assert(sizeof(Values)==64,"original Values size");
_Static_assert(__builtin_offsetof(Values,hard)==44,"original Values hard minimum");
_Static_assert(__builtin_offsetof(Values,to_exit)==56,"original Values exit addition");

typedef const void *(*P0)(void);
typedef const void *(*P1)(const void *);
typedef const void *(*PI)(const void *,uint32_t);
typedef bool (*B1)(const void *);
typedef int32_t (*I1)(const void *);
typedef uint32_t (*U1)(const void *);
typedef uint64_t (*N1)(const void *);
typedef void (*Phase)(const void *,Values *);
typedef struct {
    uintptr_t base;
    int (*quiescent)(void *,void *);
    P0 map,parameters,rules,game,portal_manager,current_game,current_map;
    P1 game_map;
    const void *(*player)(int);
    P1 units,formations,unit_formation,formation_location,formation_members,modifiers,city_attributes,sync_random,player_manager;
    B1 manager_initialized;
    PI find_unit,find_formation;
    void (*ids)(const void *,void *);
    void (*vector_ctor)(void *),(*vector_dtor)(void *);
    U1 vector_size,member_size,bool_size,formation_id,actual_unit_id;
    const uint32_t *(*vector_at)(const void *,size_t),*(*member_at)(const void *,size_t);
    const unsigned char *(*bool_at)(const void *,size_t);
    I1 formation_owner,domain,max_moves,location_layer;
    int32_t (*base_moves)(const void *,int32_t);
    void *(*remaining)(void *,const void *),*(*plot_location)(void *,const void *);
    void (*fixed_ctor)(void *,int32_t);
    const void *(*plot_at)(const void *,const Coord *),*(*adjacent)(const void *,const Coord *,int32_t);
    const void *(*portal_at)(const void *,const void *);
    bool (*portal_exit)(const void *,const Coord *);
    bool (*is_adjacent)(const Coord *,const Coord *,int32_t,int32_t,bool,bool);
    I1 width,height;
    B1 wrap_x,wrap_y;
    B1 is_city,is_water,has_city,route_present,route_pillaged,route_bridged,can_embark,ignore_rivers,ignore_shores;
    B1 escort,member_rivers,member_shores,is_hills;
    bool (*ignore_terrain)(const void *,int32_t),(*ignore_feature)(const void *,int32_t);
    bool (*member_terrain)(const void *,int32_t),(*member_feature)(const void *,int32_t);
    bool (*domain_valid)(const void *,int32_t);
    I1 district_type;
    void *(*district_id)(void *,const void *);
    const void *(*find_district)(const void *);
    U1 actual_district_id;
    B1 district_complete;
    U1 terrain_type,feature_type,improvement_type,route_type;
    P0 definitions[5];
    PI definition_at[5];
    U1 definition_index[5],definition_hash[5];
    N1 definition_count[5];
    I1 integer_cost[3];
    float (*route_cost)(const void *);
    B1 free_embark,terrain_water;
    I1 parameter[4];
    bool (*river_crossing)(const void *,const void *,const void *);
    bool (*bridge)(const void *,const void *,const void *);
    bool (*slowed)(const void *,const void *,const void *,const void *);
    void (*values_ctor)(Values *,const void *,const void *,const void *,int32_t,const int32_t *,const int32_t *,const int32_t *,int32_t);
    void *(*final)(int32_t *,const Values *);
    void *(*full)(int32_t *,const void *,const void *,const void *,const void *,int32_t,const int32_t *,const int32_t *,uint32_t);
    void *(*no_zoc)(int32_t *,const void *,const void *,const void *,const void *,int32_t,const int32_t *,const int32_t *,uint32_t);
    void *(*values_cost)(int32_t *,Values *,const void *,uint32_t);
    Phase phase[9];
    const void *map_random;
} Api;

static int bind_api(void *module,Api *a) {
    void *anchor=dlsym(module,"_ZN8GameCore5Rules8Movement3GetEv");
    Dl_info info={0};
    if(!anchor||!dladdr(anchor,&info)||!info.dli_fbase)return 201;
    uintptr_t base=(uintptr_t)info.dli_fbase;
    a->base=base;
    if((uintptr_t)anchor!=base+0x2bece60)return 201;
#define BIND(F,R) do{ a->F=(void *)(base+(R)); }while(0)
    BIND(map,0x290b8a0); BIND(parameters,0x1fe4cb0); BIND(rules,0x2bece60);
    BIND(current_game,0x1fcc320); BIND(current_map,0x2a474d0); BIND(game_map,0x1fe2220);
    BIND(player,0x290b930); BIND(units,0x1fccc70); BIND(formations,0x27ffd10);
    BIND(player_manager,0x20e0670); BIND(manager_initialized,0x2b51830);
    BIND(find_unit,0x2ba5580); BIND(find_formation,0x2b04f10);
    BIND(ids,0x2ba7290); BIND(vector_ctor,0x1ff5700); BIND(vector_dtor,0x1fcda10);
    BIND(vector_size,0x1fcd7e0); BIND(vector_at,0x276d6a0);
    BIND(unit_formation,0x20214e0); BIND(formation_id,0x2a54680);
    BIND(actual_unit_id,0x2021490);
    BIND(formation_owner,0x26b0cb0); BIND(formation_location,0x28e1300);
    BIND(formation_members,0x2021500); BIND(member_size,0x2021ba0); BIND(member_at,0x243bc80);
    BIND(domain,0x2702e40); BIND(base_moves,0x293d130); BIND(max_moves,0x293d160);
    BIND(remaining,0x213d320); BIND(fixed_ctor,0x1fcc8b0); BIND(location_layer,0x293daf0);
    BIND(plot_at,0x2a48ba0); BIND(adjacent,0x1fe2450); BIND(plot_location,0x1fe2230);
    BIND(width,0x2004f20); BIND(height,0x21436d0); BIND(wrap_x,0x2004f10); BIND(wrap_y,0x2166e60);
    BIND(is_adjacent,0x26b1030); BIND(portal_manager,0x2bcfde0);
    BIND(portal_at,0x2bd04b0); BIND(portal_exit,0x2bcf480);
    BIND(is_city,0x1fe25b0); BIND(is_water,0x2bc97d0); BIND(has_city,0x2bca7b0);
    BIND(is_hills,0x2bc9a40);
    BIND(city_attributes,0x2bca780); BIND(domain_valid,0x2bc8350);
    BIND(district_type,0x2bc81b0); BIND(district_complete,0x2bc8200);
    BIND(district_id,0x21674b0); BIND(find_district,0x286e0b0); BIND(actual_district_id,0x21a36c0);
    BIND(terrain_type,0x20e35a0); BIND(feature_type,0x1fe2410);
    BIND(improvement_type,0x1fe2430); BIND(route_type,0x242d6a0);
    BIND(route_present,0x20e3570); BIND(route_pillaged,0x2bcea10); BIND(route_bridged,0x2beef70);
    BIND(can_embark,0x2beefa0); BIND(ignore_terrain,0x293d4d0); BIND(ignore_feature,0x293d590);
    BIND(ignore_rivers,0x293d830); BIND(ignore_shores,0x293d6d0);
    BIND(modifiers,0x2702c60); BIND(escort,0x293e9f0); BIND(member_rivers,0x293ea20);
    BIND(member_shores,0x293ea00); BIND(member_terrain,0x2c27cc0); BIND(member_feature,0x2c27d00);
    BIND(bool_size,0x2007f30); BIND(bool_at,0x1fe6b50);
    BIND(definitions[0],0x2214730); BIND(definitions[1],0x2213bf0);
    BIND(definitions[2],0x2213ee0); BIND(definitions[3],0x2214540); BIND(definitions[4],0x2213a70);
    BIND(definition_at[0],0x2194cb0); BIND(definition_at[1],0x1fe25c0);
    BIND(definition_at[2],0x1fe5b30); BIND(definition_at[3],0x295bdb0); BIND(definition_at[4],0x1fe2b00);
    BIND(definition_index[0],0x20e3240); BIND(definition_index[1],0x1fe8250);
    BIND(definition_index[2],0x1fccd10); BIND(definition_index[3],0x261fb00); BIND(definition_index[4],0x1fcccf0);
    BIND(definition_hash[0],0x2bf69d0); BIND(definition_hash[1],0x20a11a0);
    BIND(definition_hash[2],0x1fe5b50); BIND(definition_hash[3],0x2bcead0); BIND(definition_hash[4],0x21d56f0);
    BIND(definition_count[0],0x2194f90); BIND(definition_count[1],0x1fe8030);
    BIND(definition_count[2],0x1fe8290); BIND(definition_count[3],0x2300a80); BIND(definition_count[4],0x1fe8270);
    BIND(integer_cost[0],0x2bcad60); BIND(integer_cost[1],0x2bcad70); BIND(integer_cost[2],0x2bcad80);
    BIND(route_cost,0x2b98630); BIND(free_embark,0x2beefb0);
    BIND(terrain_water,0x2bcadc0);
    BIND(parameter[0],0x2bcad50); BIND(parameter[1],0x2beef80);
    BIND(parameter[2],0x2beef90); BIND(parameter[3],0x2beefc0);
    BIND(river_crossing,0x2bed3b0); BIND(bridge,0x2bed460); BIND(slowed,0x2949740);
    BIND(values_ctor,0x2945d40); BIND(final,0x2945de0); BIND(full,0x29484e0);
    BIND(no_zoc,0x2948430); BIND(values_cost,0x2949170);
    BIND(phase[0],0x2bed5a0); BIND(phase[1],0x2bed5f0); BIND(phase[2],0x2bed680);
    BIND(phase[3],0x2bed700); BIND(phase[4],0x2bed760); BIND(phase[5],0x2bed890);
    BIND(phase[6],0x2bed8f0); BIND(phase[7],0x2bed9f0); BIND(phase[8],0x2bedbb0);
#undef BIND
    /* Data symbols use actual dynamic lookup, not a synthetic Random owner. */
    a->game=dlsym(module,"_ZN8GameCore4Game11GetInstanceEv");
    a->sync_random=dlsym(module,"_ZNK8GameCore4Game8Instance13GetSyncRandomEv");
    a->map_random=dlsym(module,"_ZN8GameCore3Map9Generator10ms_kRandomE");
    if((uintptr_t)a->game!=base+0x2660650||(uintptr_t)a->sync_random!=base+0x2d7b4d0
       ||(uintptr_t)a->map_random!=base+0x3c072f0)return 201;
    a->quiescent=dlsym(RTLD_DEFAULT,"c6_linux_movement_cost_quiescent");
    if(!a->quiescent)return 201;
    return 0;
}

static const char *boolean(bool v) { return v?"true":"false"; }
static int require_idle(const Api *a,void *module,void *context) {
    int status=a->quiescent(module,context);
    if(status){
        fprintf(stderr,"C6_LINUX_MOVEMENT_COST quiescence_unproven status %d no_post_state_or_cleanup\n",status);
        fflush(stderr);_exit(status>0&&status<256?status:240);
    }
    return 0;
}
static void raw_hex(FILE *out,const void *p,size_t size) {
    const unsigned char *b=p;fputc('"',out);
    for(size_t i=0;i<size;i++)fprintf(out,"%02x",b[i]);
    fputc('"',out);
}
static uintptr_t pointer_at(const void *p,size_t offset) {
    uintptr_t value;memcpy(&value,(const unsigned char *)p+offset,8);return value;
}
static void identity_json(FILE *out,uintptr_t address) {
    Dl_info info={0};bool found=address&&dladdr((const void *)address,&info)&&info.dli_fbase;
    fprintf(out,"{\"address\":\"0x%" PRIxPTR "\",\"resolved\":%s",address,boolean(found));
    if(found){fprintf(out,",\"module_base\":\"0x%" PRIxPTR "\",\"offset\":\"0x%" PRIxPTR "\",\"module_path_hex\":",(uintptr_t)info.dli_fbase,address-(uintptr_t)info.dli_fbase);
        raw_hex(out,info.dli_fname,strlen(info.dli_fname));}
    fputc('}',out);
}
static void scalar_state(FILE *out,const Values *v) {
    fprintf(out,"[%d,%d,%d,%d]",v->hard,v->soft,v->regular,v->to_exit);
}
static int registry_json(FILE *out,const Api *a) {
    const char *names[5]={"terrain","feature","improvement","route","district"};
    for(unsigned kind=0;kind<5;kind++){
        const void *collection=a->definitions[kind]();if(!collection)return 202;
        uint64_t count=a->definition_count[kind](collection);if(count>65536)return 202;
        fprintf(out,"{\"kind\":\"registry\",\"name\":\"%s\",\"count\":%" PRIu64 ",\"entries\":[",names[kind],count);
        for(uint32_t i=0;i<count;i++){
            const void *definition=a->definition_at[kind](collection,i);if(!definition)return 202;
            fprintf(out,"%s{\"query\":%u,\"index\":%u,\"hash\":%u",i?",":"",i,a->definition_index[kind](definition),a->definition_hash[kind](definition));
            if(kind<3)fprintf(out,",\"integer_cost\":%d",a->integer_cost[kind](definition));
            if(kind==0)fprintf(out,",\"is_water\":%s",boolean(a->terrain_water(definition)));
            if(kind==3){float value=a->route_cost(definition);uint32_t bits;memcpy(&bits,&value,4);fprintf(out,",\"float_bits\":%u",bits);}
            if(kind==4)fprintf(out,",\"free_embark\":%s",boolean(a->free_embark(definition)));
            fputc('}',out);
        }
        fputs("]}\n",out);
    }
    return 0;
}
typedef struct {
    const void *definition[5];
    uint32_t query[5],index[5],hash[5],route_bits;
    int32_t cost[3],district;
    bool city,water,hills,has_city,valid_domain,route,pillaged,bridged,complete,free_embark;
    ComponentId requested_district;
    bool resolved_district;
    uint32_t actual_district;
    uint32_t working_city,owning_city;
    uintptr_t vptr,city_vptr,city_target,has_city_target;
    int16_t valid_domains;
    Coord coord;
} Plot;

static int plot_facts(const Api *a,const void *p,int domain,Plot *f) {
    memset(f,0,sizeof *f);
    f->vptr=pointer_at(p,0);
    if(f->vptr!=a->base+0x3ae23a0)return 202;
    f->city_target=pointer_at((const void *)f->vptr,0x38);
    f->has_city_target=pointer_at((const void *)f->vptr,0x48);
    if(f->city_target!=a->base+0x2bca780||f->has_city_target!=a->base+0x2bca7b0)return 202;
    a->plot_location(&f->coord,p);
    f->city=a->is_city(p);f->water=a->is_water(p);f->hills=a->is_hills(p);f->has_city=a->has_city(p);
    const void *attributes=a->city_attributes(p);if(!attributes)return 202;
    f->city_vptr=pointer_at(attributes,0);if(f->city_vptr!=a->base+0x3ae2360)return 202;
    f->valid_domain=a->domain_valid(attributes,domain);
    f->district=a->district_type(attributes);f->complete=a->district_complete(attributes);
    a->district_id(&f->requested_district,attributes);
    const void *district=a->find_district(&f->requested_district);
    f->resolved_district=district!=NULL;if(district)f->actual_district=a->actual_district_id(district);
    /* Read-only omitted live field, independently established by CityAttributes
     * DWARF/getter analysis. No domain-mask state is manufactured or patched. */
    memcpy(&f->valid_domains,(const unsigned char *)attributes+0x14,2);
    memcpy(&f->working_city,(const unsigned char *)attributes+0x8,4);
    memcpy(&f->owning_city,(const unsigned char *)attributes+0xc,4);
    f->route=a->route_present(p);f->pillaged=a->route_pillaged(p);f->bridged=a->route_bridged(p);
    f->query[0]=a->terrain_type(p);f->query[1]=a->feature_type(p);
    f->query[2]=a->improvement_type(p);f->query[3]=a->route_type(p);f->query[4]=(uint32_t)f->district;
    for(unsigned i=0;i<5;i++) {
        const void *collection=a->definitions[i]();if(!collection)return 202;
        f->definition[i]=a->definition_at[i](collection,f->query[i]);
        if(f->definition[i]) {
            f->index[i]=a->definition_index[i](f->definition[i]);
            f->hash[i]=a->definition_hash[i](f->definition[i]);
            if(i<3)f->cost[i]=a->integer_cost[i](f->definition[i]);
            if(i==3){float cost=a->route_cost(f->definition[i]);memcpy(&f->route_bits,&cost,4);}
            if(i==4)f->free_embark=a->free_embark(f->definition[i]);
        }
    }
    /* Shore code dereferences the original District definition for a real type.
     * A null result must abort this supported probe, never become invented false. */
    return f->district!=-1&&!f->definition[4]?202:0;
}

static void optional_integer(FILE *out,bool present,int32_t value) {
    if(present)fprintf(out,"%d",value);else fputs("null",out);
}
static void plot_json(FILE *out,const Plot *p) {
    fprintf(out,"{\"x\":%d,\"y\":%d,\"is_hills\":%s,\"is_city\":%s,\"is_water\":%s,\"has_city_attributes\":%s,\"city_domain_valid\":%s,\"terrain_cost\":",
        p->coord.x,p->coord.y,boolean(p->hills),boolean(p->city),boolean(p->water),boolean(p->has_city),boolean(p->valid_domain));
    optional_integer(out,p->definition[0]!=NULL,p->cost[0]);fputs(",\"feature_change\":",out);
    optional_integer(out,p->definition[1]!=NULL,p->cost[1]);fputs(",\"improvement_change\":",out);
    optional_integer(out,p->definition[2]!=NULL,p->cost[2]);fputs(",\"route\":",out);
    if(!p->route)fputs("null",out);
    else {
        fprintf(out,"{\"pillaged\":%s,\"bridged\":%s,\"movement_cost_float_bits\":",boolean(p->pillaged),boolean(p->bridged));
        if(p->definition[3])fprintf(out,"%u",p->route_bits);else fputs("null",out);
        fputc('}',out);
    }
    fprintf(out,",\"district_complete\":%s,\"district_free_embark\":",boolean(p->complete));
    if(p->district==-1)fputs("null",out);else fputs(boolean(p->free_embark),out);
    fprintf(out,",\"working_city_id\":%u,\"owning_city_id\":%u,\"district_owner\":%d,\"district_component_type\":%d,\"requested_district_id\":%u,\"district_resolved\":%s,\"valid_domains_mask\":%d",p->working_city,p->owning_city,p->requested_district.owner,p->requested_district.type,p->requested_district.id,boolean(p->resolved_district),p->valid_domains);
    if(p->resolved_district)fprintf(out,",\"actual_district_id\":%u",p->actual_district);
    fputs(",\"plot_vptr\":",out);identity_json(out,p->vptr);
    fputs(",\"city_attributes_vptr\":",out);identity_json(out,p->city_vptr);
    fputs(",\"get_city_attributes_target\":",out);identity_json(out,p->city_target);
    fputs(",\"has_city_attributes_target\":",out);identity_json(out,p->has_city_target);
    fputs(",\"definitions\":[",out);
    for(unsigned i=0;i<5;i++){
        fprintf(out,"%s{\"query\":%u,\"present\":%s",i?",":"",p->query[i],boolean(p->definition[i]!=NULL));
        if(p->definition[i])fprintf(out,",\"index\":%u,\"hash\":%u",p->index[i],p->hash[i]);
        fputc('}',out);
    }
    fputs("]}",out);
}

/* Read-only actual Unit vector subobjects established by original GetIgnore*
 * instructions and matching Unit DWARF; original vector getters inspect them.
 * No pointer/length/capacity fields are manufactured. */
static int bool_vector_json(FILE *out,const Api *a,const void *unit,size_t offset) {
    const void *vector=(const unsigned char *)unit+offset;
    uint32_t size=a->bool_size(vector);if(size>65536)return 203;
    fputc('[',out);
    for(uint32_t i=0;i<size;i++){
        const unsigned char *v=a->bool_at(vector,i);if(!v)return 203;
        fprintf(out,"%s%u",i?",":"",(unsigned)(*v!=0));
    }
    fputc(']',out);return 0;
}
static int members_json(FILE *out,const Api *a,const void *formation,const void *units,int terrain,int feature) {
    const void *members=a->formation_members(formation);if(!members)return 203;
    uint32_t count=a->member_size(members);if(count>65536)return 203;
    fputc('[',out);
    for(uint32_t i=0;i<count;i++) {
        const uint32_t *id=a->member_at(members,i);if(!id)return 203;
        const void *unit=a->find_unit(units,*id);
        fprintf(out,"%s{\"id\":%u,\"found\":%s",i?",":"",*id,boolean(unit!=NULL));
        if(unit){
            const void *mod=a->modifiers(unit);if(!mod)return 203;
            fprintf(out,",\"actual_unit_id\":%u,\"terrain\":%s,\"feature\":%s,\"rivers\":%s,\"shores\":%s,\"escort\":%s,\"invalid_terrain_bits\":",a->actual_unit_id(unit),
                boolean(a->member_terrain(unit,terrain)),boolean(a->member_feature(unit,feature)),
                boolean(a->member_rivers(mod)),boolean(a->member_shores(mod)),boolean(a->escort(mod)));
            int rc=bool_vector_json(out,a,unit,0x378);if(rc)return rc;
            fputs(",\"ignore_terrain_bits\":",out);rc=bool_vector_json(out,a,unit,0x3a0);if(rc)return rc;
            fputs(",\"ignore_feature_bits\":",out);rc=bool_vector_json(out,a,unit,0x3c8);if(rc)return rc;
        }
        fputc('}',out);
    }
    fputc(']',out);return 0;
}

static int observe_costs(const Api *a,void *module,void *context,FILE *out,const void *rules,
                         const void *formation,const void *from,const void *to,
                         int32_t base,const int32_t *max,const int32_t *remaining,const int32_t *exit) {
    bool slowed;int32_t full,no_zoc,phased,overload;Values values,aware;
#define QUERY(E) do{E;int idle=require_idle(a,module,context);if(idle)return idle;}while(0)
    /* The initial requested destination is the original MoveTo exception plot.
     * For this single-edge query that is the genuine adjacent destination. */
    QUERY(slowed=a->slowed(formation,from,to,to));
    QUERY(a->full(&full,formation,from,to,to,base,max,remaining,0));
    QUERY(a->no_zoc(&no_zoc,formation,from,to,to,base,max,remaining,0));
    QUERY(a->values_ctor(&values,formation,from,to,base,max,remaining,exit,1));
    fprintf(out,",\"is_slowed_by_zoc\":%s,\"full_final\":%d,\"no_zoc_final\":%d,\"initial\":",boolean(slowed),full,no_zoc);
    scalar_state(out,&values);fputs(",\"phases\":[",out);
    for(unsigned p=0;p<9;p++){QUERY(a->phase[p](rules,&values));if(p)fputc(',',out);scalar_state(out,&values);}
    QUERY(a->final(&phased,&values));
    QUERY(a->values_ctor(&aware,formation,from,to,base,max,remaining,exit,0));
    QUERY(a->values_cost(&overload,&aware,to,0));
    fprintf(out,"],\"phased_final\":%d,\"values_overload_final\":%d,\"aware_values\":",phased,overload);scalar_state(out,&aware);
    fprintf(out,",\"native_paths_agree\":%s",boolean(phased==no_zoc&&overload==full));
#undef QUERY
    return 0;
}

int c6_linux_movement_cost_probe(void *module,void *context) {
    const char *output=getenv("C6_MOVEMENT_COST_OUTPUT");
    const char *paths[4]={getenv("C6_MOVEMENT_COST_SNAPSHOT_BEFORE"),getenv("C6_MOVEMENT_COST_SNAPSHOT_CONTROL"),getenv("C6_MOVEMENT_COST_SNAPSHOT_AFTER"),getenv("C6_MOVEMENT_COST_SNAPSHOT_REPEAT")};
    const char *mode=getenv("C6_MOVEMENT_COST_EXECUTE");
    const char *profile=getenv("C6_MOVEMENT_COST_PROFILE_SHA256");
    bool execute=mode&&!strcmp(mode,"1");
    if(!module||!context||!output||!profile||strlen(profile)!=64||!paths[0]||!paths[1]||!paths[2]||!paths[3]||(mode&&strcmp(mode,"0")&&strcmp(mode,"1")))return 204;
    for(unsigned i=0;i<64;i++)if(!((profile[i]>='0'&&profile[i]<='9')||(profile[i]>='a'&&profile[i]<='f')))return 204;
    unsigned limit=256;const char *limit_text=getenv("C6_MOVEMENT_COST_MAX_FORMATIONS");
    if(limit_text){char *end=NULL;unsigned long value=strtoul(limit_text,&end,10);if(!*limit_text||*end||!value||value>4096)return 204;limit=(unsigned)value;}
    Api a={0};int rc=bind_api(module,&a);if(rc)return rc;
    rc=require_idle(&a,module,context);if(rc)return rc;
    unsigned mxcsr=_mm_getcsr();
    /* Preserve the numerical environment; reject instead of silently changing it. */
    if(fegetround()!=FE_TONEAREST||(mxcsr&0xffc0)!=0x1f80)return 205;
    const void *map=a.map(),*parameters=a.parameters(),*rules=a.rules(),*game=a.game();
    if(!map||!parameters||!rules||!game){
        fprintf(stderr,"C6_LINUX_MOVEMENT_COST admission_failure missing_receiver map %d parameters %d rules %d game %d\n",map!=NULL,parameters!=NULL,rules!=NULL,game!=NULL);return 206;
    }
    const void *observed=a.current_game();
    if(observed!=game){fprintf(stderr,"C6_LINUX_MOVEMENT_COST admission_failure globals_game actual %p expected %p\n",observed,game);return 206;}
    observed=a.current_map();
    if(observed!=map){fprintf(stderr,"C6_LINUX_MOVEMENT_COST admission_failure singleton_map actual %p expected %p\n",observed,map);return 206;}
    observed=a.game_map(game);
    if(observed!=map){fprintf(stderr,"C6_LINUX_MOVEMENT_COST admission_failure game_map actual %p expected %p\n",observed,map);return 206;}
    uintptr_t map_vptr=pointer_at(map,0);
    if(map_vptr!=a.base+0x3afb160){fprintf(stderr,"C6_LINUX_MOVEMENT_COST admission_failure map_vptr actual 0x%" PRIxPTR " expected 0x%" PRIxPTR "\n",map_vptr,a.base+0x3afb160);return 206;}
    uintptr_t plot_target=pointer_at((const void *)map_vptr,0x78);
    if(plot_target!=a.base+0x2a48ba0){fprintf(stderr,"C6_LINUX_MOVEMENT_COST admission_failure map_get_plot actual 0x%" PRIxPTR " expected 0x%" PRIxPTR "\n",plot_target,a.base+0x2a48ba0);return 206;}
    const void *manager=a.player_manager(game);
    if(!manager){fputs("C6_LINUX_MOVEMENT_COST admission_failure missing_manager\n",stderr);return 206;}
    if(!a.manager_initialized(manager)){fprintf(stderr,"C6_LINUX_MOVEMENT_COST admission_failure manager_not_initialized manager %p\n",manager);return 206;}
    /* Original Manager.Initialize allocates 65 pointer slots at +0x20. The
     * matching DWARF vector and GetPlayer load establish its begin/end fields.
     * Check the real live extent before any unchecked original GetPlayer call. */
    uintptr_t player_begin,player_end;
    memcpy(&player_begin,(const unsigned char *)manager+0x20,8);
    memcpy(&player_end,(const unsigned char *)manager+0x28,8);
    if(!player_begin||player_end<player_begin||(player_end-player_begin)%8
       ||(player_end-player_begin)/8<64||(player_end-player_begin)/8>4096){
        fprintf(stderr,"C6_LINUX_MOVEMENT_COST admission_failure player_table_extent begin 0x%" PRIxPTR " end 0x%" PRIxPTR "\n",player_begin,player_end);return 206;
    }
    const void *sync=a.sync_random(game);if(!sync){fputs("C6_LINUX_MOVEMENT_COST admission_failure missing_sync_random\n",stderr);return 206;}
    unsigned char rng_before[64];memcpy(rng_before,sync,32);memcpy(rng_before+32,a.map_random,32);
    /* Save controls precede all discovery/fact/cost calls; repeat saves retain raw
     * normalization differences and are never treated as hidden-byte exclusions. */
    for(unsigned i=0;i<2;i++){rc=require_idle(&a,module,context);if(!rc)rc=c6_linux_context_save(module,context,paths[i]);if(!rc)rc=require_idle(&a,module,context);if(rc)return rc;}
    FILE *out=fopen(output,"wx");if(!out)return 207;
    fprintf(out,"{\"kind\":\"header\",\"schema\":\"c6-linux-movement-cost-native-v1\",\"probe_contract_revision\":\"xp2-map-and-terrain-water-2\",\"game_sha256\":\"a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa\",\"host_sha256\":\"106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4\",\"profile_sha256\":\"%s\",\"execute\":%s,\"options\":0,\"actual_move_options_available\":false,\"mxcsr\":%u,\"formation_limit\":%u,\"complete_affected_state\":false,\"complete_rng_inventory\":false}\n",profile,boolean(execute),mxcsr,limit);
    fprintf(out,"{\"kind\":\"attestation\",\"game\":\"0x%" PRIxPTR "\",\"globals_game\":\"0x%" PRIxPTR "\",\"map\":\"0x%" PRIxPTR "\",\"manager\":\"0x%" PRIxPTR "\",\"rules\":\"0x%" PRIxPTR "\",\"parameters\":\"0x%" PRIxPTR "\",\"player_table_slots\":%" PRIuPTR ",\"map_width\":%d,\"map_height\":%d,\"wrap_x\":%s,\"wrap_y\":%s,\"map_vptr\":",(uintptr_t)game,(uintptr_t)a.current_game(),(uintptr_t)map,(uintptr_t)manager,(uintptr_t)rules,(uintptr_t)parameters,(player_end-player_begin)/8,a.width(map),a.height(map),boolean(a.wrap_x(map)),boolean(a.wrap_y(map)));
    identity_json(out,map_vptr);fputs(",\"map_get_plot_target\":",out);identity_json(out,pointer_at((const void *)map_vptr,0x78));
    fputs(",\"quiescence_hook\":",out);identity_json(out,(uintptr_t)a.quiescent);fputs("}\n",out);
    rc=registry_json(out,&a);
    if(!rc)rc=require_idle(&a,module,context);
    int32_t parameter[4];for(unsigned i=0;i<4;i++)parameter[i]=a.parameter[i](parameters);
    struct { int owner;uint32_t id;const void *pointer; } seen[4096];
    unsigned formations=0,cases=0,units_seen=0,skipped=0;bool truncated=false;
    for(int owner=0;owner<64&&!rc&&!truncated;owner++) {
        const void *player=a.player(owner);if(!player)continue;
        const void *units=a.units(player),*container=a.formations(player);if(!units||!container){rc=208;break;}
        _Alignas(8) unsigned char ids[24];a.vector_ctor(ids);a.ids(units,ids);
        uint32_t count=a.vector_size(ids);if(count>1000000){a.vector_dtor(ids);rc=208;break;}
        for(uint32_t n=0;n<count&&!rc;n++) {
            const uint32_t *id=a.vector_at(ids,n);if(!id){rc=208;break;}
            const void *unit=a.find_unit(units,*id);units_seen++;
            if(!unit){skipped++;continue;}
            const void *formation=a.unit_formation(unit);if(!formation){skipped++;continue;}
            uint32_t fid=a.formation_id(formation);
            if(a.formation_owner(formation)!=owner||a.find_formation(container,fid)!=formation){rc=209;break;}
            bool duplicate=false;
            for(unsigned j=0;j<formations;j++)if(seen[j].owner==owner&&seen[j].id==fid){if(seen[j].pointer!=formation)rc=209;duplicate=true;break;}
            if(rc)break;
            if(duplicate)continue;
            if(formations==limit){truncated=true;break;}
            seen[formations].owner=owner;seen[formations].id=fid;seen[formations++].pointer=formation;
            const Coord *location=a.formation_location(formation);if(!location){rc=209;break;}
            const void *from=a.plot_at(map,location);if(!from){skipped++;continue;}
            int domain=a.domain(formation),layer=a.location_layer(formation);
            Plot source;rc=plot_facts(&a,from,domain,&source);if(rc)break;
            for(int direction=0;direction<6&&!rc;direction++) {
                const void *to=a.adjacent(map,location,direction);if(!to)continue;
                Plot destination;rc=plot_facts(&a,to,domain,&destination);if(rc)break;
                /* These arguments follow the original MoveTo cost call: destination
                 * IsWater selects domain0 versus -1 for GetBaseMoves; GetMaxMoves
                 * is integer, then converted by native FixedPoint constructor. */
                int32_t base_moves=a.base_moves(formation,destination.water?0:-1),max_moves,remaining,exit;
                a.fixed_ctor(&max_moves,a.max_moves(formation));a.remaining(&remaining,formation);a.fixed_ctor(&exit,0);
                int terrain=destination.definition[0]?(int32_t)destination.index[0]:-1;
                int feature=destination.definition[1]?(int32_t)destination.index[1]:-1;
                bool ignore_terrain=a.ignore_terrain(formation,terrain),ignore_feature=a.ignore_feature(formation,feature);
                bool ignore_rivers=a.ignore_rivers(formation),ignore_shores=a.ignore_shores(formation),embark=a.can_embark(formation);
                bool river=a.river_crossing(rules,from,to),bridge=a.bridge(rules,from,to);
                bool adjacent=a.is_adjacent(&source.coord,&destination.coord,a.width(map),a.height(map),a.wrap_x(map),a.wrap_y(map));
                bool portal_exit=false;
                if(!adjacent){const void *manager=a.portal_manager();if(!manager){rc=206;break;}
                    const void *portal=a.portal_at(manager,from);portal_exit=portal&&a.portal_exit(portal,&destination.coord);}
                fprintf(out,"{\"kind\":\"case\",\"owner\":%d,\"unit_id\":%u,\"actual_unit_id\":%u,\"formation_id\":%u,\"formation_domain\":%d,\"formation_layer\":%d,\"direction\":%d,\"budget\":{\"base_moves\":%d,\"max_moves\":%d,\"remaining\":%d,\"exit_moves\":%d},\"parameters\":{\"city_movement_cost\":%d,\"movement_river_cost\":%d,\"movement_portal_cost\":%d,\"movement_embark_cost\":%d},\"formation\":{\"ignore_terrain\":%s,\"ignore_feature\":%s,\"ignore_rivers\":%s,\"ignore_shores\":%s,\"can_embark\":%s},\"terrain_ignore_index\":%d,\"feature_ignore_index\":%d,\"members\":",
                    owner,*id,a.actual_unit_id(unit),fid,domain,layer,direction,base_moves,max_moves,remaining,exit,parameter[0],parameter[1],parameter[2],parameter[3],
                    boolean(ignore_terrain),boolean(ignore_feature),boolean(ignore_rivers),boolean(ignore_shores),boolean(embark),terrain,feature);
                rc=members_json(out,&a,formation,units,terrain,feature);if(rc)break;
                fputs(",\"from\":",out);plot_json(out,&source);fputs(",\"to\":",out);plot_json(out,&destination);
                fprintf(out,",\"edge\":{\"river_crossing\":%s,\"bridge_between\":%s,\"adjacent\":%s,\"source_portal_has_exit_at_destination\":%s}",boolean(river),boolean(bridge),boolean(adjacent),boolean(portal_exit));
                fputs(",\"formation_archive_vptr\":",out);identity_json(out,pointer_at(formation,0));
                rc=require_idle(&a,module,context);if(rc)break;
                if(execute) {
                    rc=observe_costs(&a,module,context,out,rules,formation,from,to,base_moves,&max_moves,&remaining,&exit);
                    if(rc)break;
                }
                fputs("}\n",out);cases++;
            }
        }
        a.vector_dtor(ids);
    }
    (void)require_idle(&a,module,context);
    for(unsigned i=2;i<4&&!rc;i++){rc=require_idle(&a,module,context);if(!rc)rc=c6_linux_context_save(module,context,paths[i]);if(!rc)rc=require_idle(&a,module,context);}
    /* Even a discovery/Save error cannot authorize post-state reads while the
     * worker's status is unknown. The mandatory hook _exits on that condition. */
    (void)require_idle(&a,module,context);
    bool rng_same=!memcmp(rng_before,sync,32)&&!memcmp(rng_before+32,a.map_random,32);
    fprintf(out,"{\"kind\":\"summary\",\"status\":%d,\"formations\":%u,\"units_seen\":%u,\"cases\":%u,\"skipped\":%u,\"truncated\":%s,\"known_rng_equal\":%s,\"rng_before\":",rc,formations,units_seen,cases,skipped,boolean(truncated),boolean(rng_same));
    raw_hex(out,rng_before,64);fputs(",\"rng_after\":",out);raw_hex(out,sync,32);fputs(",\"map_rng_after\":",out);raw_hex(out,a.map_random,32);fputs("}\n",out);
    if(ferror(out)&&!rc)rc=210;
    if(fclose(out)&&!rc)rc=210;
    fprintf(stderr,"C6_LINUX_MOVEMENT_COST {\"status\":%d,\"execute\":%s,\"cases\":%u,\"known_rng_equal\":%s,\"gameplay_parity\":false}\n",rc,boolean(execute),cases,boolean(rng_same));
    return rc;
}
