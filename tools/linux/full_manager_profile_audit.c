/* Offline wrapper contract fixtures. No original module is loaded or called. */
#include "../../native/linux/full_manager_profile.c"
#include <assert.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <fcntl.h>

static void *test_core,*test_game,*test_unit;
static uintptr_t test_base;
static unsigned char test_manager[7416],test_player[0x6d8],test_units[232],test_block[192];
static uintptr_t test_players[65],test_blocks[3];
static int32_t test_mapping[65];
static unsigned fixture_calls,predicate_calls,metadata_calls,owner_checks;
static int metadata_failure=-1;
static bool saves_enabled,fixture_error;
static const char *test_mode;
static char *test_paths[10];
static const uint32_t actor_id=0xa5a50012u;
static void setptr(void *p,size_t off,uintptr_t value){memcpy((unsigned char *)p+off,&value,8);}
static void set32(void *p,size_t off,uint32_t value){memcpy((unsigned char *)p+off,&value,4);}
static void env(const char *suffix,const char *value){
 char key[128];int n=snprintf(key,sizeof key,"C6_FULL_MANAGER_FIXTURE_%s",suffix);assert(n>0&&(size_t)n<sizeof key);
 assert(value?!setenv(key,value,1):!unsetenv(key));
}
static void setup(const char *directory){
 test_core=mmap(NULL,0x3c08000,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(test_core!=MAP_FAILED);test_base=(uintptr_t)test_core;
 test_game=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(test_game!=MAP_FAILED);
 test_unit=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(test_unit!=MAP_FAILED);
 setptr(test_game,0x248,(uintptr_t)test_manager);
 setptr(test_manager,0x20,(uintptr_t)test_players);setptr(test_manager,0x28,(uintptr_t)(test_players+65));setptr(test_manager,0x30,(uintptr_t)(test_players+65));
 setptr(test_manager,0x1a08,(uintptr_t)test_mapping);setptr(test_manager,0x1a10,(uintptr_t)(test_mapping+65));setptr(test_manager,0x1a18,(uintptr_t)(test_mapping+65));
 for(unsigned i=0;i<65;i++)test_mapping[i]=-1;
 test_mapping[13]=64;test_players[13]=(uintptr_t)test_player;test_players[64]=1;
 set32(test_player,0xe0,13);setptr(test_player,0x6d0,(uintptr_t)test_units);setptr(test_units,152,(uintptr_t)test_player);
 unsigned char *list=test_units+160,*entry=test_block+48;test_blocks[2]=(uintptr_t)test_block;
 setptr(list,0,(uintptr_t)test_blocks);setptr(list,8,(uintptr_t)(test_blocks+3));setptr(list,16,(uintptr_t)(test_blocks+3));
 set32(list,32,1);setptr(list,56,(uintptr_t)entry);setptr(list,64,(uintptr_t)entry);setptr(entry,0,(uintptr_t)test_unit);
 setptr(test_unit,0,test_base+0x3ae5f80);set32(test_unit,0xb8,actor_id);set32(test_unit,0x130,13);
 const char *suffixes[]={"OUTPUT","CONTROL_OUTPUT","METADATA_BEFORE","METADATA_CONTROL","METADATA_AFTER","METADATA_REPEAT","SNAPSHOT_BEFORE","SNAPSHOT_CONTROL","SNAPSHOT_AFTER","SNAPSHOT_REPEAT"};
 for(unsigned i=0;i<10;i++){assert(asprintf(&test_paths[i],"%s/%s.jsonl",directory,suffixes[i])>0);env(suffixes[i],i<6?test_paths[i]:NULL);}
 env("ACTOR_OWNER","13");env("ACTOR_FULL_ID","2779054098");env("VISIBLE_ONLY","1");
 env("PROFILE_SHA256","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
}
int c6_linux_full_manager_profile_identity(void *module,void *context,uintptr_t *base,const void **game,long *tid){
 assert(module==(void *)1&&context==(void *)2);*base=test_base;*game=test_game;*tid=syscall(SYS_gettid);return 0;
}
static int owners(void *module,void *context,const void *game){assert(module==(void *)1&&context==(void *)2&&game==test_game);owner_checks++;return 0;}
int c6_linux_full_manager_pre_acquisition(void *m,void *c,const void *g){return owners(m,c,g);}
int c6_linux_full_manager_retained_owner(void *m,void *c,const void *g){return owners(m,c,g);}
int c6_linux_full_manager_locked_end(void *m,void *c,const void *g){return owners(m,c,g);}
int c6_linux_full_manager_released_owner(void *m,void *c,const void *g){return owners(m,c,g);}
int c6_linux_full_manager_locked_begin(void *m,void *c,const void *g,uintptr_t base,long tid){assert(base==test_base&&tid==syscall(SYS_gettid));return owners(m,c,g);}
int c6_linux_current_handler_interruption_capture(const C6CurrentHandlerInterruptionAdmission *a){
 assert(metadata_calls<4&&a->module==(void *)1&&a->borrowed_context==(void *)2&&a->retained_game==test_game);
 assert(a->quiescent==c6_linux_full_manager_retained_owner&&!a->quiescent(a->module,a->borrowed_context,a->retained_game));
 assert(!strcmp(a->output_path,test_paths[2+metadata_calls]));
 FILE *out=fopen(a->output_path,"wx");assert(out);int status=(metadata_failure==-2||(int)metadata_calls==metadata_failure)?245:0;
 fprintf(out,"{\"mock_snapshot\":%u,\"status\":%d}\n",metadata_calls,status);assert(!fclose(out));metadata_calls++;return status;
}
static void observe_phase(const C6FullManagerFixtureAdmission *a,C6FullManagerFixtureResult *r,C6FullManagerObservation *o,C6FullManagerPhase phase){
 o->phase=phase;((unsigned char *)test_game)[0xf8]=(unsigned char)phase;*(unsigned char *)(test_base+0x3c072f0)=(unsigned char)(0x80+(unsigned)phase);
 if(phase==C6_FULL_MANAGER_PREDICATE_ENTER||phase==C6_FULL_MANAGER_PREDICATE_RETURN)o->unit=test_unit;else o->unit=NULL;
 if(phase==C6_FULL_MANAGER_RELEASED)o->facade=NULL;
 o->predicate_returned=r->predicate_returned;o->allowed=r->allowed;
 if(!strcmp(test_mode,"observation-identity"))o->context=(void *)3;
 if(!strcmp(test_mode,"rng-unreadable"))assert(!mprotect(test_game,4096,PROT_NONE));
 int status=a->observe(o);if(status&&!r->observation_status)r->observation_status=status;
}
int c6_linux_full_manager_fixture_run(const C6FullManagerFixtureAdmission *a,C6FullManagerFixtureResult *r){
 if(!strcmp(test_mode,"controls-write-error")){
  int fd=open("/dev/full",O_WRONLY|O_CLOEXEC);assert(fd>=0&&dup2(fd,fileno(full_observation.controls))>=0);assert(!close(fd));
 }
 fixture_calls++;assert(fixture_calls==1&&a->module==(void *)1&&a->context==(void *)2&&a->expected_game==test_game&&a->expected_unit==test_unit);
 assert(a->actor_owner==13&&a->actor_full_id==actor_id&&a->visible_only&&a->main_entry_tid==syscall(SYS_gettid));
 assert(a->pre_acquisition==c6_linux_full_manager_pre_acquisition&&a->locked_begin==c6_linux_full_manager_locked_begin&&a->retained_owner==c6_linux_full_manager_retained_owner&&a->locked_end==c6_linux_full_manager_locked_end&&a->released_owner==c6_linux_full_manager_released_owner);
 for(unsigned i=0;i<4;i++)assert(saves_enabled?(a->diagnostic_save_paths[i]&&!strcmp(a->diagnostic_save_paths[i],test_paths[6+i])):!a->diagnostic_save_paths[i]);
 *r=(C6FullManagerFixtureResult){.operation_id=18};C6FullManagerObservation o={.module=a->module,.context=a->context,.game=a->expected_game,.core_base=test_base,.facade=(void *)4,.actor_owner=13,.actor_full_id=actor_id,.visible_only=true};
 observe_phase(a,r,&o,C6_FULL_MANAGER_FACADE_ACQUIRED);observe_phase(a,r,&o,C6_FULL_MANAGER_BEFORE);observe_phase(a,r,&o,C6_FULL_MANAGER_CONTROL);
 /* Match the reviewed real fixture's observation-status veto. Passive metadata
  * errors must be retained by the wrapper without becoming this admission veto. */
 if(!r->observation_status){
  observe_phase(a,r,&o,C6_FULL_MANAGER_PREDICATE_ENTER);
  if(!r->observation_status){predicate_calls++;r->predicate_invoked=true;r->predicate_returned=true;r->allowed=strcmp(test_mode,"answer-false")!=0;
   /* The actor becomes unreadable at the call boundary. All later wrapper
    * observations must retain only its address token, preserving RNG effects. */
   *(unsigned char *)(test_base+0x3c05cb8)=0x7f;
   assert(!mprotect(test_unit,4096,PROT_NONE));observe_phase(a,r,&o,C6_FULL_MANAGER_PREDICATE_RETURN);
  }
 }
 observe_phase(a,r,&o,C6_FULL_MANAGER_AFTER);observe_phase(a,r,&o,C6_FULL_MANAGER_REPEAT);observe_phase(a,r,&o,C6_FULL_MANAGER_BEFORE_RELEASE);r->facade_released=true;observe_phase(a,r,&o,C6_FULL_MANAGER_RELEASED);
 if(saves_enabled)r->save_status[0]=13;
 return fixture_error?249:r->observation_status;
}
int main(int argc,char **argv){
 assert(argc==3);test_mode=argv[1];setup(argv[2]);
 if(!strcmp(test_mode,"metadata-all"))metadata_failure=-2;
 if(!strcmp(test_mode,"metadata-before"))metadata_failure=0;
 if(!strcmp(test_mode,"metadata-control"))metadata_failure=1;
 if(!strcmp(test_mode,"metadata-after"))metadata_failure=2;
 if(!strcmp(test_mode,"metadata-repeat"))metadata_failure=3;
 if(!strcmp(test_mode,"saves")){saves_enabled=true;const char *names[]={"SNAPSHOT_BEFORE","SNAPSHOT_CONTROL","SNAPSHOT_AFTER","SNAPSHOT_REPEAT"};for(unsigned i=0;i<4;i++)env(names[i],test_paths[6+i]);}
 if(!strcmp(test_mode,"fixture-error"))fixture_error=true;
 if(!strcmp(test_mode,"missing-owner"))env("ACTOR_OWNER",NULL);
 if(!strcmp(test_mode,"empty-owner"))env("ACTOR_OWNER","");
 if(!strcmp(test_mode,"negative-owner"))env("ACTOR_OWNER","-1");
 if(!strcmp(test_mode,"invalid-owner"))env("ACTOR_OWNER","1x");
 if(!strcmp(test_mode,"owner-range"))env("ACTOR_OWNER","65");
 if(!strcmp(test_mode,"id-range"))env("ACTOR_FULL_ID","4294967296");
 if(!strcmp(test_mode,"visible-range"))env("VISIBLE_ONLY","2");
 if(!strcmp(test_mode,"profile-missing"))env("PROFILE_SHA256",NULL);
 if(!strcmp(test_mode,"profile-short"))env("PROFILE_SHA256","abc");
 if(!strcmp(test_mode,"profile-invalid"))env("PROFILE_SHA256","AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
 if(!strcmp(test_mode,"output-missing"))env("OUTPUT",NULL);
 if(!strcmp(test_mode,"metadata-missing"))env("METADATA_CONTROL",NULL);
 if(!strcmp(test_mode,"duplicate-control"))env("CONTROL_OUTPUT",test_paths[0]);
 if(!strcmp(test_mode,"duplicate-metadata"))env("METADATA_BEFORE",test_paths[3]);
 if(!strcmp(test_mode,"duplicate-save")){
  env("SNAPSHOT_BEFORE",test_paths[2]);env("SNAPSHOT_CONTROL",test_paths[7]);env("SNAPSHOT_AFTER",test_paths[8]);env("SNAPSHOT_REPEAT",test_paths[9]);
 }
 if(!strcmp(test_mode,"partial-saves"))env("SNAPSHOT_CONTROL",test_paths[7]);
 if(!strcmp(test_mode,"controls-existing")){FILE *out=fopen(test_paths[1],"wx");assert(out);assert(!fclose(out));}
 if(!strcmp(test_mode,"nested")){full_observation.controls=tmpfile();assert(full_observation.controls);}
 if(!strcmp(test_mode,"list-prefix")){
  setptr(test_units+160,56,(uintptr_t)test_block);set32(test_units+160,32,2);setptr(test_block,16,(uintptr_t)(test_block+48));setptr(test_block+48,8,(uintptr_t)test_block);
 }
 if(!strcmp(test_mode,"numeric-boundaries")){
  env("ACTOR_FULL_ID","4294967295");assert(number_env("ACTOR_FULL_ID",UINT32_MAX)==UINT32_MAX);
  env("ACTOR_FULL_ID","0");assert(number_env("ACTOR_FULL_ID",UINT32_MAX)==0);env("ACTOR_FULL_ID","2779054098");
 }
 if(!strcmp(test_mode,"actor-unmapped"))test_mapping[13]=-1;
 if(!strcmp(test_mode,"actor-player-id"))set32(test_player,0xe0,14);
 if(!strcmp(test_mode,"actor-owner"))set32(test_unit,0x130,14);
 if(!strcmp(test_mode,"actor-generation"))set32(test_unit,0xb8,actor_id^0x10000u);
 if(!strcmp(test_mode,"actor-vptr"))setptr(test_unit,0,1);
 if(!strcmp(test_mode,"actor-parent"))setptr(test_units,152,(uintptr_t)test_player+8);
 if(!strcmp(test_mode,"actor-block-range"))setptr(test_units+160,8,(uintptr_t)(test_blocks+2));
 if(!strcmp(test_mode,"actor-block-null"))test_blocks[2]=0;
 if(!strcmp(test_mode,"actor-block-overflow"))test_blocks[2]=UINTPTR_MAX-8;
 if(!strcmp(test_mode,"actor-list-missing"))set32(test_units+160,32,0);
 if(!strcmp(test_mode,"actor-list-not-member"))setptr(test_units+160,56,(uintptr_t)test_block);
 if(!strcmp(test_mode,"actor-link-previous"))setptr(test_block+48,8,1);
 if(!strcmp(test_mode,"actor-count"))set32(test_units+160,32,65537);
 if(!strcmp(test_mode,"actor-vector-shape"))setptr(test_manager,0x28,(uintptr_t)test_players+519);
 if(!strcmp(test_mode,"actor-unreadable"))assert(!mprotect(test_unit,4096,PROT_NONE));
 int status=c6_linux_full_manager_profile_probe((void *)1,(void *)2);
 bool unsupported=!strncmp(test_mode,"actor-",6);
 assert(status==(unsupported?244:fixture_error?249:!strcmp(test_mode,"controls-write-error")?247:0));
 assert(fixture_calls==(unsupported?0u:1u)&&predicate_calls==(unsupported?0u:1u)&&metadata_calls==(unsupported?0u:4u));
 assert(!full_observation.controls&&!full_observation.module);
 if(!unsupported){assert(owner_checks>=13);assert(((unsigned char *)test_game)[0xf8]==9&&*(unsigned char *)(test_base+0x3c072f0)==0x89&&*(unsigned char *)(test_base+0x3c05cb8)==0x7f);}
 printf("{\"status\":%d,\"fixture_calls\":%u,\"predicate_calls\":%u,\"metadata_calls\":%u,\"owner_checks\":%u}\n",status,fixture_calls,predicate_calls,metadata_calls,owner_checks);
 for(unsigned i=0;i<10;i++)free(test_paths[i]);
 assert(!munmap(test_unit,4096)&&!munmap(test_game,4096)&&!munmap(test_core,0x3c08000));return 0;
}
