/* In-memory interfaces for unchanged original Random::Write/Read.
   Request descriptors are immutable INPUT; +0x40 is the original no-op wait hook. */
#include <stdint.h>
#include <stddef.h>
#include <string.h>
typedef struct {void **vtable;uint8_t data[18];uint32_t pos,calls,waits,fault;} MemoryStream;
static uint64_t word(const void*p){uint64_t value;memcpy(&value,p,8);return value;}
static void __attribute__((ms_abi)) wait_noop(MemoryStream*s){s->waits++;}
static void __attribute__((ms_abi)) write_bytes(MemoryStream*s,const void*data,uint32_t size){
 s->calls++;
 if(size>18 || s->pos>18-size){s->fault=1;return;}
 memcpy(s->data+s->pos,data,size);s->pos+=size;
}
static void __attribute__((ms_abi)) read_bytes(MemoryStream*s,const void*request,void*dest,uint32_t size){
 s->calls++;
 if(word(request)!=UINT64_MAX||word((const char*)request+8)!=size||word((const char*)request+16)!=16||word((const char*)request+24)||word((const char*)request+32))s->fault=1;
 if(size>18 || s->pos>18-size){s->fault=1;return;}
 memcpy(dest,s->data+s->pos,size);s->pos+=size;
}
int c6_random_write(void*state,uint8_t out[18]){
 void*vtable[9]={0};vtable[5]=(void*)write_bytes;
 MemoryStream stream={.vtable=vtable};
 ((void(__attribute__((ms_abi))*)(void*,void*))(uintptr_t)0x1805fa390ULL)(state,&stream);
 if(stream.fault||stream.pos!=18||stream.calls!=6)return -1;
 memcpy(out,stream.data,18);return 0;
}
int c6_random_read(void*state,const uint8_t in[18],uint32_t size){
 if(size<18)return -2;
 if(in[17])return -3; /* Legacy vector branch is outside this bounded adapter. */
 void*vtable[9]={0};vtable[6]=(void*)read_bytes;vtable[8]=(void*)wait_noop;
 MemoryStream stream={.vtable=vtable};memcpy(stream.data,in,18);
 ((void(__attribute__((ms_abi))*)(void*,void*))(uintptr_t)0x1805fa190ULL)(state,&stream);
 return stream.fault||stream.pos!=18||stream.calls!=6||stream.waits!=6 ? -1:0;
}
