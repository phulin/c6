/* Synthetic UnitOperation body only. No message construction or gameplay calls. */
#include <stdint.h>
#include <string.h>
typedef struct {void **vtable;uint8_t bytes[24];uint32_t position,calls,fault,sizes[7];} Stream;
static void __attribute__((ms_abi)) write_bytes(Stream*s,const void*p,uint32_t n){
 if(s->calls>=7 || n>24 || s->position>24-n){s->fault=1;return;}
 s->sizes[s->calls++]=n;memcpy(s->bytes+s->position,p,n);s->position+=n;
}
int c6_action_body(const uint8_t input[80],uint8_t output[24]){
 uint8_t message[80];memcpy(message,input,80);
 uint64_t parameter_pointer;memcpy(&parameter_pointer,message+0x28,8);
 if(parameter_pointer)return -2;
 void *vtable[6]={0};vtable[5]=(void*)write_bytes;
 Stream stream={.vtable=vtable};
 ((void(__attribute__((ms_abi))*)(void*,void*))(uintptr_t)0x18003abd0ULL)(message,&stream);
 const uint32_t expected[]={2,2,4,4,4,4,4};
 if(stream.fault||stream.position!=24||stream.calls!=7||memcmp(stream.sizes,expected,sizeof expected)||memcmp(message,input,80))return -1;
 memcpy(output,stream.bytes,24);return 0;
}
