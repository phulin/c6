#define _GNU_SOURCE
#include "core_component.h"
#include "action_state.h"
#include "context_stream.h"
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sched.h>
#include <time.h>
#include <unistd.h>

#define WORKER_DEADLINE_MS 30000u

/* A timeout can leave native work live. Discard the isolated process before
 * observations, Save, facade teardown or wrapper Uninitialize can race it. */
static _Noreturn void worker_failure(const char *reason) {
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"worker_wait_failed\",\"reason\":\"%s\",\"worker_quiescence_proven\":false,\"post_state_captured\":false,\"full_action_acceptance\":0,\"gameplay_parity\":false}\n",reason);
    fflush(NULL);
    _exit(240);
}

static uint64_t monotonic_ms(void) {
    struct timespec now;
    if(clock_gettime(CLOCK_MONOTONIC,&now))worker_failure("monotonic_clock_failed");
    return (uint64_t)now.tv_sec*1000u+(uint64_t)now.tv_nsec/1000000u;
}

static void wait_for_original_pass(void **controller,bool (*executing)(void *),
                                   void (*synchronize)(void *),void (*join)(void *),
                                   bool (*current_thread)(void *),bool (*suspended)(void *)) {
    /* Exact original Execute/IsExecuting/Join/Synchronize instructions prove
     * the borrowed global worker slot is exported controller-static +8. */
    void *worker=NULL;memcpy(&worker,(const unsigned char *)controller+8,sizeof worker);
    if(!worker||current_thread(worker))worker_failure("missing_or_current_worker");
    uint64_t start=monotonic_ms(),polls=0;
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"worker_wait_begin\",\"deadline_ms\":%u,\"original_worker_present\":true,\"original_worker_is_current_thread\":false}\n",WORKER_DEADLINE_MS);
    /* Execute sets the original counter before Resume; Run clears it only
     * after ProcessMessages. Yielding is resource scheduling, not completion. */
    while(executing(*controller)){
        if(monotonic_ms()-start>=WORKER_DEADLINE_MS)worker_failure("original_pass_deadline");
        ++polls;(void)sched_yield();
    }
    /* Controller.Synchronize returns early when the counter is already zero.
     * Direct original Join is required to acquire/release the worker lock and
     * finish its post-counter tail before observing the native state. */
    join(worker);synchronize(*controller);
    /* Run releases its lock before its final original IThread.Suspend. Wait
     * for that native lifecycle boundary too, so teardown/next wake cannot
     * outrun suspension. This profile still executes only one command. */
    while(!suspended(worker)){
        if(monotonic_ms()-start>=WORKER_DEADLINE_MS)worker_failure("original_suspend_deadline");
        ++polls;(void)sched_yield();
    }
    void *same_worker=NULL;memcpy(&same_worker,(const unsigned char *)controller+8,sizeof same_worker);
    if(same_worker!=worker||executing(*controller))worker_failure("worker_identity_or_completion_changed");
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"worker_quiescent\",\"original_is_executing\":false,\"original_join_returned\":true,\"original_synchronize_returned\":true,\"original_worker_suspended\":true,\"worker_identity_unchanged\":true,\"polls\":%llu,\"elapsed_ms\":%llu}\n",(unsigned long long)polls,(unsigned long long)(monotonic_ms()-start));
}

struct status {int turn,phase,local;bool pending,update,advance,ai,processing;};
struct api {
    void *(*game)(void *);void (*release)(void *);int (*local)(const void *);
    int (*turn)(const void *);int (*phase)(const void *);
    bool (*pending)(void *),(*update)(const void *),(*advance)(const void *),(*ai)(const void *),(*processing)(const void *);
};
static struct status status(struct api *a,void *context,void *game,const char *stage) {
    struct status s={a->turn(game),a->phase(game),a->local(context),a->pending(context),a->update(game),a->advance(game),a->ai(game),a->processing(game)};
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"%s\",\"turn\":%d,\"phase\":%d,\"local\":%d,\"pending\":%d,\"needs_update\":%d,\"needs_advance\":%d,\"ai_processing\":%d,\"turn_processing\":%d}\n",stage,s.turn,s.phase,s.local,s.pending,s.update,s.advance,s.ai,s.processing);
    return s;
}
/* Storage is initialized only by original constructors. Request sizeof 32 is
 * matching DWARF; original CreateMessage allocates exactly 0x48 bytes. Streams
 * follow the original constructors/destructors used by context_stream.c. */
static int build_and_queue(void *module,void *context,const void *identity,
                           const void *parameters,int sender,bool execute) {
    void (*request_ctor)(void *),(*request_dtor)(void *);
    void (*message_ctor)(void *),(*message_dtor)(void *);
    void (*message_init)(void *,const void *,const void *,int);
    void (*put)(const void *,void *),(*queue)(void *,int,void *);
    void *(*manager_get)(void),*(*allocate)(size_t);
    void (*output_ctor)(void *,void *,size_t),(*output_delete)(void *);
    void (*input_ctor)(void *,const void *,size_t),(*input_dtor)(void *);
    size_t (*offset)(const void *);
#define C(V,S) do{V=dlsym(module,S);if(!V)return 234;}while(0)
#define H(V,S) do{V=dlsym(RTLD_DEFAULT,S);if(!V)return 234;}while(0)
    C(request_ctor,"_ZN8GameCore4Unit10Operations7RequestC2Ev");
    C(request_dtor,"_ZN8GameCore4Unit10Operations9OperationD2Ev");
    C(message_ctor,"_ZN8GameCore8Messages13UnitOperationC1Ev");
    C(message_dtor,"_ZN8GameCore8Messages13UnitOperationD2Ev");
    C(message_init,"_ZN8GameCore8Messages13UnitOperation4InitERKNS_6Player11ComponentIDERKNS_4Unit10Operations7RequestENS7_10InsertModeE");
    C(manager_get,"_ZN8GameCore4Unit10Operations7Manager3GetEv");
    C(put,"_ZNK8GameCore8Messages11BaseMessage11PutInBufferERN13Serialization7OStreamE");
    C(queue,"_ZN8GameCore7Exports7Context12QueueMessageEiRN13Serialization13MemoryIStreamE");
    H(allocate,"_Znwm");
    H(output_ctor,"_ZN13Serialization13MemoryOStreamC1EPvm");
    H(output_delete,"_ZN13Serialization13MemoryOStreamD0Ev");
    H(input_ctor,"_ZN13Serialization13MemoryIStreamC1EPKvm");
    H(input_dtor,"_ZN13Serialization13MemoryIStreamD1Ev");
    H(offset,"_ZNK13Serialization13MemoryOStream9GetOffsetEv");
#undef C
#undef H
    const char *path=getenv("C6_CORE_COMPONENT_COMMAND");
    void *manager=manager_get();
    if(!manager||!path)return 235;
    bool (*make)(const void *,uint32_t,const void *,void *)=(*(void ***)manager)[0x100/8];
    if(!make)return 235;
    _Alignas(16) unsigned char request[0x20],message[0x48],input[0x28];
    unsigned char bytes[4096];
    void *output=allocate(0x28);if(!output)return 236;
    output_ctor(output,bytes,sizeof bytes);
    request_ctor(request);
    bool accepted=make(manager,UINT32_C(0x2c681908),parameters,request);
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"conversion\",\"accepted\":%s,\"bAppend\":%u,\"bFromUser\":%u,\"source\":\"fresh_original_constructor\"}\n",accepted?"true":"false",request[28],request[29]);
    int result=accepted?0:237;
    if(accepted){
        message_ctor(message);message_init(message,identity,request,0);
        put(message,output);
        size_t length=offset(output);
        fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"serialized\",\"length\":%zu,\"sender\":%d,\"execute\":%s,\"host_admission_proven\":false}\n",length,sender,execute?"true":"false");
        /* Empty-map SkipTurn's original encoding is bounded and 28 bytes.
         * No encoded fields or operation IDs are manufactured here. */
        if(length!=28)result=238;
        if(!result){
            FILE *file=fopen(path,"wbx");
            if(!file)result=239;
            else{bool written=fwrite(bytes,1,length,file)==length;
                if(fclose(file)||!written)result=239;}
        }
        if(!result&&execute){
            input_ctor(input,bytes,length);
            queue(context,sender,input);
            input_dtor(input);
        }
        message_dtor(message);
    }
    request_dtor(request);output_delete(output);
    return result;
}
int c6_linux_core_component(void *module,void *context,bool execute) {
    if(!module||!context)return 220;
    struct api a={0};
#define CORE(VAR,SYMBOL) do{VAR=dlsym(module,SYMBOL);if(!VAR)return 221;}while(0)
#define HOST(VAR,SYMBOL) do{VAR=dlsym(RTLD_DEFAULT,SYMBOL);if(!VAR)return 221;}while(0)
    CORE(a.game,"_ZN8GameCore7Exports7Context7GetGameEv");
    CORE(a.release,"_ZN8GameCore10Interfaces10ICvUnknowndlEPv");
    CORE(a.local,"_ZNK8GameCore7Exports7Context14GetLocalPlayerEv");
    CORE(a.turn,"_ZNK8GameCore7Exports4Game14GetCurrentTurnEv");
    CORE(a.phase,"_ZNK8GameCore7Exports4Game15GetCurrentPhaseEv");
    CORE(a.pending,"_ZN8GameCore7Exports7Context18HasPendingMessagesEv");
    CORE(a.update,"_ZNK8GameCore7Exports4Game11NeedsUpdateEv");
    CORE(a.advance,"_ZNK8GameCore7Exports4Game16NeedsTurnAdvanceEv");
    CORE(a.ai,"_ZNK8GameCore7Exports4Game14IsAiProcessingEv");
    CORE(a.processing,"_ZNK8GameCore7Exports4Game16IsTurnProcessingEv");
    void *(*get_player)(void *,int),*(*get_units)(const void *),*(*operations)(void *);
    void *(*cached_player)(const void *,int);bool (*facade_active)(const void *),(*facade_human)(const void *);
    void *(*begin)(const void *),*(*end)(const void *),*(*next)(const void *,void *),*(*get)(const void *,void *);
    unsigned (*count)(const void *),(*unit_id)(const void *);int (*owner)(const void *);
    void *(*identity)(void *,const void *);
    bool (*can_start)(const void *,const void *,uint32_t,const void *,bool,void *);
    const void *(*core_player)(int),*(*core_units)(const void *),*(*find)(const void *,unsigned);
    bool (*complete)(const void *),(*pending_actions)(void),(*pending_sessions)(void);
    bool (*playback)(void *),(*recording)(void *);
    void *(*default_manager)(void),*(*map_create)(void *,int);void (*map_delete)(void *);
    void **controller;bool (*executing)(void *),(*suspended)(const void *);
    bool (*has_lock)(const void *);
    bool (*native_loading)(void),(*native_saving)(void);
    int (*request_in_progress)(void);void *(*current_state)(void *);
    void **application;const bool *request_active;const int *request_type;
    void (*pump)(void *),(*sync)(void *),(*join)(void *);bool (*worker_current)(void *),(*thread_suspended)(void *);
    void *(*network)(void);int (*network_local)(const void *);
    CORE(get_player,"_ZN8GameCore7Exports7Context9GetPlayerENS_11PlayerTypesE");
    CORE(get_units,"_ZNK8GameCore7Exports6Player8GetUnitsEv");
    CORE(cached_player,"_ZNK8GameCore7Exports7Context14GetCachePlayerENS_11PlayerTypesE");
    CORE(facade_active,"_ZNK8GameCore7Exports6Player12IsTurnActiveEv");CORE(facade_human,"_ZNK8GameCore7Exports6Player7IsHumanEv");
    CORE(operations,"_ZN8GameCore7Exports7Context17GetUnitOperationsEv");
    CORE(begin,"_ZNK8GameCore7Exports11PlayerUnits5BeginEv");CORE(end,"_ZNK8GameCore7Exports11PlayerUnits3EndEv");
    CORE(next,"_ZNK8GameCore7Exports11PlayerUnits4NextEPv");CORE(get,"_ZNK8GameCore7Exports11PlayerUnits3GetEPv");
    CORE(count,"_ZNK8GameCore7Exports11PlayerUnits7GetSizeEv");CORE(unit_id,"_ZNK8GameCore7Exports4Unit5GetIDEv");
    CORE(owner,"_ZNK8GameCore7Exports4Unit8GetOwnerEv");CORE(identity,"_ZNK8GameCore7Exports4Unit14GetComponentIDEv");
    CORE(can_start,"_ZNK8GameCore7Exports14UnitOperations8CanStartERKNS_6Player11ComponentIDENS_4Unit10Operations5Types8TypeHashERKN4Data15TypedVariantMapEbPSB_");
    CORE(core_player,"_ZN8GameCore7Context7Globals9GetPlayerENS_11PlayerTypesE");
    CORE(core_units,"_ZNK8GameCore6Player8Instance8GetUnitsEv");CORE(find,"_ZNK8GameCore6Player5Units4FindEj");
    CORE(complete,"_ZNK8GameCore6Player8Instance20IsTurnActiveCompleteEv");
    CORE(pending_actions,"_ZN8GameCore9Diplomacy7Manager17HasPendingActionsEv");
    CORE(pending_sessions,"_ZN8GameCore9Diplomacy7Manager27HasPendingSessionStatementsEv");
    CORE(playback,"_ZN8GameCore7Exports7Context17IsMessagePlaybackEv");CORE(recording,"_ZN8GameCore7Exports7Context18IsMessageRecordingEv");
    HOST(default_manager,"_ZN4Data19TypedVariantManager10GetDefaultEv");HOST(map_create,"_ZN4Data19TypedVariantManager32CreateKeyedTypedVariantContainerENS_21VariantContainerTypesE");HOST(map_delete,"_ZN4Data15TypedVariantMapD0Ev");
    HOST(controller,"_ZN18GameCoreController21ms_pkActiveControllerE");HOST(executing,"_ZN18GameCoreController11IsExecutingEv");HOST(suspended,"_ZNK18GameCoreController25IsGameCoreUpdateSuspendedEv");
    HOST(has_lock,"_ZNK18GameCoreController7HasLockEv");
    HOST(pump,"_ZN18GameCoreController21ExecuteGameCoreUpdateEv");HOST(sync,"_ZN18GameCoreController11SynchronizeEv");
    HOST(join,"_ZN14GameCoreThread18JoinOnPassCompleteEv");HOST(worker_current,"_ZN14GameCoreThread15IsCurrentThreadEv");
    HOST(thread_suspended,"_ZN14GameCoreThread11IsSuspendedEv");
    HOST(native_loading,"_ZN8GameCore10Serializer7Manager16IsLoadInProgressEv");HOST(native_saving,"_ZN8GameCore10Serializer7Manager16IsSaveInProgressEv");
    HOST(request_in_progress,"_ZN8GameCore10Serializer7Manager20GetRequestInProgressEv");
    HOST(current_state,"_ZN7Civ6App19GetCurrentGameStateEv");
    HOST(application,"g_pkApplication");
    HOST(request_active,"_ZN8GameCore10Serializer21ms_bRequestInProgressE");
    HOST(request_type,"_ZN8GameCore10Serializer15ms_eRequestTypeE");
    HOST(network,"_ZN7Network7Manager11GetInstanceEv");HOST(network_local,"_ZNK7Network7Manager16GetLocalPlayerIDEv");
#undef CORE
#undef HOST
    void (*trace_mark)(unsigned)=dlsym(RTLD_DEFAULT,"c6_linux_action_trace_mark");
    unsigned (*trace_units)(void)=dlsym(RTLD_DEFAULT,"c6_linux_action_trace_unit_executed");
    if(!trace_mark||!trace_units)return 221;
    int (*identity_owner)(const void *)=dlsym(module,"_ZNK8GameCore6Player11ComponentID8GetOwnerEv");
    if(!identity_owner)return 221;
    const char *before=getenv("C6_CORE_COMPONENT_SNAPSHOT_BEFORE"),*after=getenv("C6_CORE_COMPONENT_SNAPSHOT_AFTER"),*repeat=getenv("C6_CORE_COMPONENT_SNAPSHOT_REPEAT");
    bool present=*controller!=NULL;
    bool worker_executing=present&&executing(*controller);
    bool worker_suspended=present&&suspended(*controller);
    bool loading=native_loading(),saving=native_saving();
    int request_kind=request_in_progress();
    void *app_state=*application?current_state(*application):NULL,*load_state=NULL;
    /* This is the original IsLoadInProgress comparison operand, read only.
     * The getter can report true for null==null in a minimal host fixture. */
    if(*application)memcpy(&load_state,(const unsigned char *)*application+0x2310,sizeof load_state);
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"serializer_observation\",\"request_active\":%d,\"request_type_raw\":%d,\"request_in_progress\":%d,\"application_present\":%d,\"current_state\":\"%p\",\"load_state\":\"%p\",\"state_identity_equal\":%d}\n",*request_active,*request_type,request_kind,*application!=NULL,app_state,load_state,app_state==load_state);
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"host_worker_preconditions\",\"snapshot_before_present\":%d,\"snapshot_after_present\":%d,\"snapshot_repeat_present\":%d,\"controller_present\":%d,\"worker_executing\":%d,\"worker_suspended\":%d,\"native_loading\":%d,\"native_saving\":%d}\n",before!=NULL,after!=NULL,repeat!=NULL,present,worker_executing,worker_suspended,loading,saving);
    /* A minimal original App has no current/loading state objects. Original
     * IsLoadInProgress then returns true for NULL==NULL despite no request.
     * Accept only this observed profile, never an actual request/non-null load. */
    bool null_state_profile=*application&&app_state==NULL&&load_state==NULL;
    bool serializer_idle=!*request_active&&request_kind==0&&!saving&&(!loading||null_state_profile);
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"serializer_guard\",\"no_state_profile\":%d,\"actual_request_idle\":%d}\n",null_state_profile,serializer_idle);
    if(!before||!after||!repeat||!present||worker_executing||worker_suspended||!serializer_idle)return 222;
    /* Actual constructor and ExecuteGameCoreUpdate bytes establish this native
     * threaded-mode selector. Read only; do not force a mode or construct a worker. */
    unsigned char threaded=*((const unsigned char *)*controller+4);
    if(threaded!=1)return 223;
    void *game=a.game(context);if(!game)return 224;
    struct status initial=status(&a,context,game,"before");
    bool dip_actions=pending_actions(),dip_sessions=pending_sessions();
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"diplomacy_before\",\"pending_actions\":%d,\"open_session_map_nonempty\":%d,\"threaded\":%u}\n",dip_actions,dip_sessions,threaded);
    if(initial.local<0||initial.local>=64||initial.pending||initial.update||initial.advance||initial.ai||initial.processing||dip_actions||dip_sessions||playback(context)||recording(context)){a.release(game);return 225;}
    const void *player_core=core_player(initial.local),*unit_container=player_core?core_units(player_core):NULL;
    bool activation_complete=player_core&&complete(player_core);
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"player_activation\",\"core_player_present\":%d,\"unit_container_present\":%d,\"activation_complete\":%d}\n",player_core!=NULL,unit_container!=NULL,activation_complete);
    /* IsTurnActiveComplete means active with no pending activation update,
     * not that the player has ended the turn. Retain original readiness. */
    if(!unit_container||!activation_complete){a.release(game);return 226;}
    void *player=get_player(context,initial.local),*cached=cached_player(context,initial.local);
    bool control_ready=player&&cached&&facade_active(player)&&facade_human(player)&&facade_active(cached)&&facade_human(cached);
    if(cached)a.release(cached);
    if(!control_ready){if(player)a.release(player);a.release(game);return 232;}
    void *list=get_units(player),*ops=operations(context),*params=map_create(default_manager(),2);
    if(!player||!list||!ops||!params){if(params)map_delete(params);if(ops)a.release(ops);if(list)a.release(list);if(player)a.release(player);a.release(game);return 227;}
    uint64_t selected_bits=0;unsigned selected_id=0,selected_queued=0,seen=0,total=count(list);const void *unit=NULL;
    for(void *it=begin(list),*stop=end(list);it!=stop;it=next(list,it)){
        if(seen++>=total)break;
        void *facade=get(list,it);if(!facade)break;
        unsigned id=unit_id(facade);int who=owner(facade);uint64_t bits;identity(&bits,facade);
        const void *candidate=who==initial.local?find(unit_container,id):NULL;unsigned queued=~0u;
        bool allowed=can_start(ops,&bits,UINT32_C(0x2c681908),params,false,NULL);
        int code=allowed&&candidate?c6_linux_action_unit_queue_size(module,candidate,&queued):0;
        fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"actor_candidate\",\"owner\":%d,\"id\":%u,\"component_bits\":\"%016llx\",\"public_can_start\":%d,\"core_found\":%d,\"queue_query_status\":%d,\"queue_size\":%u}\n",who,id,(unsigned long long)bits,allowed,candidate!=NULL,code,queued);
        if(allowed&&candidate&&!code&&queued<=1){selected_queued=queued;selected_id=id;identity(&selected_bits,facade);unit=candidate;a.release(facade);break;}
        a.release(facade);
    }
    bool component_enqueued=false;
    int result=unit?0:228;
    if(!result){
        fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"selected\",\"mode\":\"%s\",\"owner\":%d,\"id\":%u,\"component_bits\":\"%016llx\",\"operation\":\"SKIP_TURN\",\"hash\":\"2c681908\",\"native_queue_size\":%u,\"bAppend\":false,\"bFromUser\":false}\n",execute?"execute":"control",initial.local,selected_id,(unsigned long long)selected_bits,selected_queued);
        result=c6_linux_action_state_capture(module,unit,"before_a");
        if(!result)result=c6_linux_action_state_capture(module,unit,"before_b");
        if(!result)result=c6_linux_context_save(module,context,before);
    }
    if(!result){
        void *net=network();int sender=net?network_local(net):-1;
        fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"admission_assumptions\",\"source\":\"fresh_native_make_request_component\",\"sender\":%d,\"identity_owner\":%d,\"host_admission_proven\":false,\"full_action_acceptance\":0}\n",sender,identity_owner(&selected_bits));
        if(sender!=initial.local||identity_owner(&selected_bits)!=initial.local)result=233;
        if(!result){trace_mark(1);result=build_and_queue(module,context,&selected_bits,params,sender,execute);trace_mark(2);}
        component_enqueued=execute&&a.pending(context);
        if(!result){
            if(execute&&!a.pending(context))result=229;
            else{
                /* Genuine live facade constructors acquire the controller's
                 * game-core lock. The worker must acquire that same lock in
                 * EditGame: destroy every acquired facade before Resume.
                 * Native params/identity and borrowed core refs survive.
                 * Control performs the same release/reacquisition boundary. */
                a.release(ops);ops=NULL;a.release(list);list=NULL;
                a.release(player);player=NULL;a.release(game);game=NULL;
                bool lock_held=has_lock(*controller);
                fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"facades_released\",\"execute\":%s,\"game\":true,\"player\":true,\"unit_list\":true,\"operations\":true,\"native_parameters_retained\":true,\"original_controller_lock_held\":%s}\n",execute?"true":"false",lock_held?"true":"false");
                if(lock_held){map_delete(params);return 242;}
                if(execute){trace_mark(5);pump(*controller);wait_for_original_pass(controller,executing,sync,join,worker_current,thread_suspended);trace_mark(6);}
                game=a.game(context);ops=operations(context);
                if(!game||!ops){
                    fprintf(stderr,"C6_LINUX_CORE_COMPONENT observation_facade_reacquisition_failed\n");
                    map_delete(params);if(ops)a.release(ops);if(game)a.release(game);return 241;
                }
                lock_held=has_lock(*controller);
                fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"observation_facades_reacquired\",\"game\":true,\"operations\":true,\"after_original_worker_quiescence\":%s,\"original_controller_lock_held\":%s}\n",execute?"true":"false",lock_held?"true":"false");
                if(!lock_held){map_delete(params);a.release(ops);a.release(game);return 242;}
                if(execute&&(executing(*controller)||a.pending(context)||trace_units()!=1))result=230;}
        }
        if(!result&&!execute&&(a.pending(context)||trace_units()))result=230;
    }
    trace_mark(7);
    if(unit){
        unit=find(unit_container,selected_id);
        bool allowed=unit&&can_start(ops,&selected_bits,UINT32_C(0x2c681908),params,false,NULL);
        fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"predicate_after\",\"can_start_same_skip_turn\":%d,\"unit_present\":%d}\n",allowed,unit!=NULL);
        int observed=c6_linux_action_state_capture(module,unit,"after_a");if(!result)result=observed;
        observed=c6_linux_action_state_capture(module,unit,"after_b");if(!result)result=observed;
        observed=c6_linux_context_save(module,context,after);if(!result)result=observed;
        observed=c6_linux_context_save(module,context,repeat);if(!result)result=observed;

    }
    struct status final=status(&a,context,game,"after");
    bool final_actions=pending_actions(),final_sessions=pending_sessions();
    fprintf(stderr,"C6_LINUX_CORE_COMPONENT {\"stage\":\"diplomacy_after\",\"pending_actions\":%d,\"open_session_map_nonempty\":%d,\"result\":%d,\"component_enqueued\":%s,\"gameplay_parity\":false,\"full_action_acceptance\":0}\n",final_actions,final_sessions,result,component_enqueued?"true":"false");
    if(!result&&(final.turn!=initial.turn||final.phase!=initial.phase||final.local!=initial.local||final.pending||final.update||final.advance||final.ai||final.processing||final_actions||final_sessions))result=231;
    map_delete(params);if(ops)a.release(ops);if(list)a.release(list);if(player)a.release(player);if(game)a.release(game);
    return result;
}
