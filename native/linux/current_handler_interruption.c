#define _GNU_SOURCE
#include "current_handler_interruption.h"
#include <dlfcn.h>
#include <elf.h>
#include <fcntl.h>
#include <inttypes.h>
#include <link.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/uio.h>
#include <sys/sysmacros.h>
#include <unistd.h>

#define GAME_SHA "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
#define HOST_SHA "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4"
#define LIMIT 65536u
#define MANAGER_SLOTS 0x800u
#define QUEUE_BYTES 200u
#define NODE_BYTES 40u
_Static_assert(sizeof(uintptr_t)==8,"Linux x86-64 ABI");

/* Local SHA-256 hashes original file descriptors, never executes their code.
 * Bit operations are uint32_t and wrap as required by SHA-256. */
typedef struct {uint32_t h[8];uint64_t bytes;unsigned used;unsigned char block[64];} Sha;
static uint32_t rotr(uint32_t x,unsigned n){return (x>>n)|(x<<(32u-n));}
static void sha_block(Sha *s,const unsigned char *p) {
    static const uint32_t k[64]={
      0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
      0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
      0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
      0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
      0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
      0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
      0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
      0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2};
    uint32_t w[64]={0};
    for(unsigned i=0;i<16;i++)w[i]=((uint32_t)p[4*i]<<24)|((uint32_t)p[4*i+1]<<16)|((uint32_t)p[4*i+2]<<8)|p[4*i+3];
    for(unsigned i=16;i<64;i++){
        uint32_t x=w[i-15],y=w[i-2];
        w[i]=w[i-16]+(rotr(x,7)^rotr(x,18)^(x>>3))+w[i-7]+(rotr(y,17)^rotr(y,19)^(y>>10));
    }
    uint32_t a=s->h[0],b=s->h[1],c=s->h[2],d=s->h[3],e=s->h[4],f=s->h[5],g=s->h[6],h=s->h[7];
    for(unsigned i=0;i<64;i++){
        uint32_t t=h+(rotr(e,6)^rotr(e,11)^rotr(e,25))+((e&f)^(~e&g))+k[i]+w[i];
        uint32_t u=(rotr(a,2)^rotr(a,13)^rotr(a,22))+((a&b)^(a&c)^(b&c));
        h=g;g=f;f=e;e=d+t;d=c;c=b;b=a;a=t+u;
    }
    s->h[0]+=a;s->h[1]+=b;s->h[2]+=c;s->h[3]+=d;s->h[4]+=e;s->h[5]+=f;s->h[6]+=g;s->h[7]+=h;
}
static Sha sha_init(void){return (Sha){.h={0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19}};}
static void sha_add(Sha *s,const void *data,size_t size){
    const unsigned char *p=data;s->bytes+=size;
    while(size){size_t n=64u-s->used;if(n>size)n=size;memcpy(s->block+s->used,p,n);s->used+=(unsigned)n;p+=n;size-=n;
        if(s->used==64){sha_block(s,s->block);s->used=0;}}
}
static void sha_finish(Sha *s,char hex[65]){
    uint64_t bits=s->bytes*8u;unsigned char one=0x80,zero=0;sha_add(s,&one,1);
    while(s->used!=56)sha_add(s,&zero,1);
    unsigned char tail[8];for(unsigned i=0;i<8;i++)tail[7-i]=(unsigned char)(bits>>(8*i));sha_add(s,tail,8);
    for(unsigned i=0;i<8;i++)for(unsigned j=0;j<4;j++){
        unsigned v=(s->h[i]>>(24u-8u*j))&255u;hex[8*i+2*j]="0123456789abcdef"[v>>4];hex[8*i+2*j+1]="0123456789abcdef"[v&15u];}
    hex[64]=0;
}
static bool file_hash(int fd,const char *expected){
    Sha s=sha_init();unsigned char buf[32768];off_t pos=0;
    for(;;){ssize_t n=pread(fd,buf,sizeof buf,pos);if(n<0)return false;if(!n)break;sha_add(&s,buf,(size_t)n);pos+=n;}
    char hex[65];sha_finish(&s,hex);return !strcmp(hex,expected);
}
static bool digest(const char *s){
    if(!s||strlen(s)!=64)return false;
    for(size_t i=0;i<64;i++)if(!((s[i]>='0'&&s[i]<='9')||(s[i]>='a'&&s[i]<='f')))return false;
    return true;
}
static bool extent(uintptr_t p,size_t n){return p!=0&&n<=UINTPTR_MAX-p;}
/* Safe kernel copies reject inaccessible/null/overflowing extents rather than
 * crashing or inventing zero bytes. The source pointers must still be derived
 * from the retained exact owners/container membership below. */
static bool read_self(uintptr_t p,void *out,size_t n){
    if(!n)return true;
    if(!out||!extent(p,n))return false;
    struct iovec local={out,n},remote={(void *)p,n};
    return process_vm_readv(getpid(),&local,1,&remote,1,0)==(ssize_t)n;
}
static uintptr_t ptr(const unsigned char *p,size_t off){uintptr_t v;memcpy(&v,p+off,8);return v;}
static uint32_t u32(const unsigned char *p,size_t off){uint32_t v;memcpy(&v,p+off,4);return v;}
static int32_t i32(const unsigned char *p,size_t off){int32_t v;memcpy(&v,p+off,4);return v;}
static bool span(uintptr_t begin,uintptr_t end,size_t width,size_t limit,size_t *n){
    if(!width||end<begin||(end-begin)%width||(!begin&&end))return false;
    uintptr_t count=(end-begin)/width;if(count>limit)return false;*n=(size_t)count;return true;
}
static void raw(FILE *out,const unsigned char *p,size_t n){
    fputc('"',out);for(size_t i=0;i<n;i++)fprintf(out,"%02x",p[i]);fputc('"',out);
}
typedef struct {uintptr_t lo,hi;uint64_t offset;uint32_t flags;} Segment;
typedef struct {uintptr_t base;int fd;size_t count;Segment segments[16];} Image;
static int image_segments(struct dl_phdr_info *info,size_t size,void *data){
    (void)size;Image *im=data;if(info->dlpi_addr!=im->base)return 0;
    Elf64_Ehdr elf;
    if(pread(im->fd,&elf,sizeof elf,0)!=(ssize_t)sizeof elf||memcmp(elf.e_ident,"\177ELF\002\001",6)||
       elf.e_type!=ET_DYN||elf.e_machine!=EM_X86_64||elf.e_phentsize!=sizeof(Elf64_Phdr)||elf.e_phnum!=info->dlpi_phnum){im->count=0;return 1;}
    for(size_t i=0;i<info->dlpi_phnum;i++)if(info->dlpi_phdr[i].p_type==PT_LOAD){
        const ElfW(Phdr) *h=&info->dlpi_phdr[i];Elf64_Phdr expected;
        uint64_t off=elf.e_phoff+i*sizeof expected;
        if(off>INT64_MAX||pread(im->fd,&expected,sizeof expected,(off_t)off)!=(ssize_t)sizeof expected||memcmp(h,&expected,sizeof expected)){im->count=0;return 1;}
        if(im->count==16||h->p_vaddr>UINTPTR_MAX-im->base||h->p_filesz>UINTPTR_MAX-im->base-h->p_vaddr){im->count=0;return 1;}
        im->segments[im->count++]=(Segment){im->base+h->p_vaddr,im->base+h->p_vaddr+h->p_filesz,h->p_offset,h->p_flags};
    }
    return 1;
}
static const Segment *segment(const Image *im,uintptr_t p,size_t n,unsigned flags){
    if(!extent(p,n))return NULL;
    for(size_t i=0;i<im->count;i++)if(p>=im->segments[i].lo&&p+n<=im->segments[i].hi&&(im->segments[i].flags&flags)==flags)return &im->segments[i];
    return NULL;
}
/* ELF PT_LOAD flags alone do not prove current mprotect state or file owner. */
static bool runtime_code_mapping(const Image *im,uintptr_t p,size_t n){
    const Segment *s=segment(im,p,n,PF_R|PF_X);struct stat file;
    if(!s||fstat(im->fd,&file))return false;
    FILE *maps=fopen("/proc/self/maps","r");if(!maps)return false;
    char line[4096],permissions[5];bool found=false;unsigned major_id,minor_id;
    unsigned long long lo,hi,offset,inode;
    while(fgets(line,sizeof line,maps)){
        if(sscanf(line,"%llx-%llx %4s %llx %x:%x %llu",&lo,&hi,permissions,&offset,&major_id,&minor_id,&inode)!=7)continue;
        if(p<lo||p>=hi||n>hi-p)continue;
        found=permissions[0]=='r'&&permissions[2]=='x'&&inode==(uintmax_t)file.st_ino&&
            major_id==major(file.st_dev)&&minor_id==minor(file.st_dev)&&
            offset+p-lo==s->offset+p-s->lo;
        break;
    }
    if(fclose(maps))return false;
    return found;
}
static bool original_code(const Image *im,uintptr_t p,size_t n,unsigned char *live){
    const Segment *s=segment(im,p,n,PF_R|PF_X);unsigned char expected[128];
    if(!s||n>sizeof expected||!read_self(p,live,n))return false;
    uint64_t off=s->offset+p-s->lo;if(off>INT64_MAX)return false;
    return pread(im->fd,expected,n,(off_t)off)==(ssize_t)n&&!memcmp(live,expected,n);
}
static void idle(const C6CurrentHandlerInterruptionAdmission *a){
    if(a->quiescent(a->module,a->borrowed_context,a->retained_game)){
        fputs("C6_LINUX_CURRENT_HANDLER_INTERRUPTION quiescence_unknown no_poststate_or_cleanup\n",stderr);fflush(stderr);_exit(240);
    }
}
/* No virtual is invoked: capture its actual slot, require mapped original owner
 * and compare complete live code to the independently SHA-gated file image. */
static bool method(FILE *out,const Image *im,uintptr_t table,size_t offset,uintptr_t rva,size_t size){
    uintptr_t actual;unsigned char code[128];
    if(offset>SIZE_MAX-8||!extent(table,offset+8)||!segment(im,table+offset,8,PF_R)||!read_self(table+offset,&actual,8)||actual!=im->base+rva||!runtime_code_mapping(im,actual,size)||!original_code(im,actual,size,code))return false;
    Dl_info owner={0};if(!dladdr((void *)actual,&owner)||owner.dli_fbase!=(void *)im->base)return false;
    fprintf(out,"{\"slot_offset\":%zu,\"actual_pointer\":%" PRIuPTR ",\"owner_base\":%" PRIuPTR ",\"rva\":%" PRIuPTR ",\"runtime_mapping_readable_executable\":true,\"runtime_mapping_matches_hashed_file\":true,\"live_code_raw\":",offset,actual,(uintptr_t)owner.dli_fbase,rva);raw(out,code,size);fputc('}',out);return true;
}
typedef struct {uintptr_t pointer,target,slot_address;unsigned char raw[16];} HandlerSnapshot;
static int registry(FILE *out,const Image *im,uintptr_t game,uintptr_t *manager_out,HandlerSnapshot **handlers_out,size_t *count_out){
    unsigned char vector[24],index_raw[4],manager[104];size_t count,capacity;
    if(!extent(game,32)||!read_self(im->base+0x3c00684,index_raw,4)||!read_self(game+8,vector,sizeof vector))return 244;
    uintptr_t begin=ptr(vector,0);uint32_t index=u32(index_raw,0);
    if(!span(begin,ptr(vector,8),288,4096,&count)||!span(begin,ptr(vector,16),288,4096,&capacity)||count>capacity||index>=count)return 244;
    uintptr_t current;if(!read_self(begin+(size_t)index*288,&current,8)||!read_self(current,manager,sizeof manager)||ptr(manager,0)!=im->base+0x3aff880)return 244;
    fprintf(out,"{\"kind\":\"manager\",\"pointer\":%" PRIuPTR ",\"component_index\":%u,\"component_vector_raw\":",current,index);raw(out,vector,sizeof vector);
    fputs(",\"manager_raw\":",out);raw(out,manager,sizeof manager);fputs(",\"live_methods\":[",out);
    const size_t offsets[]={0x88,0x98,0xa0,0xd8},sizes[]={82,40,86,116};
    const uintptr_t targets[]={0x2da1650,0x2da16f0,0x2da1720,0x2da2a60};
    for(size_t i=0;i<4;i++){if(i)fputc(',',out);if(!method(out,im,ptr(manager,0),offsets[i],targets[i],sizes[i]))return 244;}
    fputs("]}\n",out);
    begin=ptr(manager,24);
    if(!span(begin,ptr(manager,32),8,MANAGER_SLOTS,&count)||!span(begin,ptr(manager,40),8,MANAGER_SLOTS,&capacity)||count>capacity||!count)return 244;
    HandlerSnapshot *handlers=calloc(count,sizeof *handlers);if(!handlers)return 245;

    for(size_t i=0;i<count;i++){
        unsigned char h[16];
        handlers[i].slot_address=begin+i*8;
        if(!read_self(handlers[i].slot_address,&handlers[i].pointer,8)){free(handlers);return 245;}
        fprintf(out,"{\"kind\":\"handler\",\"slot\":%zu,\"pointer\":%" PRIuPTR,i,handlers[i].pointer);
        if(handlers[i].pointer){
            if(!read_self(handlers[i].pointer,h,sizeof h)||i32(h,8)!=(int32_t)i){free(handlers);return 245;}
            uintptr_t table=ptr(h,0),actual;
            if(!segment(im,table,0x28,PF_R)||!read_self(table+0x20,&actual,8)||actual<im->base){free(handlers);return 245;}
            memcpy(handlers[i].raw,h,sizeof h);handlers[i].target=actual;
            uintptr_t rva=actual-im->base;if(rva!=0x2d95660&&rva!=0x2da0920){free(handlers);return 245;}
            fprintf(out,",\"runtime_id\":%d,\"operation_hash\":%u,\"vptr\":%" PRIuPTR ",\"handler_raw\":",i32(h,8),u32(h,12),table);raw(out,h,sizeof h);fputs(",\"actual_method\":",out);
            if(!method(out,im,table,0x20,rva,3)){free(handlers);return 245;}
        }
        fputs("}\n",out);
    }
    *manager_out=current;*handlers_out=handlers;*count_out=count;return 0;
}
static bool queue_shape(uintptr_t address,const unsigned char q[QUEUE_BYTES],size_t *allocation,uintptr_t *block,bool *inlined){
    if(!extent(address,QUEUE_BYTES))return false;
    uint32_t count=u32(q,192),capacity=u32(q,184);uintptr_t p=ptr(q,176);
    if(count>capacity||capacity>LIMIT||(!p&&capacity)||(capacity&&!extent(p,(size_t)capacity*NODE_BYTES)))return false;
    *allocation=(size_t)capacity*NODE_BYTES;*block=p;
    *inlined=p>=address&&p<address+176;
    if(*inlined&&(*allocation>176||p-address>176-*allocation))return false;
    if(*allocation&&!(*inlined)&&(p<address+QUEUE_BYTES&&address<p+*allocation))return false;
    return true;
}
static int unit_queue(FILE *out,const Image *im,uintptr_t unit,int32_t owner,uint32_t id,uintptr_t native_manager,const HandlerSnapshot *handlers,size_t handler_count){
    if(!extent(unit,0xc10+QUEUE_BYTES))return 247;
    unsigned char queue[QUEUE_BYTES];uintptr_t address=unit+0xc10,block;size_t allocation;bool inlined;
    if(!read_self(address,queue,sizeof queue)||!queue_shape(address,queue,&allocation,&block,&inlined))return 247;
    unsigned char *storage=NULL;
    if(allocation){storage=malloc(allocation);if(!storage||!read_self(block,storage,allocation)){free(storage);return 247;}}
    unsigned char queue_after[QUEUE_BYTES];
    if(!read_self(address,queue_after,sizeof queue_after)||memcmp(queue,queue_after,sizeof queue)||
       (inlined&&allocation&&memcmp(storage,queue+(block-address),allocation))){free(storage);return 247;}
    uint32_t count=u32(queue,192);
    fprintf(out,"{\"kind\":\"unit_queue\",\"owner\":%d,\"id\":%u,\"unit\":%" PRIuPTR ",\"queue\":%" PRIuPTR ",\"count\":%u,\"capacity\":%u,\"block\":%" PRIuPTR ",\"inline\":%s,\"queue_raw\":",owner,id,unit,address,count,u32(queue,184),block,inlined?"true":"false");raw(out,queue,sizeof queue);
    fputs(",\"block_allocation_raw\":",out);raw(out,storage,allocation);
    if(count){
        uint32_t current=u32(storage,0);
        if(current>=MANAGER_SLOTS||current>=handler_count){free(storage);return 247;}
        unsigned char current_vector[24];uintptr_t actual;size_t current_count,current_capacity;
        if(!extent(native_manager,48)||!read_self(native_manager+24,current_vector,sizeof current_vector)||
           !span(ptr(current_vector,0),ptr(current_vector,8),8,MANAGER_SLOTS,&current_count)||current_count!=handler_count||
           !span(ptr(current_vector,0),ptr(current_vector,16),8,MANAGER_SLOTS,&current_capacity)||current_count>current_capacity||
           ptr(current_vector,0)+(size_t)current*8!=handlers[current].slot_address||
           !read_self(handlers[current].slot_address,&actual,8)||actual!=handlers[current].pointer){free(storage);return 247;}
        fprintf(out,",\"native_front_bits\":%u,\"current_handler\":%" PRIuPTR,current,handlers[current].pointer);
        if(handlers[current].pointer){
            unsigned char h[16];if(!read_self(handlers[current].pointer,h,sizeof h)||i32(h,8)!=(int32_t)current||memcmp(h,handlers[current].raw,sizeof h)){free(storage);return 247;}
            uintptr_t target;if(!segment(im,ptr(h,0)+0x20,8,PF_R)||!read_self(ptr(h,0)+0x20,&target,8)||target!=handlers[current].target){free(storage);return 247;}
            fprintf(out,",\"current_handler_hash\":%u,\"current_handler_vptr\":%" PRIuPTR ",\"current_actual_target\":%" PRIuPTR,u32(h,12),ptr(h,0),target);
        }
    }
    fputs("}\n",out);free(storage);return 0;
}
static int players(FILE *out,const Image *im,const C6CurrentHandlerInterruptionAdmission *a,uintptr_t native_manager,const HandlerSnapshot *handlers,size_t handler_count){
    uintptr_t game=(uintptr_t)a->retained_game,manager;
    unsigned char header[7416];
    if(!extent(game,0x250)||!read_self(game+0x248,&manager,8)||!read_self(manager,header,sizeof header))return 246;
    uintptr_t begin=ptr(header,0x20),map=ptr(header,0x1a08);size_t n,cap,map_n,map_cap;
    if(!span(begin,ptr(header,0x28),8,65,&n)||n!=65||!span(begin,ptr(header,0x30),8,65,&cap)||n>cap||
       !span(map,ptr(header,0x1a10),4,65,&map_n)||map_n!=65||!span(map,ptr(header,0x1a18),4,65,&map_cap)||map_n>map_cap)return 246;
    uintptr_t stored[65];int32_t mappings[65];
    if(!read_self(begin,stored,sizeof stored)||!read_self(map,mappings,sizeof mappings))return 246;
    size_t player_count=0,total=0;
    fprintf(out,"{\"kind\":\"player_manager\",\"pointer\":%" PRIuPTR ",\"header_raw\":",manager);raw(out,header,sizeof header);fputs("}\n",out);
    for(int32_t p=0;p<65;p++){
        idle(a);uintptr_t player=mappings[p]!=-1?stored[p]:0;
        fprintf(out,"{\"kind\":\"player_slot\",\"query\":%d,\"mapping\":%d,\"stored\":%" PRIuPTR ",\"resolved\":%" PRIuPTR "}\n",p,mappings[p],stored[p],player);
        if(!player)continue;
        player_count++;unsigned char player_raw[0x6d8],units_raw[232];
        if(!read_self(player,player_raw,sizeof player_raw)||i32(player_raw,0xe0)!=p)return 246;
        uintptr_t units=ptr(player_raw,0x6d0);
        if(!read_self(units,units_raw,sizeof units_raw)||ptr(units_raw,152)!=player)return 246;
        const unsigned char *list=units_raw+160;uint32_t count=u32(list,32);size_t blocks,block_cap;
        uintptr_t block_begin=ptr(list,0),entry=ptr(list,56),tail=ptr(list,64),previous=0;
        if(count>LIMIT||!span(block_begin,ptr(list,8),8,8192,&blocks)||!span(block_begin,ptr(list,16),8,8192,&block_cap)||blocks>block_cap||blocks*8<count)return 246;
        unsigned char seen[LIMIT/8]={0};
        fprintf(out,"{\"kind\":\"unit_container\",\"owner\":%d,\"pointer\":%" PRIuPTR ",\"parent\":%" PRIuPTR ",\"container_raw\":",p,units,player);raw(out,list,72);fputs("}\n",out);
        for(uint32_t ordinal=0;ordinal<count;ordinal++){
            idle(a);unsigned char link[24],unit_raw[0x134];
            if(!read_self(entry,link,sizeof link)||ptr(link,8)!=previous)return 246;
            uintptr_t unit=ptr(link,0);
            if(!read_self(unit,unit_raw,sizeof unit_raw)||ptr(unit_raw,0)!=im->base+0x3ae5f80||i32(unit_raw,0x130)!=p)return 246;
            uint32_t id=u32(unit_raw,0xb8),slot=id&65535u;uintptr_t block_pointer;
            if(id==UINT32_MAX||slot/8>=blocks||seen[slot/8]&(1u<<(slot%8))||
               !read_self(block_begin+(slot/8)*8,&block_pointer,8)||!extent(block_pointer,192)||entry!=block_pointer+(slot%8)*24)return 246;
            seen[slot/8]|=(unsigned char)(1u<<(slot%8));
            fprintf(out,"{\"kind\":\"unit_membership\",\"owner\":%d,\"ordinal\":%u,\"id\":%u,\"unit\":%" PRIuPTR ",\"entry\":%" PRIuPTR ",\"block\":%" PRIuPTR ",\"entry_raw\":",p,ordinal,id,unit,entry,block_pointer);raw(out,link,sizeof link);fputs("}\n",out);
            int rc=unit_queue(out,im,unit,p,id,native_manager,handlers,handler_count);if(rc)return rc;
            previous=entry;entry=ptr(link,16);
        }
        if(entry||previous!=tail)return 246;
        total+=count;
    }
    fprintf(out,"{\"kind\":\"roster_summary\",\"player_slots\":65,\"players\":%zu,\"units\":%zu}\n",player_count,total);return 0;
}
int c6_linux_current_handler_interruption_capture(const C6CurrentHandlerInterruptionAdmission *a){
    if(!a||!a->module||!a->borrowed_context||!a->retained_game||!a->quiescent||!digest(a->profile_sha256)||!a->output_path||!*a->output_path)return 241;
    /* No original symbol is called, including this identity anchor. The caller
     * quiescence callback is separately admitted and may use proven const getters. */
    void *anchor=dlsym(a->module,"_ZN8GameCore4Unit10Operations7Manager3GetEv");Dl_info info={0};
    if(!anchor||!dladdr(anchor,&info)||!info.dli_fbase||!info.dli_fname)return 242;
    Image im={.base=(uintptr_t)info.dli_fbase,.fd=-1};
    if((uintptr_t)anchor!=im.base+0x2da1260)return 242;
    im.fd=open(info.dli_fname,O_RDONLY|O_CLOEXEC|O_NOFOLLOW);int host=open("/proc/self/exe",O_RDONLY|O_CLOEXEC);
    bool identities=im.fd>=0&&host>=0&&file_hash(im.fd,GAME_SHA)&&file_hash(host,HOST_SHA);
    if(host>=0)close(host);
    if(!identities){if(im.fd>=0)close(im.fd);return 242;}
    dl_iterate_phdr(image_segments,&im);if(!im.count){close(im.fd);return 242;}
    idle(a);uintptr_t game;
    if(!read_self(im.base+0x3c051a8,&game,8)||game!=(uintptr_t)a->retained_game){close(im.fd);return 243;}
    FILE *out=fopen(a->output_path,"wx");if(!out){close(im.fd);return 243;}
    struct stat identity;if(fstat(im.fd,&identity)){fclose(out);close(im.fd);return 243;}
    fprintf(out,"{\"kind\":\"profile\",\"schema\":\"c6-linux-current-handler-interruption-metadata-v1\",\"profile_sha256\":\"%s\",\"gamecore_sha256\":\"%s\",\"host_sha256\":\"%s\",\"module_base\":%" PRIuPTR ",\"file_device\":%" PRIuMAX ",\"file_inode\":%" PRIuMAX ",\"context\":%" PRIuPTR ",\"game\":%" PRIuPTR ",\"helper_original_calls\":false,\"caller_quiescence_callback_invoked\":true,\"predicate_results\":false}\n",a->profile_sha256,GAME_SHA,HOST_SHA,im.base,(uintmax_t)identity.st_dev,(uintmax_t)identity.st_ino,(uintptr_t)a->borrowed_context,game);
    uintptr_t manager=0;HandlerSnapshot *handlers=NULL;size_t count=0;
    int rc=registry(out,&im,game,&manager,&handlers,&count);
    if(!rc)rc=players(out,&im,a,manager,handlers,count);
    if(!rc){idle(a);uintptr_t after;if(!read_self(im.base+0x3c051a8,&after,8)||after!=game||!file_hash(im.fd,GAME_SHA))rc=248;}
    if(!rc)fprintf(out,"{\"kind\":\"complete\",\"manager\":%" PRIuPTR ",\"handler_slots\":%zu,\"player_slots\":65}\n",manager,count);
    free(handlers);if(ferror(out))rc=249;if(fclose(out))rc=249;close(im.fd);return rc;
}
