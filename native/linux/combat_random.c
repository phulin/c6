#define _GNU_SOURCE
#include "combat_random.h"
#include "context_stream.h"
#include <stdbool.h>

/* Reuse our accepted immutable average-fixture support and binding tables.
 * Its entrypoint is not called by this profile. The runner must compile this
 * translation unit alone, never also combat_kernel.c, and hash both sources. */
#include "combat_kernel.c"

#define MAX_VARIABLES 4096
#define MAX_CASES 2048

typedef struct {
    unsigned char random[32], wrapper[16], owner[152];
    unsigned char node_bytes[MAX_VARIABLES][40];
    uintptr_t keys[MAX_VARIABLES], nodes[MAX_VARIABLES];
    size_t ranks[MAX_VARIABLES], count;
} RandomSnapshot;

typedef struct {
    void *module, *context, *game, *owner, *wrapper;
    uintptr_t base, registrations[MAX_VARIABLES], sorted[MAX_VARIABLES];
    size_t registered, sync_id, sync_rank;
    void (*construct)(void *);
    int (*calculate)(void *,const void *);
    int (*rounding)(void);
    int extra;
} RandomEnvironment;

static uintptr_t pointer_at(const void *p, size_t offset) {
    uintptr_t value; memcpy(&value,(const unsigned char *)p+offset,8); return value;
}
static uint32_t word_at(const void *p, size_t offset) {
    uint32_t value; memcpy(&value,(const unsigned char *)p+offset,4); return value;
}
static int compare_pointer(const void *a,const void *b) {
    uintptr_t x=*(const uintptr_t *)a,y=*(const uintptr_t *)b;
    return (x>y)-(x<y);
}
static size_t rank_of(const RandomEnvironment *e,uintptr_t key) {
    size_t lo=0,hi=e->registered;
    while(lo<hi){size_t mid=lo+(hi-lo)/2;if(e->sorted[mid]<key)lo=mid+1;else hi=mid;}
    return lo<e->registered&&e->sorted[lo]==key?lo:MAX_VARIABLES;
}
static void hex_bytes(FILE *out,const void *p,size_t n) {
    const unsigned char *b=p; fputc('"',out);
    for(size_t i=0;i<n;i++)fprintf(out,"%02x",b[i]);
    fputc('"',out);
}

static int snapshot(const RandomEnvironment *e,RandomSnapshot *s) {
    memcpy(s->random,(unsigned char *)e->game+248,32);
    memcpy(s->wrapper,e->wrapper,16); memcpy(s->owner,e->owner,152);
    if(pointer_at(e->wrapper,8)!=(uintptr_t)e->owner)return 230;
    uintptr_t begin=pointer_at(e->owner,8),end=pointer_at(e->owner,16);
    if(end<begin||(end-begin)/8!=e->registered||(end-begin)%8)return 231;
    if(memcmp((void *)begin,e->registrations,e->registered*8))return 231;
    size_t expected=pointer_at(e->owner,72);
    if(expected>e->registered)return 232;
    uintptr_t stack[MAX_VARIABLES],node=pointer_at(e->owner,64),first=0;
    size_t depth=0,pushes=0;s->count=0;
    const uintptr_t sentinel=(uintptr_t)e->owner+64;
    if(node&&pointer_at((void *)node,16)!=sentinel)return 232;
    while(node||depth){
        while(node){
            if(++pushes>expected||depth>=MAX_VARIABLES||(node&7))return 232;
            stack[depth++]=node;
            uintptr_t left=pointer_at((void *)node,0);
            if(left&&pointer_at((void *)left,16)!=node)return 232;
            node=left;
        }
        node=stack[--depth];if(!first)first=node;
        if(s->count>=expected)return 232;
        uintptr_t key=pointer_at((void *)node,32);
        size_t rank=rank_of(e,key);
        if(rank==MAX_VARIABLES||(s->count&&s->keys[s->count-1]>=key))return 233;
        s->keys[s->count]=key;s->ranks[s->count]=rank;s->nodes[s->count]=node;
        memcpy(s->node_bytes[s->count++],(void *)node,40);
        uintptr_t right=pointer_at((void *)node,8);
        if(right&&pointer_at((void *)right,16)!=node)return 232;
        node=right;
    }
    if(s->count!=expected||pointer_at(e->owner,56)!=(first?first:sentinel))return 232;
    return 0;
}

static void print_state(FILE *out,const RandomSnapshot *s) {
    fprintf(out,"{\"seed\":%u,\"calls\":%u,\"resets\":%u,\"locks\":%u,\"synchronous_byte\":%u,\"dirty_ranks\":[",
        word_at(s->random,8),word_at(s->random,12),word_at(s->random,16),word_at(s->random,20),s->random[24]);
    for(size_t i=0;i<s->count;i++)fprintf(out,"%s%zu",i?",":"",s->ranks[i]);
    fprintf(out,"],\"random32\":");hex_bytes(out,s->random,32);
    fprintf(out,",\"wrapper16\":");hex_bytes(out,s->wrapper,16);
    fprintf(out,",\"archive152\":");hex_bytes(out,s->owner,152);
    fprintf(out,",\"tree_nodes\":[");
    for(size_t i=0;i<s->count;i++){
        fprintf(out,"%s{\"address\":\"0x%lx\",\"bytes40\":",i?",":"",(unsigned long)s->nodes[i]);
        hex_bytes(out,s->node_bytes[i],40);fputc('}',out);
    }
    fprintf(out,"]}");
}

static int verify_average_dependencies(RandomEnvironment *e) {
    e->construct=required_symbol(e->module,"_ZN8GameCore5Rules6Combat8Instance16DamageParametersC2Ev","xp2");
    e->calculate=required_symbol(e->module,"_ZN8GameCore5Rules6Combat8Instance15CalculateDamageERKNS2_16DamageParametersE","xp2");
    void *(*parameters)(void)=required_symbol(e->module,"_ZN8GameCore7Context7Globals13GetParametersEv","xp2");
    void *(*current_game)(void)=required_symbol(e->module,"_ZN8GameCore7Context7Globals7GetGameEv","xp2");
    int (*max_extra)(const void *)=required_symbol(e->module,"_ZNK8GameCore4Game16GlobalParameters23COMBAT_MAX_EXTRA_DAMAGEEv","xp2");
    e->rounding=required_symbol(RTLD_DEFAULT,"fegetround","host");
    Dl_info module;
    if(!e->construct||!e->calculate||!parameters||!current_game||!max_extra||!e->rounding
       ||!dladdr(e->calculate,&module)||!module.dli_fbase)return 234;
    e->base=(uintptr_t)module.dli_fbase;
    if((uintptr_t)e->calculate!=e->base+0x2bd8980||(uintptr_t)e->construct!=e->base+0x2be20e0
       ||(uintptr_t)parameters!=e->base+0x1fe4cb0||(uintptr_t)current_game!=e->base+0x1fcc320
       ||(uintptr_t)max_extra!=e->base+0x2be20d0)return 234;
    for(size_t i=0;i<sizeof core_bindings/sizeof core_bindings[0];i++){
        uintptr_t expected=e->base+core_bindings[i].target;
        for(size_t j=0;j<sizeof host_weak_bindings/sizeof host_weak_bindings[0];j++)
            if(core_bindings[i].target==host_weak_bindings[j].target)expected=host_weak_bindings[j].host_va;
        if(*(uintptr_t *)(e->base+core_bindings[i].slot)!=expected)return 234;
    }
    /* Actual extra nonaverage dependencies. Tree insertion is a genuine host
     * weak-template binding, not a synthesized archive or replaced allocator. */
    const struct {uintptr_t slot,target;int host;} dependencies[]={
        {0x3bdf110,0x1fcc320,0},{0x3b1c648,0x1fe5240,0},
        {0x3bb4370,0x1fe7230,0},{0x3b60060,0x2d48d60,0},
        {0x3b7cdc8,0x3c4782e,1},{0x3bb80a0,0x1fe31d0,0},
        {0x3b17068,0x1fe4810,0},{0x3bcb128,0x2d48e60,0},
    };
    for(size_t i=0;i<sizeof dependencies/sizeof dependencies[0];i++){
        uintptr_t actual=*(uintptr_t *)(e->base+dependencies[i].slot);
        uintptr_t expected=dependencies[i].target+(dependencies[i].host?0:e->base);
        if(actual!=expected){
            Dl_info info={0};dladdr((void *)actual,&info);
            fprintf(stderr,"C6_LINUX_COMBAT_RANDOM unexpected_binding %zu actual 0x%lx module %s expected 0x%lx\n",
                i,(unsigned long)actual,info.dli_fname?info.dli_fname:"unknown",(unsigned long)expected);return 235;
        }
    }
    void *rules=parameters();e->game=current_game();if(!rules||!e->game)return 236;
    e->extra=max_extra(rules);if(e->extra!=12)return 237;
    e->wrapper=(unsigned char *)e->game+232;
    e->owner=(void *)pointer_at(e->wrapper,8);
    if(e->owner!=(unsigned char *)e->game+32||pointer_at(e->owner,144)!=(uintptr_t)e->game)return 238;
    uintptr_t begin=pointer_at(e->owner,8),end=pointer_at(e->owner,16),cap=pointer_at(e->owner,24);
    if(!begin||end<begin||cap<end||(end-begin)%8||(end-begin)/8>MAX_VARIABLES)return 238;
    e->registered=(end-begin)/8;memcpy(e->registrations,(void *)begin,end-begin);
    memcpy(e->sorted,e->registrations,end-begin);qsort(e->sorted,e->registered,8,compare_pointer);
    for(size_t i=0;i<e->registered;i++)if(!e->sorted[i]||(i&&e->sorted[i-1]>=e->sorted[i]))return 238;
    e->sync_id=MAX_VARIABLES;
    for(size_t i=0;i<e->registered;i++)if(e->registrations[i]==(uintptr_t)e->wrapper)e->sync_id=i;
    e->sync_rank=rank_of(e,(uintptr_t)e->wrapper);
    return e->sync_id==MAX_VARIABLES||e->sync_rank==MAX_VARIABLES?238:0;
}

static bool nonfaulting(const DamageParameters *p) {
    uint32_t raw=((uint32_t)p->outer_max_hp-(uint32_t)p->outer_damage)<<8;
    int32_t remaining;memcpy(&remaining,&raw,4);
    return !p->attacker||!p->defender||p->defense_type!=0x5e97d629||p->bypass_defenses
        ||remaining<=0||p->outer_max_hp<=0||((uint32_t)p->outer_max_hp<<8)!=0;
}

int c6_linux_combat_random_probe(void *module,void *context) {
    const char *input=getenv("C6_COMBAT_RANDOM_INPUT"),*output=getenv("C6_COMBAT_RANDOM_OUTPUT");
    const char *control=getenv("C6_COMBAT_RANDOM_CONTROL_SAVE"),*post=getenv("C6_COMBAT_RANDOM_POST_SAVE");
    if(!input||!output||!control||!post||!context)return 239;
    RandomEnvironment e={.module=module,.context=context};int rc=verify_average_dependencies(&e);if(rc)return rc;
    FILE *in=fopen(input,"r"),*out=NULL;if(!in)return 240;
    DamageParameters *cases=calloc(MAX_CASES,sizeof *cases);if(!cases){fclose(in);return 240;}
    size_t count=0,executed=0;
    for(;;){
        DamageParameters pending;memset(&pending,0xa5,sizeof pending);
        DamageParameters *p=&pending;e.construct(p);unsigned bypass;
        int n=fscanf(in,"%d %d %d %d %d %u %d %d %d %d %d %u",
            &p->attacker,&p->defender,&p->defender_max_hp,&p->outer_damage,&p->outer_max_hp,
            &p->defense_type,&p->power_scaling,&p->minimum_damage,&p->base_damage,
            &p->power_dampener,&p->max_extra_damage,&bypass);
        if(n==EOF)break;
        if(n!=12||bypass>1||count>=MAX_CASES){rc=241;break;}
        p->use_average=0;p->bypass_defenses=(uint8_t)bypass;
        if(!nonfaulting(p)){fprintf(stderr,"C6_LINUX_COMBAT_RANDOM refused_faulting_corpus_before_draw %zu\n",count);rc=242;break;}
        cases[count++]=pending;
    }
    if(ferror(in))rc=240;
    fclose(in);
    if(!rc&&!count)rc=241;
    if(!rc){out=fopen(output,"wx");if(!out)rc=240;}
    RandomSnapshot *before=malloc(sizeof *before),*after=malloc(sizeof *after);
    if(!before||!after)rc=240;
    if(!rc)rc=snapshot(&e,before);
    if(!rc){
        Dl_info math={0},floor_owner={0};void *exp=*(void **)(e.base+0x3bed160),*floor=*(void **)(e.base+0x3bb4398);
        if(!dladdr(exp,&math)||!math.dli_fname||!dladdr(floor,&floor_owner)||floor_owner.dli_fbase!=math.dli_fbase)rc=243;
        else{
            fprintf(out,"{\"kind\":\"profile\",\"global_extra_damage\":%d,\"libm_path\":\"%s\",\"expf_offset\":%lu,\"floorf_offset\":%lu,\"registered_count\":%zu,\"sync_id\":%zu,\"sync_rank\":%zu,\"pointer_mapping\":[",
                e.extra,math.dli_fname,(unsigned long)((uintptr_t)exp-(uintptr_t)math.dli_fbase),
                (unsigned long)((uintptr_t)floor-(uintptr_t)math.dli_fbase),e.registered,e.sync_id,e.sync_rank);
            for(size_t i=0;i<e.registered;i++)fprintf(out,"%s{\"id\":%zu,\"rank\":%zu,\"pointer\":\"0x%lx\"}",
                i?",":"",i,rank_of(&e,e.registrations[i]),(unsigned long)e.registrations[i]);
            fprintf(out,"]}\n");floating_environment(out,"floating_environment_before",e.rounding);
            fprintf(out,"{\"kind\":\"initial\",\"state\":");print_state(out,before);fprintf(out,"}\n");
        }
    }
    if(!rc)rc=c6_linux_context_save(module,context,control);
    if(!rc)rc=snapshot(&e,after);
    if(!rc&&(memcmp(before->random,after->random,32)||memcmp(before->wrapper,after->wrapper,16)
       ||memcmp(before->owner,after->owner,152)||before->count!=after->count
       ||memcmp(before->keys,after->keys,before->count*8)))rc=244;
    for(size_t i=0;!rc&&i<count;i++){
        rc=snapshot(&e,before);if(rc)break;
        DamageParameters immutable=cases[i];int damage=e.calculate(NULL,&cases[i]);executed++;
        rc=snapshot(&e,after);if(rc)break;
        if(memcmp(&immutable,&cases[i],sizeof immutable)||memcmp(before->wrapper,after->wrapper,16)
           ||memcmp(before->random,after->random,8)||memcmp(before->random+16,after->random+16,16))rc=245;
        for(size_t b=0;b<152;b++)if((b<56||b>=80)&&before->owner[b]!=after->owner[b])rc=245;
        fprintf(out,"{\"kind\":\"case\",\"index\":%zu,\"damage\":%d,\"before\":",i,damage);
        print_state(out,before);fprintf(out,",\"after\":");print_state(out,after);fprintf(out,"}\n");
    }
    if(!rc)rc=c6_linux_context_save(module,context,post);
    if(!rc)rc=snapshot(&e,before);
    if(!rc&&(memcmp(before->random,after->random,32)||before->count!=after->count
       ||memcmp(before->keys,after->keys,before->count*8)||memcmp(before->wrapper,after->wrapper,16)
       ||memcmp(before->owner,after->owner,152)))rc=246;
    if(out){floating_environment(out,"floating_environment_after",e.rounding);if(ferror(out))rc=240;if(fclose(out))rc=240;}
    fprintf(stderr,"C6_LINUX_COMBAT_RANDOM calculations_returned %zu requested %zu result %d complete_rng_inventory 0 full_combat_parity 0\n",executed,count,rc);
    free(before);free(after);free(cases);return rc;
}
