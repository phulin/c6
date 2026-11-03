/* Actual wrapper + actual run_bound, with independently reviewed original-call
 * mocks. No original module is loaded or called. */
#define main caller_leaf_review_main
#include CALLER_REVIEW_SOURCE
#undef main
#include <fcntl.h>

static int integrated_run(const C6FullManagerFixtureAdmission *,C6FullManagerFixtureResult *);
static ssize_t wrapper_readv(pid_t,const struct iovec *,unsigned long,const struct iovec *,unsigned long,unsigned long);
#define c6_linux_full_manager_fixture_run integrated_run
#define process_vm_readv wrapper_readv
#include "../../native/linux/full_manager_profile.c"
#undef process_vm_readv
#undef c6_linux_full_manager_fixture_run

static unsigned char roster[7416],player_raw[0x6d8],units_raw[232],block_raw[192];
static uintptr_t players_raw[65],blocks_raw[1];
static int32_t mapping_raw[65];
static unsigned metadata_count,fixture_count;
static bool metadata_errors,control_error;
static C6FullManagerFixtureResult integrated_result;
static ssize_t wrapper_readv(pid_t pid,const struct iovec *local,unsigned long lc,
                             const struct iovec *remote,unsigned long rc,unsigned long flags) {
    if(lc==1&&rc==1&&(uintptr_t)remote->iov_base==(uintptr_t)game_bytes+0x248&&remote->iov_len==8&&local->iov_len==8){
        uintptr_t p=(uintptr_t)roster;memcpy(local->iov_base,&p,8);return 8;
    }
    return process_vm_readv(pid,local,lc,remote,rc,flags);
}

int c6_linux_full_manager_profile_identity(void *m,void *c,uintptr_t *b,const void **g,long *t) {
    roots(m,c,game_bytes);*b=image_base;*g=game_bytes;*t=syscall(SYS_gettid);return 0;
}
int c6_linux_full_manager_pre_acquisition(void *m,void *c,const void *g){return pre(m,c,g);}
int c6_linux_full_manager_locked_begin(void *m,void *c,const void *g,uintptr_t b,long t){return begin(m,c,g,b,t);}
int c6_linux_full_manager_retained_owner(void *m,void *c,const void *g){return retained(m,c,g);}
int c6_linux_full_manager_locked_end(void *m,void *c,const void *g){return retained(m,c,g);}
int c6_linux_full_manager_released_owner(void *m,void *c,const void *g){return released(m,c,g);}
int c6_linux_current_handler_interruption_capture(const C6CurrentHandlerInterruptionAdmission *a) {
    assert(!a->quiescent(a->module,a->borrowed_context,a->retained_game));
    metadata_count++;
    game_bytes[0xf8]=(unsigned char)(0x40+metadata_count);
    *(unsigned char *)(image_base+0x3c072f0)=(unsigned char)(0x80+metadata_count);
    return metadata_errors?245:0;
}
static int integrated_run(const C6FullManagerFixtureAdmission *a,C6FullManagerFixtureResult *r) {
    fixture_count++;
    assert(a->expected_unit==unit_page&&a->actor_owner==7&&a->actor_full_id==0x120001);
    assert(a->observe==observe);
    if(control_error){
        int fd=open("/dev/full",O_WRONLY);assert(fd>=0);
        assert(dup2(fd,fileno(full_observation.controls))>=0);assert(!close(fd));
    }
    Bindings b={.acquire=acquire,.release=release_facade,.manager_get=get_manager,
        .id_for_hash=id_for_hash,.handler_for_hash=handler_for_hash,.construct=operation_ctor,
        .destroy=operation_dtor,.can_start=full_predicate,.players=get_players,.player=get_player,
        .units=get_units,.find_unit=find_unit,.unit_id=get_id,.formation=get_formation};
    *r=(C6FullManagerFixtureResult){.operation_id=-1};
    int status=run_bound(&b,a,r,image_base);integrated_result=*r;return status;
}
static void setenv_part(const char *suffix,const char *value) {
    char key[128];int n=snprintf(key,sizeof key,"C6_FULL_MANAGER_FIXTURE_%s",suffix);
    assert(n>0&&(size_t)n<sizeof key);assert(value?!setenv(key,value,1):!unsetenv(key));
}
int main(int argc,char **argv) {
    assert(argc==3);unsigned which=(unsigned)strtoul(argv[1],NULL,10);assert(which<9);
    counts=mmap(NULL,sizeof *counts,PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);assert(counts!=MAP_FAILED);
    mode=which==0?FALSE_NO_SAVES:which==1?TRUE_SAVE_EFFECTS:which==2?SAVE_REMOVES_ACTOR:
         which==3?SAVE_CHANGES_ID:which==4?SAVE_REPLACES_MANAGER:REMOVE_ACTOR_IN_QUERY;
    setup();
    /* Wrapper's authoritative expectation uses +b8. The original-call mock's
     * independently seeded GetID uses +b0, so seed both with the same full ID. */
    put32(unit_page,0xb8,0x120001);
    /* Raw expectation is tested against real fixture original-Find alignment.
     * Mapping is validity only; it deliberately differs from owner index. */
    /* Existing caller mocks require game_bytes identity. Its adjacent globals
     * are not extended; intercept only the raw pre-acquisition Game+248 read. */
    putptr(roster,0x20,(uintptr_t)players_raw);putptr(roster,0x28,(uintptr_t)(players_raw+65));putptr(roster,0x30,(uintptr_t)(players_raw+65));
    putptr(roster,0x1a08,(uintptr_t)mapping_raw);putptr(roster,0x1a10,(uintptr_t)(mapping_raw+65));putptr(roster,0x1a18,(uintptr_t)(mapping_raw+65));
    mapping_raw[7]=64;players_raw[7]=(uintptr_t)player_raw;players_raw[64]=1;
    put32(player_raw,0xe0,7);putptr(player_raw,0x6d0,(uintptr_t)units_raw);putptr(units_raw,152,(uintptr_t)player_raw);
    blocks_raw[0]=(uintptr_t)block_raw;unsigned char *list=units_raw+160,*entry=block_raw+24;
    putptr(list,0,(uintptr_t)blocks_raw);putptr(list,8,(uintptr_t)(blocks_raw+1));putptr(list,16,(uintptr_t)(blocks_raw+1));
    put32(list,32,1);putptr(list,56,(uintptr_t)entry);putptr(list,64,(uintptr_t)entry);putptr(entry,0,(uintptr_t)unit_page);
    char paths[10][512];const char *names[]={"OUTPUT","CONTROL_OUTPUT","METADATA_BEFORE","METADATA_CONTROL","METADATA_AFTER","METADATA_REPEAT","SNAPSHOT_BEFORE","SNAPSHOT_CONTROL","SNAPSHOT_AFTER","SNAPSHOT_REPEAT"};
    bool saves=which>=1&&which<=4;
    for(unsigned i=0;i<10;i++){int n=snprintf(paths[i],sizeof paths[i],"%s/%s",argv[2],names[i]);assert(n>0&&(size_t)n<sizeof paths[i]);setenv_part(names[i],i<6||saves?paths[i]:NULL);}
    setenv_part("ACTOR_OWNER","7");setenv_part("ACTOR_FULL_ID","1179649");setenv_part("VISIBLE_ONLY",which==0?"0":"1");
    setenv_part("PROFILE_SHA256","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    metadata_errors=which==5;control_error=which==6;
    if(which==7)current_actor=NULL; /* raw expectation succeeds, original Find refuses */
    if(which==8)put32(unit_page,0xb0,0x130001); /* native identity rejects raw-only match */
    int status=c6_linux_full_manager_profile_probe((void *)1,(void *)2);
    assert(status==(control_error?247:0));assert(fixture_count==1&&counts->queries==1&&counts->finds==1);
    assert(counts->constructed==1&&counts->destroyed==1&&counts->acquired==1&&counts->released==1);
    assert(metadata_count==4&&!lock_held&&!full_observation.controls&&integrated_result.facade_released);
    assert(game_bytes[0xf8]==0x44&&*(unsigned char *)(image_base+0x3c072f0)==0x84);
    assert(integrated_result.allowed==(which!=0));
    if(saves)assert(counts->saves==4&&integrated_result.save_status[0]==13);
    if(metadata_errors)assert(integrated_result.observation_status==0);
    printf("integrated wrapper case %u passed\n",which);return 0;
}
