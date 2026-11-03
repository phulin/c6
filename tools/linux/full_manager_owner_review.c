/* Public guard-entry tests with a mock DSO; no original game code is loaded. */
#define process_vm_readv reviewed_readv
#include "../../native/linux/full_manager_profile_guard.c"
#undef process_vm_readv
#include <assert.h>
#include <stdbool.h>
#include <pthread.h>
#include <sys/mman.h>
#include <sys/wait.h>

static long witnessed_tid;
static unsigned witness_calls;
static uintptr_t game_address;
static bool change_worker_on_game_read;
#ifndef NO_MAIN_WITNESS
__attribute__((visibility("default")))
long c6_linux_mod_main_thread_tid(void) { witness_calls++;return witnessed_tid; }
#endif
ssize_t reviewed_readv(pid_t pid,const struct iovec *local,unsigned long local_count,
                       const struct iovec *remote,unsigned long remote_count,unsigned long flags) {
    ssize_t n=syscall(SYS_process_vm_readv,pid,local,local_count,remote,remote_count,flags);
    if(change_worker_on_game_read&&remote_count==1&&
       (uintptr_t)remote[0].iov_base==game_address+0x2ac) {
        uintptr_t worker=1;memcpy((void *)0x6499108,&worker,8);
        change_worker_on_game_read=false;
    }
    return n;
}
static void put(uintptr_t p,uintptr_t value) { memcpy((void *)p,&value,8); }
static void put32(uintptr_t p,uint32_t value) { memcpy((void *)p,&value,4); }
static void *module,*context;
static uintptr_t core_base,controller,native_context;
static const uint32_t native_token=0x4ee17a11u;
static void lock_words(uint32_t owner,uint32_t depth) {
    put32(controller+0x4a0,owner);put32(controller+0x4a4,depth);
}
static void begin_locked(void) {
    lock_words(native_token,1);
    assert(!c6_linux_full_manager_locked_begin(module,context,(void *)game_address,core_base,witnessed_tid));
}
static void *wrong_thread(void *ignored) {
    (void)ignored;c6_linux_full_manager_retained_owner(module,context,(void *)game_address);
    return NULL;
}
static void setup(void *library,const void *vtable,uintptr_t base) {
    void *host=mmap((void *)0x6499000,4096,PROT_READ|PROT_WRITE,
                   MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0);
    assert(host==(void *)0x6499000);
    unsigned char *objects=mmap(NULL,16384,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
    assert(objects!=MAP_FAILED);
    module=library;core_base=base;controller=(uintptr_t)objects;
    context=objects+4096;native_context=(uintptr_t)(objects+8192);game_address=(uintptr_t)(objects+12288);
    put(0x6499100,controller);*(unsigned char *)(controller+0xb)=1;
    put(controller+0x450,(uintptr_t)context);put((uintptr_t)context,(uintptr_t)vtable);
    put((uintptr_t)context+8,native_context);*(unsigned char *)(native_context+8)=1;
    put(native_context+0x28,game_address);put(core_base+0x3c051a8,game_address);
    *(unsigned char *)(game_address+0x2ac)=1;
    witnessed_tid=syscall(SYS_gettid);assert((uint32_t)witnessed_tid!=native_token);
}
static void one(unsigned mode,void *library,const void *vtable,uintptr_t base) {
    setup(library,vtable,base);
    if(mode==2)witnessed_tid++;
    if(mode==27)put((uintptr_t)context+8,UINTPTR_MAX-8);
    if(mode==28)assert(!mprotect((void *)native_context,4096,PROT_NONE));
    assert(!c6_linux_full_manager_profile_admission_begin(module,context));
    assert(witness_calls==1);
    if(mode==1)c6_linux_full_manager_profile_admission_begin(module,context);
    if(mode==17) {
        lock_words(native_token,1);
        c6_linux_full_manager_locked_begin(module,context,(void *)game_address,core_base+8,witnessed_tid);
    }
    if(mode==18) {
        lock_words(native_token,1);
        c6_linux_full_manager_locked_begin(module,context,(void *)game_address,core_base,witnessed_tid+1);
    }
    if(mode==15||mode==16) {
        lock_words(mode==15?0:native_token,mode==16?2:1);
        c6_linux_full_manager_locked_begin(module,context,(void *)game_address,core_base,witnessed_tid);
    }
    assert(!c6_linux_full_manager_pre_acquisition(module,context,(void *)game_address));
    begin_locked();
    switch(mode) {
    case 3:put(0x6499100,controller+8);break;
    case 4:put(0x6499108,1);break;
    case 5:*(unsigned char *)(controller+0xb)=0;break;
    case 6:put(controller+0x450,(uintptr_t)context+8);break;
    case 7:put((uintptr_t)context,(uintptr_t)vtable+8);break;
    case 8:put((uintptr_t)context+8,native_context+8);break;
    case 9:*(unsigned char *)(native_context+8)=0;break;
    case 10:put(native_context+0x28,game_address+8);break;
    case 11:put(core_base+0x3c051a8,game_address+8);break;
    case 12:*(unsigned char *)(game_address+0x2ac)=0;break;
    case 13:lock_words(native_token+1,1);break;
    case 14:lock_words(native_token,2);break;
    case 19:c6_linux_full_manager_locked_begin(module,context,(void *)game_address,core_base,witnessed_tid);break;
    case 20:c6_linux_full_manager_released_owner(module,context,(void *)game_address);break;
    case 21:
        c6_linux_full_manager_locked_end(module,context,(void *)game_address);
        c6_linux_full_manager_retained_owner(module,context,(void *)game_address);break;
    case 22:c6_linux_full_manager_retained_owner((void *)1,context,(void *)game_address);break;
    case 23:c6_linux_full_manager_retained_owner(module,(unsigned char *)context+8,(void *)game_address);break;
    case 24:c6_linux_full_manager_retained_owner(module,context,(void *)(game_address+8));break;
    case 25:change_worker_on_game_read=true;break;
    case 26: {
        pthread_t thread;assert(!pthread_create(&thread,NULL,wrong_thread,NULL));
        assert(!pthread_join(thread,NULL));break;
    }
    default:break;
    }
    /* Queue/work/AI changes must survive. Guard never reads an actor. */
    *(unsigned char *)(core_base+0x3c05cb8)=1;
    *(unsigned char *)(core_base+0x3c05cb9)=1;
    *(unsigned char *)(core_base+0x3c05cbb)=1;
    unsigned char *ai=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
    assert(ai!=MAP_FAILED);put32((uintptr_t)ai+0x110,23);put(game_address+0x258,(uintptr_t)ai);
    put(core_base+0x3c07348,(uintptr_t)ai);put(core_base+0x3c07360,(uintptr_t)ai+16);
    void *actor=mmap(NULL,4096,PROT_NONE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(actor!=MAP_FAILED);
    assert(!c6_linux_full_manager_retained_owner(module,context,(void *)game_address));
    assert(*(unsigned char *)(core_base+0x3c05cb8)==1&&*(unsigned char *)(core_base+0x3c05cb9)==1&&
           *(unsigned char *)(core_base+0x3c05cbb)==1&&full_word((uintptr_t)ai+0x110)==23&&
           full_ptr(core_base+0x3c07348)==(uintptr_t)ai&&full_ptr(core_base+0x3c07360)==(uintptr_t)ai+16);
    assert(witness_calls==1);
    assert(!c6_linux_full_manager_locked_end(module,context,(void *)game_address));
    lock_words(0,0);
    assert(!c6_linux_full_manager_released_owner(module,context,(void *)game_address));
    assert(!c6_linux_full_manager_profile_caller_idle(module,context));
    assert(!c6_linux_full_manager_profile_admission_end(module,context));
    assert(!full_scope.module&&!full_scope.game&&!full_scope.controller);
}
int main(int argc,char **argv) {
    assert(argc==2);void *library=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);assert(library);
    const void *vtable=dlsym(library,"full_guard_mock_vtable");Dl_info info={0};
    assert(vtable&&dladdr(vtable,&info)&&info.dli_fbase);
    const char *reasons[]={NULL,"duplicate_or_missing_admission","scope_or_main_thread_unknown",
        "controller_or_worker_unknown","controller_or_worker_unknown","controller_or_worker_unknown",
        "controller_or_worker_unknown","context_or_game_owner_changed","context_or_game_owner_changed",
        "context_or_game_owner_changed","context_or_game_owner_changed","context_or_game_owner_changed",
        "context_or_game_owner_changed","retained_native_lock_changed","retained_native_lock_changed",
        "retained_native_lock_changed","retained_native_lock_changed","locked_begin_identity_unknown",
        "locked_begin_identity_unknown","locked_begin_identity_unknown","unlocked_boundary_unknown",
        "retained_native_lock_changed","admission_identity_changed","admission_identity_changed",
        "admission_identity_changed","owner_changed_during_observation","scope_or_main_thread_unknown",
        "object_extent_unknown","field_unreadable"};
#ifdef NO_MAIN_WITNESS
    unsigned total=1;
#else
    unsigned total=(unsigned)(sizeof reasons/sizeof reasons[0]);
#endif
    for(unsigned i=0;i<total;i++) {
        const char *reason=reasons[i];
#ifdef NO_MAIN_WITNESS
        reason="main_entry_unwitnessed";
#endif
        FILE *error=tmpfile();assert(error);pid_t child=fork();assert(child>=0);
        if(!child) {
            assert(dup2(fileno(error),STDERR_FILENO)>=0);
            one(i,library,vtable,(uintptr_t)info.dli_fbase);_exit(0);
        }
        int status;assert(waitpid(child,&status,0)==child&&WIFEXITED(status));
        assert(WEXITSTATUS(status)==(reason?240:0));
        if(reason) {
            rewind(error);char text[512]={0};assert(fread(text,1,sizeof text-1,error)>0&&strstr(text,reason));
        }
        assert(!fclose(error));printf("guard case %u: %s\n",i,reason?reason:"public-entry cycle, queued effects preserved");
    }
    assert(!dlclose(library));return 0;
}
