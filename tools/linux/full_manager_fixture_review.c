/* Independent synthetic lifetime challenges. Never loads original game code. */
#include "../../native/linux/full_manager_fixture.c"
#include <assert.h>
#include <sys/mman.h>
#include <sys/wait.h>

enum { TEST_RUNTIME_ID=3 };
enum Mode {
    FALSE_NO_SAVES, TRUE_SAVE_EFFECTS, REMOVE_ACTOR_IN_QUERY,
    OBSERVE_ACQUIRED_FAIL, OBSERVE_ENTER_FAIL, OBSERVE_RETURN_FAIL,
    SAVE_REMOVES_ACTOR, SAVE_CHANGES_ID, SAVE_REPLACES_MANAGER,
    PRE_GUARD_FAIL, LOCKED_BEGIN_FAIL, QUERY_OWNER_FAIL, QUERY_UTILITY_CHANGE,
    QUERY_FACADE_REF_CHANGE, BAD_TID, BAD_PROFILE, PARTIAL_SAVE_PATHS,
    DUPLICATE_SAVE_PATHS
};
typedef struct {
    unsigned acquired,released,constructed,destroyed,queries,saves,finds,get_ids;
    unsigned phases[16],phase_count;
    int returned_status;
} Counters;
static Counters *counts;
static enum Mode mode;
static uintptr_t image_base;
static unsigned char *unit_page;
static unsigned char game_bytes[0x120],facade_bytes[32],utility_bytes[8],other_utility[8];
static unsigned char controller_bytes[0x4a8],manager_bytes[104],other_manager[104],handler_bytes[16];
static uintptr_t handler_slots[4];
static const void *current_manager,*current_actor;
static bool lock_held,after_query;
static char output_path[256];

static void put32(void *p,size_t offset,uint32_t value) {
    memcpy((unsigned char *)p+offset,&value,sizeof value);
}
static void putptr(void *p,size_t offset,uintptr_t value) {
    memcpy((unsigned char *)p+offset,&value,sizeof value);
}
static void native_lock(bool held) {
    lock_held=held;
    /* A synthetic native owner token, deliberately unrelated to OS TID. */
    put32(controller_bytes,0x4a0,held?0x18422011u:0u);
    put32(controller_bytes,0x4a4,held?1u:0u);
}
static void roots(void *m,void *c,const void *g) {
    assert(m==(void *)1&&c==(void *)2&&g==game_bytes);
}
static int pre(void *m,void *c,const void *g) {
    roots(m,c,g);assert(!lock_held);
    return mode==PRE_GUARD_FAIL?1:0;
}
static int begin(void *m,void *c,const void *g,uintptr_t base,long tid) {
    roots(m,c,g);assert(lock_held&&base==image_base&&tid==syscall(SYS_gettid));
    return mode==LOCKED_BEGIN_FAIL?1:0;
}
static int retained(void *m,void *c,const void *g) {
    roots(m,c,g);assert(lock_held);
    return mode==QUERY_OWNER_FAIL&&after_query?1:0;
}
static int released(void *m,void *c,const void *g) {
    roots(m,c,g);assert(!lock_held);return 0;
}
static void *acquire(void *context) {
    assert(context==(void *)2&&!lock_held);counts->acquired++;
    native_lock(true);return facade_bytes;
}
static void release_facade(void *facade) {
    assert(facade==facade_bytes&&lock_held);counts->released++;native_lock(false);
}
static const void *get_manager(void) { assert(lock_held);return current_manager; }
static int32_t id_for_hash(const void *manager,uint32_t hash) {
    assert(manager==current_manager&&hash==0x2c681908);return TEST_RUNTIME_ID;
}
static const void *handler_for_hash(const void *manager,uint32_t hash) {
    assert(manager==current_manager&&hash==0x2c681908);
    return (const void *)ptr(ptr((uintptr_t)manager+24)+8*TEST_RUNTIME_ID);
}
static const void *get_players(const void *game) { assert(game==game_bytes);return (void *)3; }
static const void *get_player(const void *players,int32_t owner) {
    assert(players==(void *)3&&owner==7);return (void *)4;
}
static const void *get_units(const void *player) { assert(player==(void *)4);return (void *)5; }
static const void *find_unit(const void *units,uint32_t id) {
    assert(units==(void *)5&&id==0x120001);counts->finds++;
    assert(!after_query);return current_actor;
}
static uint32_t get_id(const void *unit) {
    assert(unit==unit_page&&!after_query);counts->get_ids++;
    return word((uintptr_t)unit+0xb0);
}
static const void *get_formation(const void *unit) {
    assert(unit==unit_page&&!after_query);return (const void *)ptr((uintptr_t)unit+0xbc0);
}
static void operation_ctor(void *operation,int32_t id) {
    assert(id==TEST_RUNTIME_ID&&lock_held);assert((uintptr_t)operation%8==0);counts->constructed++;
    memset(operation,0xa5,32);put32(operation,0,(uint32_t)id);put32(operation,4,UINT32_MAX);
    putptr(operation,8,0);putptr(operation,16,0);put32(operation,24,0);
}
static void operation_dtor(void *operation) {
    assert(lock_held&&word((uintptr_t)operation)==TEST_RUNTIME_ID);counts->destroyed++;
}
static bool full_predicate(const void *manager,const void *unit,const void *operation,
                           const void *plot,bool visible,void *results) {
    assert(lock_held&&manager==current_manager&&unit==unit_page&&plot==NULL&&results==NULL);
    assert(visible==(mode!=FALSE_NO_SAVES));
    assert(word((uintptr_t)operation)==TEST_RUNTIME_ID&&word((uintptr_t)operation+4)==UINT32_MAX);
    assert(!ptr((uintptr_t)operation+8)&&!ptr((uintptr_t)operation+16));
    assert(word((uintptr_t)operation+24)==0);counts->queries++;
    if(mode==TRUE_SAVE_EFFECTS)assert(word((uintptr_t)unit+0x20)==2);
    if(mode==SAVE_REPLACES_MANAGER)assert(manager==other_manager);
    after_query=true;
    if(mode==REMOVE_ACTOR_IN_QUERY) {
        current_actor=NULL;
        assert(!mprotect(unit_page,4096,PROT_NONE));
    }
    if(mode==QUERY_UTILITY_CHANGE)
        putptr((void *)(image_base+0x3c051b8),0,(uintptr_t)other_utility);
    if(mode==QUERY_FACADE_REF_CHANGE)put32(facade_bytes,0x18,2);
    return mode!=FALSE_NO_SAVES;
}
static int observe(const C6FullManagerObservation *o) {
    assert(o->game==game_bytes&&o->facade==(o->phase==C6_FULL_MANAGER_RELEASED?NULL:facade_bytes));
    assert(counts->phase_count<16);counts->phases[counts->phase_count++]=(unsigned)o->phase;
    if(o->phase==C6_FULL_MANAGER_PREDICATE_ENTER) {
        assert(o->unit==unit_page&&o->operation);
        assert(word((uintptr_t)o->operation)==TEST_RUNTIME_ID);
    }
    if(o->phase==C6_FULL_MANAGER_PREDICATE_RETURN) {
        /* Unit is now PROT_NONE in the removal case. Retain its address only. */
        assert(o->unit==unit_page&&o->operation&&o->predicate_returned);
    }
    if(o->phase>=C6_FULL_MANAGER_AFTER)assert(o->unit==NULL&&o->operation==NULL);
    if((mode==OBSERVE_ACQUIRED_FAIL&&o->phase==C6_FULL_MANAGER_FACADE_ACQUIRED)||
       (mode==OBSERVE_ENTER_FAIL&&o->phase==C6_FULL_MANAGER_PREDICATE_ENTER)||
       (mode==OBSERVE_RETURN_FAIL&&o->phase==C6_FULL_MANAGER_PREDICATE_RETURN))return 29;
    return 0;
}
int c6_linux_context_save(void *module,void *context,const char *path) {
    assert(module==(void *)1&&context==(void *)2&&path&&lock_held);counts->saves++;
    if(mode==TRUE_SAVE_EFFECTS&&counts->saves<=2)put32(unit_page,0x20,counts->saves);
    if(mode==SAVE_REMOVES_ACTOR&&counts->saves==2) {
        current_actor=NULL;assert(!mprotect(unit_page,4096,PROT_NONE));
    }
    if(mode==SAVE_CHANGES_ID&&counts->saves==2)put32(unit_page,0xb0,0x130001);
    if(mode==SAVE_REPLACES_MANAGER&&counts->saves==2)current_manager=other_manager;
    return counts->saves==1?13:0;
}
static void setup(void) {
    void *memory=mmap(NULL,0x3c08000,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
    assert(memory!=MAP_FAILED);image_base=(uintptr_t)memory;
    assert(mmap((void *)0x53a6000,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0)==(void *)0x53a6000);
    assert(mmap((void *)0x6499000,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0)==(void *)0x6499000);
    unit_page=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(unit_page!=MAP_FAILED);
    putptr((void *)0x6499100,0,(uintptr_t)controller_bytes);
    putptr((void *)(image_base+0x3b13848),0,image_base+0x3c051b8);
    putptr((void *)(image_base+0x3c051b8),0,(uintptr_t)utility_bytes);
    putptr(utility_bytes,0,0x53a6c90);putptr(other_utility,0,0x53a6c90);
    putptr((void *)0x53a6c90,0x30,0x32b3da0);putptr((void *)0x53a6c90,0x38,0x32b3d60);putptr((void *)0x53a6c90,0x40,0x32b3d40);
    putptr(facade_bytes,0,image_base+0x3aecf60);putptr(facade_bytes,8,(uintptr_t)game_bytes);put32(facade_bytes,0x18,1);
    putptr((void *)(image_base+0x3aecf68),0,image_base+0x2c86370);
    handler_slots[TEST_RUNTIME_ID]=(uintptr_t)handler_bytes;
    for(unsigned i=0;i<2;i++) {
        void *manager=i?other_manager:manager_bytes;
        putptr(manager,0,image_base+0x3aff880);putptr(manager,24,(uintptr_t)handler_slots);putptr(manager,32,(uintptr_t)(handler_slots+4));
    }
    putptr((void *)(image_base+0x3aff880),0xa0,image_base+0x2da1720);
    putptr((void *)(image_base+0x3aff880),0x88,image_base+0x2da1650);
    putptr(handler_bytes,0,image_base+0x3b00db0);put32(handler_bytes,8,TEST_RUNTIME_ID);put32(handler_bytes,12,0x2c681908);
    putptr((void *)(image_base+0x3b00db0),0x18,image_base+0x2dac950);
    putptr(unit_page,0,image_base+0x3ae5f80);put32(unit_page,0x130,7);put32(unit_page,0xb0,0x120001);putptr(unit_page,0xbc0,6);
    current_manager=manager_bytes;current_actor=unit_page;after_query=false;native_lock(false);
}
static void run_case(unsigned index) {
    mode=(enum Mode)index;setup();
    Bindings b={.acquire=acquire,.release=release_facade,.manager_get=get_manager,
        .id_for_hash=id_for_hash,.handler_for_hash=handler_for_hash,.construct=operation_ctor,
        .destroy=operation_dtor,.can_start=full_predicate,.players=get_players,.player=get_player,
        .units=get_units,.find_unit=find_unit,.unit_id=get_id,.formation=get_formation};
    snprintf(output_path,sizeof output_path,"/tmp/c6-full-manager-review-%ld-%u.jsonl",(long)getpid(),index);
    C6FullManagerFixtureAdmission a={.module=(void *)1,.context=(void *)2,.expected_game=game_bytes,
        .expected_unit=unit_page,.actor_owner=7,.actor_full_id=0x120001,.visible_only=mode!=FALSE_NO_SAVES,
        .main_entry_tid=syscall(SYS_gettid),.profile_sha256="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        .output_path=output_path,.pre_acquisition=pre,.locked_begin=begin,.retained_owner=retained,
        .locked_end=retained,.released_owner=released,.observe=observe};
    if(mode==TRUE_SAVE_EFFECTS||mode==SAVE_REMOVES_ACTOR||mode==SAVE_CHANGES_ID||mode==SAVE_REPLACES_MANAGER) {
        static const char *paths[]={"mock-before","mock-control","mock-after","mock-repeat"};
        for(unsigned i=0;i<4;i++)a.diagnostic_save_paths[i]=paths[i];
    }
    if(mode==BAD_TID)a.main_entry_tid++;
    if(mode==BAD_PROFILE)a.profile_sha256="gaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    if(mode==PARTIAL_SAVE_PATHS)a.diagnostic_save_paths[0]="one-only";
    if(mode==DUPLICATE_SAVE_PATHS)for(unsigned i=0;i<4;i++)a.diagnostic_save_paths[i]="same";
    C6FullManagerFixtureResult result={.operation_id=-1};
    int status;
    if(mode>=BAD_TID)status=c6_linux_full_manager_fixture_run(&a,&result);
    else status=run_bound(&b,&a,&result,image_base);
    counts->returned_status=status;
    if(counts->constructed)assert(result.operation_id==TEST_RUNTIME_ID&&result.operation_hash==0x2c681908);
    assert(result.facade_released&&counts->acquired==1&&counts->released==1);
    assert(!lock_held);
    if(mode==OBSERVE_ACQUIRED_FAIL) {
        assert(status==29&&counts->queries==0&&counts->constructed==0&&counts->destroyed==0);
    } else if(mode==OBSERVE_ENTER_FAIL) {
        assert(status==29&&counts->queries==0&&counts->constructed==1&&counts->destroyed==1);
        assert(!result.predicate_invoked&&!result.predicate_returned);
    } else {
        assert(counts->queries==1&&counts->constructed==1&&counts->destroyed==1);
        assert(result.predicate_invoked&&result.predicate_returned);
        assert(result.allowed==(mode!=FALSE_NO_SAVES));
        assert(status==(mode==OBSERVE_RETURN_FAIL?29:0));
    }
    if(mode==TRUE_SAVE_EFFECTS)assert(counts->saves==4&&result.save_status[0]==13);
    unlink(output_path);
}
int main(void) {
    counts=mmap(NULL,sizeof *counts,PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);assert(counts!=MAP_FAILED);
    const char *reasons[]={NULL,NULL,NULL,NULL,NULL,NULL,"actor_not_loaded_member","actor_identity_unknown",NULL,
        "ownership_unproven","locked_begin_unproven","ownership_unproven","release_utility_changed","facade_release_changed",
        "main_entry_unwitnessed","profile_invalid","diagnostic_paths_unknown","diagnostic_paths_duplicate"};
    for(unsigned i=0;i<sizeof reasons/sizeof reasons[0];i++) {
        memset(counts,0,sizeof *counts);FILE *errors=tmpfile();assert(errors);
        pid_t child=fork();assert(child>=0);
        if(!child) {assert(dup2(fileno(errors),STDERR_FILENO)>=0);run_case(i);_exit(0);}
        int status;assert(waitpid(child,&status,0)==child);assert(WIFEXITED(status));
        assert(WEXITSTATUS(status)==(reasons[i]?240:0));
        if(reasons[i]) {
            rewind(errors);char message[512]={0};assert(fread(message,1,sizeof message-1,errors)>0);assert(strstr(message,reasons[i]));
            assert(counts->released==0);
            if(i==SAVE_REMOVES_ACTOR||i==SAVE_CHANGES_ID)assert(counts->queries==0&&counts->constructed==0&&counts->saves==2);
            if(i==QUERY_OWNER_FAIL)assert(counts->queries==1&&counts->destroyed==0);
            if(i==QUERY_UTILITY_CHANGE||i==QUERY_FACADE_REF_CHANGE)assert(counts->queries==1&&counts->destroyed==1);
            if(i>=BAD_TID)assert(counts->acquired==0&&counts->queries==0);
        }
        assert(!fclose(errors));
        printf("case %u: exit=%d calls=%u ctor=%u dtor=%u acquire=%u release=%u saves=%u\n",i,WEXITSTATUS(status),counts->queries,counts->constructed,counts->destroyed,counts->acquired,counts->released,counts->saves);
        char path[256];snprintf(path,sizeof path,"/tmp/c6-full-manager-review-%ld-%u.jsonl",(long)child,i);unlink(path);
    }
    puts("18 independent synthetic lifecycle/refusal cases passed; no original execution");
    return 0;
}
