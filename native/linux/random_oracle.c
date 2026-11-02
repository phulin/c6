#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
/* Isolated actual Linux Random objects, not initialized game RNG instances.
 * DWARF object size 32; fields +8,+12,+16,+20,+24. Synthetic input fields
 * enable wrapping boundary fixtures; constructors/vptrs remain original. */
typedef struct { void *vptr; uint32_t seed,calls,resets,locks; uint8_t sync,pad[7]; } Random;
_Static_assert(sizeof(Random)==32,"Random ABI");
static void *sym(void *h,const char *s){void *p=dlsym(h,s);if(!p){fprintf(stderr,"missing %s\n",s);_exit(81);}return p;}
static int run(int argc,char **argv,char **env){
 (void)argc;(void)argv;(void)env;
 void *g=dlopen(getenv("C6_RANDOM_GAMECORE"),RTLD_NOW|RTLD_LOCAL);if(!g){fprintf(stderr,"%s\n",dlerror());return 80;}
 void *(*create)(void)=sym(g,"DllCreateGameContext");void *context=create();if(!context)return 82;
 void (*ctor)(void*)=sym(g,"_ZN8GameCore6RandomC1Ev");
 unsigned (*get)(void*,unsigned short,const char*)=sym(g,"_ZN8GameCore6Random3GetEtPKc");
 void (*reset)(void*,unsigned)=sym(g,"_ZN8GameCore6Random5ResetEj");
 void (*init)(void*,unsigned)=sym(g,"_ZN8GameCore6Random10InitializeEj");
 void (*lock)(void*)=sym(g,"_ZN8GameCore6Random4LockEv");void (*unlock)(void*)=sym(g,"_ZN8GameCore6Random6UnlockEv");
 void (*write_rng)(void*,void*)=sym(g,"_ZNK8GameCore6Random5WriteERN13Serialization7OStreamE");
 void (*read_rng)(void*,void*)=sym(g,"_ZN8GameCore6Random4ReadERN13Serialization7IStreamE");
 void *(*alloc)(size_t)=sym(RTLD_DEFAULT,"_Znwm");void (*release)(void*)=sym(RTLD_DEFAULT,"_ZdlPv");
 void (*oc)(void*,void*,size_t)=sym(RTLD_DEFAULT,"_ZN13Serialization13MemoryOStreamC1EPvm");
 void (*od)(void*)=sym(RTLD_DEFAULT,"_ZN13Serialization13MemoryOStreamD0Ev");
 size_t (*oo)(void*)=sym(RTLD_DEFAULT,"_ZNK13Serialization13MemoryOStream9GetOffsetEv");
 void (*ic)(void*,const void*,size_t)=sym(RTLD_DEFAULT,"_ZN13Serialization13MemoryIStreamC1EPKvm");
 void (*id)(void*)=sym(RTLD_DEFAULT,"_ZN13Serialization13MemoryIStreamD1Ev");
 size_t (*io)(void*)=sym(RTLD_DEFAULT,"_ZNK13Serialization13MemoryIStream9GetOffsetEv");
 Random streams[4];for(unsigned i=0;i<4;i++){
  memset(&streams[i],0xa5,32);ctor(&streams[i]);
  if(streams[i].seed||streams[i].calls||streams[i].resets!=1||streams[i].locks||streams[i].sync)return 88;
 }
 /* Genuine host LoggingManager; explicit test binding to original core global.
  * Oversized allocation supplies storage only: ctor writes every service field.
  * Disable global file output before Initialize registers the RNG channel. */
 void (*log_ctor)(void*)=sym(RTLD_DEFAULT,"_ZN8GameCore14LoggingManagerC1Ev");
 void (*log_enable)(void*,int)=sym(RTLD_DEFAULT,"_ZN8GameCore14LoggingManager10SetEnabledEb");
 void *logger=alloc(0x4700);log_ctor(logger);log_enable(logger,0);
 void **logging=sym(g,"_ZN8GameCore7Context7Globals19ms_pkLoggingManagerE");
 if(*logging)return 87;
 *logging=logger;
 unsigned op,stream,arg,seed,calls,resets,locks,sync;
 while(scanf("%u %u %u %u %u %u %u %u",&op,&stream,&arg,&seed,&calls,&resets,&locks,&sync)==8){
  int retain=op>=10&&op<=14;
  if(retain)op-=10;
  if(stream>=4||sync>255||op>6||(op==6&&(arg<18||arg>64)))return 83;
  unsigned char bytes[64]={0};
  if(op==6){
   for(unsigned i=0;i<arg;i++){unsigned v;if(scanf("%2x",&v)!=1)return 84;bytes[i]=(unsigned char)v;}
   /* Normal complete records only; unsupported input is rejected before state mutation. */
   if(bytes[17])return 91;
  }
  Random *r=&streams[stream];
  if(!retain){r->seed=seed;r->calls=calls;r->resets=resets;r->locks=locks;r->sync=sync;}
  unsigned result=0;size_t consumed=0;Random before=*r;Random others[4];memcpy(others,streams,sizeof streams);
  switch(op){case 0:result=get(r,(unsigned short)arg,NULL);break;case 1:reset(r,arg);break;case 2:init(r,arg);break;case 3:lock(r);break;case 4:unlock(r);break;
   case 5:{void *s=alloc(40);oc(s,bytes,sizeof bytes);write_rng(r,s);consumed=oo(s);od(s);break;}
   case 6:{void *s=alloc(40);ic(s,bytes,arg);read_rng(r,s);consumed=io(s);id(s);release(s);break;}
   default:return 85;}
  /* Include every named state field and check untouched vptr/padding. */
  if(r->vptr!=before.vptr||memcmp(r->pad,before.pad,7))return 86;
  for(unsigned j=0;j<4;j++){if(j!=stream&&memcmp(&others[j],&streams[j],32))return 89;}
  printf("%u %u %u %u %u %u %zu ",result,r->seed,r->calls,r->resets,r->locks,r->sync,consumed);
  for(size_t i=0;i<(op==5?consumed:0);i++){printf("%02x",bytes[i]);}
  puts("");fflush(stdout);
 }
 void (*rng_destroy)(void*)=sym(g,"_ZN8GameCore6RandomD1Ev");
 for(unsigned i=0;i<4;i++){rng_destroy(&streams[i]);}
 void (*log_destroy)(void*)=sym(RTLD_DEFAULT,"_ZN8GameCore14LoggingManager7DestroyEv");
 *logging=NULL;log_destroy(logger);
 void (*destroy)(void*)=sym(g,"DllDestroyGameContext");destroy(context);
 return dlclose(g)?90:0;
}
static int main_probe(int a,char **b,char **c){int rc=run(a,b,c);fflush(NULL);_exit(rc);}
int __libc_start_main(int(*main)(int,char**,char**),int argc,char **argv,void(*init)(void),void(*fini)(void),void(*rtld)(void),void *end){
 (void)main;typedef int(*Start)(int(*)(int,char**,char**),int,char**,void(*)(void),void(*)(void),void(*)(void),void*);
 Start real=dlsym(RTLD_NEXT,"__libc_start_main");return real(main_probe,argc,argv,init,fini,rtld,end);
}
