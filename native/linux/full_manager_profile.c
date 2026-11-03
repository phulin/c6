#define _GNU_SOURCE
#include "full_manager_fixture.h"
#include "full_manager_profile_guard.h"
#include "current_handler_interruption.h"
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/uio.h>
#include <unistd.h>

/* One externally selected predicate. This wrapper adds no game getter, Lua,
 * policy, turn or actor mutation. The separately reviewed fixture owns its
 * original parameter construction, predicate and facade/diagnostic lifetimes. */
static struct {
    void *module,*context;
    const void *game;
    uintptr_t base;
    const char *profile;
    char *metadata[4];
    FILE *controls;
    int metadata_status[4],io_status;
} full_observation;
static _Noreturn void profile_fatal(const char *why) {
    fprintf(stderr,"C6_FULL_MANAGER_PROFILE fatal %s no_further_original_calls\n",why);fflush(stderr);_exit(240);
}
static bool read_bytes(uintptr_t p,void *out,size_t n) {
    if(!p||n>UINTPTR_MAX-p)return false;
    struct iovec a={out,n},b={(void *)p,n};return process_vm_readv(getpid(),&a,1,&b,1,0)==(ssize_t)n;
}
static bool read_ptr(uintptr_t p,uintptr_t *v){return read_bytes(p,v,8);}
static uint32_t raw32(const unsigned char *p,size_t off){uint32_t v;memcpy(&v,p+off,4);return v;}
static uintptr_t rawptr(const unsigned char *p,size_t off){uintptr_t v;memcpy(&v,p+off,8);return v;}
static bool vector_extent(uintptr_t begin,uintptr_t end,uintptr_t cap,size_t width,size_t limit) {
    return begin&&begin<=end&&end<=cap&&(end-begin)%width==0&&(cap-begin)%width==0&&(cap-begin)/width<=limit;
}
static const void *expected_actor(const void *game,uintptr_t base,int32_t owner,uint32_t id) {
    /* An untrusted address expectation only: the fixture subsequently acquires
     * its genuine lock and resolves owner/fullID again with original Find.
     * Safe copies here cannot authorize a stale object for the native call. */
    if(!game||(uintptr_t)game>UINTPTR_MAX-0x250)return NULL;
    uintptr_t manager;if(!read_ptr((uintptr_t)game+0x248,&manager))return NULL;
    unsigned char pm[7416];if(!read_bytes(manager,pm,sizeof pm))return NULL;
    uintptr_t players=rawptr(pm,0x20),mapping=rawptr(pm,0x1a08);
    if(!vector_extent(players,rawptr(pm,0x28),rawptr(pm,0x30),8,65)||rawptr(pm,0x28)-players!=65*8||
       !vector_extent(mapping,rawptr(pm,0x1a10),rawptr(pm,0x1a18),4,65)||rawptr(pm,0x1a10)-mapping!=65*4)return NULL;
    int32_t mapped;if(!read_bytes(mapping+(uintptr_t)owner*4,&mapped,4)||mapped==-1)return NULL;
    uintptr_t player;if(!read_ptr(players+(uintptr_t)owner*8,&player))return NULL;
    unsigned char p[0x6d8];if(!read_bytes(player,p,sizeof p)||(int32_t)raw32(p,0xe0)!=owner)return NULL;
    uintptr_t units=rawptr(p,0x6d0);unsigned char u[232];
    if(!read_bytes(units,u,sizeof u)||rawptr(u,152)!=player)return NULL;
    const unsigned char *list=u+160;uintptr_t blocks=rawptr(list,0),end=rawptr(list,8),cap=rawptr(list,16),entry=rawptr(list,56),previous=0;
    uint32_t count=raw32(list,32),slot=id&65535u;
    if(count>65536||!vector_extent(blocks,end,cap,8,8192)||slot/8>=(end-blocks)/8)return NULL;
    uintptr_t block;if(!read_ptr(blocks+(slot/8)*8,&block)||!block||block>UINTPTR_MAX-192)return NULL;
    uintptr_t wanted=block+(slot%8)*24;
    for(uint32_t i=0;i<count;i++) {
        unsigned char link[24];if(!read_bytes(entry,link,sizeof link)||rawptr(link,8)!=previous)return NULL;
        if(entry==wanted) {
            uintptr_t unit=rawptr(link,0);unsigned char unit_header[0x134];
            if(!read_bytes(unit,unit_header,sizeof unit_header)||rawptr(unit_header,0)!=base+0x3ae5f80||
               raw32(unit_header,0xb8)!=id||(int32_t)raw32(unit_header,0x130)!=owner)return NULL;
            return (void *)unit;
        }
        previous=entry;entry=rawptr(link,16);
    }
    return NULL;
}
static char *path_env(const char *suffix,bool required) {
    char key[128];snprintf(key,sizeof key,"C6_FULL_MANAGER_FIXTURE_%s",suffix);
    const char *p=getenv(key);if(!p||!*p){if(required)profile_fatal("required_path_missing");return NULL;}
    char *owned=strdup(p);if(!owned)profile_fatal("path_allocation");return owned;
}
static uint32_t number_env(const char *suffix,uint32_t maximum) {
    char key[128];snprintf(key,sizeof key,"C6_FULL_MANAGER_FIXTURE_%s",suffix);const char *p=getenv(key);
    if(!p||!*p)profile_fatal("numeric_input_missing");
    uint32_t value=0;for(const char *q=p;*q;q++){
        if(*q<'0'||*q>'9')profile_fatal("numeric_input_invalid");
        unsigned digit=(unsigned)(*q-'0');if(value>maximum/10||(value==maximum/10&&digit>maximum%10))profile_fatal("numeric_input_range");value=value*10+digit;
    }
    return value;
}
static void hex(FILE *out,const unsigned char *p,size_t n){fputc('"',out);for(size_t i=0;i<n;i++)fprintf(out,"%02x",p[i]);fputc('"',out);}
static int observe(const C6FullManagerObservation *o) {
    if(!o||o->module!=full_observation.module||o->context!=full_observation.context||o->game!=full_observation.game||o->core_base!=full_observation.base)profile_fatal("observation_identity");
    if(o->phase==C6_FULL_MANAGER_RELEASED)c6_linux_full_manager_released_owner(o->module,o->context,o->game);
    else c6_linux_full_manager_retained_owner(o->module,o->context,o->game);
    unsigned char rng[64];
    if(!read_bytes((uintptr_t)o->game+0xf8,rng,32)||!read_bytes(o->core_base+0x3c072f0,rng+32,32))profile_fatal("known_rng_unreadable");
    /* Unit/Operation are identity tokens here; never dereference stale actors. */
    fprintf(full_observation.controls,"{\"kind\":\"observation\",\"phase\":%u,\"actor_owner\":%d,\"actor_full_id\":%u,\"unit_address_token\":%" PRIuPTR ",\"predicate_returned\":%s,\"allowed\":%s,\"known_rng_raw\":",(unsigned)o->phase,o->actor_owner,o->actor_full_id,(uintptr_t)o->unit,o->predicate_returned?"true":"false",o->predicate_returned?(o->allowed?"true":"false"):"null");
    hex(full_observation.controls,rng,sizeof rng);fputs("}\n",full_observation.controls);
    int index=o->phase==C6_FULL_MANAGER_BEFORE?0:o->phase==C6_FULL_MANAGER_CONTROL?1:o->phase==C6_FULL_MANAGER_AFTER?2:o->phase==C6_FULL_MANAGER_REPEAT?3:-1;
    int status=0;
    if(index>=0) {
        C6CurrentHandlerInterruptionAdmission a={.module=o->module,.borrowed_context=o->context,.retained_game=o->game,
            .profile_sha256=full_observation.profile,.output_path=full_observation.metadata[index],.quiescent=c6_linux_full_manager_retained_owner};
        status=c6_linux_current_handler_interruption_capture(&a);
    }
    if(index>=0){
        full_observation.metadata_status[index]=status;
        fprintf(full_observation.controls,"{\"kind\":\"metadata_status\",\"phase\":%u,\"status\":%d,\"predicate_admission_veto\":false}\n",(unsigned)o->phase,status);
    }
    if(fflush(full_observation.controls)||ferror(full_observation.controls))full_observation.io_status=247;
    /* Optional passive evidence is not a legality or predicate-admission gate.
     * Required owner/code/VM admission is independently enforced by guards and
     * the actual hardware observer. Preserve errors for post-call assessment. */
    return 0;
}
int c6_linux_full_manager_profile_probe(void *module,void *context) {
    if(full_observation.controls)profile_fatal("nested_profile");
    uintptr_t base;const void *game;long main_tid;
    c6_linux_full_manager_profile_identity(module,context,&base,&game,&main_tid);
    uint32_t owner=number_env("ACTOR_OWNER",64),id=number_env("ACTOR_FULL_ID",UINT32_MAX),visible=number_env("VISIBLE_ONLY",1);
    const char *profile=getenv("C6_FULL_MANAGER_FIXTURE_PROFILE_SHA256");if(!profile||strlen(profile)!=64)profile_fatal("profile_missing");
    for(unsigned i=0;i<64;i++)if(!strchr("0123456789abcdef",profile[i]))profile_fatal("profile_invalid");
    char retained_profile[65];memcpy(retained_profile,profile,65);profile=retained_profile;
    char *output=path_env("OUTPUT",true),*control=path_env("CONTROL_OUTPUT",true);
    const char *meta_names[]={"METADATA_BEFORE","METADATA_CONTROL","METADATA_AFTER","METADATA_REPEAT"};
    const char *save_names[]={"SNAPSHOT_BEFORE","SNAPSHOT_CONTROL","SNAPSHOT_AFTER","SNAPSHOT_REPEAT"};
    char *paths[10]={output,control};
    for(unsigned i=0;i<4;i++){paths[2+i]=path_env(meta_names[i],true);paths[6+i]=path_env(save_names[i],false);}
    for(unsigned i=7;i<10;i++)if((paths[i]!=NULL)!=(paths[6]!=NULL))profile_fatal("optional_save_paths_incomplete");
    for(unsigned i=0;i<10;i++)for(unsigned j=0;j<i;j++)if(paths[i]&&paths[j]&&!strcmp(paths[i],paths[j]))profile_fatal("duplicate_path");
    FILE *controls=fopen(control,"wx");if(!controls)profile_fatal("controls_unavailable");
    fprintf(controls,"{\"kind\":\"header\",\"schema\":\"c6-linux-full-manager-profile-controls-v1\",\"profile_sha256\":\"%s\",\"known_rng_bytes\":64,\"complete_rng_inventory\":false,\"pure_query_claimed\":false,\"Save_equality_is_gate\":false}\n",profile);
    const void *unit=expected_actor(game,base,(int32_t)owner,id);
    if(!unit){fputs("{\"kind\":\"unsupported\",\"reason\":\"raw_actor_expectation_unavailable\",\"predicate_invoked\":false}\n",controls);fclose(controls);for(unsigned i=0;i<10;i++)free(paths[i]);return 244;}
    full_observation.module=module;full_observation.context=context;full_observation.game=game;full_observation.base=base;full_observation.profile=profile;full_observation.controls=controls;
    for(unsigned i=0;i<4;i++)full_observation.metadata[i]=paths[2+i];
    C6FullManagerFixtureAdmission a={.module=module,.context=context,.expected_game=game,.expected_unit=unit,.actor_owner=(int32_t)owner,.actor_full_id=id,.visible_only=visible!=0,.main_entry_tid=main_tid,.profile_sha256=profile,.output_path=output,
        .pre_acquisition=c6_linux_full_manager_pre_acquisition,.locked_begin=c6_linux_full_manager_locked_begin,.retained_owner=c6_linux_full_manager_retained_owner,.locked_end=c6_linux_full_manager_locked_end,.released_owner=c6_linux_full_manager_released_owner,.observe=observe};
    for(unsigned i=0;i<4;i++)a.diagnostic_save_paths[i]=paths[6+i];
    C6FullManagerFixtureResult result;int status=c6_linux_full_manager_fixture_run(&a,&result);
    fprintf(controls,"{\"kind\":\"summary\",\"status\":%d,\"predicate_invoked\":%s,\"predicate_returned\":%s,\"facade_released\":%s,\"observation_status\":%d,\"complete_affected_state_claimed\":false,\"native_action_requested_by_fixture\":false}\n",status,result.predicate_invoked?"true":"false",result.predicate_returned?"true":"false",result.facade_released?"true":"false",result.observation_status);
    fprintf(controls,"{\"kind\":\"evidence_status\",\"metadata_status\":[%d,%d,%d,%d],\"io_status\":%d,\"predicate_admission_veto\":false}\n",full_observation.metadata_status[0],full_observation.metadata_status[1],full_observation.metadata_status[2],full_observation.metadata_status[3],full_observation.io_status);
    if(ferror(controls)&&!status)status=247;
    if(fclose(controls)&&!status)status=247;
    memset(&full_observation,0,sizeof full_observation);for(unsigned i=0;i<10;i++)free(paths[i]);return status;
}
