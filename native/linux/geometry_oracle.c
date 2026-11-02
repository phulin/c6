#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
/* Exact target PlotCoord layout, constructed by original code. No Map object. */
typedef struct { int32_t x,y; } Coord;
_Static_assert(sizeof(Coord)==8,"PlotCoord ABI");
static void *symbol(void *game,const char *name){
 void *p=dlsym(game,name);
 if(!p){fprintf(stderr,"missing symbol: %s\n",name);_exit(81);}
 return p;
}
static int run(int argc,char **argv,char **envp){
 (void)argc;(void)argv;(void)envp;
 void *game=dlopen(getenv("C6_GEOMETRY_GAMECORE"),RTLD_NOW|RTLD_LOCAL);
 if(!game){fprintf(stderr,"%s\n",dlerror());return 80;}
 void (*construct)(void*,int,int)=symbol(game,"_ZN8GameCore9PlotCoordC2Eii");
 bool (*valid)(const void*)=symbol(game,"_ZNK8GameCore9PlotCoord7IsValidEv");
 int (*distance)(int,int,int,int,int,int,bool,bool)=symbol(game,"_ZN8GameCore3Map15GetPlotDistanceEiiiiiibb");
 int (*coord_distance)(const void*,const void*,int,int,bool,bool)=symbol(game,"_ZN8GameCore3Map15GetPlotDistanceERKNS_9PlotCoordES3_iibb");
 int op,width,wrap,x1,y1,x2,y2;
 while(scanf("%d %d %d %d %d %d %d",&op,&width,&wrap,&x1,&y1,&x2,&y2)==7){
  if((op!=0&&op!=1)||(wrap!=0&&wrap!=1))return 82;
  Coord a,b;construct(&a,x1,y1);construct(&b,x2,y2);
  if(a.x!=x1||a.y!=y1||b.x!=x2||b.y!=y2)return 83;
  Coord before_a=a,before_b=b;int result;
  if(op==0){result=valid(&a);}
  else{
   /* Height/wrapY do not affect this original helper, verified in target bytes.
    * Both native overloads must agree; no map fields are synthesized. */
   result=distance(x1,y1,x2,y2,width,0,wrap,false);
   if(coord_distance(&a,&b,width,0,wrap,false)!=result)return 84;
  }
  if(memcmp(&a,&before_a,8)||memcmp(&b,&before_b,8))return 85;
  printf("%d\n",result);
 }
 return dlclose(game)?86:0;
}
static int probe_main(int a,char **b,char **c){int rc=run(a,b,c);fflush(NULL);_exit(rc);}
int __libc_start_main(int(*main)(int,char**,char**),int argc,char **argv,void(*init)(void),void(*fini)(void),void(*rtld)(void),void *end){
 (void)main;
 typedef int(*Start)(int(*)(int,char**,char**),int,char**,void(*)(void),void(*)(void),void(*)(void),void*);
 Start real=dlsym(RTLD_NEXT,"__libc_start_main");
 return real(probe_main,argc,argv,init,fini,rtld,end);
}
