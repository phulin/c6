#define _GNU_SOURCE
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <errno.h>

/* Only invoked by the SHA256-gating Python front end. No DllMain/import binding. */
static unsigned char *image;
typedef struct { uint64_t vptr; uint32_t seed,calls,resets,lock_count; uint8_t synchronous; uint8_t padding[7]; } Random;
_Static_assert(sizeof(Random)==32,"Random ABI size");
_Static_assert(offsetof(Random,seed)==8 && offsetof(Random,synchronous)==24,"Random ABI offsets");
static void * __attribute__((ms_abi)) no_log(void *self, uint32_t category) { (void)self;(void)category;return NULL; }
static void __attribute__((ms_abi)) no_op(void) {}
static void *logger_vtable[11];
static void **logger=logger_vtable;
static uint16_t u16(const unsigned char *p){uint16_t x;memcpy(&x,p,2);return x;}
static uint32_t u32(const unsigned char *p){uint32_t x;memcpy(&x,p,4);return x;}
static uint64_t u64(const unsigned char *p){uint64_t x;memcpy(&x,p,8);return x;}
int c6_load(const char *path) {
 if(image) return -1;
 FILE *f=fopen(path,"rb"); if(!f)return -2;
 if(fseek(f,0,SEEK_END)) {fclose(f);return -3;}
 long length=ftell(f);rewind(f);
 if(length<4096){fclose(f);return -4;}
 unsigned char *raw=malloc((size_t)length);if(!raw){fclose(f);return -5;}
 if(fread(raw,1,length,f)!=(size_t)length){free(raw);fclose(f);return -6;}fclose(f);
 uint32_t pe=u32(raw+0x3c);
 if(pe>(uint32_t)length-264 || memcmp(raw+pe,"PE\0\0",4)||u16(raw+pe+4)!=0x8664 ||u16(raw+pe+24)!=0x20b){free(raw);return -7;}
 unsigned char *opt=raw+pe+24;
 uint64_t base=u64(opt+24);uint32_t size=u32(opt+56),headers=u32(opt+60);
 if(base!=0x180000000ULL||size!=0xc60000||headers>(uint32_t)length){free(raw);return -8;}
 unsigned char *mapped=mmap((void*)(uintptr_t)base,size,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0);
 if(mapped==MAP_FAILED){free(raw);return -9;}
 memcpy(mapped,raw,headers);
 size_t sec=pe+24+u16(raw+pe+20);unsigned count=u16(raw+pe+6);
 for(unsigned i=0;i<count;i++,sec+=40){
  if(sec+40>(size_t)length)goto bad;
  uint32_t va=u32(raw+sec+12),n=u32(raw+sec+16),off=u32(raw+sec+20);
  if((uint64_t)va+n>size||(uint64_t)off+n>(uint64_t)length)goto bad;
  memcpy(mapped+va,raw+off,n);
 }
 /* Permit execution only of the original .text section. All other pages stay NX. */
 sec=pe+24+u16(raw+pe+20);
 for(unsigned i=0;i<count;i++,sec+=40)if(!memcmp(raw+sec,".text",5)){
  uint32_t va=u32(raw+sec+12),n=u32(raw+sec+8);
  if(va%4096 ||(uint64_t)va+((n+4095UL)&~4095UL)>size ||mprotect(mapped+va,(n+4095UL)&~4095UL,PROT_READ|PROT_EXEC))goto bad;
 }
 free(raw);image=mapped;
 for(int i=0;i<11;i++)logger_vtable[i]=(void*)no_op;
 logger_vtable[10]=(void*)no_log;
 *(void**)(image+0xb8aa90)=&logger;
 return 0;
 bad:munmap(mapped,size);free(raw);return -10;
}
void c6_construct(Random *r){((void*(__attribute__((ms_abi))*)(Random*))(image+0x5f9ee0))(r);}
uint16_t c6_get(Random *r,uint32_t bound){return ((uint16_t(__attribute__((ms_abi))*)(Random*,uint16_t,const char*))(image+0x5fa040))(r,(uint16_t)bound,NULL);}
void c6_reset(Random*r,uint32_t seed){((void(__attribute__((ms_abi))*)(Random*,uint32_t))(image+0x5fa370))(r,seed);}
void c6_initialize(Random*r,uint32_t seed){((void(__attribute__((ms_abi))*)(Random*,uint32_t))(image+0x5fa120))(r,seed);}
void c6_lock(Random*r){((void(__attribute__((ms_abi))*)(Random*))(image+0x5fa180))(r);}
void c6_unlock(Random*r){((void(__attribute__((ms_abi))*)(Random*))(image+0x5fa380))(r);}
/* Synthetic map contains only the two fields this original method reads. */
int32_t c6_plot_distance(int32_t width,uint8_t wrap,int32_t x1,int32_t y1,int32_t x2,int32_t y2){
 unsigned char map[0x48]={0};memcpy(map+0x28,&width,4);map[0x40]=wrap;
 return ((int32_t(__attribute__((ms_abi))*)(void*,int32_t,int32_t,int32_t,int32_t))(image+0x15ec0))(map,x1,y1,x2,y2);
}
uint8_t c6_plot_valid(int32_t x,int32_t y){
 int32_t coord[2]={x,y};return ((uint8_t(__attribute__((ms_abi))*)(void*))(image+0x315b80))(coord);
}
#ifndef C6_SOURCE_SHA256
#error Build through Makefile to embed source identity
#endif
const char *c6_host_source_sha256(void){return C6_SOURCE_SHA256;}
