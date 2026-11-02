#define _GNU_SOURCE
#include "random_variants.h"
#include <dlfcn.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static void *symbol(void *handle, const char *name) {
 void *p=dlsym(handle,name); if(!p){fprintf(stderr,"missing %s\n",name);_exit(81);}return p;
}
static int run(int argc,char **argv,char **envp) {
 (void)argc;(void)argv;(void)envp;
 void *g=dlopen(getenv("C6_RANDOM_GAMECORE"),RTLD_NOW|RTLD_LOCAL);
 if(!g){fprintf(stderr,"%s\n",dlerror());return 80;}
 void (*sc)(void*,unsigned)=symbol(g,"_ZN8GameCore12ScopedRandomC1Ej");
 void (*sr)(void*,unsigned)=symbol(g,"_ZN8GameCore12ScopedRandom5ResetEj");
 unsigned (*sg)(void*,unsigned short,const char*)=symbol(g,"_ZN8GameCore12ScopedRandom3GetEtPKc");
 unsigned (*ss)(const void*)=symbol(g,"_ZNK8GameCore12ScopedRandom7GetSeedEv");
 void (*pc)(void*)=symbol(RTLD_DEFAULT,"_ZN4Math11RandomPCG32C1Ev");
 void (*pcs)(void*,unsigned)=symbol(RTLD_DEFAULT,"_ZN4Math11RandomPCG32C1Ej");
 void (*ps)(void*,unsigned)=symbol(RTLD_DEFAULT,"_ZN4Math11RandomPCG324SeedEj");
 unsigned (*pg)(void*)=symbol(RTLD_DEFAULT,"_ZN4Math11RandomPCG324RandEv");
 C6ScopedRandom s[4];C6RandomPCG32 p[4];
 for(unsigned i=0;i<4;i++){sc(&s[i],0);pc(&p[i]);}
 uint64_t op,index,arg,state,inc,retain,ordinal=0;
 while(scanf("%"SCNu64" %"SCNu64" %"SCNu64" %"SCNu64" %"SCNu64" %"SCNu64,&op,&index,&arg,&state,&inc,&retain)==6){
  if(op>6||index>3||arg>UINT32_MAX||retain>1||(op<=2&&state>UINT32_MAX))return 82;
  if(!retain){s[index].seed=state;p[index].state=state;p[index].increment=inc;}
  C6ScopedRandom before_s[4];C6RandomPCG32 before_p[4];
  memcpy(before_s,s,sizeof s);memcpy(before_p,p,sizeof p);
  unsigned result=0;
  switch(op){case 0:sc(&s[index],arg);break;case 1:sr(&s[index],arg);break;
   case 2:result=sg(&s[index],(unsigned short)arg,NULL);break;case 3:pc(&p[index]);break;
   case 4:pcs(&p[index],arg);break;case 5:ps(&p[index],arg);break;case 6:result=pg(&p[index]);break;}
  for(unsigned j=0;j<4;j++){
   if((j!=index||op>2)&&memcmp(&s[j],&before_s[j],sizeof s[j]))return 83;
   if((j!=index||op<=2)&&memcmp(&p[j],&before_p[j],sizeof p[j]))return 84;
  }
  if(op<=2){if(ss(&s[index])!=s[index].seed)return 85;printf("C6_RNG_VARIANT %"PRIu64" %u %u 0\n",ordinal,result,s[index].seed);}
  else printf("C6_RNG_VARIANT %"PRIu64" %u %"PRIu64" %"PRIu64"\n",ordinal,result,p[index].state,p[index].increment);
  ordinal++;
 }
 return dlclose(g)?86:0;
}
static int probe(int a,char **b,char **c){int rc=run(a,b,c);fflush(NULL);_exit(rc);}
int __libc_start_main(int(*main)(int,char**,char**),int argc,char **argv,void(*init)(void),void(*fini)(void),void(*rtld)(void),void *end){
 (void)main;typedef int(*Start)(int(*)(int,char**,char**),int,char**,void(*)(void),void(*)(void),void(*)(void),void*);
 Start real=dlsym(RTLD_NEXT,"__libc_start_main");return real(probe,argc,argv,init,fini,rtld,end);
}
