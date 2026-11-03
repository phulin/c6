#define _GNU_SOURCE
#include <assert.h>
#include <stdlib.h>
#include <sys/mman.h>
static void *review_dlsym(void *,const char *);
#include <dlfcn.h>
static int review_dladdr(const void *,Dl_info *);
#define dlsym review_dlsym
#define dladdr review_dladdr
#include "../../native/linux/unit_movement_query_fixture.c"
#undef dlsym
#undef dladdr

/* Independent collector test. All executable stubs jump into the C mocks below;
 * no original image is loaded or called. Exact source is retained by the audit. */
enum { TOTAL=1262, STRIDE=0x850 };
static uintptr_t core;
static unsigned char manager_data[0x1b00],game_data[0x300],dip_data[0x2100],relations_data[0x230];
static unsigned char units_data[TOTAL][STRIDE],definition_data[16],plot_data[16],plot_vtable[0x30];
static uintptr_t slots[65];
static int32_t index_map[65];
static int players[2],parameters_data,context_data,module_data;
typedef struct {unsigned first,count;} Container;
static Container containers[2]={{0,TOTAL-1},{TOTAL-1,1}};
static unsigned target_step,guards,definitions_seen;
static const char *scenario;
static FILE *events;
static void write_ptr(void *p,uintptr_t value){memcpy(p,&value,8);}
static void write_i32(void *p,int32_t value){memcpy(p,&value,4);}
static unsigned unit_index(const void *p){
 uintptr_t start=(uintptr_t)units_data,at=(uintptr_t)p;
 assert(at>=start&&at<start+sizeof units_data&&(at-start)%STRIDE==0);
 return (unsigned)((at-start)/STRIDE);
}
static int is_case(const char *name){return !strcmp(scenario,name);}
static int retained(void *m,void *c,const void *g){
 assert(m==&module_data&&c==&context_data&&g==game_data);guards++;
 if(is_case("guard-between-targets")&&target_step==1){fprintf(events,"fatal_guard %u\n",target_step);fflush(events);return 1;}
 return 0;
}
static const void *manager_mock(const void *g){assert(g==game_data);return manager_data;}
static bool initialized_mock(const void *m){assert(m==manager_data);return true;}
static const int32_t *mapping_mock(const void *m){assert(m==manager_data);return index_map;}
static uint64_t mapping_size_mock(const void *p){assert(p==manager_data+0x1a08);return 65;}
static bool valid_mock(const void *m,int32_t owner){assert(m==manager_data);return owner==0||owner==64;}
static const void *player_mock(const void *m,int32_t owner){assert(m==manager_data);return owner==0?&players[0]:owner==64?&players[1]:NULL;}
static int32_t owner_mock(const void *p){assert(p==&players[0]||p==&players[1]);return p==&players[0]?0:64;}
static const void *container_mock(const void *p){return &containers[p==&players[0]?0:1];}
static const void *same_mock(const void *p){assert(p);return p;}
static const void *dip_mock(const void *p){assert(p);return dip_data;}
static int32_t zero_mock(const void *p){assert(p);return 0;}
static bool true_mock(const void *p){assert(p);return true;}
static Iterator begin_mock(const void *p){const Container *c=p;return (Iterator){units_data[c->first]};}
static Iterator end_mock(const void *p){const Container *c=p;return (Iterator){(unsigned char *)units_data+(c->first+c->count)*STRIDE};}
static const void *deref_mock(const Iterator *it){assert(it);return it->entry;}
static Iterator *next_mock(Iterator *it){it->entry=(const unsigned char *)it->entry+STRIDE;return it;}
static uint32_t count_mock(const void *p){return ((const Container *)p)->count;}
static uint32_t id_mock(const void *u){return 0xabcd0000u+unit_index(u);}
static int32_t unit_owner_mock(const void *u){return unit_index(u)==TOTAL-1?64:0;}
static const void *find_mock(const void *p,uint32_t id){
 const Container *c=p;unsigned i=id-0xabcd0000u;
 if(is_case("wrong-full-id")&&i==0)return NULL;
 return i>=c->first&&i<c->first+c->count?units_data[i]:NULL;
}
static const void *definition_mock(const void *u){definitions_seen++;return unit_index(u)==TOTAL-1?NULL:definition_data;}
static uint32_t def_index_mock(const void *p){assert(p==definition_data);return 17;}
static uint32_t def_hash_mock(const void *p){assert(p==definition_data);return 0x12345678;}
static const void *plot_mock(const void *u){return unit_index(u)%2?NULL:plot_data;}
static const void *parameters_mock(void){return &parameters_data;}
static const void *relations_mock(void){return relations_data;}
static int32_t max_mock(const void *u){
 unsigned i=unit_index(u);assert(target_step==i*2);
 assert(definitions_seen==(is_case("count-mismatch")?TOTAL-1:TOTAL));target_step++;
 fprintf(events,"max %u %08x\n",i,id_mock(u));fflush(events);
 /* Demonstrate preservation, rather than a query-effect equality veto. */
 game_data[0xf8]=(unsigned char)i;*(unsigned char *)(core+0x3c072f0)=(unsigned char)(i+1);
 return (int32_t)i-7;
}
static bool moved_mock(const void *u){
 unsigned i=unit_index(u);assert(target_step==i*2+1);target_step++;
 fprintf(events,"moved %u %08x\n",i,id_mock(u));fflush(events);return i%2!=0;
}
static void *review_dlsym(void *m,const char *name){
 assert(m==&module_data&&!strcmp(name,"_ZNK8GameCore4Unit8Instance11GetMaxMovesEv"));return (void *)(core+0x2c27180);
}
static int review_dladdr(const void *p,Dl_info *info){
 assert((uintptr_t)p>=core&&(uintptr_t)p<core+0x4000000);memset(info,0,sizeof *info);info->dli_fbase=(void *)core;return 1;
}
static void stub(uintptr_t offset,void *function){
 unsigned char code[]={0x48,0xb8,0,0,0,0,0,0,0,0,0xff,0xe0};
 uintptr_t target=(uintptr_t)function;memcpy(code+2,&target,8);memcpy((void *)(core+offset),code,sizeof code);
}
int main(int argc,char **argv){
 assert(argc==2);scenario=argv[1];events=fopen("events","wx");assert(events);
 void *mapped=mmap(NULL,0x4000000,PROT_READ|PROT_WRITE|PROT_EXEC,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(mapped!=MAP_FAILED);core=(uintptr_t)mapped;
#define S(field,offset,function) do { Api typed={.field=(function)}; (void)typed; stub(offset,(void *)(function)); } while(0)
 S(manager,0x20e0670,manager_mock);S(initialized,0x2b51830,initialized_mock);S(mapping,0x2004680,mapping_mock);S(mapping_size,0x27e4520,mapping_size_mock);
 S(valid,0x2004fc0,valid_mock);S(player,0x2003940,player_mock);S(owner,0x1fcc140,owner_mock);S(units,0x1fccc70,container_mock);
 S(configuration,0x2b315e0,same_mock);S(status,0x28ebe70,zero_mock);S(team,0x28ebce0,zero_mock);S(alive,0x2003cb0,true_mock);S(turn_active,0x2021030,true_mock);
 S(diplomacy,0x1fe5670,dip_mock);S(influence,0x208ec20,same_mock);S(suzerain,0x2b20950,zero_mock);S(embarked_bonus,0x2c2c380,zero_mock);
 S(begin,0x2021450,begin_mock);S(end,0x2021470,end_mock);S(deref,0x2021480,deref_mock);S(next,0x20214a0,next_mock);S(count,0x1fe4ee0,count_mock);
 S(id,0x2021490,id_mock);S(unit_owner,0x2004670,unit_owner_mock);S(find,0x2ba5580,find_mock);S(definition,0x20037c0,definition_mock);
 S(domain,0x1fe4f10,zero_mock);S(base_moves,0x20df1d0,zero_mock);S(definition_index,0x1fcccb0,def_index_mock);S(definition_hash,0x1fe52c0,def_hash_mock);
 S(plot,0x2c1c820,plot_mock);S(plot_owner,0x1fe23d0,zero_mock);S(plot_cost,0x2bc9510,zero_mock);
 S(turn,0x1fe4810,zero_mock);S(parameters,0x1fe4cb0,parameters_mock);S(embarked_base,0x2c2c370,zero_mock);S(city_cost,0x2bcad50,zero_mock);S(relations,0x2980f10,relations_mock);
 S(is_embarked,0x2c12ca0,true_mock);S(max_moves,0x2c27180,max_mock);S(has_moved,0x2c27b50,moved_mock);
#undef S
 for(unsigned i=0;i<65;i++)index_map[i]=-1;
 index_map[0]=0;index_map[64]=1;slots[0]=(uintptr_t)&players[0];slots[64]=(uintptr_t)&players[1];
 write_ptr(manager_data+0x20,(uintptr_t)slots);write_ptr(manager_data+0x28,(uintptr_t)(slots+65));
 write_ptr(plot_data,(uintptr_t)plot_vtable);write_ptr(plot_vtable+0x20,core+0x2bc9510);
 for(unsigned i=0;i<TOTAL;i++){
  write_i32(units_data[i]+0x4f8,(int32_t)i);write_i32(units_data[i]+0x100,i<21?-9999:(int32_t)i);
  write_i32(units_data[i]+0x104,i<21?-9999:1);units_data[i][0x2f0]=(unsigned char)(i<21);
 }
 if(is_case("count-mismatch"))containers[1].count=0;
 if(is_case("output-exists")){FILE *f=fopen("inputs","wx");assert(f);assert(!fclose(f));}
 C6UnitMovementQueryAdmission a={.module=&module_data,.borrowed_context=&context_data,.retained_game=game_data,
 .retained_owner=retained,.profile_sha256="1111111111111111111111111111111111111111111111111111111111111111",
 .candidate_core_sha256="ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884",
 .inputs_path="inputs",.answers_path="answers",.expected_units=TOTAL};
 int status=c6_linux_unit_movement_query_capture(&a);
 assert(status==(is_case("count-mismatch")?243:is_case("output-exists")?242:0));
 unsigned expected=is_case("output-exists")?0:is_case("count-mismatch")?(TOTAL-1)*2:TOTAL*2;
 assert(target_step==expected);
 assert(definitions_seen==(is_case("output-exists")?0:target_step));
 if(target_step)assert(game_data[0xf8]==(unsigned char)(target_step/2-1)&&*(unsigned char *)(core+0x3c072f0)==(unsigned char)(target_step/2));
 assert(!fclose(events));printf("status=%d targets=%u guards=%u effects_retained=1\n",status,target_step,guards);return 0;
}
