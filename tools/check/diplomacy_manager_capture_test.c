#define _GNU_SOURCE
#include "../../native/linux/diplomacy_manager_capture.h"
#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>
/* Synthetic byte images only: never dlopen/execute a game module. */
#define BASE UINT64_C(0x10000000)
#define GLOBAL (BASE+UINT64_C(0x3c052f8))
#define MANAGER UINT64_C(0x20000000)
#define NODE UINT64_C(0x30000000)
#define SESSION UINT64_C(0x40000000)
#define PLAYER UINT64_C(0x50000000)
#define ACTION UINT64_C(0x60000000)
#define SLOTS UINT64_C(0x70000000)
#define IDS UINT64_C(0x71000000)
#define EVER UINT64_C(0x72000000)
typedef struct {uintptr_t base;unsigned char *data;size_t size;} Region;
typedef struct {
    unsigned char globals[88],manager[0x3c0],node[24],session[0x40],player[0x280],action[12],slots[520],ids[520],ever[16];
    Region regions[9];size_t reads,guards,deny_at;int reject_reads;
} Fixture;
static Fixture *current;
static int owner(void *m,void *c,const void *g){assert(m==(void *)1&&c==(void *)2&&g==(void *)3);current->guards++;return current->deny_at&&current->guards>=current->deny_at;}
static int reader(void *ctx,uintptr_t p,void *out,size_t n){
    Fixture *f=ctx;f->reads++;if(f->reject_reads)return -1;
    for(size_t i=0;i<9;i++){Region *r=&f->regions[i];if(p>=r->base&&p-r->base<=r->size&&n<=r->size-(p-r->base)){memcpy(out,r->data+(p-r->base),n);return 0;}}
    return -1;
}
static void u64(unsigned char *p,size_t at,uint64_t value){memcpy(p+at,&value,8);}
static void i32(unsigned char *p,size_t at,int32_t value){memcpy(p+at,&value,4);}
static void vec(unsigned char *p,size_t at,uintptr_t b,size_t n,size_t stride){u64(p,at,b);u64(p,at+8,b+n*stride);u64(p,at+16,b+n*stride);}
static void init(Fixture *f){
    memset(f,0,sizeof *f);current=f;
    Region regions[]={{GLOBAL,f->globals,sizeof f->globals},{MANAGER,f->manager,sizeof f->manager},{NODE,f->node,sizeof f->node},{SESSION,f->session,sizeof f->session},{PLAYER,f->player,sizeof f->player},{ACTION,f->action,sizeof f->action},{SLOTS,f->slots,sizeof f->slots},{IDS,f->ids,sizeof f->ids},{EVER,f->ever,sizeof f->ever}};
    memcpy(f->regions,regions,sizeof regions);
    u64(f->globals,0,GLOBAL);u64(f->globals,8,GLOBAL);u64(f->globals,24,GLOBAL+24);u64(f->globals,32,GLOBAL+24);
    vec(f->globals,56,ACTION,0,12);u64(f->globals,80,0xfeed);
    vec(f->manager,8,SLOTS,65,8);vec(f->manager,0x20,IDS,65,8);vec(f->manager,0x3a8,EVER,0,8);
    i32(f->player,0xe0,17);f->player[0x278]=1;
}
static C6DiplomacyCaptureAdmission admission(Fixture *f,FILE *out){
    C6DiplomacyCaptureAdmission a={.module=(void *)1,.context=(void *)2,.game=(void *)3,.core_base=BASE,.main_tid=(long)syscall(SYS_gettid),.player_manager=(void *)MANAGER,.profile_sha256="1111111111111111111111111111111111111111111111111111111111111111",.core_sha256="a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa",.stage="synthetic.fixture",.retained_owner=owner,.read=reader,.read_context=f,.max_bytes=8*1024*1024,.max_items=65536,.output=out};return a;
}
static int run(Fixture *f,const char *wanted,int expected){
    FILE *out=tmpfile();assert(out);C6DiplomacyCaptureAdmission a=admission(f,out);
    int result=c6_linux_diplomacy_manager_capture(&a);assert((result==0)==(expected==0));
    char text[65536];rewind(out);size_t n=fread(text,1,sizeof text-1,out);assert(n<sizeof text-1);text[n]=0;assert(strstr(text,wanted));assert(strstr(text,"\"kind\":\"snapshot_end\""));assert(!fclose(out));return result;
}
static void nonempty(Fixture *f){
    u64(f->globals,0,NODE);u64(f->globals,8,NODE);u64(f->globals,16,1);
    u64(f->node,0,GLOBAL);u64(f->node,8,GLOBAL);u64(f->node,16,SESSION);
    i32(f->session,0,-1);i32(f->session,4,17);u64(f->session,0x28,SESSION+0x28);u64(f->session,0x30,SESSION+0x28);
    vec(f->globals,56,ACTION,1,12);i32(f->action,0,-1);i32(f->action,4,-2);i32(f->action,8,17);
    vec(f->manager,0x3a8,EVER,2,8);u64(f->ever,0,PLAYER);u64(f->ever,8,PLAYER);u64(f->slots,8,PLAYER);u64(f->ids,17*8,PLAYER);
}
int main(void){
    Fixture f;init(&f);run(&f,"\"complete\":true",0);
    init(&f);nonempty(&f);run(&f,"\"id\":4294967295,\"from\":-2,\"to\":17",0);
    init(&f);nonempty(&f);run(&f,"\"pending_items\":\"not_captured_layout_unclosed\"",0);
    init(&f);nonempty(&f);u64(f.node,8,NODE);u64(f.globals,16,2);run(&f,"list_cycle",-1);
    init(&f);nonempty(&f);u64(f.node,0,99);run(&f,"list_prev_link",-1);
    init(&f);u64(f.globals,16,1);run(&f,"list_short_count",-1);
    init(&f);u64(f.globals,0,NODE);run(&f,"list_count_or_tail",-1);
    init(&f);u64(f.globals,16,4097);run(&f,"list_count_cap",-1);
    init(&f);u64(f.globals,56,ACTION+12);u64(f.globals,64,ACTION);run(&f,"vector_span",-1);
    init(&f);u64(f.globals,64,ACTION+1);run(&f,"vector_span",-1);
    init(&f);vec(f.globals,56,ACTION,65537,12);run(&f,"vector_count_cap",-1);
    init(&f);nonempty(&f);u64(f.node,16,UINTPTR_MAX-1);run(&f,"pointer_extent",-1);
    init(&f);f.reject_reads=1;run(&f,"unreadable",-1);assert(f.reads==1);
    init(&f);f.deny_at=1;run(&f,"owner_admission",-1);assert(!f.reads);
    init(&f);f.deny_at=3;run(&f,"owner_admission",-1);assert(f.reads==1);
    init(&f);FILE *out=tmpfile();assert(out);C6DiplomacyCaptureAdmission a=admission(&f,out);a.max_bytes=1;assert(c6_linux_diplomacy_manager_capture(&a));assert(!f.reads);fclose(out);
    init(&f);out=tmpfile();assert(out);a=admission(&f,out);a.max_items=1;assert(c6_linux_diplomacy_manager_capture(&a));fclose(out);
    init(&f);out=tmpfile();assert(out);a=admission(&f,out);a.core_sha256="incorrect";assert(c6_linux_diplomacy_manager_capture(&a));assert(!f.reads&&!f.guards);fclose(out);
    init(&f);out=tmpfile();assert(out);a=admission(&f,out);a.main_tid=-1;assert(c6_linux_diplomacy_manager_capture(&a));assert(!f.reads&&!f.guards);fclose(out);
    init(&f);out=tmpfile();assert(out);a=admission(&f,out);a.core_base=UINTPTR_MAX-1;assert(c6_linux_diplomacy_manager_capture(&a));assert(!f.reads);fclose(out);
    init(&f);out=tmpfile();assert(out);a=admission(&f,out);a.stage="invalid\"stage";assert(c6_linux_diplomacy_manager_capture(&a));assert(!f.reads&&!f.guards);fclose(out);
    /* Exercise the default bounded reader against actual synthetic local memory. */
    init(&f);out=tmpfile();assert(out);a=admission(&f,out);a.read=NULL;
    a.core_base=(uintptr_t)f.globals-UINT64_C(0x3c052f8);a.player_manager=f.manager;
    u64(f.globals,0,(uintptr_t)f.globals);u64(f.globals,8,(uintptr_t)f.globals);
    u64(f.globals,24,(uintptr_t)f.globals+24);u64(f.globals,32,(uintptr_t)f.globals+24);
    vec(f.globals,56,(uintptr_t)f.action,0,12);
    vec(f.manager,8,(uintptr_t)f.slots,65,8);vec(f.manager,0x20,(uintptr_t)f.ids,65,8);vec(f.manager,0x3a8,(uintptr_t)f.ever,0,8);
    assert(!c6_linux_diplomacy_manager_capture(&a));fclose(out);
    puts("synthetic diplomacy capture checks passed; no original-code observation");return 0;
}
