#define _GNU_SOURCE
#include "action_dispatch.h"
#include "action_state.h"
#include "context_stream.h"
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct status {int turn,phase,local;bool pending,update,advance,ai,processing;};
struct api {
    void *(*game)(void *);void (*release)(void *);int (*local)(const void *);
    int (*turn)(const void *);int (*phase)(const void *);
    bool (*pending)(void *),(*update)(const void *),(*advance)(const void *),(*ai)(const void *),(*processing)(const void *);
};
static struct status status(struct api *a,void *context,void *game,const char *stage) {
    struct status s={a->turn(game),a->phase(game),a->local(context),a->pending(context),a->update(game),a->advance(game),a->ai(game),a->processing(game)};
    fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"%s\",\"turn\":%d,\"phase\":%d,\"local\":%d,\"pending\":%d,\"needs_update\":%d,\"needs_advance\":%d,\"ai_processing\":%d,\"turn_processing\":%d}\n",stage,s.turn,s.phase,s.local,s.pending,s.update,s.advance,s.ai,s.processing);
    return s;
}
int c6_linux_action_dispatch(void *module,void *context,bool execute) {
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
    void (*request)(void *,const void *,uint32_t,const void *);
    const void *(*core_player)(int),*(*core_units)(const void *),*(*find)(const void *,unsigned);
    bool (*complete)(const void *),(*pending_actions)(void),(*pending_sessions)(void);
    bool (*playback)(void *),(*recording)(void *);
    void *(*default_manager)(void),*(*map_create)(void *,int);void (*map_delete)(void *);
    void **controller;bool (*executing)(void *),(*suspended)(const void *);
    bool (*native_loading)(void),(*native_saving)(void);
    int (*request_in_progress)(void);void *(*current_state)(void *);
    void **application;const bool *request_active;const int *request_type;
    void (*pump)(void *),(*sync)(void *);void *(*network)(void);void (*network_update)(void *,float);
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
    CORE(request,"_ZN8GameCore7Exports7Context20RequestUnitOperationERKNS_6Player11ComponentIDENS_4Unit10Operations5Types8TypeHashERKN4Data15TypedVariantMapE");
    CORE(core_player,"_ZN8GameCore7Context7Globals9GetPlayerENS_11PlayerTypesE");
    CORE(core_units,"_ZNK8GameCore6Player8Instance8GetUnitsEv");CORE(find,"_ZNK8GameCore6Player5Units4FindEj");
    CORE(complete,"_ZNK8GameCore6Player8Instance20IsTurnActiveCompleteEv");
    CORE(pending_actions,"_ZN8GameCore9Diplomacy7Manager17HasPendingActionsEv");
    CORE(pending_sessions,"_ZN8GameCore9Diplomacy7Manager27HasPendingSessionStatementsEv");
    CORE(playback,"_ZN8GameCore7Exports7Context17IsMessagePlaybackEv");CORE(recording,"_ZN8GameCore7Exports7Context18IsMessageRecordingEv");
    HOST(default_manager,"_ZN4Data19TypedVariantManager10GetDefaultEv");HOST(map_create,"_ZN4Data19TypedVariantManager32CreateKeyedTypedVariantContainerENS_21VariantContainerTypesE");HOST(map_delete,"_ZN4Data15TypedVariantMapD0Ev");
    HOST(controller,"_ZN18GameCoreController21ms_pkActiveControllerE");HOST(executing,"_ZN18GameCoreController11IsExecutingEv");HOST(suspended,"_ZNK18GameCoreController25IsGameCoreUpdateSuspendedEv");
    HOST(pump,"_ZN18GameCoreController21ExecuteGameCoreUpdateEv");HOST(sync,"_ZN18GameCoreController11SynchronizeEv");
    HOST(native_loading,"_ZN8GameCore10Serializer7Manager16IsLoadInProgressEv");HOST(native_saving,"_ZN8GameCore10Serializer7Manager16IsSaveInProgressEv");
    HOST(request_in_progress,"_ZN8GameCore10Serializer7Manager20GetRequestInProgressEv");
    HOST(current_state,"_ZN7Civ6App19GetCurrentGameStateEv");
    HOST(application,"g_pkApplication");
    HOST(request_active,"_ZN8GameCore10Serializer21ms_bRequestInProgressE");
    HOST(request_type,"_ZN8GameCore10Serializer15ms_eRequestTypeE");
    HOST(network,"_ZN7Network7Manager11GetInstanceEv");HOST(network_update,"_ZN7Network7Manager6UpdateEf");
#undef CORE
#undef HOST
    int (*identity_owner)(const void *)=dlsym(module,"_ZNK8GameCore6Player11ComponentID8GetOwnerEv");
    void *(*get_session)(void *)=dlsym(RTLD_DEFAULT,"_ZN7Network7Manager10GetSessionEv");
    void *(*get_sync)(const void *)=dlsym(RTLD_DEFAULT,"_ZNK11FNetSession14GetSyncManagerEv");
    void *(*engine_utility)(void)=dlsym(module,"_ZN8GameCore7Context7Globals16GetEngineUtilityEv");
    if(!identity_owner||!get_session||!get_sync||!engine_utility)return 221;
    const char *before=getenv("C6_ACTION_SNAPSHOT_BEFORE"),*after=getenv("C6_ACTION_SNAPSHOT_AFTER"),*repeat=getenv("C6_ACTION_SNAPSHOT_REPEAT");
    bool present=*controller!=NULL;
    bool worker_executing=present&&executing(*controller);
    bool worker_suspended=present&&suspended(*controller);
    bool loading=native_loading(),saving=native_saving();
    int request_kind=request_in_progress();
    void *app_state=*application?current_state(*application):NULL,*load_state=NULL;
    /* This is the original IsLoadInProgress comparison operand, read only.
     * The getter can report true for null==null in a minimal host fixture. */
    if(*application)memcpy(&load_state,(const unsigned char *)*application+0x2310,sizeof load_state);
    fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"serializer_observation\",\"request_active\":%d,\"request_type_raw\":%d,\"request_in_progress\":%d,\"application_present\":%d,\"current_state\":\"%p\",\"load_state\":\"%p\",\"state_identity_equal\":%d}\n",*request_active,*request_type,request_kind,*application!=NULL,app_state,load_state,app_state==load_state);
    fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"host_worker_preconditions\",\"snapshot_before_present\":%d,\"snapshot_after_present\":%d,\"snapshot_repeat_present\":%d,\"controller_present\":%d,\"worker_executing\":%d,\"worker_suspended\":%d,\"native_loading\":%d,\"native_saving\":%d}\n",before!=NULL,after!=NULL,repeat!=NULL,present,worker_executing,worker_suspended,loading,saving);
    /* A minimal original App has no current/loading state objects. Original
     * IsLoadInProgress then returns true for NULL==NULL despite no request.
     * Accept only this observed profile, never an actual request/non-null load. */
    bool null_state_profile=*application&&app_state==NULL&&load_state==NULL;
    bool serializer_idle=!*request_active&&request_kind==0&&!saving&&(!loading||null_state_profile);
    fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"serializer_guard\",\"no_state_profile\":%d,\"actual_request_idle\":%d}\n",null_state_profile,serializer_idle);
    if(!before||!after||!repeat||!present||worker_executing||worker_suspended||!serializer_idle)return 222;
    /* Actual constructor and ExecuteGameCoreUpdate bytes establish this native
     * threaded-mode selector. Read only; do not force a mode or construct a worker. */
    unsigned char threaded=*((const unsigned char *)*controller+4);
    if(threaded!=1)return 223;
    void *game=a.game(context);if(!game)return 224;
    struct status initial=status(&a,context,game,"before");
    bool dip_actions=pending_actions(),dip_sessions=pending_sessions();
    fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"diplomacy_before\",\"pending_actions\":%d,\"open_session_map_nonempty\":%d,\"threaded\":%u}\n",dip_actions,dip_sessions,threaded);
    if(initial.local<0||initial.local>=64||initial.pending||initial.update||initial.advance||initial.ai||initial.processing||dip_actions||dip_sessions||playback(context)||recording(context)){a.release(game);return 225;}
    const void *player_core=core_player(initial.local),*unit_container=player_core?core_units(player_core):NULL;
    bool activation_complete=player_core&&complete(player_core);
    fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"player_activation\",\"core_player_present\":%d,\"unit_container_present\":%d,\"activation_complete\":%d}\n",player_core!=NULL,unit_container!=NULL,activation_complete);
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
        fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"actor_candidate\",\"owner\":%d,\"id\":%u,\"component_bits\":\"%016llx\",\"public_can_start\":%d,\"core_found\":%d,\"queue_query_status\":%d,\"queue_size\":%u}\n",who,id,(unsigned long long)bits,allowed,candidate!=NULL,code,queued);
        if(allowed&&candidate&&!code&&queued<=1){selected_queued=queued;selected_id=id;identity(&selected_bits,facade);unit=candidate;a.release(facade);break;}
        a.release(facade);
    }
    bool request_issued=false;
    int result=unit?0:228;
    if(!result){
        fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"selected\",\"mode\":\"%s\",\"owner\":%d,\"id\":%u,\"component_bits\":\"%016llx\",\"operation\":\"SKIP_TURN\",\"hash\":\"2c681908\",\"native_queue_size\":%u,\"bAppend\":false,\"bFromUser\":false}\n",execute?"execute":"control",initial.local,selected_id,(unsigned long long)selected_bits,selected_queued);
        result=c6_linux_action_state_capture(module,unit,"before_a");
        if(!result)result=c6_linux_action_state_capture(module,unit,"before_b");
        if(!result)result=c6_linux_context_save(module,context,before);
    }
    if(!result&&execute){
        void *net=network(),*session=net?get_session(net):NULL,*transport=NULL;
        if(session)memcpy(&transport,(const unsigned char *)session+0x28,sizeof transport);
        void *utility=engine_utility();
        void *send=dlsym(RTLD_DEFAULT,"_ZN8GameCore13EngineUtility11SendMessageERN13Serialization13MemoryOStreamE");
        bool utility_bound=utility&&send&&(*(void ***)utility)[0x78/8]==send;
        fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"delivery_prerequisites\",\"identity_owner\":%d,\"network_present\":%d,\"session_present\":%d,\"session_sync_present\":%d,\"session_transport_28_present\":%d,\"engine_utility_present\":%d,\"engine_utility_original_slot_bound\":%d}\n",identity_owner(&selected_bits),net!=NULL,session!=NULL,session&&get_sync(session)!=NULL,transport!=NULL,utility!=NULL,utility_bound);
        fprintf(stderr,"C6_LINUX_ACTION_DISPATCH request_enter\n");
        request_issued=true;
        request(context,&selected_bits,UINT32_C(0x2c681908),params);
        bool delivered=a.pending(context);
        fprintf(stderr,"C6_LINUX_ACTION_DISPATCH request_return pending %d\n",delivered);
        if(!delivered){void *native_network=network();if(native_network)network_update(native_network,0.0f);delivered=a.pending(context);fprintf(stderr,"C6_LINUX_ACTION_DISPATCH network_update_zero_return pending %d\n",delivered);}
        if(!delivered)result=229;
        else {
            fprintf(stderr,"C6_LINUX_ACTION_DISPATCH native_worker_enter\n");pump(*controller);sync(*controller);
            fprintf(stderr,"C6_LINUX_ACTION_DISPATCH native_worker_return executing %d pending %d\n",executing(*controller),a.pending(context));
            if(executing(*controller)||a.pending(context))result=230;
        }
    }
    if(unit){
        unit=find(unit_container,selected_id);
        bool allowed=unit&&can_start(ops,&selected_bits,UINT32_C(0x2c681908),params,false,NULL);
        fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"predicate_after\",\"can_start_same_skip_turn\":%d,\"unit_present\":%d}\n",allowed,unit!=NULL);
        int observed=c6_linux_action_state_capture(module,unit,"after_a");if(!result)result=observed;
        observed=c6_linux_action_state_capture(module,unit,"after_b");if(!result)result=observed;
        observed=c6_linux_context_save(module,context,after);if(!result)result=observed;
        observed=c6_linux_context_save(module,context,repeat);if(!result)result=observed;

    }
    struct status final=status(&a,context,game,"after");
    bool final_actions=pending_actions(),final_sessions=pending_sessions();
    fprintf(stderr,"C6_LINUX_ACTION_DISPATCH {\"stage\":\"diplomacy_after\",\"pending_actions\":%d,\"open_session_map_nonempty\":%d,\"result\":%d,\"request_issued\":%s,\"gameplay_parity\":false}\n",final_actions,final_sessions,result,request_issued?"true":"false");
    if(!result&&(final.turn!=initial.turn||final.phase!=initial.phase||final.local!=initial.local||final.pending||final.ai||final.processing||final_actions||final_sessions))result=231;
    map_delete(params);a.release(ops);a.release(list);a.release(player);a.release(game);
    return result;
}
