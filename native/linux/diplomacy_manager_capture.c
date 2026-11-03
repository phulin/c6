#define _GNU_SOURCE
#include "diplomacy_manager_capture.h"
#include <inttypes.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

#define TARGET "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
#define MAX_NODES 4096u
#define MAX_ITEMS 65536u
#define MAX_BYTES (8u*1024u*1024u)
_Static_assert(sizeof(uintptr_t)==8,"Linux x86-64 capture only");
typedef struct { const C6DiplomacyCaptureAdmission *a; size_t bytes,items; const char *error; } Capture;
typedef struct { uint64_t prev,next,count; } List;
typedef struct { uint64_t begin,end,capacity; } Vector;
/* Session uses EASTL ListBase, unlike process-global libc++ Session lists. */
typedef struct { uint64_t next,prev,allocator; } SessionPendingHeader;
_Static_assert(sizeof(List)==24&&sizeof(Vector)==24&&sizeof(SessionPendingHeader)==24,"source header sizes");
static int fail(Capture *c,const char *reason){if(!c->error)c->error=reason;return -1;}
static int hash(const char *s){if(!s||strlen(s)!=64)return 0;for(size_t i=0;i<64;i++)if(!strchr("0123456789abcdef",s[i]))return 0;return 1;}
static int stage(const char *s){
    if(!s)return 0;
    size_t n=strnlen(s,65);if(!n||n>64)return 0;
    for(size_t i=0;i<n;i++)if(!((s[i]>='a'&&s[i]<='z')||(s[i]>='A'&&s[i]<='Z')||(s[i]>='0'&&s[i]<='9')||s[i]=='_'||s[i]=='.'||s[i]=='-'))return 0;
    return 1;
}
static int guard(Capture *c){
    if((long)syscall(SYS_gettid)!=c->a->main_tid)return fail(c,"main_thread");
    if(c->a->retained_owner(c->a->module,c->a->context,c->a->game))return fail(c,"owner_admission");
    return 0;
}
static int add(Capture *c,uintptr_t p,size_t off,uintptr_t *out){
    if(!p||off>UINTPTR_MAX-p)return fail(c,"pointer_extent");
    *out=p+off;return 0;
}
static int read_field(Capture *c,uintptr_t p,size_t off,void *out,size_t n){
    uintptr_t at;if(guard(c)||add(c,p,off,&at))return -1;
    if(n>UINTPTR_MAX-at)return fail(c,"pointer_extent");
    if(n>c->a->max_bytes-c->bytes)return fail(c,"byte_cap");
    c->bytes+=n;
    if(c->a->read){if(c->a->read(c->a->read_context,at,out,n))return fail(c,"unreadable");}
    else {struct iovec local={out,n},remote={(void *)at,n};if(process_vm_readv(getpid(),&local,1,&remote,1,0)!=(ssize_t)n)return fail(c,"unreadable");}
    return 0;
}
static int item(Capture *c){if(c->items==c->a->max_items)return fail(c,"item_cap");c->items++;return 0;}
static void raw(FILE *out,const void *p,size_t n){const unsigned char *b=p;fputc('"',out);for(size_t i=0;i<n;i++)fprintf(out,"%02x",b[i]);fputc('"',out);}
static int session(Capture *c,uintptr_t p){
    int32_t endpoints[2];SessionPendingHeader pending;
    if(!p)return fail(c,"null_session");
    if(item(c)||read_field(c,p,0,endpoints,sizeof endpoints)||read_field(c,p,0x28,&pending,sizeof pending))return -1;
    fprintf(c->a->output,"{\"kind\":\"session\",\"pointer\":\"0x%"PRIxPTR"\",\"from\":%"PRId32",\"to\":%"PRId32",\"pending_header_raw\":",p,endpoints[0],endpoints[1]);raw(c->a->output,&pending,sizeof pending);
    fprintf(c->a->output,",\"pending_prev\":\"0x%"PRIx64"\",\"pending_next\":\"0x%"PRIx64"\",\"pending_allocator_raw\":\"0x%"PRIx64"\",\"pending_count\":\"unknown_not_captured\",\"statement_payload\":\"unknown\",\"pending_items\":\"not_captured_layout_unclosed\"}\n",pending.prev,pending.next,pending.allocator);
    return 0;
}
static int list(Capture *c,uintptr_t address,const char *name){
    List h,node;uintptr_t seen[MAX_NODES],cur,prev=address;
    if(read_field(c,address,0,&h,sizeof h))return -1;
    fprintf(c->a->output,"{\"kind\":\"list_header\",\"container\":\"%s\",\"address\":\"0x%"PRIxPTR"\",\"prev\":\"0x%"PRIx64"\",\"next\":\"0x%"PRIx64"\",\"count\":%"PRIu64",\"raw\":",name,address,h.prev,h.next,h.count);raw(c->a->output,&h,sizeof h);fputs("}\n",c->a->output);
    if(h.count>MAX_NODES)return fail(c,"list_count_cap");
    cur=h.next;
    for(size_t i=0;i<h.count;i++){
        if(cur==address)return fail(c,"list_short_count");
        for(size_t j=0;j<i;j++)if(seen[j]==cur)return fail(c,"list_cycle");
        seen[i]=cur;
        if(item(c)||read_field(c,cur,0,&node,sizeof node))return -1;
        if(node.prev!=prev)return fail(c,"list_prev_link");
        fprintf(c->a->output,"{\"kind\":\"list_item\",\"container\":\"%s\",\"ordinal\":%zu,\"node\":\"0x%"PRIxPTR"\",\"prev\":\"0x%"PRIx64"\",\"next\":\"0x%"PRIx64"\",\"session\":\"0x%"PRIx64"\"}\n",name,i,cur,node.prev,node.next,node.count);
        if(session(c,node.count))return -1;
        prev=cur;cur=node.next;
    }
    if(cur!=address||h.prev!=prev)return fail(c,"list_count_or_tail");
    return 0;
}
static int vector(Capture *c,uintptr_t owner,size_t offset,const char *name,size_t stride,size_t limit,Vector *v,size_t *count){
    uintptr_t address;
    if(add(c,owner,offset,&address)||read_field(c,address,0,v,sizeof *v))return -1;
    fprintf(c->a->output,"{\"kind\":\"vector_header\",\"container\":\"%s\",\"address\":\"0x%"PRIxPTR"\",\"begin\":\"0x%"PRIx64"\",\"end\":\"0x%"PRIx64"\",\"capacity\":\"0x%"PRIx64"\",\"raw\":",name,address,v->begin,v->end,v->capacity);raw(c->a->output,v,sizeof *v);fputs("}\n",c->a->output);
    if(v->end<v->begin||v->capacity<v->end||(!v->begin&&(v->end||v->capacity))||(v->end-v->begin)%stride||(v->capacity-v->begin)%stride)return fail(c,"vector_span");
    if((v->end-v->begin)/stride>limit||(v->capacity-v->begin)/stride>MAX_ITEMS)return fail(c,"vector_count_cap");
    *count=(v->end-v->begin)/stride;return 0;
}
static int player(Capture *c,const char *name,size_t index,uintptr_t p){
    int32_t id=0;unsigned char alive=0;
    if(item(c))return -1;
    if(p&&(read_field(c,p,0xe0,&id,sizeof id)||read_field(c,p,0x278,&alive,1)))return -1;
    fprintf(c->a->output,"{\"kind\":\"player\",\"container\":\"%s\",\"ordinal\":%zu,\"pointer\":\"0x%"PRIxPTR"\"",name,index,p);
    if(p)fprintf(c->a->output,",\"id\":%"PRId32",\"was_ever_alive_raw\":%u,\"is_ai\":\"unknown_configuration_schema\"",id,(unsigned)alive);
    fputs("}\n",c->a->output);return 0;
}
static int players(Capture *c,size_t offset,const char *name,size_t limit){
    Vector v;size_t n;
    if(vector(c,(uintptr_t)c->a->player_manager,offset,name,8,limit,&v,&n))return -1;
    for(size_t i=0;i<n;i++){uintptr_t p;if(read_field(c,v.begin,i*8,&p,8)||player(c,name,i,p))return -1;}
    return 0;
}
int c6_linux_diplomacy_manager_capture(const C6DiplomacyCaptureAdmission *a){
    if(!a||!a->output)return -1;
    Capture c={a,0,0,NULL};
    int valid=a->module&&a->context&&a->game&&a->core_base&&a->player_manager&&a->retained_owner&&stage(a->stage)&&hash(a->profile_sha256)&&a->core_sha256&&!strcmp(a->core_sha256,TARGET)&&a->max_bytes&&a->max_bytes<=MAX_BYTES&&a->max_items&&a->max_items<=MAX_ITEMS;
    fputs("{\"kind\":\"snapshot_begin\",\"schema\":\"c6-linux-diplomacy-capture-v1\",\"semantic_complete\":false,\"gameplay_parity\":false",a->output);
    if(valid)fprintf(a->output,",\"core_sha256\":\"%s\",\"profile_sha256\":\"%s\",\"core_base\":\"0x%"PRIxPTR"\",\"player_manager\":\"0x%"PRIxPTR"\",\"main_tid\":%ld",TARGET,a->profile_sha256,a->core_base,(uintptr_t)a->player_manager,a->main_tid);
    if(valid)fprintf(a->output,",\"stage\":\"%s\",\"module\":\"0x%"PRIxPTR"\",\"context\":\"0x%"PRIxPTR"\",\"game\":\"0x%"PRIxPTR"\"",a->stage,(uintptr_t)a->module,(uintptr_t)a->context,(uintptr_t)a->game);
    fputs("}\n",a->output);
    if(!valid){fail(&c,"admission_fields");goto end;}
    if(guard(&c))goto end;
    uintptr_t open,closed,active,pending;
    if(add(&c,a->core_base,0x3c052f8,&open)||add(&c,a->core_base,0x3c05310,&closed)||add(&c,a->core_base,0x3c05328,&active)||add(&c,a->core_base,0x3c05330,&pending))goto end;
    if(list(&c,open,"open")||list(&c,closed,"closed"))goto end;
    uintptr_t active_session;if(read_field(&c,active,0,&active_session,8))goto end;
    fprintf(a->output,"{\"kind\":\"active_session\",\"pointer\":\"0x%"PRIxPTR"\"}\n",active_session);
    if(active_session&&session(&c,active_session))goto end;
    Vector v;size_t n;uint64_t allocator;
    if(vector(&c,pending,0,"pending_actions",12,MAX_ITEMS,&v,&n)||read_field(&c,pending,24,&allocator,8))goto end;
    fprintf(a->output,"{\"kind\":\"pending_allocator\",\"raw\":\"0x%"PRIx64"\"}\n",allocator);
    for(size_t i=0;i<n;i++){
        struct {uint32_t id;int32_t from,to;} action;
        if(item(&c)||read_field(&c,v.begin,i*12,&action,sizeof action))goto end;
        fprintf(a->output,"{\"kind\":\"pending_action\",\"ordinal\":%zu,\"id\":%"PRIu32",\"from\":%"PRId32",\"to\":%"PRId32",\"raw\":",i,action.id,action.from,action.to);raw(a->output,&action,sizeof action);fputs("}\n",a->output);
    }
    if(players(&c,0x3a8,"ever_alive",MAX_NODES)||players(&c,8,"allocation_slots",65)||players(&c,0x20,"id_slots",65))goto end;
end:
    if(valid)guard(&c);
    if(ferror(a->output)&&!c.error)fail(&c,"output_error");
    fprintf(a->output,"{\"kind\":\"snapshot_end\",\"complete\":%s,\"read_bytes\":%zu,\"items\":%zu,\"failure\":",c.error?"false":"true",c.bytes,c.items);
    if(c.error)fprintf(a->output,"\"%s\"",c.error);else fputs("null",a->output);
    fputs("}\n",a->output);
    return c.error||ferror(a->output)?-1:0;
}
