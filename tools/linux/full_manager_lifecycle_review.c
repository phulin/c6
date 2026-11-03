/* Enclosing lifecycle selector tests with typed mocks only. */
#define dlsym reviewed_dlsym
#include "../../native/linux/action_lifecycle.c"
#undef dlsym
#include <assert.h>
#include <string.h>
#include <sys/wait.h>
#include <sys/mman.h>

typedef struct {
    unsigned initializes,caches,uninitializes,trace_begins,legacy_ends,observational_ends;
    unsigned observational_lookups,idles,probes,admission_begins,admission_ends,dispatches;
    unsigned expected_messages;
} Counts;
static Counts *counts;
static bool initialized_flag,context_bound=true,missing_observational;
static int trace_status,probe_status;
static unsigned test_mode;
static char controller_storage;
static void *active_controller=&controller_storage;
static void *const expected_context=(void *)0x44;
static bool is_initialized(void *controller) {assert(controller==active_controller);return initialized_flag;}
static bool is_executing(void *controller) {assert(controller==active_controller);return false;}
static void *get_context(void *out,const void *controller) {
    assert(controller==active_controller);*(void **)out=context_bound?expected_context:NULL;return out;
}
static void initialize(void *controller) {assert(controller==active_controller);counts->initializes++;initialized_flag=true;}
static void cache(void *controller) {assert(controller==active_controller&&initialized_flag);counts->caches++;}
static void uninitialize(void *controller) {assert(controller==active_controller&&initialized_flag);counts->uninitializes++;initialized_flag=false;context_bound=false;}
static int trace_begin(void *module) {assert(module==(void *)0x33);counts->trace_begins++;return 0;}
static int legacy_end(unsigned messages) {counts->legacy_ends++;counts->expected_messages=messages;return trace_status;}
static int observational_end(void) {counts->observational_ends++;return trace_status;}
static void parameters(void *module,void *context) {assert(module==(void *)0x33&&context==expected_context);}
static int idle(void *module,void *context) {parameters(module,context);counts->idles++;return 0;}
static int probe(void *module,void *context) {parameters(module,context);counts->probes++;return probe_status;}
static int admission_begin(void *module,void *context) {parameters(module,context);counts->admission_begins++;return 0;}
static int admission_end(void *module,void *context) {parameters(module,context);counts->admission_ends++;return 0;}
int c6_linux_action_registration(void *m,void *c) {parameters(m,c);assert(false);return 99;}
int c6_linux_action_execution(void *m,void *c) {parameters(m,c);assert(test_mode==3);return 0;}
int c6_linux_action_observe(void *m,void *c) {parameters(m,c);assert(test_mode==3);return 0;}
int c6_linux_action_dispatch(void *m,void *c,bool execute) {parameters(m,c);assert(test_mode==3&&execute);counts->dispatches++;return 0;}
void *reviewed_dlsym(void *handle,const char *name) {
    (void)handle;
    if(!strcmp(name,"_ZN18GameCoreController21ms_pkActiveControllerE"))return &active_controller;
    if(!strcmp(name,"_ZN18GameCoreController13IsInitializedEv"))return (void *)is_initialized;
    if(!strcmp(name,"_ZNK18GameCoreController10GetContextEv"))return (void *)get_context;
    if(!strcmp(name,"_ZN18GameCoreController10InitializeEv"))return (void *)initialize;
    if(!strcmp(name,"_ZN18GameCoreController15InitializeCacheEv"))return (void *)cache;
    if(!strcmp(name,"_ZN18GameCoreController12UninitializeEv"))return (void *)uninitialize;
    if(!strcmp(name,"_ZN18GameCoreController11IsExecutingEv"))return (void *)is_executing;
    if(!strcmp(name,"c6_linux_action_trace_begin"))return (void *)trace_begin;
    if(!strcmp(name,"c6_linux_action_trace_end"))return (void *)legacy_end;
    if(!strcmp(name,"c6_linux_action_trace_end_observational")) {
        counts->observational_lookups++;return missing_observational?NULL:(void *)observational_end;
    }
    const char *prefix=test_mode==1||test_mode==7?"c6_linux_current_handler_metadata_":
                       test_mode==2?"c6_linux_movement_cost_":"c6_linux_full_manager_profile_";
    assert(!strncmp(name,prefix,strlen(prefix)));
    const char *suffix=name+strlen(prefix);
    if(!strcmp(suffix,"caller_idle")||!strcmp(suffix,"quiescent"))return (void *)idle;
    if(!strcmp(suffix,"probe"))return (void *)probe;
    if(!strcmp(suffix,"admission_begin"))return (void *)admission_begin;
    if(!strcmp(suffix,"admission_end"))return (void *)admission_end;
    assert(false);return NULL;
}
int main(void) {
    counts=mmap(NULL,sizeof *counts,PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);
    assert(counts!=MAP_FAILED);
    for(unsigned mode=0;mode<9;mode++) {
        memset(counts,0,sizeof *counts);FILE *error=tmpfile();assert(error);pid_t child=fork();assert(child>=0);
        if(!child) {
            assert(dup2(fileno(error),STDERR_FILENO)>=0);test_mode=mode;
            missing_observational=mode==6||mode==7;
            trace_status=mode==1||mode==2?209:mode==4||mode==8?208:mode==5?207:0;
            probe_status=mode==8?29:0;
            int status=mode==1||mode==7?c6_linux_action_prepare_current_handler_metadata((void *)0x33,expected_context):
                       mode==2?c6_linux_action_prepare_movement_cost((void *)0x33,expected_context):
                       mode==3?c6_linux_action_prepare_dispatch((void *)0x33,expected_context,true):
                       c6_linux_action_prepare_full_manager_fixture((void *)0x33,expected_context);
            assert(mode!=6);assert(status==(probe_status?probe_status:trace_status));
            assert(counts->initializes==1&&counts->caches==1&&counts->uninitializes==1&&counts->trace_begins==1);
            if(mode==1||mode==2||mode==3||mode==7) {
                assert(counts->legacy_ends==1&&counts->observational_ends==0&&counts->observational_lookups==0);
                assert(counts->expected_messages==(mode==3?1u:0u));
            } else assert(counts->legacy_ends==0&&counts->observational_ends==1&&counts->observational_lookups==1);
            if(mode==3)assert(counts->dispatches==1&&!counts->probes);
            else assert(counts->probes==1&&counts->idles==2&&counts->admission_begins==1&&counts->admission_ends==1);
            _exit(0);
        }
        int status;assert(waitpid(child,&status,0)==child&&WIFEXITED(status));
        assert(WEXITSTATUS(status)==(mode==6?184:0));
        if(mode==6) {
            assert(counts->trace_begins==0&&counts->initializes==0);
            rewind(error);char text[512]={0};assert(fread(text,1,sizeof text-1,error)>0&&strstr(text,"missing_observational_trace_symbol"));
        }
        assert(!fclose(error));printf("lifecycle case %u passed\n",mode);
    }
    return 0;
}
