#define _GNU_SOURCE
#include "placement_metadata.h"
#include "context_stream.h"
#include <dlfcn.h>
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
typedef uint64_t (*N1)(const void *);
typedef int32_t (*I1)(const void *);
typedef bool (*B1)(const void *);
typedef struct { const void *entry; } Iterator;
typedef struct { int32_t x,y; } Coord;
_Static_assert(sizeof(Iterator)==8,"native one-word iterator");
_Static_assert(sizeof(Coord)==8,"native PlotCoord");
typedef struct {
    uintptr_t base;
    int (*idle)(void *,void *);
    P0 game,current_game,map,current_map,definitions,parameters,relations;
    P1 game_map,manager,sync_random,map_units,configuration,units,formations;
    P1 unit_location,formation_location,unit_definition,unit_formation;
    B1 initialized,alive,coord_valid;
    const int32_t *(*mapping)(const void *);
    N1 mapping_size;
    bool (*valid)(const void *,int32_t);
    const void *(*player)(const void *,int32_t);
    I1 owner,slot_status,team,plot_limit,width,height;
    N1 definition_count;
    PI definition_at,definition_find;
    U1 definition_hash,definition_index,domain,formation_class;
    B1 trade,spy;
    I1 combat,range;
    U1 unit_count,formation_count,unit_id,formation_id;
    I1 unit_owner,formation_owner,unit_layer,formation_layer;
    PI find_unit,find_formation;
    Iterator (*begin[2])(const void *),(*end[2])(const void *);
    const void *(*deref[2])(const Iterator *);
    Iterator *(*next[2])(Iterator *);
    U1 war_count;
    const void *(*war_at)(const void *,size_t);
    const void *(*plot)(const void *,const Coord *),*(*adjacent)(const void *,const Coord *,int32_t);
    void *(*plot_coord)(void *,const void *);
    const void *(*occupancy)(const void *,int32_t,int32_t,uint32_t);
    uint32_t (*count[2])(const void *,const void *,bool);
    const void *map_random;
} Api;
static uintptr_t pointer_at(const void *p,size_t offset) {
    uintptr_t value;memcpy(&value,(const unsigned char *)p+offset,sizeof value);return value;
}
static int32_t signed_at(const void *p,size_t offset) {
    int32_t value;memcpy(&value,(const unsigned char *)p+offset,sizeof value);return value;
}
static const char *boolean(bool value){return value?"true":"false";}
static void hex(FILE *out,const void *p,size_t size) {
    const unsigned char *bytes=p;fputc('"',out);
    for(size_t i=0;i<size;i++)fprintf(out,"%02x",bytes[i]);
    fputc('"',out);
}
static void idle(const Api *a,void *module,void *context) {
    int rc=a->idle(module,context);
    if(rc){fprintf(stderr,"C6_LINUX_PLACEMENT_METADATA quiescence_unproven %d no_post_state_or_cleanup\n",rc);fflush(stderr);_exit(240);}
}
static int bind_api(void *module,Api *a) {
    void *anchor=dlsym(module,"_ZN8GameCore4Game11GetInstanceEv");Dl_info info={0};
    if(!anchor||!dladdr(anchor,&info)||!info.dli_fbase)return 241;
    a->base=(uintptr_t)info.dli_fbase;
    if((uintptr_t)anchor!=a->base+0x2660650)return 241;
#define BIND(F,R) do { a->F=(void *)(a->base+(R)); } while(0)
    BIND(game,0x2660650); BIND(current_game,0x1fcc320); BIND(map,0x290b8a0);
    BIND(current_map,0x2a474d0); BIND(game_map,0x1fe2220); BIND(manager,0x20e0670);
    BIND(initialized,0x2b51830); BIND(sync_random,0x2d7b4d0); BIND(map_units,0x2a48750);
    BIND(mapping,0x2004680); BIND(mapping_size,0x27e4520);
    BIND(valid,0x2004fc0); BIND(player,0x2003940); BIND(owner,0x1fcc140);
    BIND(configuration,0x2b315e0); BIND(slot_status,0x28ebe70); BIND(team,0x28ebce0); BIND(alive,0x2003cb0);
    BIND(definitions,0x22147d0); BIND(definition_count,0x1fe82a0); BIND(definition_at,0x1fcd120);
    BIND(definition_find,0x1ffc150); BIND(definition_hash,0x1fe52c0); BIND(definition_index,0x1fcccb0);
    BIND(domain,0x1fe5870); BIND(formation_class,0x2429d80); BIND(trade,0x200ddf0); BIND(spy,0x20b4490);
    BIND(combat,0x1fe4c20); BIND(range,0x21c1000);
    BIND(parameters,0x1fe4cb0); BIND(plot_limit,0x2807b10); BIND(relations,0x2980f10);
    BIND(war_count,0x2989170); BIND(war_at,0x2989180);
    BIND(units,0x1fccc70); BIND(formations,0x27ffd10); BIND(unit_count,0x1fe4ee0); BIND(formation_count,0x2b058a0);
    BIND(begin[0],0x2021450); BIND(end[0],0x2021470); BIND(deref[0],0x2021480); BIND(next[0],0x20214a0);
    BIND(begin[1],0x2ba1850); BIND(end[1],0x2ba1840); BIND(deref[1],0x2ba1870); BIND(next[1],0x2ba1880);
    BIND(unit_id,0x2021490); BIND(formation_id,0x2a54680); BIND(unit_owner,0x2004670); BIND(formation_owner,0x26b0cb0);
    BIND(find_unit,0x2ba5580); BIND(find_formation,0x2b04f10); BIND(unit_formation,0x20214e0);
    BIND(unit_location,0x2004620); BIND(formation_location,0x28e1300); BIND(unit_definition,0x20037c0);
    BIND(unit_layer,0x2c1c790); BIND(formation_layer,0x293daf0); BIND(coord_valid,0x1fe2340);
    BIND(width,0x2004f20); BIND(height,0x21436d0); BIND(plot,0x2a48ba0); BIND(adjacent,0x1fe2450);
    BIND(plot_coord,0x1fe2230); BIND(occupancy,0x2a4e6c0); BIND(count[0],0x2c432a0); BIND(count[1],0x2c433f0);
#undef BIND
    a->map_random=dlsym(module,"_ZN8GameCore3Map9Generator10ms_kRandomE");
    a->idle=dlsym(RTLD_DEFAULT,"c6_linux_placement_metadata_quiescent");
    return !a->idle||(uintptr_t)a->map_random!=a->base+0x3c072f0?241:0;
}
static int registry(FILE *out,const Api *a) {
    const void *collection=a->definitions();if(!collection)return 243;
    uint64_t count=a->definition_count(collection);if(count>=UINT32_MAX)return 243;
    fprintf(out,"{\"kind\":\"unit_definitions\",\"count\":%llu,\"entries\":[",(unsigned long long)count);
    for(uint32_t i=0;i<count;i++) {
        const void *d=a->definition_at(collection,i);if(!d)return 243;
        uint32_t hash=a->definition_hash(d);const void *found=a->definition_find(collection,hash),*direct=a->definition_find(collection,i);
        fprintf(out,"%s{\"ordinal\":%u,\"index\":%u,\"hash\":%u,\"find_hash_index\":",i?",":"",i,a->definition_index(d),hash);
        if(found)fprintf(out,"%u",a->definition_index(found));else fputs("null",out);
        fputs(",\"find_ordinal_index\":",out);if(direct)fprintf(out,"%u",a->definition_index(direct));else fputs("null",out);
        fprintf(out,",\"domain_hash\":%u,\"formation_class_hash\":%u,\"make_trade_route\":%s,\"spy\":%s,\"combat\":%d,\"range\":%d}",a->domain(d),a->formation_class(d),boolean(a->trade(d)),boolean(a->spy(d)),a->combat(d),a->range(d));
    }
    fputs("],\"complete\":true}\n",out);return 0;
}
static int relations(FILE *out,const Api *a) {
    const void *relations=a->relations();if(!relations)return 244;
    int32_t players=signed_at(relations,0xc0);
    if(players<0||players>65)return 244;
    const void *vector=(const unsigned char *)relations+0x1a0;
    uint32_t count=a->war_count(vector);
    if(count!=(uint32_t)(players*(players-1)/2))return 244;
    fprintf(out,"{\"kind\":\"relations\",\"player_count\":%d,\"war_types\":[",players);
    for(uint32_t i=0;i<count;i++) {
        const void *entry=a->war_at(vector,i);if(!entry)return 244;
        fprintf(out,"%s%d",i?",":"",signed_at(entry,0));
    }
    fputs("],\"complete\":true}\n",out);return 0;
}
/* The native GetUnits returns a reference into Map.UnitContainer: either a
 * genuine cell vector or its owned empty vector at +8. No copied/synthetic
 * vector, hidden sret, allocation or destructor is involved. The Map owner is
 * retained by the runner; each borrowed vector is consumed synchronously. */
static int queries(FILE *expected,const Api *a,void *module,void *context,const void *map,const void *map_units,const void *member,unsigned kind,int32_t owner,uint32_t ordinal) {
    const Coord *location=kind?a->formation_location(member):a->unit_location(member);if(!location)return 245;
    Coord anchor=*location;uint32_t id=kind?a->formation_id(member):a->unit_id(member);
    bool valid=a->coord_valid(&anchor);int32_t layer=kind?a->formation_layer(member):a->unit_layer(member);
    for(int direction=-1;direction<6;direction++) {
        idle(a,module,context);
        const void *plot=valid?(direction<0?a->plot(map,&anchor):a->adjacent(map,&anchor,direction)):NULL;
        fprintf(expected,"{\"kind\":\"count_expected\",\"member_kind\":\"%s\",\"owner\":%d,\"ordinal\":%u,\"id\":%u,\"anchor\":[%d,%d],\"anchor_valid\":%s,\"layer\":%d,\"direction\":%d,\"plot_present\":%s",kind?"formation":"unit",owner,ordinal,id,anchor.x,anchor.y,boolean(valid),layer,direction,boolean(plot!=NULL));
        if(plot) {
            if(pointer_at(plot,0)!=a->base+0x3ae23a0)return 245;
            Coord coordinate;a->plot_coord(&coordinate,plot);
            const void *vector=a->occupancy(map_units,coordinate.x,coordinate.y,(uint32_t)layer);if(!vector)return 245;
            idle(a,module,context);
            uint32_t count=a->count[kind](member,vector,false);
            idle(a,module,context);
            fprintf(expected,",\"plot\":[%d,%d],\"bool_argument\":false,\"count\":%u",coordinate.x,coordinate.y,count);
        }
        fputs("}\n",expected);
    }
    return 0;
}
static int members(FILE *out,FILE *expected,const Api *a,void *module,void *context,const void *map,const void *map_units,const void *player,int32_t owner,unsigned kind) {
    const void *container=kind?a->formations(player):a->units(player);if(!container)return 246;
    /* Formations wrapper adds +8 before its RecycledIDContainer methods. Size
     * counts active entries, not capacity. Begin/End themselves only construct
     * local one-word iterators even though Formations omits const overloads. */
    uint32_t count=kind?a->formation_count((const unsigned char *)container+8):a->unit_count(container);
    if(count>65536)return 246;
    Iterator it=a->begin[kind](container),end=a->end[kind](container);
    fprintf(out,"{\"kind\":\"members\",\"member_kind\":\"%s\",\"owner\":%d,\"count\":%u,\"entries\":[",kind?"formation":"unit",owner,count);
    uint32_t n=0;unsigned char seen_slots[65536/8]={0};
    for(;it.entry!=end.entry;n++) {
        if(n>=count)return 246;
        idle(a,module,context);
        const void *member=a->deref[kind](&it);if(!member)return 246;
        uint32_t id=kind?a->formation_id(member):a->unit_id(member);
        int32_t actual_owner=kind?a->formation_owner(member):a->unit_owner(member);
        if(actual_owner!=owner||(kind?a->find_formation(container,id):a->find_unit(container,id))!=member)return 246;
        uint32_t slot=id&65535u;
        if(seen_slots[slot/8]&(1u<<(slot%8)))return 246;
        seen_slots[slot/8]|=(unsigned char)(1u<<(slot%8));
        fprintf(out,"%s{\"ordinal\":%u,\"id\":%u,\"slot\":%u,\"actual_owner\":%d}",n?",":"",n,id,slot,actual_owner);
        if(!kind) {
            /* Runtime binding is an expected invariant only, never input. */
            const void *definition=a->unit_definition(member),*formation=a->unit_formation(member);
            fprintf(expected,"{\"kind\":\"unit_binding_expected\",\"owner\":%d,\"ordinal\":%u,\"id\":%u,\"definition_index\":",owner,n,id);
            if(definition)fprintf(expected,"%u",a->definition_index(definition));else fputs("null",expected);
            fputs(",\"formation_id\":",expected);if(formation)fprintf(expected,"%u",a->formation_id(formation));else fputs("null",expected);
            fputs("}\n",expected);
        }
        int rc=queries(expected,a,module,context,map,map_units,member,kind,owner,n);if(rc)return rc;
        a->next[kind](&it);
    }
    if(n!=count)return 246;
    fputs("],\"complete\":true}\n",out);return 0;
}
static int players(FILE *out,FILE *expected,const Api *a,void *module,void *context,const void *manager,const void *map,const void *map_units) {
    if(a->mapping_size((const unsigned char *)manager+0x1a08)!=65)return 247;
    const int32_t *mapping=a->mapping(manager);if(!mapping)return 247;
    uintptr_t begin=pointer_at(manager,0x20),end=pointer_at(manager,0x28);
    if(!begin||end<begin||end-begin!=65*sizeof(void *))return 247;
    for(int32_t i=0;i<65;i++) {
        idle(a,module,context);
        bool valid=a->valid(manager,i);const void *player=a->player(manager,i);
        const void *stored=(const void *)pointer_at((const void *)begin,(size_t)i*sizeof(void *));
        if(player!=(valid?stored:NULL))return 247;
        fprintf(out,"{\"kind\":\"player\",\"query\":%d,\"mapped_index\":%d,\"valid\":%s,\"stored_nonnull\":%s,\"resolved\":%s",i,mapping[i],boolean(valid),boolean(stored!=NULL),boolean(player!=NULL));
        if(player) {
            int32_t owner=a->owner(player);const void *configuration=a->configuration(player);
            if(!valid||owner!=i||!configuration)return 247;
            fprintf(out,",\"actual_owner\":%d,\"slot_status\":%d,\"team\":%d,\"alive\":%s",owner,a->slot_status(configuration),a->team(configuration),boolean(a->alive(player)));
        }
        fputs("}\n",out);
        if(player)for(unsigned kind=0;kind<2;kind++) {
            int rc=members(out,expected,a,module,context,map,map_units,player,i,kind);if(rc)return rc;
        }
    }
    return 0;
}
int c6_linux_placement_metadata_probe(void *module,void *context) {
    const char *output=getenv("C6_PLACEMENT_METADATA_OUTPUT"),*expected_path=getenv("C6_PLACEMENT_METADATA_EXPECTED_OUTPUT"),*profile=getenv("C6_PLACEMENT_METADATA_PROFILE_SHA256");
    const char *paths[4]={getenv("C6_PLACEMENT_METADATA_SNAPSHOT_BEFORE"),getenv("C6_PLACEMENT_METADATA_SNAPSHOT_CONTROL"),getenv("C6_PLACEMENT_METADATA_SNAPSHOT_AFTER"),getenv("C6_PLACEMENT_METADATA_SNAPSHOT_REPEAT")};
    if(!module||!context||!output||!expected_path||!strcmp(output,expected_path)||!profile||strlen(profile)!=64)return 242;
    for(unsigned i=0;i<64;i++)if(!((profile[i]>='0'&&profile[i]<='9')||(profile[i]>='a'&&profile[i]<='f')))return 242;
    for(unsigned i=0;i<4;i++)if(!paths[i]||!strcmp(output,paths[i])||!strcmp(expected_path,paths[i]))return 242;
    for(unsigned i=0;i<4;i++)for(unsigned j=0;j<i;j++)if(!strcmp(paths[i],paths[j]))return 242;
    Api a={0};int rc=bind_api(module,&a);if(rc)return rc;
    idle(&a,module,context);
    const void *game=a.game(),*map=a.map();
    if(!game||!map||a.current_game()!=game||a.current_map()!=map||a.game_map(game)!=map)return 248;
    if(pointer_at(map,0)!=a.base+0x3afb160)return 248;
    const void *manager=a.manager(game),*map_units=a.map_units(map),*parameters=a.parameters();
    if(!manager||!map_units||!parameters||!a.initialized(manager))return 248;
    const void *sync=a.sync_random(game);if(!sync)return 248;
    unsigned char rng_before[64];memcpy(rng_before,sync,32);memcpy(rng_before+32,a.map_random,32);
    for(unsigned i=0;i<2;i++) {
        idle(&a,module,context);rc=c6_linux_context_save(module,context,paths[i]);idle(&a,module,context);if(rc)return rc;
    }
    FILE *out=fopen(output,"wx");if(!out)return 249;
    FILE *expected=fopen(expected_path,"wx");if(!expected){fclose(out);return 249;}
    fprintf(out,"{\"kind\":\"header\",\"schema\":\"c6-linux-placement-metadata-native-v1\",\"game_sha256\":\"a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa\",\"profile_sha256\":\"%s\",\"player_slots\":65,\"map_width\":%d,\"map_height\":%d,\"plot_unit_limit\":%d,\"complete_allocator_state\":false,\"complete_state_actions_turns\":false}\n",profile,a.width(map),a.height(map),a.plot_limit(parameters));
    fprintf(expected,"{\"kind\":\"header\",\"schema\":\"c6-linux-placement-count-expected-v1\",\"profile_sha256\":\"%s\",\"answers_are_inputs\":false,\"native_handler_query_available\":false}\n",profile);
    rc=registry(out,&a);idle(&a,module,context);
    if(!rc)rc=relations(out,&a);
    if(!rc)rc=players(out,expected,&a,module,context,manager,map,map_units);
    idle(&a,module,context);
    for(unsigned i=2;i<4&&!rc;i++){rc=c6_linux_context_save(module,context,paths[i]);idle(&a,module,context);}
    bool rng_equal=!memcmp(rng_before,sync,32)&&!memcmp(rng_before+32,a.map_random,32);
    if(!rng_equal&&!rc)rc=250;
    fprintf(out,"{\"kind\":\"summary\",\"status\":%d,\"known_rng_equal\":%s,\"complete_rng_inventory\":false,\"rng_before\":",rc,boolean(rng_equal));hex(out,rng_before,64);
    fputs(",\"sync_rng_after\":",out);hex(out,sync,32);fputs(",\"map_rng_after\":",out);hex(out,a.map_random,32);fputs("}\n",out);
    fprintf(expected,"{\"kind\":\"summary\",\"status\":%d}\n",rc);
    if((ferror(out)||ferror(expected))&&!rc)rc=249;
    if(fclose(expected)&&!rc)rc=249;
    if(fclose(out)&&!rc)rc=249;
    return rc;
}
