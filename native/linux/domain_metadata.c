#define _GNU_SOURCE
#include "domain_metadata.h"
#include "context_stream.h"
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
typedef bool (*B1)(const void *);
/* Original const_iterator is a one-word, trivially returned value. Native
 * Begin/End construct it; -> and ++ operate on our local iterator only. */
typedef struct { const void *entry; } Iterator;
_Static_assert(sizeof(Iterator)==8,"original iterator size");
typedef struct { int32_t x,y; } Coord;
typedef struct {
    uintptr_t base;
    int (*idle)(void *,void *);
    P0 game,current_game,map,current_map;
    P1 game_map,manager,sync_random;
    B1 initialized;
    const int32_t *(*mapping)(const void *);
    U1 mapping_size;
    bool (*valid)(const void *,int32_t);
    const void *(*player)(const void *,int32_t);
    I1 player_id;
    P0 definitions[3],xp2[2];
    U1 count[3],hash[3],index[3];
    PI at[3],find[3],find_xp2[2];
    B1 internal,canal,canal_wonder,bridge,terrain_water;
    P1 members[2];
    U1 member_count[2],member_id[2];
    I1 member_owner[2];
    Iterator (*begin[2])(const void *),(*end[2])(const void *);
    const void *(*deref[2])(const Iterator *);
    Iterator *(*next[2])(Iterator *);
    PI find_member[2];
    I1 width,height;
    const void *(*plot)(const void *,const Coord *);
    P1 attributes;
    bool (*coastal)(const void *,int32_t),(*domain)(const void *,int32_t);
    B1 wonder,has_attributes;
    const void *map_random;
} Api;

static int bind_api(void *module,Api *a) {
    void *anchor=dlsym(module,"_ZN8GameCore4Game11GetInstanceEv");
    Dl_info info={0};
    if(!anchor||!dladdr(anchor,&info)||!info.dli_fbase)return 221;
    uintptr_t base=(uintptr_t)info.dli_fbase;
    if((uintptr_t)anchor!=base+0x2660650)return 221;
    a->base=base;
#define BIND(F,R) do { a->F=(void *)(base+(R)); } while(0)
    BIND(game,0x2660650); BIND(current_game,0x1fcc320);
    BIND(map,0x290b8a0); BIND(current_map,0x2a474d0); BIND(game_map,0x1fe2220);
    BIND(manager,0x20e0670); BIND(initialized,0x2b51830); BIND(sync_random,0x2d7b4d0);
    BIND(mapping,0x2004680); BIND(mapping_size,0x27e4520);
    BIND(valid,0x2004fc0); BIND(player,0x2003940); BIND(player_id,0x1fcc140);
    BIND(definitions[0],0x2213a70); BIND(definitions[1],0x2213730);
    BIND(xp2[0],0x2213ae0); BIND(xp2[1],0x22137a0);
    BIND(count[0],0x1fe2250); BIND(count[1],0x1fe2260);
    BIND(at[0],0x1fe2b00); BIND(at[1],0x1fe4ad0);
    BIND(hash[0],0x21d56f0); BIND(hash[1],0x20a1260);
    BIND(definitions[2],0x2214730); BIND(count[2],0x2194f80);
    BIND(at[2],0x2194cb0); BIND(hash[2],0x2bf69d0);
    BIND(index[0],0x1fcccf0); BIND(index[1],0x1fccc60); BIND(index[2],0x20e3240);
    BIND(find[0],0x1ffbc00); BIND(find[1],0x1ffb960); BIND(find[2],0x20f36d0);
    BIND(terrain_water,0x2bcadc0);
    BIND(find_xp2[0],0x21d5680); BIND(find_xp2[1],0x20a11f0);
    BIND(internal,0x1fe2500); BIND(canal,0x21d5a40);
    BIND(canal_wonder,0x2a3e3f0); BIND(bridge,0x28696c0);
    BIND(members[0],0x1fccc20); BIND(members[1],0x1fcccd0);
    BIND(member_count[0],0x1fcd080); BIND(member_count[1],0x2428160);
    BIND(begin[0],0x1fcd090); BIND(end[0],0x1fcd0b0);
    BIND(begin[1],0x21433b0); BIND(end[1],0x21433c0);
    BIND(deref[0],0x1fcd0c0); BIND(deref[1],0x2ae46e0);
    BIND(next[0],0x1fcd0e0); BIND(next[1],0x20a1280);
    BIND(member_id[0],0x1fe2720); BIND(member_id[1],0x21a36c0);
    BIND(member_owner[0],0x1fe2290); BIND(member_owner[1],0x291e670);
    BIND(find_member[0],0x2a597e0); BIND(find_member[1],0x2ae12e0);
    BIND(width,0x2004f20); BIND(height,0x21436d0); BIND(plot,0x2a48ba0);
    BIND(attributes,0x2bca780); BIND(has_attributes,0x2bca7b0); BIND(coastal,0x2bc9710);
    BIND(wonder,0x2bc8100); BIND(domain,0x2bc8350);
#undef BIND
    a->map_random=dlsym(module,"_ZN8GameCore3Map9Generator10ms_kRandomE");
    a->idle=dlsym(RTLD_DEFAULT,"c6_linux_domain_metadata_quiescent");
    return !a->idle||(uintptr_t)a->map_random!=base+0x3c072f0?221:0;
}
static const char *boolean(bool v) { return v?"true":"false"; }
static void require_idle(const Api *a,void *module,void *context) {
    int rc=a->idle(module,context);
    if(rc){fprintf(stderr,"C6_LINUX_DOMAIN_METADATA quiescence_unproven %d no_post_state_or_cleanup\n",rc);fflush(stderr);_exit(240);}
}
static uintptr_t pointer_at(const void *p,size_t offset) {
    uintptr_t value;memcpy(&value,(const unsigned char *)p+offset,sizeof value);return value;
}
static void hex(FILE *out,const void *p,size_t size) {
    const unsigned char *bytes=p;fputc('"',out);
    for(size_t i=0;i<size;i++)fprintf(out,"%02x",bytes[i]);
    fputc('"',out);
}
static int registries(FILE *out,const Api *a) {
    const char *names[3]={"district","building","terrain"};
    for(unsigned k=0;k<3;k++) {
        const void *base=a->definitions[k](),*xp2=k<2?a->xp2[k]():NULL;
        /* Null collection is unsupported; only an actual Find null is absence. */
        if(!base||(k<2&&!xp2))return 223;
        uint32_t count=a->count[k](base);if(count>65536)return 223;
        fprintf(out,"{\"kind\":\"registry\",\"name\":\"%s\",\"count\":%u,\"complete\":true,\"entries\":[",names[k],count);
        for(uint32_t i=0;i<count;i++) {
            const void *definition=a->at[k](base,i);if(!definition)return 223;
            uint32_t hash=a->hash[k](definition);
            const void *by_hash=a->find[k](base,hash),*by_query=a->find[k](base,i);
            fprintf(out,"%s{\"query\":%u,\"index\":%u,\"hash\":%u,\"find_index\":",i?",":"",i,a->index[k](definition),hash);
            if(by_hash)fprintf(out,"%u",a->index[k](by_hash));else fputs("null",out);
            fputs(",\"direct_index\":",out);
            if(by_query)fprintf(out,"%u",a->index[k](by_query));else fputs("null",out);
            if(k==2){fprintf(out,",\"is_water\":%s}",boolean(a->terrain_water(definition)));continue;}
            const void *extension=a->find_xp2[k](xp2,hash);
            if(!k)fprintf(out,",\"internal_only\":%s",boolean(a->internal(definition)));
            fprintf(out,",\"xp2\":{\"status\":\"%s\"",extension?"present":"absent");
            if(extension) {
                if(k)fprintf(out,",\"canal_wonder\":%s,\"bridge\":%s",boolean(a->canal_wonder(extension)),boolean(a->bridge(extension)));
                else fprintf(out,",\"is_canal\":%s",boolean(a->canal(extension)));
            }
            fputs("}}",out);
        }
        fputs("]}\n",out);
    }
    return 0;
}
static int membership(FILE *out,const Api *a,const void *player,unsigned kind) {
    const void *container=a->members[kind](player);if(!container)return 224;
    uint32_t count=a->member_count[kind](container);if(count>65536)return 224;
    Iterator it=a->begin[kind](container),end=a->end[kind](container);
    fprintf(out,"{\"count\":%u,\"entries\":[",count);
    uint32_t n=0;
    for(;it.entry!=end.entry;n++) {
        if(n>=count)return 224;
        const void *member=a->deref[kind](&it);if(!member)return 224;
        if(!kind&&pointer_at(member,0)!=a->base+0x3abf460)return 224;
        uint32_t id=a->member_id[kind](member);
        if(a->find_member[kind](container,id)!=member)return 224;
        int32_t owner=a->member_owner[kind](member);
        fprintf(out,"%s{\"ordinal\":%u,\"id\":%u,\"slot\":%u,\"actual_owner\":%d}",n?",":"",n,id,id&65535u,owner);
        a->next[kind](&it);
    }
    if(n!=count)return 224;
    fputs("],\"complete\":true}",out);return 0;
}
static int players(FILE *out,const Api *a,void *module,void *context,const void *manager) {
    /* The original Manager.Initialize creates exactly 65 mapping and player
     * slots. Prove current live extents before unchecked native mapping reads. */
    const void *vector=(const unsigned char *)manager+0x1a08;
    if(a->mapping_size(vector)!=65)return 224;
    const int32_t *mapping=a->mapping(manager);if(!mapping)return 224;
    uintptr_t begin=pointer_at(manager,0x20),end=pointer_at(manager,0x28);
    if(!begin||end<begin||end-begin!=65*sizeof(void *))return 224;
    for(int32_t i=0;i<65;i++) {
        require_idle(a,module,context);
        bool valid=a->valid(manager,i);const void *player=a->player(manager,i);
        fprintf(out,"{\"kind\":\"player\",\"query\":%d,\"mapped_index\":%d,\"is_valid\":%s,\"resolved\":%s",i,mapping[i],boolean(valid),boolean(player!=NULL));
        if(player) {
            if(!valid)return 224;
            fprintf(out,",\"actual_owner\":%d,\"cities\":",a->player_id(player));
            int rc=membership(out,a,player,0);if(rc)return rc;
            fputs(",\"districts\":",out);rc=membership(out,a,player,1);if(rc)return rc;
        }
        fputs("}\n",out);
    }
    return 0;
}
static int plots(FILE *out,const Api *a,void *module,void *context,const void *map,uint32_t *count) {
    int32_t width=a->width(map),height=a->height(map);
    if(width<=0||height<=0||(uint64_t)width*(uint64_t)height>1000000)return 225;
    for(int32_t y=0;y<height;y++)for(int32_t x=0;x<width;x++) {
        require_idle(a,module,context);
        Coord coord={x,y};const void *plot=a->plot(map,&coord);if(!plot)return 225;
        uintptr_t vptr=pointer_at(plot,0);
        if(vptr!=a->base+0x3ae23a0||pointer_at((const void *)vptr,0x38)!=a->base+0x2bca780
           ||pointer_at((const void *)vptr,0x48)!=a->base+0x2bca7b0)return 225;
        bool has_attributes=a->has_attributes(plot);
        const void *attributes=a->attributes(plot);
        if(!attributes||pointer_at(attributes,0)!=a->base+0x3ae2360)return 225;
        /* Signed i16 at +0x14 is proven by original IsDomainValid(DomainMask).
         * This answer is expected-only, including fallback slot zero attributes. */
        int16_t valid_domains_mask;memcpy(&valid_domains_mask,(const unsigned char *)attributes+0x14,2);
        bool coastal=a->coastal(plot,1),wonder=a->wonder(attributes);
        bool domains[3];for(int32_t d=0;d<3;d++)domains[d]=a->domain(attributes,d);
        fprintf(out,"{\"kind\":\"plot_expected\",\"x\":%d,\"y\":%d,\"has_city_attributes\":%s,\"valid_domains_mask\":%d,\"coastal_min_one\":%s,\"wonder_complete\":%s,\"valid_domains\":[%s,%s,%s]}\n",x,y,boolean(has_attributes),(int)valid_domains_mask,boolean(coastal),boolean(wonder),boolean(domains[0]),boolean(domains[1]),boolean(domains[2]));
        (*count)++;
    }
    return 0;
}
int c6_linux_domain_metadata_probe(void *module,void *context) {
    const char *output=getenv("C6_DOMAIN_METADATA_OUTPUT"),*profile=getenv("C6_DOMAIN_METADATA_PROFILE_SHA256");
    const char *paths[4]={getenv("C6_DOMAIN_METADATA_SNAPSHOT_BEFORE"),getenv("C6_DOMAIN_METADATA_SNAPSHOT_CONTROL"),getenv("C6_DOMAIN_METADATA_SNAPSHOT_AFTER"),getenv("C6_DOMAIN_METADATA_SNAPSHOT_REPEAT")};
    if(!module||!context||!output||!profile||strlen(profile)!=64)return 222;
    for(unsigned i=0;i<64;i++)if(!((profile[i]>='0'&&profile[i]<='9')||(profile[i]>='a'&&profile[i]<='f')))return 222;
    for(unsigned i=0;i<4;i++)if(!paths[i]||!strcmp(output,paths[i]))return 222;
    for(unsigned i=0;i<4;i++)for(unsigned j=0;j<i;j++)if(!strcmp(paths[i],paths[j]))return 222;
    Api a={0};int rc=bind_api(module,&a);if(rc)return rc;
    require_idle(&a,module,context);
    const void *game=a.game(),*map=a.map();
    if(!game||!map||a.current_game()!=game||a.current_map()!=map||a.game_map(game)!=map)return 226;
    uintptr_t map_vptr=pointer_at(map,0);
    if(map_vptr!=a.base+0x3afb160||pointer_at((const void *)map_vptr,0x78)!=a.base+0x2a48ba0)return 226;
    const void *manager=a.manager(game);if(!manager||!a.initialized(manager))return 226;
    const void *sync=a.sync_random(game);if(!sync)return 226;
    unsigned char rng_before[64];memcpy(rng_before,sync,32);memcpy(rng_before+32,a.map_random,32);
    for(unsigned i=0;i<2;i++) {
        require_idle(&a,module,context);rc=c6_linux_context_save(module,context,paths[i]);
        require_idle(&a,module,context);if(rc)return rc;
    }
    FILE *out=fopen(output,"wx");if(!out)return 227;
    fprintf(out,"{\"kind\":\"header\",\"schema\":\"c6-linux-domain-metadata-native-v1\",\"game_sha256\":\"a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa\",\"host_sha256\":\"106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4\",\"profile_sha256\":\"%s\",\"player_slots\":65,\"map_width\":%d,\"map_height\":%d,\"expected_outputs_are_inputs\":false,\"complete_state_actions_turns\":false,\"complete_rng_inventory\":false}\n",profile,a.width(map),a.height(map));
    rc=registries(out,&a);require_idle(&a,module,context);
    if(!rc)rc=players(out,&a,module,context,manager);
    uint32_t plot_count=0;if(!rc)rc=plots(out,&a,module,context,map,&plot_count);
    require_idle(&a,module,context);
    for(unsigned i=2;i<4&&!rc;i++) {
        rc=c6_linux_context_save(module,context,paths[i]);require_idle(&a,module,context);
    }
    bool rng_equal=!memcmp(rng_before,sync,32)&&!memcmp(rng_before+32,a.map_random,32);
    if(!rng_equal&&!rc)rc=228;
    fprintf(out,"{\"kind\":\"summary\",\"status\":%d,\"plots\":%u,\"known_rng_equal\":%s,\"rng_before\":",rc,plot_count,boolean(rng_equal));
    hex(out,rng_before,64);fputs(",\"sync_rng_after\":",out);hex(out,sync,32);fputs(",\"map_rng_after\":",out);hex(out,a.map_random,32);fputs("}\n",out);
    if(ferror(out)&&!rc)rc=227;
    if(fclose(out)&&!rc)rc=227;
    return rc;
}
