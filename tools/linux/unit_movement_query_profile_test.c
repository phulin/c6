#define _GNU_SOURCE
#include <assert.h>
#include <dlfcn.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>
static void *test_dlsym(void *,const char *);
static int test_dladdr(const void *,Dl_info *);
#define dlsym test_dlsym
#define dladdr test_dladdr
#include "../../native/linux/unit_movement_query_profile_guard.c"
#include "../../native/linux/unit_movement_query_profile.c"
#undef dlsym
#undef dladdr

/* Synthetic contract test only. These executable trampolines invoke our mock
 * C functions, never bytes from a game/host ELF. No original module is loaded. */
static uintptr_t test_base,test_controller,test_context,test_native,test_game;
static unsigned char *test_facade;
static uintptr_t test_utility_global,test_utility;
static long test_main;
static unsigned acquired,captured,released;
static const char *mode;
static void put(uintptr_t p,uintptr_t v){memcpy((void *)p,&v,8);}
static void put32(uintptr_t p,uint32_t v){memcpy((void *)p,&v,4);}
static void event(const char *name){FILE *f=fopen("events.txt","a");assert(f);fprintf(f,"%s\n",name);assert(!fclose(f));}
static int is(const char *s){return !strcmp(mode,s);}
static long main_witness(void){return is("main-zero")?0:test_main;}
static void *mock_acquire(void *context){
 assert(context==(void *)test_context);assert(!acquired&&!captured&&!released);acquired++;event("acquire");
 put32(test_controller+0x4a0,is("zero-token")?0:7);put32(test_controller+0x4a4,is("recursive-lock")?2:1);
 if(is("wrong-lock-base"))movement_scope.base++;
 if(is("wrong-lock-tid"))movement_scope.main_tid++;
 if(is("acquired-facade-null"))return NULL;
 if(is("acquired-facade-vptr"))put((uintptr_t)test_facade,0);
 if(is("acquired-facade-refcount"))put32((uintptr_t)test_facade+0x18,2);
 return test_facade;
}
static void mock_release(void *facade){
 assert(facade==test_facade&&acquired==1&&captured==1&&!released);released++;event("release");
 assert(!movement_scope.locked);assert(movement_word(test_controller+0x4a0)==7);
 if(!is("release-lock-remains")){put32(test_controller+0x4a0,0);put32(test_controller+0x4a4,0);}
 if(is("release-context-changed"))put(test_context+8,0);
 /* Caller must not dereference the released object, even for diagnostics. */
 assert(!mprotect(test_facade,4096,PROT_NONE));
}
static void *test_dlsym(void *module,const char *name){
 if(!strcmp(name,"c6_linux_mod_main_thread_tid"))return is("main-missing")?NULL:(void *)main_witness;
 assert(module==(void *)1);
 if(!strcmp(name,"_ZN8GameCore7Exports7Context7GetGameEv"))return (void *)(test_base+0x2c7cae0+(is("acquire-binding")?8u:0u));
 if(!strcmp(name,"_ZN8GameCore10Interfaces10ICvUnknowndlEPv"))return (void *)(test_base+0x290bba0+(is("release-binding")?8u:0u));
 if(!strcmp(name,"_ZN8GameCore7Exports4Game18DecrementReferenceEv"))return (void *)(test_base+0x2c86b80+(is("decrement-binding")?8u:0u));
 assert(!"unexpected dlsym");return NULL;
}
static int test_dladdr(const void *pointer,Dl_info *info){
 if(is("context-dladdr"))return 0;
 uintptr_t p=(uintptr_t)pointer;
 if(p<test_base||p>=test_base+0x4000000)return 0;
 memset(info,0,sizeof *info);info->dli_fbase=(void *)test_base;return 1;
}
static void *wrong_thread(void *opaque){
 const C6UnitMovementQueryAdmission *a=opaque;
 a->retained_owner(a->module,a->borrowed_context,a->retained_game);return NULL;
}
int c6_linux_unit_movement_query_capture(const C6UnitMovementQueryAdmission *a){
 assert(acquired==1&&!captured&&!released);captured++;event("capture");
 assert(a->module==(void *)1&&a->borrowed_context==(void *)test_context&&a->retained_game==(void *)test_game);
 assert(a->retained_owner==c6_linux_unit_movement_query_retained_owner&&a->expected_units==1262);
 assert(!strcmp(a->profile_sha256,"1111111111111111111111111111111111111111111111111111111111111111"));
 assert(!strcmp(a->candidate_core_sha256,"ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884"));
 assert(!a->retained_owner(a->module,a->borrowed_context,a->retained_game));
 /* Pending flags/list tokens are effects, not admission/purity predicates. */
 *(unsigned char *)(test_base+0x3c05cb8)=3;*(unsigned char *)(test_base+0x3c05cb9)=5;
 *(unsigned char *)(test_base+0x3c05cbb)=7;put(test_base+0x3c07348,0x1234);
 if(is("rng-effects")){*(unsigned char *)(test_game+0xf8)=0xa5;*(unsigned char *)(test_base+0x3c072f0)=0x5a;}
 if(is("changed-env")){
  assert(!setenv("C6_UNIT_MOVEMENT_QUERY_PROFILE_SHA256","short",1));
  assert(!setenv("C6_UNIT_MOVEMENT_QUERY_CANDIDATE_CORE_SHA256","short",1));
  assert(!strcmp(a->profile_sha256,"1111111111111111111111111111111111111111111111111111111111111111"));
 }
 if(is("nested-profile"))c6_linux_unit_movement_query_profile_probe(a->module,a->borrowed_context);
 if(is("capture-fatal"))_exit(240);
 if(is("wrong-thread")){pthread_t thread;assert(!pthread_create(&thread,NULL,wrong_thread,(void *)a));assert(!pthread_join(thread,NULL));}
 if(is("worker"))put(0x6499108,1);
 if(is("native-owner"))put32(test_controller+0x4a0,8);
 if(is("native-depth"))put32(test_controller+0x4a4,2);
 if(is("game-owner"))put(test_native+0x28,0);
 if(is("controller-owner"))put(0x6499100,test_controller+8);
 if(is("facade-vptr"))put((uintptr_t)test_facade,0);
 if(is("facade-refcount"))put32((uintptr_t)test_facade+0x18,2);
 if(is("facade-game"))put((uintptr_t)test_facade+8,0);
 if(is("facade-destroy"))put(test_base+0x3aecf68,0);
 if(is("utility-object")){static uintptr_t replacement;replacement=0x53a6c90;put(test_utility_global,(uintptr_t)&replacement);}
 if(is("utility-vptr"))put(test_utility,0);
 if(is("utility-unlock"))put(0x53a6c90+0x38,0);
 a->retained_owner(a->module,a->borrowed_context,a->retained_game);
 FILE *f=fopen(a->inputs_path,"wx");assert(f);fputs("{\"synthetic_capture\":true,\"contains_desired_answers\":false}\n",f);assert(!fclose(f));
 f=fopen(a->answers_path,"wx");assert(f);fputs("{\"synthetic_capture\":true,\"native_answers\":false}\n",f);assert(!fclose(f));
 return is("capture-error")?243:0;
}
static void trampoline(uintptr_t at,void *function){
 unsigned char code[]={0x48,0xb8,0,0,0,0,0,0,0,0,0xff,0xe0};
 uintptr_t value=(uintptr_t)function;memcpy(code+2,&value,8);memcpy((void *)at,code,sizeof code);
 uintptr_t page=at&~(uintptr_t)4095;assert(!mprotect((void *)page,4096,PROT_READ|PROT_EXEC));
}
int main(int argc,char **argv){
 assert(argc==2);mode=argv[1];test_main=syscall(SYS_gettid);
 void *host=mmap((void *)0x6499000,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0);assert(host==(void *)0x6499000);
 void *table=mmap((void *)0x53a6000,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0);assert(table==(void *)0x53a6000);
 void *core=mmap(NULL,0x4000000,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(core!=MAP_FAILED);test_base=(uintptr_t)core;
 static unsigned char controller[0x500],context[16],native[0x40],game[0x300];
 static uintptr_t utility,utility_global;
 test_controller=(uintptr_t)controller;test_context=(uintptr_t)context;test_native=(uintptr_t)native;test_game=(uintptr_t)game;
 test_utility=(uintptr_t)&utility;test_utility_global=(uintptr_t)&utility_global;
 put(0x6499100,test_controller);controller[0xb]=1;put(test_controller+0x450,test_context);
 put(test_context,test_base+0x3aec000);put(test_context+8,test_native);native[8]=1;put(test_native+0x28,test_game);game[0x2ac]=1;put(test_base+0x3c051a8,test_game);
 put(test_base+0x3b13848,test_utility_global);put(test_utility_global,test_utility);put(test_utility,0x53a6c90);
 put(0x53a6c90+0x40,0x32b3d40);put(0x53a6c90+0x38,0x32b3d60);put(test_base+0x3aecf68,test_base+0x2c86370);
 test_facade=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(test_facade!=MAP_FAILED);
 put((uintptr_t)test_facade,test_base+0x3aecf60);put((uintptr_t)test_facade+8,test_game);put32((uintptr_t)test_facade+0x18,1);
 trampoline(test_base+0x2c7cae0,(void *)mock_acquire);trampoline(test_base+0x290bba0,(void *)mock_release);
 assert(!setenv("C6_UNIT_MOVEMENT_QUERY_PROFILE_SHA256","1111111111111111111111111111111111111111111111111111111111111111",1));
 assert(!setenv("C6_UNIT_MOVEMENT_QUERY_CANDIDATE_CORE_SHA256","ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884",1));
 assert(!setenv("C6_UNIT_MOVEMENT_QUERY_INPUTS","inputs.jsonl",1));assert(!setenv("C6_UNIT_MOVEMENT_QUERY_ANSWERS","answers.jsonl",1));
 if(is("profile-missing"))assert(!unsetenv("C6_UNIT_MOVEMENT_QUERY_PROFILE_SHA256"));
 if(is("profile-invalid"))assert(!setenv("C6_UNIT_MOVEMENT_QUERY_PROFILE_SHA256","zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",1));
 if(is("candidate-unsupported"))assert(!setenv("C6_UNIT_MOVEMENT_QUERY_CANDIDATE_CORE_SHA256","1111111111111111111111111111111111111111111111111111111111111111",1));
 if(is("path-missing"))assert(!unsetenv("C6_UNIT_MOVEMENT_QUERY_INPUTS"));
 if(is("duplicate-path"))assert(!setenv("C6_UNIT_MOVEMENT_QUERY_ANSWERS","inputs.jsonl",1));
 if(is("pre-worker"))put(0x6499108,1);
 if(is("pre-lock"))put32(test_controller+0x4a0,7);
 if(is("context-unreadable"))test_context=1;
 if(is("controller-extent"))put(0x6499100,UINTPTR_MAX-8);
 if(!is("without-admission"))c6_linux_unit_movement_query_profile_admission_begin((void *)1,(void *)test_context);
 if(is("duplicate-admission"))c6_linux_unit_movement_query_profile_admission_begin((void *)1,(void *)test_context);
 int rc=c6_linux_unit_movement_query_profile_probe((void *)1,(void *)test_context);
 assert(rc==(is("capture-error")?243:0));assert(acquired==1&&captured==1&&released==1);
 assert(!c6_linux_unit_movement_query_profile_admission_end((void *)1,(void *)test_context));assert(!movement_scope.module);
 assert(*(unsigned char *)(test_base+0x3c05cb8)==3&&*(unsigned char *)(test_base+0x3c05cb9)==5&&*(unsigned char *)(test_base+0x3c05cbb)==7&&movement_ptr(test_base+0x3c07348)==0x1234);
 puts("synthetic movement caller balanced; queued effects retained; released facade protected");return 0;
}
